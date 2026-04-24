// 网易云音乐 API 客户端
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use super::crypto;

const BASE_URL: &str = "https://music.163.com";

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub struct NeteaseClient {
    http: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseSearchResult {
    pub id: u64,
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseSongUrl {
    pub url: Option<String>,
    pub br: u64,
    pub size: u64,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseLyrics {
    pub lrc: Option<String>,
    pub tlyric: Option<String>,
    pub yrc: Option<String>,
    pub ytlrc: Option<String>,
}

impl NeteaseClient {
    pub fn new(http: &Client) -> Self {
        Self { http: http.clone() }
    }

    /// WEAPI POST 请求
    async fn weapi_post(&self, url: &str, params: &Value) -> AppResult<Value> {
        let json_str = serde_json::to_string(params)?;
        let (encrypted_params, enc_sec_key) = crypto::weapi_encrypt(&json_str);

        let resp = self.http
            .post(url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", "https://music.163.com")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[("params", &encrypted_params), ("encSecKey", &enc_sec_key)])
            .send()
            .await?;

        let body: Value = resp.json().await?;
        Ok(body)
    }

    /// 搜索歌曲
    pub async fn search(&self, keyword: &str, limit: u32, offset: u32) -> AppResult<Vec<NeteaseSearchResult>> {
        let params = json!({
            "s": keyword,
            "type": "1",
            "limit": limit.to_string(),
            "offset": offset.to_string(),
            "total": "true"
        });

        let body = self.weapi_post(
            &format!("{}/weapi/cloudsearch/get/web", BASE_URL),
            &params,
        ).await?;

        let songs = body["result"]["songs"].as_array()
            .ok_or_else(|| AppError::Api("No search results".into()))?;

        let results = songs.iter().filter_map(|s| {
            Some(NeteaseSearchResult {
                id: s["id"].as_u64()?,
                name: s["name"].as_str()?.to_string(),
                artists: s["ar"].as_array()?
                    .iter()
                    .filter_map(|a| a["name"].as_str().map(String::from))
                    .collect(),
                album: s["al"]["name"].as_str().unwrap_or("").to_string(),
                duration_ms: s["dt"].as_u64().unwrap_or(0),
                cover_url: s["al"]["picUrl"].as_str().map(String::from),
            })
        }).collect();

        Ok(results)
    }

    /// 获取歌曲播放 URL（WEAPI，稳定可靠）
    pub async fn get_song_url(&self, song_id: u64, quality: &str) -> AppResult<NeteaseSongUrl> {
        let level = match quality {
            "standard" => "standard",
            "high" | "higher" => "higher",
            "exhigh" => "exhigh",
            "lossless" => "lossless",
            "hires" => "hires",
            "jyeffect" => "jyeffect",
            "sky" => "sky",
            "jymaster" => "jymaster",
            _ => "exhigh",
        };

        let params = json!({
            "ids": format!("[{}]", song_id),
            "level": level,
            "encodeType": "flac",
            "csrf_token": ""
        });

        eprintln!("[netease] get_song_url: id={}, level={}", song_id, level);

        let body = self.weapi_post(
            &format!("{}/weapi/song/enhance/player/url/v1", BASE_URL),
            &params,
        ).await?;

        eprintln!("[netease] song url response code: {:?}", body["code"]);

        let data = body["data"].as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| {
                eprintln!("[netease] No song URL data in response: {}",
                    serde_json::to_string(&body).unwrap_or_default().chars().take(500).collect::<String>());
                AppError::Api("No song URL data".into())
            })?;

        let url = data["url"].as_str().map(String::from);
        eprintln!("[netease] song url result: url={}, br={}",
            url.as_deref().unwrap_or("null"), data["br"].as_u64().unwrap_or(0));

        Ok(NeteaseSongUrl {
            url,
            br: data["br"].as_u64().unwrap_or(0),
            size: data["size"].as_u64().unwrap_or(0),
            r#type: data["type"].as_str().unwrap_or("mp3").to_string(),
        })
    }

    /// 获取歌词（plain API，无需加密，最可靠）
    pub async fn get_lyrics(&self, song_id: u64) -> AppResult<NeteaseLyrics> {
        // 使用 v1 端点获取逐字歌词支持
        let url = format!("{}/api/song/lyric/v1?id={}&cp=false&lv=0&tv=0&rv=0&kv=0&yv=0&ytv=0&yrv=0",
            BASE_URL, song_id);

        eprintln!("[netease] get_lyrics: id={}", song_id);

        let resp = self.http
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", "https://music.163.com")
            .send()
            .await?;

        let body: Value = resp.json().await
            .map_err(|e| {
                eprintln!("[netease] lyrics JSON parse failed: {}", e);
                AppError::Api(format!("Lyrics parse error: {}", e))
            })?;

        let code = body["code"].as_i64().unwrap_or(-1);
        eprintln!("[netease] lyrics response code={}, has_lrc={}, has_tlyric={}, has_yrc={}",
            code,
            body["lrc"]["lyric"].is_string(),
            body["tlyric"]["lyric"].is_string(),
            body["yrc"]["lyric"].is_string(),
        );

        if code != 200 {
            return Err(AppError::Api(format!("Lyrics API code: {}", code)));
        }

        Ok(NeteaseLyrics {
            lrc: body["lrc"]["lyric"].as_str().map(String::from),
            tlyric: body["tlyric"]["lyric"].as_str().map(String::from),
            yrc: body["yrc"]["lyric"].as_str().map(String::from),
            ytlrc: body["ytlrc"]["lyric"].as_str().map(String::from),
        })
    }

    /// 获取歌曲详情
    pub async fn get_song_detail(&self, song_ids: &[u64]) -> AppResult<Value> {
        let c: Vec<Value> = song_ids.iter()
            .map(|id| json!({"id": id}))
            .collect();
        let params = json!({
            "c": serde_json::to_string(&c).unwrap_or_default(),
            "ids": serde_json::to_string(&song_ids).unwrap_or_default()
        });

        self.weapi_post(&format!("{}/weapi/v3/song/detail", BASE_URL), &params).await
    }

    /// 获取歌单详情
    pub async fn get_playlist(&self, playlist_id: u64) -> AppResult<Value> {
        let params = json!({
            "id": playlist_id.to_string(),
            "n": 100000,
            "s": 8
        });

        self.weapi_post(
            &format!("{}/weapi/v3/playlist/detail", BASE_URL),
            &params,
        ).await
    }

    // ===== 需要登录的 API =====

    /// 获取当前登录用户信息
    pub async fn get_user_account(&self) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/w/nuser/account/get", BASE_URL),
            &json!({}),
        ).await
    }

    /// 获取用户歌单列表
    pub async fn get_user_playlists(&self, uid: u64, limit: u32, offset: u32) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/user/playlist", BASE_URL),
            &json!({
                "uid": uid.to_string(),
                "offset": offset.to_string(),
                "limit": limit.to_string(),
                "includeVideo": "true"
            }),
        ).await
    }

    /// 个性化推荐歌单（需登录）
    pub async fn get_recommended_playlists(&self, limit: u32) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/personalized/playlist", BASE_URL),
            &json!({ "limit": limit.to_string() }),
        ).await
    }

    /// 每日推荐歌曲（需登录）
    pub async fn get_recommended_songs(&self) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/v3/discovery/recommend/songs", BASE_URL),
            &json!({}),
        ).await
    }

    /// 精品歌单（按分类）
    pub async fn get_high_quality_playlists(&self, cat: &str, limit: u32) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/playlist/highquality/list", BASE_URL),
            &json!({
                "cat": cat,
                "limit": limit,
                "lasttime": 0,
                "total": true
            }),
        ).await
    }

    /// 用户喜欢的歌曲 ID 列表
    pub async fn get_liked_song_ids(&self, uid: u64) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/song/like/get", BASE_URL),
            &json!({ "uid": uid.to_string() }),
        ).await
    }

    /// 喜欢/取消喜欢歌曲
    pub async fn like_song(&self, song_id: u64, like: bool) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/radio/like", BASE_URL),
            &json!({
                "trackId": song_id.to_string(),
                "like": like.to_string(),
                "alg": "itembased",
                "time": "3"
            }),
        ).await
    }

    /// 获取歌曲下载 URL（WEAPI）
    pub async fn get_song_download_url(&self, song_id: u64, quality: &str) -> AppResult<NeteaseSongUrl> {
        let br = match quality {
            "standard" => 128000,
            "high" | "higher" => 192000,
            "exhigh" => 320000,
            "lossless" => 999000,
            "hires" => 1999000,
            _ => 320000,
        };

        let params = json!({
            "id": song_id.to_string(),
            "br": br.to_string(),
            "csrf_token": ""
        });

        let body = self.weapi_post(
            &format!("{}/weapi/song/enhance/download/url", BASE_URL),
            &params,
        ).await?;
        let data = &body["data"];

        Ok(NeteaseSongUrl {
            url: data["url"].as_str().map(String::from),
            br: data["br"].as_u64().unwrap_or(0),
            size: data["size"].as_u64().unwrap_or(0),
            r#type: data["type"].as_str().unwrap_or("mp3").to_string(),
        })
    }

    /// 获取专辑详情
    pub async fn get_album_detail(&self, album_id: u64) -> AppResult<Value> {
        let resp = self.http
            .get(format!("{}/api/v1/album/{}", BASE_URL, album_id))
            .header("User-Agent", USER_AGENT)
            .header("Referer", "https://music.163.com")
            .send()
            .await?;
        let body: Value = resp.json().await?;
        Ok(body)
    }

    /// 获取精品歌单分类标签
    pub async fn get_high_quality_tags(&self) -> AppResult<Value> {
        self.weapi_post(
            &format!("{}/weapi/playlist/highquality/tags", BASE_URL),
            &json!({}),
        ).await
    }

    /// 获取用户收藏的专辑列表
    pub async fn get_user_stared_albums(&self, offset: u32, limit: u32) -> AppResult<Value> {
        let params = json!({
            "offset": offset.to_string(),
            "limit": limit.to_string(),
            "total": "true",
            "csrf_token": ""
        });
        self.weapi_post(
            &format!("{}/weapi/album/sublist", BASE_URL),
            &params,
        ).await
    }
}
