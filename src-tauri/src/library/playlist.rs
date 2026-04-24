// 播放列表管理（JSON 持久化）
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::state::TrackInfo;
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub tracks: Vec<TrackInfo>,
    pub modified_at: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PlaylistStore {
    pub playlists: Vec<Playlist>,
    next_id: i64,
}

impl PlaylistStore {
    pub fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn create(&mut self, name: String) -> &Playlist {
        // 普通歌单 ID 从 1 开始递增
        if self.next_id < 1 { self.next_id = 1; }
        let id = self.next_id;
        self.next_id += 1;
        self.playlists.push(Playlist {
            id,
            name,
            tracks: Vec::new(),
            modified_at: chrono::Utc::now().timestamp_millis() as u64,
        });
        self.playlists.last().unwrap()
    }

    pub fn delete(&mut self, id: i64) -> bool {
        let len = self.playlists.len();
        self.playlists.retain(|p| p.id != id);
        self.playlists.len() < len
    }

    /// 确保 next_id 大于所有正数歌单 ID
    pub fn fix_next_id(&mut self) {
        let max = self.playlists.iter().map(|p| p.id).filter(|&id| id > 0).max().unwrap_or(0);
        if self.next_id <= max {
            self.next_id = max + 1;
        }
        if self.next_id < 1 { self.next_id = 1; }
    }
}
