// B站 Wbi 签名算法
use md5::{Md5, Digest};
use std::collections::BTreeMap;

// 64 元素混淆索引表
const MIXIN_INDEX: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35,
    27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13,
    37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 62, 6, 63, 57, 20, 34, 52, 59, 11, 36, 44,
];

/// 从 img_key + sub_key 生成 mixin_key
pub fn get_mixin_key(img_key: &str, sub_key: &str) -> String {
    let raw = format!("{}{}", img_key, sub_key);
    let chars: Vec<char> = raw.chars().collect();
    MIXIN_INDEX.iter()
        .filter_map(|&i| chars.get(i).copied())
        .take(32)
        .collect()
}

/// 对请求参数进行 Wbi 签名
/// 返回签名后的完整 query string（包含 w_rid 和 wts）
pub fn sign_params(params: &mut BTreeMap<String, String>, mixin_key: &str) {
    let wts = chrono::Utc::now().timestamp().to_string();
    params.insert("wts".to_string(), wts);

    // 过滤特殊字符
    let query: String = params.iter()
        .map(|(k, v)| {
            let clean_v: String = v.chars()
                .filter(|c| !"!'()*".contains(*c))
                .collect();
            format!("{}={}", urlencoding::encode(k), urlencoding::encode(&clean_v))
        })
        .collect::<Vec<_>>()
        .join("&");

    let to_hash = format!("{}{}", query, mixin_key);
    let w_rid = format!("{:x}", Md5::digest(to_hash.as_bytes()));
    params.insert("w_rid".to_string(), w_rid);
}
