// 音频播放引擎 — 专用线程架构
// OutputStream 是 !Send，必须在创建它的线程上操作。
// 所有 Sink 操作通过 channel 发送到专用音频线程执行。

use std::io::{BufReader, Cursor};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use rodio::{Decoder, OutputStream, Sink, Source};

use crate::error::{AppError, AppResult};

/// 播放操作 recv 超时（网络音频解码可能慢）
const RECV_TIMEOUT: Duration = Duration::from_secs(30);
/// seek/query 等快操作超时
const FAST_RECV_TIMEOUT: Duration = Duration::from_secs(5);

// 音频来源——seek 时需要重建 decoder
enum AudioSource {
    Bytes(Vec<u8>),
    File(String),
}

// 音频线程命令
enum AudioCmd {
    PlayBytes {
        data: Vec<u8>,
        duration_hint_ms: u64,
        reply: mpsc::Sender<Result<u64, String>>,
    },
    PlayFile {
        path: String,
        reply: mpsc::Sender<Result<u64, String>>,
    },
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    SetSpeed(f32),
    Seek {
        position_ms: u64,
        reply: mpsc::Sender<Result<(), String>>,
    },
    QueryEmpty {
        reply: mpsc::Sender<bool>,
    },
}

pub struct PlayerEngine {
    cmd_tx: mpsc::Sender<AudioCmd>,
    thread_alive: Arc<AtomicBool>,
    pub is_playing: bool,
    pub volume: f32,
    pub speed: f32,
    pub current_path: Option<String>,
    pub duration_ms: u64,
    play_start_time: Option<Instant>,
    accumulated_ms: u64,
}

unsafe impl Send for PlayerEngine {}

impl PlayerEngine {
    pub fn new() -> Self {
        let (tx, alive) = Self::spawn_audio_thread();
        Self {
            cmd_tx: tx,
            thread_alive: alive,
            is_playing: false,
            volume: 1.0,
            speed: 1.0,
            current_path: None,
            duration_ms: 0,
            play_start_time: None,
            accumulated_ms: 0,
        }
    }

    /// 启动音频线程，返回 (命令发送端, 存活标记)
    fn spawn_audio_thread() -> (mpsc::Sender<AudioCmd>, Arc<AtomicBool>) {
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_flag = alive.clone();

        std::thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || {
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Self::audio_thread(rx);
                })) {
                    eprintln!("[audio-thread] PANIC: {:?}", e);
                }
                alive_flag.store(false, Ordering::SeqCst);
                eprintln!("[audio-thread] thread exited");
            })
            .expect("Failed to spawn audio thread");

        (tx, alive)
    }

    /// 检查并重启已死的音频线程
    fn ensure_alive(&mut self) {
        if !self.thread_alive.load(Ordering::SeqCst) {
            eprintln!("[PlayerEngine] audio thread dead, respawning...");
            let (tx, alive) = Self::spawn_audio_thread();
            self.cmd_tx = tx;
            self.thread_alive = alive;
            let _ = self.cmd_tx.send(AudioCmd::SetVolume(self.volume));
            if (self.speed - 1.0).abs() > 0.01 {
                let _ = self.cmd_tx.send(AudioCmd::SetSpeed(self.speed));
            }
        }
    }

    /// 从 source 创建 decoder，成功返回 (decoder_box, duration_ms)
    fn make_decoder(source: &AudioSource) -> Result<(Box<dyn Source<Item = i16> + Send>, u64), String> {
        match source {
            AudioSource::Bytes(data) => {
                let cursor = Cursor::new(data.clone());
                let dec = Decoder::new(cursor)
                    .map_err(|e| format!("Decode error: {}", e))?;
                let dur = dec.total_duration()
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                Ok((Box::new(dec), dur))
            }
            AudioSource::File(path) => {
                let file = std::fs::File::open(path)
                    .map_err(|e| format!("Cannot open file: {}", e))?;
                let reader = BufReader::new(file);
                let dec = Decoder::new(reader)
                    .map_err(|e| format!("Decode error: {}", e))?;
                let dur = dec.total_duration()
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                Ok((Box::new(dec), dur))
            }
        }
    }

    /// 音频线程主循环
    fn audio_thread(rx: mpsc::Receiver<AudioCmd>) {
        let mut stream: Option<OutputStream> = None;
        let mut handle: Option<rodio::OutputStreamHandle> = None;
        let mut current_sink: Option<Sink> = None;
        let mut current_volume: f32 = 1.0;
        let mut current_speed: f32 = 1.0;
        // 保留当前音频来源，用于 seek 时重建 decoder
        let mut current_source: Option<AudioSource> = None;
        let mut current_duration_ms: u64 = 0;

        macro_rules! ensure_output {
            () => {
                if handle.is_none() {
                    match OutputStream::try_default() {
                        Ok((s, h)) => {
                            stream = Some(s);
                            handle = Some(h);
                        }
                        Err(e) => {
                            eprintln!("[audio-thread] Failed to open audio output: {}", e);
                        }
                    }
                }
            };
        }

        loop {
            let cmd = match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => cmd,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    eprintln!("[audio-thread] channel disconnected, exiting");
                    break;
                }
            };

            match cmd {
                AudioCmd::PlayBytes { data, duration_hint_ms, reply } => {
                    let result = (|| -> Result<u64, String> {
                        let data_len = data.len();
                        eprintln!("[audio-thread] PlayBytes: {} bytes, hint={}ms", data_len, duration_hint_ms);

                        ensure_output!();
                        let h = handle.as_ref()
                            .ok_or_else(|| "No audio output available".to_string())?;

                        if let Some(old_sink) = current_sink.take() {
                            old_sink.stop();
                        }

                        let source = AudioSource::Bytes(data);
                        let (dec, dur) = Self::make_decoder(&source)?;
                        let duration_ms = if dur > 0 { dur } else { duration_hint_ms };

                        eprintln!("[audio-thread] decoded ok, duration={}ms", duration_ms);

                        let sink = Sink::try_new(h)
                            .map_err(|e| format!("Sink error: {}", e))?;
                        sink.set_volume(current_volume);
                        sink.set_speed(current_speed);
                        sink.append(dec);

                        current_sink = Some(sink);
                        current_source = Some(source);
                        current_duration_ms = duration_ms;
                        Ok(duration_ms)
                    })();
                    let _ = reply.send(result);
                }

                AudioCmd::PlayFile { path, reply } => {
                    let result = (|| -> Result<u64, String> {
                        ensure_output!();
                        let h = handle.as_ref()
                            .ok_or_else(|| "No audio output available".to_string())?;

                        if let Some(old_sink) = current_sink.take() {
                            old_sink.stop();
                        }

                        let source = AudioSource::File(path);
                        let (dec, dur) = Self::make_decoder(&source)?;

                        let sink = Sink::try_new(h)
                            .map_err(|e| format!("Sink error: {}", e))?;
                        sink.set_volume(current_volume);
                        sink.set_speed(current_speed);
                        sink.append(dec);

                        current_sink = Some(sink);
                        current_source = Some(source);
                        current_duration_ms = dur;
                        Ok(dur)
                    })();
                    let _ = reply.send(result);
                }

                AudioCmd::Pause => {
                    if let Some(ref sink) = current_sink {
                        sink.pause();
                    }
                }

                AudioCmd::Resume => {
                    if let Some(ref sink) = current_sink {
                        sink.play();
                    }
                }

                AudioCmd::Stop => {
                    if let Some(sink) = current_sink.take() {
                        sink.stop();
                    }
                    current_source = None;
                    current_duration_ms = 0;
                }

                AudioCmd::SetVolume(vol) => {
                    current_volume = vol;
                    if let Some(ref sink) = current_sink {
                        sink.set_volume(vol);
                    }
                }

                AudioCmd::SetSpeed(spd) => {
                    current_speed = spd;
                    if let Some(ref sink) = current_sink {
                        sink.set_speed(spd);
                    }
                }

                AudioCmd::Seek { position_ms, reply } => {
                    let result = (|| -> Result<(), String> {
                        eprintln!("[audio-thread] Seek to {}ms", position_ms);

                        // File 来源尝试 rodio 原生 seek（symphonia 对文件 seek 支持良好）
                        let is_file = matches!(current_source.as_ref(), Some(AudioSource::File(_)));
                        if is_file {
                            if let Some(ref sink) = current_sink {
                                if sink.try_seek(Duration::from_millis(position_ms)).is_ok() {
                                    eprintln!("[audio-thread] Native seek OK");
                                    return Ok(());
                                }
                            }
                        }

                        // Bytes 来源或原生 seek 失败——重建 decoder + skip samples
                        let source = current_source.as_ref()
                            .ok_or_else(|| "Nothing is playing".to_string())?;
                        let h = handle.as_ref()
                            .ok_or_else(|| "No audio output available".to_string())?;

                        let was_paused = current_sink.as_ref().map(|s| s.is_paused()).unwrap_or(false);
                        if let Some(old_sink) = current_sink.take() {
                            old_sink.stop();
                        }

                        let (dec, _) = Self::make_decoder(source)?;
                        let skip_duration = Duration::from_millis(position_ms);
                        let skipped = dec.skip_duration(skip_duration);

                        let sink = Sink::try_new(h)
                            .map_err(|e| format!("Sink error: {}", e))?;
                        sink.set_volume(current_volume);
                        sink.set_speed(current_speed);
                        sink.append(skipped);
                        if was_paused {
                            sink.pause();
                        }

                        current_sink = Some(sink);
                        eprintln!("[audio-thread] Seek via rebuild OK");
                        Ok(())
                    })();
                    let _ = reply.send(result);
                }

                AudioCmd::QueryEmpty { reply } => {
                    let empty = match &current_sink {
                        Some(sink) => sink.empty(),
                        None => true,
                    };
                    let _ = reply.send(empty);
                }
            }
        }
    }

    /// 播放本地文件
    pub fn play_file(&mut self, path: &str) -> AppResult<u64> {
        self.ensure_alive();
        let (tx, rx) = mpsc::channel();
        self.cmd_tx.send(AudioCmd::PlayFile {
            path: path.to_string(),
            reply: tx,
        }).map_err(|_| AppError::Audio("Audio thread disconnected".into()))?;

        let duration_ms = rx.recv_timeout(RECV_TIMEOUT)
            .map_err(|e| AppError::Audio(format!("Audio thread timeout: {}", e)))?
            .map_err(|e| AppError::Audio(e))?;

        self.is_playing = true;
        self.current_path = Some(path.to_string());
        self.duration_ms = duration_ms;
        self.play_start_time = Some(Instant::now());
        self.accumulated_ms = 0;

        Ok(duration_ms)
    }

    /// 播放内存中的音频数据
    pub fn play_bytes(&mut self, data: Vec<u8>, duration_hint_ms: u64) -> AppResult<u64> {
        self.ensure_alive();
        let (tx, rx) = mpsc::channel();
        self.cmd_tx.send(AudioCmd::PlayBytes {
            data,
            duration_hint_ms,
            reply: tx,
        }).map_err(|_| AppError::Audio("Audio thread disconnected".into()))?;

        let duration_ms = rx.recv_timeout(RECV_TIMEOUT)
            .map_err(|e| AppError::Audio(format!("Audio thread timeout: {}", e)))?
            .map_err(|e| AppError::Audio(e))?;

        self.is_playing = true;
        self.current_path = Some("__stream__".to_string());
        self.duration_ms = duration_ms;
        self.play_start_time = Some(Instant::now());
        self.accumulated_ms = 0;

        Ok(duration_ms)
    }

    /// 获取当前播放位置（毫秒）
    /// 考虑播放速度：wall-clock 经过 1s 但 speed=1.5 时实际播放了 1.5s
    pub fn position_ms(&self) -> u64 {
        let elapsed = match (self.is_playing, self.play_start_time) {
            (true, Some(start)) => {
                let wall_ms = start.elapsed().as_millis() as f64;
                (wall_ms * self.speed as f64) as u64
            },
            _ => 0,
        };
        let pos = self.accumulated_ms + elapsed;
        if self.duration_ms > 0 { pos.min(self.duration_ms) } else { pos }
    }

    pub fn pause(&mut self) {
        if let Some(start) = self.play_start_time.take() {
            let wall_ms = start.elapsed().as_millis() as f64;
            self.accumulated_ms += (wall_ms * self.speed as f64) as u64;
        }
        let _ = self.cmd_tx.send(AudioCmd::Pause);
        self.is_playing = false;
    }

    pub fn resume(&mut self) {
        self.play_start_time = Some(Instant::now());
        let _ = self.cmd_tx.send(AudioCmd::Resume);
        self.is_playing = true;
    }

    pub fn stop(&mut self) {
        let _ = self.cmd_tx.send(AudioCmd::Stop);
        self.is_playing = false;
        self.current_path = None;
        self.duration_ms = 0;
        self.play_start_time = None;
        self.accumulated_ms = 0;
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        let _ = self.cmd_tx.send(AudioCmd::SetVolume(self.volume));
    }

    pub fn set_speed(&mut self, spd: f32) {
        if self.is_playing {
            if let Some(start) = self.play_start_time.take() {
                let wall_ms = start.elapsed().as_millis() as f64;
                self.accumulated_ms += (wall_ms * self.speed as f64) as u64;
                self.play_start_time = Some(Instant::now());
            }
        }
        self.speed = spd.clamp(0.25, 3.0);
        let _ = self.cmd_tx.send(AudioCmd::SetSpeed(self.speed));
    }

    /// Seek 到指定位置
    pub fn seek_to(&mut self, position_ms: u64) -> AppResult<()> {
        self.ensure_alive();
        let (tx, rx) = mpsc::channel();
        self.cmd_tx.send(AudioCmd::Seek { position_ms, reply: tx })
            .map_err(|_| AppError::Audio("Audio thread disconnected".into()))?;

        rx.recv_timeout(FAST_RECV_TIMEOUT)
            .map_err(|e| AppError::Audio(format!("Seek timeout: {}", e)))?
            .map_err(|e| AppError::Audio(e))?;

        self.accumulated_ms = position_ms;
        if self.is_playing {
            self.play_start_time = Some(Instant::now());
        } else {
            self.play_start_time = None;
        }
        Ok(())
    }

    /// 检测播放是否自然结束
    pub fn is_finished(&self) -> bool {
        let elapsed_ms = match self.play_start_time {
            Some(start) => start.elapsed().as_millis() as u64,
            None => return false,
        };
        if elapsed_ms < 3000 {
            return false;
        }

        let (tx, rx) = mpsc::channel();
        if self.cmd_tx.send(AudioCmd::QueryEmpty { reply: tx }).is_err() {
            return true;
        }
        let sink_empty = rx.recv_timeout(Duration::from_millis(200)).unwrap_or(true);

        if !sink_empty {
            return false;
        }

        if self.duration_ms > 0 {
            let pos = self.position_ms();
            let threshold = self.duration_ms.saturating_sub(5000);
            if pos < threshold {
                return false;
            }
        }

        true
    }

    /// 标记播放结束
    pub fn mark_ended(&mut self) {
        if let Some(start) = self.play_start_time.take() {
            let wall_ms = start.elapsed().as_millis() as f64;
            self.accumulated_ms += (wall_ms * self.speed as f64) as u64;
        }
        self.is_playing = false;
    }
}
