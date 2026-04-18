// Sakugabooru API 客户端 - 搜索和下载作画

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use futures::StreamExt;

const SAKUGABOORU_API_BASE: &str = "https://www.sakugabooru.com";

/// Sakugabooru 帖子信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SakugaPost {
    pub id: u32,
    pub tags: String,
    pub file_url: String,
    pub preview_url: String,
    pub sample_url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub source: Option<String>,
    pub rating: String,
    pub score: i32,
    pub created_at: String,
}

/// 搜索响应
#[derive(Debug, Clone, Deserialize)]
struct DanbooruResponse {
    posts: Vec<SakugaPost>,
}

/// Sakugabooru 搜索选项
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: String,
    pub page: u32,
    pub limit: u32,
}

/// Sakugabooru API 客户端
pub struct SakugabooruClient {
    http_client: reqwest::Client,
}

impl SakugabooruClient {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self { http_client }
    }

    /// 搜索帖子
    pub async fn search(&self, options: &SearchOptions) -> Result<Vec<SakugaPost>> {
        let url = format!("{}/post.json", SAKUGABOORU_API_BASE);
        
        let response = self.http_client
            .get(&url)
            .query(&[
                ("tags", &options.query),
                ("page", &options.page.to_string()),
                ("limit", &options.limit.to_string()),
            ])
            .send()
            .await
            .context("Failed to send search request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "API request failed: {}", 
                response.status()
            ));
        }

        // Sakugabooru返回的是直接数组
        let posts: Vec<SakugaPost> = response
            .json()
            .await
            .context("Failed to parse response")?;

        Ok(posts)
    }

    /// 获取单个帖子详情
    pub async fn get_post(&self, post_id: u32) -> Result<SakugaPost> {
        let url = format!("{}/post/{}.json", SAKUGABOORU_API_BASE, post_id);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch post")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "API request failed: {}", 
                response.status()
            ));
        }

        let post: SakugaPost = response
            .json()
            .await
            .context("Failed to parse post")?;

        Ok(post)
    }

    /// 下载文件到指定路径
    pub async fn download_file(&self, url: &str, output_path: &Path) -> Result<u64> {
        let response = self.http_client
            .get(url)
            .send()
            .await
            .context("Failed to start download")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Download failed: {}", 
                response.status()
            ));
        }

        // 确保父目录存在
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(output_path).await?;
        let mut stream = response.bytes_stream();
        let mut total_bytes = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Download stream error")?;
            total_bytes += chunk.len() as u64;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        Ok(total_bytes)
    }
}

/// 分析Sakugabooru帖子标签
pub fn parse_sakuga_tags(tags: &str) -> Vec<(String, crate::models::TagCategory)> {
    tags.split_whitespace()
        .map(|tag| {
            let category = categorize_tag(tag);
            (tag.to_string(), category)
        })
        .collect()
}

/// 标签分类启发式
fn categorize_tag(tag: &str) -> crate::models::TagCategory {
    // 动画相关标签
    let animation_tags = ["animated", "sakuga", "genga", "key_animation", "inbetween"];
    // 效果标签
    let effects_tags = ["effects", "attack", "impact", "smoke", "fire", "liquid", "running"];
    // 艺术家标签（大写字母开头通常是名字）
    
    if animation_tags.contains(&tag) {
        crate::models::TagCategory::Meta
    } else if effects_tags.iter().any(|e| tag.contains(e)) {
        crate::models::TagCategory::General
    } else if tag.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        crate::models::TagCategory::Artist
    } else if tag.contains('_') && !tag.starts_with("artist_") {
        crate::models::TagCategory::Copyright
    } else {
        crate::models::TagCategory::General
    }
}
