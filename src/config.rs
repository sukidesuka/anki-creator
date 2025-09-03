use anyhow::Result;
use serde::Deserialize;

// 配置文件结构
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub api: ApiConfig,
    pub processing: ProcessingConfig,
    pub database: DatabaseConfig,
    pub output: OutputConfig,
    pub input: InputConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub openrouter_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessingConfig {
    pub concurrent_requests: usize,
    pub request_delay_ms: u64,
    pub max_retries: u32,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub db_file: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    pub words_file: String,
    pub grammar_file: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputConfig {
    pub text_file: String,
}

impl Config {
    pub fn load() -> Result<Config> {
        let config_content = std::fs::read_to_string("config.toml")
            .map_err(|_| anyhow::anyhow!("配置文件 config.toml 不存在或无法读取"))?;
        let config: Config = toml::from_str(&config_content)
            .map_err(|e| anyhow::anyhow!("配置文件解析失败: {}", e))?;
        Ok(config)
    }
}
