// YouTube SAPISIDHASH 认证头计算
// 算法: SHA1(timestamp + " " + SAPISID + " " + origin)
// 头格式: SAPISIDHASH {timestamp}_{sha1hex}

use sha1::{Sha1, Digest};

/// 计算 SAPISIDHASH Authorization 头
pub fn compute_sapisidhash(sapisid: &str, origin: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp();
    let input = format!("{} {} {}", timestamp, sapisid, origin);
    let hash = Sha1::digest(input.as_bytes());
    let hex = hex::encode(hash);
    format!("SAPISIDHASH {}_{}", timestamp, hex)
}

/// 构建完整的 YouTube 认证请求头集合
pub fn build_youtube_auth_headers(
    sapisid: &str,
    cookie_header: &str,
) -> Vec<(&'static str, String)> {
    let origin = "https://music.youtube.com";
    let auth = compute_sapisidhash(sapisid, origin);

    vec![
        ("Authorization", auth),
        ("X-Goog-AuthUser", "0".to_string()),
        ("Cookie", cookie_header.to_string()),
        ("Origin", origin.to_string()),
        ("X-Origin", origin.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sapisidhash_format() {
        let result = compute_sapisidhash("test_sapisid", "https://music.youtube.com");
        assert!(result.starts_with("SAPISIDHASH "));
        let parts: Vec<&str> = result["SAPISIDHASH ".len()..].split('_').collect();
        assert_eq!(parts.len(), 2);
        // timestamp 部分是数字
        assert!(parts[0].parse::<i64>().is_ok());
        // hash 部分是 40 字符 hex
        assert_eq!(parts[1].len(), 40);
    }
}
