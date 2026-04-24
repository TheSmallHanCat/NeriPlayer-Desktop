use std::sync::Arc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::audio::player::PlayerEngine;
use crate::audio::queue::PlayQueue;
use crate::auth::state::AuthState;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// 全局应用状态，通过 tauri::State 注入
pub struct AppState {
    pub player: Mutex<PlayerEngine>,
    pub queue: Mutex<PlayQueue>,
    pub http: reqwest::Client,
    /// 共享 Cookie Jar — 允许外部注入持久化登录 Cookie
    pub cookie_jar: Arc<reqwest::cookie::Jar>,
    /// 三平台登录状态
    pub auth: Mutex<AuthState>,
}

impl AppState {
    pub fn new() -> Self {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        let http = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .user_agent(USER_AGENT)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            player: Mutex::new(PlayerEngine::new()),
            queue: Mutex::new(PlayQueue::new()),
            http,
            cookie_jar: jar,
            auth: Mutex::new(AuthState::default()),
        }
    }
}

/// 曲目信息（前后端共享）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub source: TrackSource,
    /// 本地文件路径或远程 URL
    pub url: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrackSource {
    Local,
    Netease,
    Bilibili,
    Youtube,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    Off,
    All,
    One,
}
