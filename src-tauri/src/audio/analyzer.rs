use serde::Serialize;

/// 音频电平分析数据，推送到前端用于可视化
#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelData {
    pub level: f32,
    pub beat_impulse: f32,
}

// TODO: 实际 PCM 采样分析需要 rodio Source 包装器
// 当前阶段先用占位实现，后续接入真实音频数据
pub struct AudioAnalyzer {
    prev_level: f32,
}

impl AudioAnalyzer {
    pub fn new() -> Self {
        Self { prev_level: 0.0 }
    }

    /// 计算 RMS 电平和节拍冲击
    pub fn analyze_frame(&mut self, samples: &[f32]) -> AudioLevelData {
        if samples.is_empty() {
            return AudioLevelData { level: 0.0, beat_impulse: 0.0 };
        }

        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        let beat = (rms - self.prev_level).max(0.0);
        self.prev_level = rms;

        AudioLevelData {
            level: rms.clamp(0.0, 1.0),
            beat_impulse: beat.clamp(0.0, 1.0),
        }
    }
}
