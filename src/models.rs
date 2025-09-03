use serde::{Deserialize, Serialize};

// 数据结构定义
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct JapaneseWord {
    pub id: i64,
    pub word: String,
    pub kana: String,
    pub pitch: String,
    pub part_of_speech: String,
    pub analysis: String,
    pub updated_at: Option<String>, // 更新时间，使用Option以兼容旧数据
}

// 用于支持多词性合并的结构
#[derive(Debug, Clone)]
pub struct MergedWord {
    pub id: i64,
    pub word: String,
    pub kana: String,
    pub pitch: String,
    pub parts_of_speech: Vec<String>,
    pub analysis: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct JapaneseGrammar {
    pub id: i64,
    pub word: String,
    pub kana: String,
    pub analysis: String,
}

// 第一步提取结构
#[derive(Debug, Deserialize)]
pub struct ExtractionResult {
    pub words: Vec<BasicWordInfo>,
    pub grammar: Vec<BasicGrammarInfo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BasicWordInfo {
    pub word: String,
    pub kana: String,
    pub pitch: String,
    pub part_of_speech: Vec<String>, // 支持多个词性
}

#[derive(Debug, Deserialize, Clone)]
pub struct BasicGrammarInfo {
    pub grammar: String,
    pub kana: String,
}

// 最终分析结果结构
#[derive(Debug, Clone)]
pub struct WordAnalysis {
    pub word: String,
    pub kana: String,
    pub pitch: String,
    pub part_of_speech: String,
    pub analysis: String,
}

#[derive(Debug, Clone)]
pub struct GrammarAnalysis {
    pub grammar: String,
    pub kana: String,
    pub analysis: String,
}
