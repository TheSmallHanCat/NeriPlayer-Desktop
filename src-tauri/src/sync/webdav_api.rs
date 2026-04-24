// WebDAV API 客户端
use reqwest::Client;
use sha2::{Sha256, Digest};
use crate::error::{AppError, AppResult};

pub struct WebDavApiClient {
    http: Client,
    server_url: String,
    username: String,
    password: String,
    base_path: String,
}

const SYNC_FILENAME: &str = "neriplayer-sync.json";

impl WebDavApiClient {
    pub fn new(http: &Client, server_url: &str, username: &str, password: &str, base_path: &str) -> Self {
        Self {
            http: http.clone(),
            server_url: server_url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            base_path: base_path.to_string(),
        }
    }

    fn remote_url(&self) -> String {
        if self.base_path.is_empty() {
            format!("{}/{}", self.server_url, SYNC_FILENAME)
        } else {
            let bp = self.base_path.trim_matches('/');
            format!("{}/{}/{}", self.server_url, bp, SYNC_FILENAME)
        }
    }

    fn sha256_fingerprint(content: &str) -> String {
        let hash = Sha256::digest(content.as_bytes());
        hex::encode(hash)
    }

    /// 验证连接（GET 请求，200/404 均视为连接成功）
    pub async fn validate_connection(&self) -> AppResult<()> {
        let url = self.remote_url();
        let resp = self.http.get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send().await?;

        let status = resp.status().as_u16();
        match status {
            200 | 404 => Ok(()),
            401 | 403 => Err(AppError::Api("WebDAV authentication failed".into())),
            _ => Err(AppError::Api(format!("WebDAV connection failed ({})", status))),
        }
    }

    /// 获取文件内容和指纹
    /// 不存在时返回 Ok(None)
    pub async fn get_file_content(&self) -> AppResult<Option<(String, String)>> {
        let url = self.remote_url();
        let resp = self.http.get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send().await?;

        let status = resp.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if status == 401 || status == 403 {
            return Err(AppError::Api("WebDAV authentication failed".into()));
        }
        if !resp.status().is_success() {
            return Err(AppError::Api(format!("WebDAV GET failed ({})", status)));
        }

        let content = resp.text().await?;
        let fingerprint = Self::sha256_fingerprint(&content);
        Ok(Some((content, fingerprint)))
    }

    /// 上传文件内容，返回 SHA-256 指纹
    pub async fn update_file_content(&self, content: &str) -> AppResult<String> {
        let url = self.remote_url();
        let resp = self.http.put(&url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(content.to_string())
            .send().await?;

        let status = resp.status().as_u16();
        if status == 401 || status == 403 {
            return Err(AppError::Api("WebDAV authentication failed".into()));
        }
        // WebDAV PUT 成功通常返回 200/201/204
        if !resp.status().is_success() {
            return Err(AppError::Api(format!("WebDAV PUT failed ({})", status)));
        }

        Ok(Self::sha256_fingerprint(content))
    }
}
