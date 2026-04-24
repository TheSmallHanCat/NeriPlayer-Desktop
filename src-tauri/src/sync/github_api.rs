// GitHub Contents API 客户端
use reqwest::Client;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use crate::error::{AppError, AppResult};

pub struct GitHubApiClient {
    http: Client,
    token: String,
}

impl GitHubApiClient {
    pub fn new(http: &Client, token: &str) -> Self {
        Self {
            http: http.clone(),
            token: token.to_string(),
        }
    }

    #[allow(dead_code)]
    fn auth_headers(&self) -> Vec<(&str, String)> {
        vec![
            ("Authorization", format!("Bearer {}", self.token)),
            ("Accept", "application/vnd.github+json".into()),
            ("X-GitHub-Api-Version", "2022-11-28".into()),
        ]
    }

    /// 验证 token，返回用户名
    pub async fn validate_token(&self) -> AppResult<String> {
        let resp = self.http.get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send().await?;

        if resp.status().as_u16() == 401 {
            return Err(AppError::Api("GitHub token expired or invalid".into()));
        }
        let body: serde_json::Value = resp.json().await?;
        body["login"].as_str()
            .map(String::from)
            .ok_or_else(|| AppError::Api("Failed to get GitHub username".into()))
    }

    /// 创建私有仓库
    pub async fn create_repository(&self, repo_name: &str) -> AppResult<()> {
        let body = serde_json::json!({
            "name": repo_name,
            "private": true,
            "auto_init": true,
            "description": "NeriPlayer backup data"
        });

        let resp = self.http.post("https://api.github.com/user/repos")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&body)
            .send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            // 422 = 仓库已存在，视为成功
            if status == 422 && text.contains("already exists") {
                return Ok(());
            }
            return Err(AppError::Api(format!("Failed to create repo ({}): {}", status, text)));
        }
        Ok(())
    }

    /// 检查仓库是否存在，返回默认分支名
    pub async fn check_repository(&self, owner: &str, repo: &str) -> AppResult<String> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send().await?;

        if resp.status().as_u16() == 404 {
            return Err(AppError::NotFound("Repository not found".into()));
        }
        let body: serde_json::Value = resp.json().await?;
        Ok(body["default_branch"].as_str().unwrap_or("main").to_string())
    }

    /// 获取文件内容和 SHA
    /// 文件不存在时返回 Ok(None)
    pub async fn get_file_content(&self, owner: &str, repo: &str, path: &str) -> AppResult<Option<(String, String)>> {
        let url = format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, path);
        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .send().await?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if resp.status().as_u16() == 401 {
            return Err(AppError::Api("GitHub token expired".into()));
        }

        let body: serde_json::Value = resp.json().await?;
        let content_b64 = body["content"].as_str().unwrap_or("");
        let sha = body["sha"].as_str().unwrap_or("").to_string();

        // GitHub 返回的 base64 包含换行符，需要移除
        let cleaned: String = content_b64.chars().filter(|c| !c.is_whitespace()).collect();
        let decoded = BASE64.decode(&cleaned)
            .map_err(|e| AppError::Other(format!("Base64 decode error: {}", e)))?;
        let content = String::from_utf8(decoded)
            .map_err(|e| AppError::Other(format!("UTF-8 decode error: {}", e)))?;

        Ok(Some((content, sha)))
    }

    /// 创建或更新文件
    /// sha 为空时表示新建；非空时表示更新
    pub async fn update_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        content: &str,
        sha: &str,
        message: &str,
    ) -> AppResult<String> {
        let url = format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, path);
        let encoded = BASE64.encode(content.as_bytes());

        let mut body = serde_json::json!({
            "message": message,
            "content": encoded,
        });

        if !sha.is_empty() {
            body["sha"] = serde_json::Value::String(sha.to_string());
        }

        let resp = self.http.put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&body)
            .send().await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Api(format!("Failed to update file ({}): {}", status, text)));
        }

        let result: serde_json::Value = resp.json().await?;
        Ok(result["content"]["sha"].as_str().unwrap_or("").to_string())
    }
}
