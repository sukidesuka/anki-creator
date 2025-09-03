use anyhow::Result;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use crate::config::Config;

// OpenRouter API 响应结构
#[derive(Debug, Deserialize)]
pub struct OpenRouterResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct OpenRouterRequest {
    pub model: String,
    pub messages: Vec<RequestMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Serialize)]
pub struct RequestMessage {
    pub role: String,
    pub content: String,
}

pub struct ApiClient {
    client: Client,
    config: Config,
}

impl ApiClient {
    pub fn new(config: Config) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(config.processing.request_timeout_seconds))
            .build()?;
        
        Ok(ApiClient { client, config })
    }

    // 带重试机制的API请求函数
    pub async fn make_request_with_retry(&self, request: &OpenRouterRequest) -> Result<OpenRouterResponse> {
        let mut last_error = None;
        
        for attempt in 0..=self.config.processing.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(self.config.processing.request_delay_ms * (attempt as u64 + 1));
                println!("    ⏳ 重试 {}/{} 次，等待 {:?}...", attempt, self.config.processing.max_retries, delay);
                sleep(delay).await;
            }
            
            match self.client
                .post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", self.config.api.openrouter_key))
                .header("Content-Type", "application/json")
                .json(request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<OpenRouterResponse>().await {
                            Ok(api_response) => {
                                if attempt > 0 {
                                    println!("    ✅ 重试成功！");
                                }
                                return Ok(api_response);
                            },
                            Err(e) => {
                                let error_msg = format!("JSON解析失败: {}", e);
                                println!("    ❌ 尝试 {}: {}", attempt + 1, error_msg);
                                last_error = Some(anyhow::anyhow!(error_msg));
                            }
                        }
                    } else {
                        let status = response.status();
                        match response.text().await {
                            Ok(error_text) => {
                                let error_msg = format!("API请求失败 (状态码: {}): {}", status, error_text);
                                println!("    ❌ 尝试 {}: {}", attempt + 1, error_msg);
                                last_error = Some(anyhow::anyhow!(error_msg));
                            },
                            Err(e) => {
                                let error_msg = format!("读取错误响应失败: {}", e);
                                println!("    ❌ 尝试 {}: {}", attempt + 1, error_msg);
                                last_error = Some(anyhow::anyhow!(error_msg));
                            }
                        }
                    }
                },
                Err(e) => {
                    let error_msg = format!("网络请求失败: {}", e);
                    println!("    ❌ 尝试 {}: {}", attempt + 1, error_msg);
                    last_error = Some(anyhow::anyhow!(error_msg));
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("所有重试都失败了")))
    }
}
