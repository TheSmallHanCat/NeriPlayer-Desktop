use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadedTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
    pub source: String,
    pub file_path: String,
    pub file_size: u64,
    pub downloaded_at: u64,
}

/// 清理文件名中的非法字符
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// 根据 Content-Type 推断文件扩展名
fn ext_from_content_type(content_type: &str) -> &str {
    if content_type.contains("mp4") || content_type.contains("m4a") || content_type.contains("aac") {
        "m4a"
    } else if content_type.contains("ogg") || content_type.contains("opus") {
        "ogg"
    } else if content_type.contains("webm") {
        "webm"
    } else if content_type.contains("flac") {
        "flac"
    } else if content_type.contains("wav") {
        "wav"
    } else {
        "mp3"
    }
}

/// 获取下载目录，不存在则创建
fn downloads_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()
        .map_err(|e| AppError::Other(e.to_string()))?
        .join("downloads");
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| AppError::Io(e))?;
    }
    Ok(dir)
}

/// manifest.json 路径
fn manifest_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(downloads_dir(app)?.join("manifest.json"))
}

/// 读取 manifest
fn read_manifest(app: &AppHandle) -> AppResult<Vec<DownloadedTrack>> {
    let path = manifest_path(app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = std::fs::read_to_string(&path)?;
    let tracks: Vec<DownloadedTrack> = serde_json::from_str(&data)
        .unwrap_or_default();
    Ok(tracks)
}

/// 写入 manifest
fn write_manifest(app: &AppHandle, tracks: &[DownloadedTrack]) -> AppResult<()> {
    let path = manifest_path(app)?;
    let json = serde_json::to_string_pretty(tracks)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// 下载音频文件并保存到本地
#[tauri::command]
pub async fn download_track(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    track_id: String,
    title: String,
    artist: String,
    album: String,
    duration_ms: u64,
    cover_url: Option<String>,
    source: String,
) -> AppResult<DownloadedTrack> {
    // 发送开始事件
    let _ = app.emit("download-progress", serde_json::json!({
        "trackId": &track_id,
        "status": "start",
    }));

    // 检查是否已下载
    let existing = read_manifest(&app)?;
    if existing.iter().any(|t| t.id == track_id) {
        let _ = app.emit("download-progress", serde_json::json!({
            "trackId": &track_id,
            "status": "already_exists",
        }));
        return Err(AppError::Other("Track already downloaded".into()));
    }

    // 根据 URL 域名动态设置 Referer（复用 player_cmd 逻辑）
    let referer = if url.contains("bilibili.com") || url.contains("bilivideo.") {
        "https://www.bilibili.com"
    } else if url.contains("youtube.com") || url.contains("googlevideo.com") {
        "https://music.youtube.com"
    } else {
        "https://music.163.com"
    };

    let resp = state.http.get(&url)
        .header("Referer", referer)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .send().await
        .map_err(|e| {
            let _ = app.emit("download-progress", serde_json::json!({
                "trackId": &track_id,
                "status": "error",
                "message": e.to_string(),
            }));
            AppError::Network(e)
        })?;

    if !resp.status().is_success() {
        let msg = format!("HTTP {}", resp.status());
        let _ = app.emit("download-progress", serde_json::json!({
            "trackId": &track_id,
            "status": "error",
            "message": &msg,
        }));
        return Err(AppError::Api(msg));
    }

    // 从 Content-Type 推断扩展名
    let content_type = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("audio/mpeg")
        .to_string();
    let ext = ext_from_content_type(&content_type);

    let bytes = resp.bytes().await
        .map_err(|e| {
            let _ = app.emit("download-progress", serde_json::json!({
                "trackId": &track_id,
                "status": "error",
                "message": e.to_string(),
            }));
            AppError::Network(e)
        })?;

    if bytes.is_empty() {
        let _ = app.emit("download-progress", serde_json::json!({
            "trackId": &track_id,
            "status": "error",
            "message": "Empty audio data",
        }));
        return Err(AppError::Audio("Empty audio data received".into()));
    }

    // 构造文件名：{artist} - {title}.{ext}
    let filename = if artist.is_empty() {
        format!("{}.{}", sanitize_filename(&title), ext)
    } else {
        format!("{} - {}.{}", sanitize_filename(&artist), sanitize_filename(&title), ext)
    };

    let dir = downloads_dir(&app)?;
    let file_path = dir.join(&filename);
    let file_size = bytes.len() as u64;

    // 写入文件
    std::fs::write(&file_path, &bytes)?;

    // 构造记录
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let track = DownloadedTrack {
        id: track_id.clone(),
        title,
        artist,
        album,
        duration_ms,
        cover_url,
        source,
        file_path: file_path.to_string_lossy().to_string(),
        file_size,
        downloaded_at: now,
    };

    // 追加到 manifest
    let mut manifest = read_manifest(&app)?;
    manifest.push(track.clone());
    write_manifest(&app, &manifest)?;

    // 发送完成事件
    let _ = app.emit("download-progress", serde_json::json!({
        "trackId": &track_id,
        "status": "complete",
        "fileSize": file_size,
    }));

    Ok(track)
}

/// 列出所有已下载的曲目
#[tauri::command]
pub async fn list_downloads(app: AppHandle) -> AppResult<Vec<DownloadedTrack>> {
    read_manifest(&app)
}

/// 删除已下载的曲目（文件 + manifest 记录）
#[tauri::command]
pub async fn delete_download(app: AppHandle, track_id: String) -> AppResult<()> {
    let mut manifest = read_manifest(&app)?;

    // 查找并移除
    let idx = manifest.iter().position(|t| t.id == track_id);
    if let Some(i) = idx {
        let track = manifest.remove(i);
        // 删除磁盘文件（忽略错误，文件可能已被手动删除）
        let _ = std::fs::remove_file(&track.file_path);
        write_manifest(&app, &manifest)?;
    } else {
        return Err(AppError::NotFound("Download not found".into()));
    }

    Ok(())
}
