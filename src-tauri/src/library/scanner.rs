// 本地音乐文件扫描
use std::path::Path;
use walkdir::WalkDir;
use crate::state::TrackInfo;
use crate::state::TrackSource;
use crate::error::AppResult;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "wav", "m4a", "aac", "opus", "wma",
];

/// 扫描目录下的所有音频文件，读取元数据
pub fn scan_directory(dir: &str) -> AppResult<Vec<TrackInfo>> {
    let mut tracks = Vec::new();

    for entry in WalkDir::new(dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() { continue; }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !AUDIO_EXTENSIONS.contains(&ext.as_str()) { continue; }

        match read_track_info(path) {
            Ok(track) => tracks.push(track),
            Err(e) => log::warn!("Skip {}: {}", path.display(), e),
        }
    }

    Ok(tracks)
}

fn read_track_info(path: &Path) -> AppResult<TrackInfo> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    let tagged = Probe::open(path)
        .map_err(|e| crate::error::AppError::Metadata(e.to_string()))?
        .read()
        .map_err(|e| crate::error::AppError::Metadata(e.to_string()))?;
    let properties = tagged.properties();
    let duration_ms = properties.duration().as_millis() as u64;

    // 尝试读取标签
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let title = tag.and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });
    let artist = tag.and_then(|t| t.artist().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let album = tag.and_then(|t| t.album().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown Album".to_string());

    Ok(TrackInfo {
        id: format!("local:{}", path.display()),
        title,
        artist,
        album,
        duration_ms,
        source: TrackSource::Local,
        url: path.to_string_lossy().to_string(),
        cover_url: None,
    })
}
