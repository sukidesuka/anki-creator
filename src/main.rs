use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs::File;
use std::io::Write;

// 数据结构定义
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct JapaneseWord {
    pub id: i64,
    pub word: String,
    pub kana: String,
    pub analysis: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct JapaneseGrammar {
    pub id: i64,
    pub word: String,
    pub kana: String,
    pub analysis: String,
}

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

// 分析结果结构
#[derive(Debug, Deserialize)]
pub struct AnalysisResult {
    pub words: Vec<WordAnalysis>,
    pub grammar: Vec<GrammarAnalysis>,
}

#[derive(Debug, Deserialize)]
pub struct WordAnalysis {
    pub word: String,
    pub kana: String,
    pub pitch: String,
    pub analysis: String,
}

#[derive(Debug, Deserialize)]
pub struct GrammarAnalysis {
    pub grammar: String,
    pub kana: String,
    pub analysis: String,
}

pub struct AnkiCreator {
    client: Client,
    pool: SqlitePool,
    api_key: String,
}

impl AnkiCreator {
    // 创建新实例
    pub async fn new(api_key: String) -> Result<Self> {
        let client = Client::new();
        let pool = SqlitePool::connect("sqlite:anki_cards.db").await?;
        
        // 初始化数据库表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS words (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE,
                kana TEXT NOT NULL,
                analysis TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        ).execute(&pool).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS grammar (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE,
                kana TEXT NOT NULL,
                analysis TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#
        ).execute(&pool).await?;

        Ok(AnkiCreator {
            client,
            pool,
            api_key,
        })
    }

    // 调用 OpenRouter API 分析日语文本
    pub async fn analyze_japanese_text(&self, text: &str) -> Result<String> {
        let prompt = format!(r#"
请分析以下日语文本，提取出所有单词和语法点。要求：

1. 单词部分：
   - 将所有单词转换为辞书形（原形）
   - 提供假名读音
   - 提供音调（用0-4数字表示）
   - 提供中文解释

2. 语法部分：
   - 识别语法结构和表达方式
   - 提供假名读音
   - 提供中文解释和用法

请用以下JSON格式返回结果：
{{
  "words": [
    {{
      "word": "単语辞书形",
      "kana": "かな",
      "pitch": "0", 
      "analysis": "中文解释和详细分析"
    }}
  ],
  "grammar": [
    {{
      "grammar": "语法表达",
      "kana": "かな",
      "analysis": "语法解释和用法说明"
    }}
  ]
}}

要分析的文本：
{}
"#, text);

        let request = OpenRouterRequest {
            model: "google/gemini-2.5-flash".to_string(),
            messages: vec![RequestMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            max_tokens: 4000,
            temperature: 0.1,
        };

        let response = self.client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("API 请求失败: {}", error_text);
        }

        let api_response: OpenRouterResponse = response.json().await?;
        let content = &api_response.choices[0].message.content;
        
        // 提取JSON部分
        let json_start = content.find('{').unwrap_or(0);
        let json_end = content.rfind('}').map(|i| i + 1).unwrap_or(content.len());
        let json_content = &content[json_start..json_end];
        
        Ok(json_content.to_string())
    }

    // 保存单词到数据库
    pub async fn save_words(&self, words: &[WordAnalysis]) -> Result<()> {
        for word in words {
            let analysis_with_pitch = format!("{} [音调: {}]", word.analysis, word.pitch);
            
            sqlx::query(
                "INSERT OR REPLACE INTO words (word, kana, analysis) VALUES (?, ?, ?)"
            )
            .bind(&word.word)
            .bind(&word.kana)
            .bind(&analysis_with_pitch)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    // 保存语法到数据库
    pub async fn save_grammar(&self, grammar: &[GrammarAnalysis]) -> Result<()> {
        for item in grammar {
            sqlx::query(
                "INSERT OR REPLACE INTO grammar (word, kana, analysis) VALUES (?, ?, ?)"
            )
            .bind(&item.grammar)
            .bind(&item.kana)
            .bind(&item.analysis)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    // 生成单词 Anki 卡片
    pub async fn generate_word_cards(&self) -> Result<()> {
        let words = sqlx::query_as::<_, JapaneseWord>(
            "SELECT id, word, kana, analysis FROM words ORDER BY id"
        ).fetch_all(&self.pool).await?;

        let mut file = File::create("japanese_words.csv")?;
        
        // 写入 CSV 头部（带有 ID 字段）
        writeln!(file, "id,word,kana,analysis")?;
        
        for word in words {
            // CSV 格式，确保 ID 在第一个字段
            writeln!(file, "{},\"{}\",\"{}\",\"{}\"", 
                word.id, 
                word.word.replace("\"", "\"\""),
                word.kana.replace("\"", "\"\""),
                word.analysis.replace("\"", "\"\"")
            )?;
        }
        
        println!("✅ 单词卡片已生成：japanese_words.csv");
        Ok(())
    }

    // 生成语法 Anki 卡片  
    pub async fn generate_grammar_cards(&self) -> Result<()> {
        let grammar = sqlx::query_as::<_, JapaneseGrammar>(
            "SELECT id, word, kana, analysis FROM grammar ORDER BY id"
        ).fetch_all(&self.pool).await?;

        let mut file = File::create("japanese_grammar.csv")?;
        
        // 写入 CSV 头部（带有 ID 字段）
        writeln!(file, "id,grammar,kana,analysis")?;
        
        for item in grammar {
            // CSV 格式，确保 ID 在第一个字段
            writeln!(file, "{},\"{}\",\"{}\",\"{}\"", 
                item.id, 
                item.word.replace("\"", "\"\""),
                item.kana.replace("\"", "\"\""),
                item.analysis.replace("\"", "\"\"")
            )?;
        }
        
        println!("✅ 语法卡片已生成：japanese_grammar.csv");
        Ok(())
    }

    // 处理日语文本的主要函数
    pub async fn process_japanese_text(&self, text: &str) -> Result<()> {
        println!("🔄 开始分析日语文本...");
        
        // 调用 API 分析文本
        let json_response = self.analyze_japanese_text(text).await?;
        
        // 解析 JSON 响应
        let analysis: AnalysisResult = serde_json::from_str(&json_response)
            .map_err(|e| anyhow::anyhow!("解析 API 响应失败: {}\n响应内容: {}", e, json_response))?;

        println!("📝 找到 {} 个单词，{} 个语法点", 
            analysis.words.len(), 
            analysis.grammar.len()
        );

        // 保存到数据库
        self.save_words(&analysis.words).await?;
        self.save_grammar(&analysis.grammar).await?;
        
        println!("💾 数据已保存到数据库");

        // 生成 Anki 卡片
        self.generate_word_cards().await?;
        self.generate_grammar_cards().await?;
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎌 日语 Anki 卡片生成器");
    
    // 从环境变量获取 API 密钥
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .unwrap_or_else(|_| {
            println!("⚠️  请设置 OPENROUTER_API_KEY 环境变量");
            println!("   export OPENROUTER_API_KEY=your_api_key");
            std::process::exit(1);
        });

    // 创建 Anki 卡片生成器
    let creator = AnkiCreator::new(api_key).await?;
    
    // 示例日语文本（可以修改为你想要的文本）
    let sample_text = r#"
今日は良い天気ですね。公園に散歩しに行きましょう。
桜が満開で、とても美しいです。写真を撮りたいと思います。
日本語を勉強するのは楽しいですが、時々難しいです。
"#;

    println!("📖 处理示例文本...");
    println!("文本内容: {}", sample_text);
    
    // 处理文本
    creator.process_japanese_text(sample_text).await?;
    
    println!("\n🎉 完成！生成的文件：");
    println!("   📄 japanese_words.csv - 单词卡片");
    println!("   📄 japanese_grammar.csv - 语法卡片");
    println!("   🗄️  anki_cards.db - SQLite 数据库");
    
    println!("\n📋 使用说明：");
    println!("1. 在 Anki 中导入 CSV 文件");
    println!("2. 确保字段映射正确（ID 字段用于更新现有卡片）");
    println!("3. 单词和语法会创建为不同的卡组");
    
    Ok(())
}
