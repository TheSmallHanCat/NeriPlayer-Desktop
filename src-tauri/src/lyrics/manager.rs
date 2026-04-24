// 歌词管理器 — 多源瀑布获取
use crate::error::AppResult;
use crate::lyrics::parser::{self, LyricLine};
use crate::api::netease::client::NeteaseClient;
use crate::api::lrclib::LrcLibClient;

pub struct LyricsManager {
    http: reqwest::Client,
}

impl LyricsManager {
    pub fn new(http: &reqwest::Client) -> Self {
        Self { http: http.clone() }
    }

    /// 多源获取歌词：本地 sidecar -> 网易云 API -> LRCLIB fallback
    pub async fn fetch_lyrics(
        &self,
        track_title: &str,
        track_artist: &str,
        duration_secs: u64,
        audio_path: Option<&str>,
        netease_id: Option<u64>,
    ) -> AppResult<Vec<LyricLine>> {
        eprintln!("[lyrics] fetch: title={}, artist={}, dur={}s, netease_id={:?}",
            track_title, track_artist, duration_secs, netease_id);

        // 尝试本地 sidecar .lrc 文件
        if let Some(path) = audio_path {
            let lrc_path = std::path::Path::new(path).with_extension("lrc");
            if lrc_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&lrc_path) {
                    let lines = parser::parse_lrc(&content);
                    if !lines.is_empty() {
                        eprintln!("[lyrics] found local .lrc: {} lines", lines.len());
                        return Ok(lines);
                    }
                }
            }
        }

        let client = NeteaseClient::new(&self.http);

        // 确定网易云歌曲 ID：直接提供或通过搜索获取
        let resolved_id = if let Some(id) = netease_id {
            eprintln!("[lyrics] using provided netease_id={}", id);
            Some(id)
        } else {
            // 用 title + artist 搜索网易云，取最匹配的结果
            let id = self.search_netease_id(&client, track_title, track_artist).await;
            eprintln!("[lyrics] search_netease_id result: {:?}", id);
            id
        };

        // 网易云 API 取歌词（对齐 Android：YRC 优先，LRC 回退）
        if let Some(id) = resolved_id {
            match client.get_lyrics(id).await {
                Ok(lyrics_data) => {
                    eprintln!("[lyrics] netease lyrics for id={}: lrc={}, tlyric={}, yrc={}",
                        id,
                        lyrics_data.lrc.as_ref().map_or(0, |s| s.len()),
                        lyrics_data.tlyric.as_ref().map_or(0, |s| s.len()),
                        lyrics_data.yrc.as_ref().map_or(0, |s| s.len()),
                    );

                    // 翻译歌词：优先 ytlrc（YRC 翻译），回退 tlyric
                    let translation = lyrics_data.ytlrc.as_deref()
                        .or(lyrics_data.tlyric.as_deref())
                        .filter(|s| !s.is_empty());

                    // 优先 YRC（逐字歌词），对齐 Android extractPreferredNeteaseLyricContent
                    if let Some(ref yrc_str) = lyrics_data.yrc {
                        if !yrc_str.trim().is_empty() {
                            let mut lines = parser::parse_yrc(yrc_str);
                            if !lines.is_empty() {
                                if let Some(tl) = translation {
                                    parser::merge_translation(&mut lines, tl);
                                }
                                eprintln!("[lyrics] using netease YRC: {} lines, {} with words",
                                    lines.len(), lines.iter().filter(|l| !l.words.is_empty()).count());
                                return Ok(lines);
                            }
                        }
                    }

                    // 回退 LRC
                    if let Some(ref lrc_str) = lyrics_data.lrc {
                        let mut lines = parser::parse_auto(lrc_str);
                        if !lines.is_empty() {
                            if let Some(tl) = translation {
                                parser::merge_translation(&mut lines, tl);
                            }
                            eprintln!("[lyrics] using netease LRC: {} lines", lines.len());
                            return Ok(lines);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[lyrics] netease get_lyrics failed for id={}: {}", id, e);
                }
            }
        }

        // LRCLIB fallback — 精确匹配
        let lrclib = LrcLibClient::new(&self.http);
        if let Ok(Some(result)) = lrclib.get_lyrics(track_title, track_artist, duration_secs).await {
            if let Some(synced) = result.synced_lyrics {
                let lines = parser::parse_lrc(&synced);
                if !lines.is_empty() {
                    eprintln!("[lyrics] using LRCLIB exact: {} lines", lines.len());
                    return Ok(lines);
                }
            }
        }

        // LRCLIB fallback — 模糊搜索
        let query = format!("{} {}", track_title, track_artist);
        if let Ok(results) = lrclib.search(&query).await {
            for r in results {
                if let Some(synced) = r.synced_lyrics {
                    let lines = parser::parse_lrc(&synced);
                    if !lines.is_empty() {
                        eprintln!("[lyrics] using LRCLIB search: {} lines", lines.len());
                        return Ok(lines);
                    }
                }
            }
        }

        eprintln!("[lyrics] no lyrics found for: {} - {}", track_title, track_artist);
        Ok(Vec::new())
    }

    /// 通过搜索网易云获取匹配歌曲 ID
    async fn search_netease_id(
        &self,
        client: &NeteaseClient,
        title: &str,
        artist: &str,
    ) -> Option<u64> {
        let query = format!("{} {}", title, artist);
        let results = client.search(&query, 5, 0).await.ok()?;
        if results.is_empty() {
            return None;
        }

        // 优先精确匹配标题
        let title_lower = title.to_lowercase();
        for r in &results {
            if r.name.to_lowercase() == title_lower {
                return Some(r.id);
            }
        }
        // 没有精确匹配，取第一个结果
        Some(results[0].id)
    }
}
