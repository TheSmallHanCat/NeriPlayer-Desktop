// Cookie 持久化 — tauri-plugin-store 读写 + reqwest::Jar 注入
use std::sync::Arc;
use reqwest::cookie::Jar;
use reqwest::Url;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use super::state::{AuthState, CookieEntry};

const STORE_FILE: &str = "auth.json";
const STORE_KEY: &str = "auth_state";

/// 将 AuthState 持久化到 tauri-plugin-store
pub fn save_auth(app: &AppHandle, auth: &AuthState) {
    if let Ok(store) = app.store(STORE_FILE) {
        let value = serde_json::to_value(auth).unwrap_or_default();
        store.set(STORE_KEY, value);
        let _ = store.save();
    }
}

/// 启动时从 store 恢复 AuthState
pub fn load_auth(app: &AppHandle) -> AuthState {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return AuthState::default(),
    };

    match store.get(STORE_KEY) {
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_default(),
        None => AuthState::default(),
    }
}

/// 将所有已登录平台的 Cookie 注入 Jar
pub fn inject_all(jar: &Arc<Jar>, auth: &AuthState) {
    if let Some(ref netease) = auth.netease {
        inject_cookies(jar, &netease.cookies);
    }
    if let Some(ref bilibili) = auth.bilibili {
        inject_cookies(jar, &bilibili.cookies);
    }
    if let Some(ref youtube) = auth.youtube {
        inject_cookies(jar, &youtube.cookies);
    }
}

/// 将 Cookie 列表注入 Jar（包含 Domain 属性，确保子域名可用）
pub fn inject_cookies(jar: &Arc<Jar>, entries: &[CookieEntry]) {
    for entry in entries {
        let url = domain_to_url(&entry.domain);
        if let Ok(url) = url.parse::<Url>() {
            // 必须设置 Domain 属性，否则 reqwest 按精确域名匹配，子域名 API 拿不到 cookie
            jar.add_cookie_str(
                &format!("{}={}; Domain={}; Path=/", entry.name, entry.value, entry.domain),
                &url,
            );
        }
    }
}

/// 登出时过期指定平台的 Cookie
pub fn expire_platform_cookies(jar: &Arc<Jar>, auth: &AuthState, platform: &str) {
    let entries = match platform {
        "netease" => auth.netease.as_ref().map(|a| &a.cookies),
        "bilibili" => auth.bilibili.as_ref().map(|a| &a.cookies),
        "youtube" => auth.youtube.as_ref().map(|a| &a.cookies),
        _ => None,
    };

    if let Some(entries) = entries {
        for entry in entries {
            let url = domain_to_url(&entry.domain);
            if let Ok(url) = url.parse::<Url>() {
                // 必须带 Domain + Path 属性，与注入时一致，才能正确覆盖并过期
                jar.add_cookie_str(
                    &format!("{}=deleted; Domain={}; Path=/; Max-Age=0", entry.name, entry.domain),
                    &url,
                );
            }
        }
    }
}

/// 解析 document.cookie 字符串为 CookieEntry 列表
pub fn parse_document_cookies(cookie_str: &str, domain: &str) -> Vec<CookieEntry> {
    cookie_str
        .split(';')
        .filter_map(|pair| {
            let pair = pair.trim();
            let (name, value) = pair.split_once('=')?;
            let name = name.trim();
            let value = value.trim();
            if name.is_empty() {
                return None;
            }
            Some(CookieEntry {
                name: name.to_string(),
                value: value.to_string(),
                domain: domain.to_string(),
            })
        })
        .collect()
}

/// 解析用户粘贴的原始 Cookie 文本（对齐 Android RawCookieTextParser）
/// 支持分号、换行、回车分隔
pub fn parse_raw_cookie_text(raw: &str, platform: &str) -> Vec<CookieEntry> {
    let domain = match platform {
        "netease" => "music.163.com",
        "bilibili" => ".bilibili.com",
        "youtube" => ".youtube.com",
        _ => "unknown",
    };

    let mut entries = Vec::new();
    // 按 ; \r \n 分割
    for segment in raw.split(|c: char| c == ';' || c == '\r' || c == '\n') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        if let Some((name, value)) = segment.split_once('=') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();
            if !name.is_empty() {
                entries.push(CookieEntry { name, value, domain: domain.to_string() });
            }
        }
    }

    // YouTube 需要额外为 google.com 注入部分 Cookie
    if platform == "youtube" {
        let google_entries: Vec<CookieEntry> = entries.iter()
            .filter(|c| matches!(c.name.as_str(), "SID" | "HSID" | "SSID" | "APISID" | "SAPISID" | "LSID" | "SIDCC"))
            .map(|c| CookieEntry {
                name: c.name.clone(),
                value: c.value.clone(),
                domain: ".google.com".into(),
            })
            .collect();
        entries.extend(google_entries);
    }

    entries
}

/// 域名转 URL（用于 Jar.add_cookie_str）
fn domain_to_url(domain: &str) -> String {
    let d = domain.trim_start_matches('.');
    format!("https://{}", d)
}
