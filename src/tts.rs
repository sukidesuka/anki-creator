use anyhow::Result;
use azure_speech::{synthesizer, Auth, stream::StreamExt};
use crate::config::TtsConfig as ConfigTtsConfig;

/// Azure TTS 配置
pub struct TtsConfig {
    pub subscription_key: String,
    pub region: String,
    pub voice_name: String,
    pub output_file: String,
}

impl TtsConfig {
    /// 从配置文件创建配置
    pub fn from_config(config: &ConfigTtsConfig) -> Self {
        Self {
            subscription_key: config.azure_speech_key.clone(),
            region: config.azure_speech_region.clone(),
            voice_name: config.azure_voice_name.clone(),
            output_file: "test_tts.wav".to_string(),
        }
    }
}

/// Azure TTS 服务
pub struct AzureTts {
    config: TtsConfig,
}

impl AzureTts {
    /// 创建新的 TTS 实例
    pub fn new(config: TtsConfig) -> Self {
        Self { config }
    }

    /// 从文本文件生成语音
    pub async fn synthesize_from_file(&self, input_file: &str) -> Result<()> {
        // 读取输入文件
        let text_content = std::fs::read_to_string(input_file)
            .map_err(|e| anyhow::anyhow!("无法读取文件 {}: {}", input_file, e))?;
        
        if text_content.trim().is_empty() {
            return Err(anyhow::anyhow!("输入文件为空"));
        }
        
        // 调用 TTS 服务
        self.synthesize_text(&text_content).await
    }

    /// 从文本生成语音到指定文件
    pub async fn synthesize_text_to_file(&self, text: &str, output_file: &str) -> Result<()> {
        // 创建认证
        let auth = Auth::from_subscription(
            self.config.region.clone(),
            self.config.subscription_key.clone(),
        );

        // 创建合成器配置
        let config = synthesizer::Config::new()
            .with_language(synthesizer::Language::JaJp)
            .with_voice(synthesizer::Voice::JaJpNanamiNeural);

        // 创建合成器客户端
        let client = synthesizer::Client::connect(auth, config).await?;
        
        // 执行语音合成
        let mut stream = client.synthesize(text).await?;
        
        // 收集音频数据
        let mut audio_data = Vec::new();
        
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    match event {
                        synthesizer::Event::Synthesising(_, audio_chunk) => {
                            audio_data.extend_from_slice(&audio_chunk);
                        }
                        synthesizer::Event::Synthesised(_) => {
                            break;
                        }
                        _ => {
                            // 忽略其他事件
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("语音合成过程中出错: {}", e));
                }
            }
        }
        
        if audio_data.is_empty() {
            return Err(anyhow::anyhow!("未收到音频数据"));
        }
        
        // 保存音频文件到指定路径
        self.save_audio_file_to_path(&audio_data, output_file)?;
        
        Ok(())
    }

    /// 从文本生成语音
    pub async fn synthesize_text(&self, text: &str) -> Result<()> {
        // 创建认证
        let auth = Auth::from_subscription(
            self.config.region.clone(),
            self.config.subscription_key.clone(),
        );

        // 创建合成器配置
        let config = synthesizer::Config::new()
            .with_language(synthesizer::Language::JaJp)
            .with_voice(synthesizer::Voice::JaJpNanamiNeural);

        // 创建合成器客户端
        let client = synthesizer::Client::connect(auth, config).await?;
        
        // 执行语音合成
        let mut stream = client.synthesize(text).await?;
        
        // 收集音频数据
        let mut audio_data = Vec::new();
        
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    match event {
                        synthesizer::Event::Synthesising(_, audio_chunk) => {
                            audio_data.extend_from_slice(&audio_chunk);
                        }
                        synthesizer::Event::Synthesised(_) => {
                            break;
                        }
                        _ => {
                            // 忽略其他事件
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("语音合成过程中出错: {}", e));
                }
            }
        }
        
        if audio_data.is_empty() {
            return Err(anyhow::anyhow!("未收到音频数据"));
        }
        
        // 保存音频文件
        self.save_audio_file(&audio_data)?;
        
        Ok(())
    }

    /// 保存音频文件到指定路径
    fn save_audio_file_to_path(&self, audio_data: &[u8], output_file: &str) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        // 确保目录存在
        if let Some(parent) = std::path::Path::new(output_file).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("无法创建目录 {}: {}", parent.display(), e))?;
        }
        
        let mut file = File::create(output_file)
            .map_err(|e| anyhow::anyhow!("无法创建输出文件 {}: {}", output_file, e))?;
        
        file.write_all(audio_data)
            .map_err(|e| anyhow::anyhow!("无法写入音频数据: {}", e))?;
        
        Ok(())
    }

    /// 保存音频文件
    fn save_audio_file(&self, audio_data: &[u8]) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(&self.config.output_file)
            .map_err(|e| anyhow::anyhow!("无法创建输出文件 {}: {}", self.config.output_file, e))?;
        
        file.write_all(audio_data)
            .map_err(|e| anyhow::anyhow!("无法写入音频数据: {}", e))?;
        
        Ok(())
    }

}
