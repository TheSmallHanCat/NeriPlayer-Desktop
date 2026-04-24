// 三平台登录状态数据结构
use serde::{Deserialize, Serialize};

/// 全局登录状态
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthState {
    pub netease: Option<NeteaseAuth>,
    pub bilibili: Option<BiliAuth>,
    pub youtube: Option<YouTubeAuth>,
}

/// 网易云登录凭证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseAuth {
    pub cookies: Vec<CookieEntry>,
    pub user_id: Option<u64>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}

/// B站登录凭证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiliAuth {
    pub cookies: Vec<CookieEntry>,
    pub mid: Option<u64>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}

/// YouTube Music 登录凭证
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeAuth {
    pub cookies: Vec<CookieEntry>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}

/// 持久化的单条 Cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: String,
}

impl NeteaseAuth {
    /// 检查是否有有效的 MUSIC_U Cookie
    pub fn has_login(&self) -> bool {
        self.cookies.iter().any(|c| c.name == "MUSIC_U" && !c.value.is_empty())
    }
}

impl BiliAuth {
    /// 检查是否有有效的 SESSDATA Cookie
    pub fn has_login(&self) -> bool {
        self.cookies.iter().any(|c| c.name == "SESSDATA" && !c.value.is_empty())
    }
}

impl YouTubeAuth {
    /// 检查是否有 SAPISID 或 __Secure-3PAPISID
    pub fn has_login(&self) -> bool {
        self.cookies.iter().any(|c| {
            (c.name == "SAPISID" || c.name == "__Secure-3PAPISID") && !c.value.is_empty()
        })
    }

    /// 获取 SAPISID 值（优先 SAPISID，fallback __Secure-3PAPISID）
    pub fn get_sapisid(&self) -> Option<&str> {
        self.cookies.iter()
            .find(|c| c.name == "SAPISID")
            .or_else(|| self.cookies.iter().find(|c| c.name == "__Secure-3PAPISID"))
            .map(|c| c.value.as_str())
    }
}

/// 前端友好的登录状态摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    pub platform: String,
    pub logged_in: bool,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
}

/// 三平台登录状态聚合响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub netease: AuthInfo,
    pub bilibili: AuthInfo,
    pub youtube: AuthInfo,
}

impl AuthState {
    /// 生成前端状态摘要
    pub fn to_status_response(&self) -> AuthStatusResponse {
        AuthStatusResponse {
            netease: match &self.netease {
                Some(a) => AuthInfo {
                    platform: "netease".into(),
                    logged_in: a.has_login(),
                    nickname: a.nickname.clone(),
                    avatar_url: a.avatar_url.clone(),
                },
                None => AuthInfo {
                    platform: "netease".into(),
                    logged_in: false,
                    nickname: None,
                    avatar_url: None,
                },
            },
            bilibili: match &self.bilibili {
                Some(a) => AuthInfo {
                    platform: "bilibili".into(),
                    logged_in: a.has_login(),
                    nickname: a.nickname.clone(),
                    avatar_url: a.avatar_url.clone(),
                },
                None => AuthInfo {
                    platform: "bilibili".into(),
                    logged_in: false,
                    nickname: None,
                    avatar_url: None,
                },
            },
            youtube: match &self.youtube {
                Some(a) => AuthInfo {
                    platform: "youtube".into(),
                    logged_in: a.has_login(),
                    nickname: a.nickname.clone(),
                    avatar_url: a.avatar_url.clone(),
                },
                None => AuthInfo {
                    platform: "youtube".into(),
                    logged_in: false,
                    nickname: None,
                    avatar_url: None,
                },
            },
        }
    }
}
