use anyhow::Result;
use sqlx::SqlitePool;
use std::fs::File;
use std::io::Write;

use crate::config::Config;
use crate::models::*;

pub struct DatabaseManager {
    pool: SqlitePool,
    config: Config,
}

impl DatabaseManager {
    pub async fn new(config: Config) -> Result<Self> {
        // 创建数据库文件路径
        let db_path = std::env::current_dir()?.join(&config.database.db_file);
        let db_url = format!("sqlite:{}", db_path.display());
        
        println!("💾 连接数据库: {}", db_path.display());
        
        // 如果数据库文件不存在，先创建一个空文件
        if !db_path.exists() {
            std::fs::File::create(&db_path)?;
            println!("✨ 创建新数据库文件: {}", db_path.display());
        }
        
        let pool = SqlitePool::connect(&db_url).await?;
        
        let manager = DatabaseManager { pool, config };
        manager.initialize_tables().await?;
        
        Ok(manager)
    }

    async fn initialize_tables(&self) -> Result<()> {
        // 初始化数据库表
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS words (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL,
                kana TEXT NOT NULL,
                pitch TEXT NOT NULL DEFAULT '0',
                part_of_speech TEXT NOT NULL,
                analysis TEXT NOT NULL,
                created_at DATETIME DEFAULT (datetime('now')),
                updated_at DATETIME DEFAULT (datetime('now')),
                UNIQUE(word, kana, pitch)
            )
            "#
        ).execute(&self.pool).await?;

        // 清理可能存在的重复数据，合并词性
        println!("🔧 检查并合并数据库中的重复单词...");
        
        // 检查是否有重复数据需要合并
        let duplicate_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) - COUNT(DISTINCT word || '|' || kana || '|' || pitch) FROM words"
        ).fetch_one(&self.pool).await?;
        
        if duplicate_count.0 > 0 {
            println!("   发现 {} 个重复记录，正在合并...", duplicate_count.0);
            
            sqlx::query(
                r#"
                CREATE TEMPORARY TABLE temp_words AS 
                SELECT 
                    MIN(id) as id,
                    word, 
                    kana, 
                    pitch, 
                    GROUP_CONCAT(part_of_speech, '｜') as part_of_speech,
                    MIN(analysis) as analysis
                FROM words 
                GROUP BY word, kana, pitch
                "#
            ).execute(&self.pool).await?;

            sqlx::query("DELETE FROM words").execute(&self.pool).await?;
            
            sqlx::query(
                r#"
                INSERT INTO words (id, word, kana, pitch, part_of_speech, analysis)
                SELECT id, word, kana, pitch, part_of_speech, analysis FROM temp_words
                "#
            ).execute(&self.pool).await?;
            
            sqlx::query("DROP TABLE temp_words").execute(&self.pool).await?;
            
            println!("   ✅ 重复数据合并完成");
        } else {
            println!("   ✅ 没有发现重复数据");
        }

        // 检查并添加缺失的列
        println!("🔧 检查数据库表结构...");
        
        // 检查pitch列是否存在
        let pitch_exists = sqlx::query("SELECT pitch FROM words LIMIT 1")
            .execute(&self.pool)
            .await
            .is_ok();
        
        if !pitch_exists {
            println!("   添加 pitch 列...");
            sqlx::query("ALTER TABLE words ADD COLUMN pitch TEXT NOT NULL DEFAULT '0'")
                .execute(&self.pool)
                .await?;
            println!("   ✅ pitch 列添加成功");
        } else {
            println!("   ✅ pitch 列已存在");
        }
        
        // 检查updated_at列是否存在
        let updated_at_exists = sqlx::query("SELECT updated_at FROM words LIMIT 1")
            .execute(&self.pool)
            .await
            .is_ok();
        
        if !updated_at_exists {
            println!("   添加 updated_at 列...");
            // SQLite不允许ALTER TABLE时使用CURRENT_TIMESTAMP作为默认值，所以先添加NULL默认值的列
            sqlx::query("ALTER TABLE words ADD COLUMN updated_at DATETIME")
                .execute(&self.pool)
                .await?;
            
            // 然后为现有记录设置当前时间
            sqlx::query("UPDATE words SET updated_at = datetime('now') WHERE updated_at IS NULL")
                .execute(&self.pool)
                .await?;
            
            println!("   ✅ updated_at 列添加成功");
        } else {
            println!("   ✅ updated_at 列已存在");
        }

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
        ).execute(&self.pool).await?;

        Ok(())
    }

    // 检查单词是否已存在（根据单词、假名，不依据音调和词性）
    pub async fn check_word_exists(&self, word: &str, kana: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM words WHERE word = ? AND kana = ?"
        )
        .bind(word)
        .bind(kana)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count.0 > 0)
    }

    // 获取已存在的单词信息（只基于 word 和 kana）
    pub async fn get_existing_word_by_word_kana(&self, word: &str, kana: &str) -> Result<Option<JapaneseWord>> {
        let result = sqlx::query_as::<_, JapaneseWord>(
            "SELECT id, word, kana, pitch, part_of_speech, analysis, updated_at FROM words WHERE word = ? AND kana = ? LIMIT 1"
        )
        .bind(word)
        .bind(kana)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result)
    }

    // 获取已存在的单词信息（支持多词性合并）
    pub async fn get_existing_word(&self, word: &str, kana: &str, pitch: &str) -> Result<Option<MergedWord>> {
        let result = sqlx::query_as::<_, JapaneseWord>(
            "SELECT id, word, kana, pitch, part_of_speech, analysis, updated_at FROM words WHERE word = ? AND kana = ? AND pitch = ? LIMIT 1"
        )
        .bind(word)
        .bind(kana)
        .bind(pitch)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(existing) = result {
            // 获取该单词的所有词性
            let all_pos: Vec<(String,)> = sqlx::query_as(
                "SELECT DISTINCT part_of_speech FROM words WHERE word = ? AND kana = ? AND pitch = ?"
            )
            .bind(word)
            .bind(kana)
            .bind(pitch)
            .fetch_all(&self.pool)
            .await?;
            
            let parts_of_speech: Vec<String> = all_pos.into_iter().map(|row| row.0).collect();
            
            Ok(Some(MergedWord {
                id: existing.id,
                word: existing.word,
                kana: existing.kana,
                pitch: existing.pitch,
                parts_of_speech,
                analysis: existing.analysis,
            }))
        } else {
            Ok(None)
        }
    }

    // 获取所有单词
    pub async fn get_all_words(&self) -> Result<Vec<JapaneseWord>> {
        let words = sqlx::query_as::<_, JapaneseWord>(
            "SELECT id, word, kana, pitch, part_of_speech, analysis, updated_at FROM words ORDER BY id"
        ).fetch_all(&self.pool).await?;
        
        Ok(words)
    }

    // 保存单词到数据库（新的词性覆盖旧的，不再合并）
    pub async fn save_words(&self, words: &[WordAnalysis]) -> Result<()> {
        for word in words {
            // 检查是否已存在同样的单词（不考虑词性）
            let existing = sqlx::query_as::<_, JapaneseWord>(
                "SELECT id, word, kana, pitch, part_of_speech, analysis, updated_at FROM words WHERE word = ? AND kana = ? AND pitch = ? LIMIT 1"
            )
            .bind(&word.word)
            .bind(&word.kana)
            .bind(&word.pitch)
            .fetch_optional(&self.pool)
            .await?;
            
            if let Some(existing_word) = existing {
                // 如果已存在，检查词性是否不同
                if existing_word.part_of_speech != word.part_of_speech {
                    println!("  🔄 更新单词词性: {} ({}) - {} -> {}", 
                        word.word, word.kana, 
                        existing_word.part_of_speech, 
                        word.part_of_speech
                    );
                    
                    // 更新记录，以新的词性和分析为准，并更新时间
                    sqlx::query(
                        "UPDATE words SET part_of_speech = ?, analysis = ?, updated_at = datetime('now') WHERE id = ?"
                    )
                    .bind(&word.part_of_speech)
                    .bind(&word.analysis)
                    .bind(existing_word.id)
                    .execute(&self.pool)
                    .await?;
                } else {
                    println!("  ✅ 单词词性未变化，跳过更新: {} ({})", word.word, word.kana);
                }
            } else {
                // 如果不存在，直接插入
                println!("  ➕ 新增单词: {} ({}) - {}", word.word, word.kana, word.part_of_speech);
                sqlx::query(
                    "INSERT INTO words (word, kana, pitch, part_of_speech, analysis, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'))"
                )
                .bind(&word.word)
                .bind(&word.kana)
                .bind(&word.pitch)
                .bind(&word.part_of_speech)
                .bind(&word.analysis)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    // 更新单词词性
    pub async fn update_word_part_of_speech(&self, id: i64, new_pos: &str) -> Result<()> {
        sqlx::query(
            "UPDATE words SET part_of_speech = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(new_pos)
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    // 更新单词的 pitch 和词性（处理唯一约束冲突）
    pub async fn update_word_pitch_and_pos(&self, id: i64, new_pitch: &str, new_pos: &str) -> Result<()> {
        // 首先获取当前记录的信息
        let current_word = self.get_word_by_id(id).await?;
        if current_word.is_none() {
            return Err(anyhow::anyhow!("单词 ID {} 不存在", id));
        }
        let current = current_word.unwrap();
        
        // 检查是否存在相同 (word, kana, pitch) 的其他记录
        let existing_conflict = sqlx::query_as::<_, JapaneseWord>(
            "SELECT id, word, kana, pitch, part_of_speech, analysis, updated_at FROM words WHERE word = ? AND kana = ? AND pitch = ? AND id != ? LIMIT 1"
        )
        .bind(&current.word)
        .bind(&current.kana)
        .bind(new_pitch)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(conflict_record) = existing_conflict {
            // 如果存在冲突记录，删除冲突记录，然后更新当前记录
            println!("  🔄 发现冲突记录，删除旧记录 ID {}，更新当前记录 ID {}", 
                conflict_record.id, id
            );
            
            // 删除冲突记录
            sqlx::query("DELETE FROM words WHERE id = ?")
                .bind(conflict_record.id)
                .execute(&self.pool)
                .await?;
            
            // 更新当前记录
            sqlx::query(
                "UPDATE words SET pitch = ?, part_of_speech = ?, updated_at = datetime('now') WHERE id = ?"
            )
            .bind(new_pitch)
            .bind(new_pos)
            .bind(id)
            .execute(&self.pool)
            .await?;
            
            println!("  ✅ 冲突处理完成: {} ({}) - pitch: {}->{}, pos: {}->{}", 
                current.word, current.kana, 
                current.pitch, new_pitch,
                current.part_of_speech, new_pos
            );
        } else {
            // 没有冲突，直接更新
            sqlx::query(
                "UPDATE words SET pitch = ?, part_of_speech = ?, updated_at = datetime('now') WHERE id = ?"
            )
            .bind(new_pitch)
            .bind(new_pos)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        
        Ok(())
    }

    // 更新单词解析
    pub async fn update_word_analysis(&self, id: i64, new_analysis: &str) -> Result<()> {
        sqlx::query(
            "UPDATE words SET analysis = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(new_analysis)
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    // 根据ID获取单词信息
    pub async fn get_word_by_id(&self, id: i64) -> Result<Option<JapaneseWord>> {
        let word = sqlx::query_as::<_, JapaneseWord>(
            "SELECT id, word, kana, pitch, part_of_speech, analysis, updated_at FROM words WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(word)
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

    // 获取所有语法
    pub async fn get_all_grammar(&self) -> Result<Vec<JapaneseGrammar>> {
        let grammar = sqlx::query_as::<_, JapaneseGrammar>(
            "SELECT id, word, kana, analysis FROM grammar ORDER BY id"
        ).fetch_all(&self.pool).await?;
        
        Ok(grammar)
    }
}

// 辅助函数：将音调数字转换为上标符号
pub fn pitch_to_superscript(pitch: &str) -> String {
    match pitch {
        "0" => "⓪".to_string(),
        "1" => "①".to_string(),
        "2" => "②".to_string(),
        "3" => "③".to_string(),
        "4" => "④".to_string(),
        "5" => "⑤".to_string(),
        "6" => "⑥".to_string(),
        "7" => "⑦".to_string(),
        "8" => "⑧".to_string(),
        "9" => "⑨".to_string(),
        "10" => "⑩".to_string(),
        "11" => "⑪".to_string(),
        "12" => "⑫".to_string(),
        "13" => "⑬".to_string(),
        "14" => "⑭".to_string(),
        "15" => "⑮".to_string(),
        "16" => "⑯".to_string(),
        "17" => "⑰".to_string(),
        "18" => "⑱".to_string(),
        "19" => "⑲".to_string(),
        "20" => "⑳".to_string(),
        _ => pitch.to_string(), // 如果不是0-20的数字，直接返回原文
    }
}

// 生成单词 Anki 卡片（支持词性合并和HTML格式）
pub fn generate_word_cards(words: &[JapaneseWord], output_file: &str) -> Result<()> {
    let mut file = File::create(output_file)?;
    
    for word in words {
        // 解析词性字段（用｜分隔）
        let parts_of_speech: Vec<&str> = word.part_of_speech.split('｜').collect();
        
        // 直接使用数据库中的pitch字段
        let pitch = &word.pitch;
        
        // 生成正面内容（HTML格式）
        let word_with_pitch = format!("{}{}", word.word, pitch_to_superscript(&pitch));
        
        // 添加语音文件引用
        let audio_tag = format!("[sound:japanese_word_{}.wav]", word.id);
        
        let front = if word.word == word.kana {
            // 只有假名的情况
            format!(
                "<div style=\"font-size: 20px; font-weight: bold;\">{} {}</div><div style=\"font-size: 14px; color: #666; margin-top: 5px;\">{}</div>",
                word_with_pitch,
                audio_tag,
                parts_of_speech.join("·")
            )
        } else {
            // 有汉字和假名的情况
            format!(
                "<div style=\"font-size: 20px; font-weight: bold;\">{} {}</div><div style=\"font-size: 16px; margin-top: 2px;\">{}</div><div style=\"font-size: 14px; color: #666; margin-top: 3px;\">{}</div>",
                word_with_pitch,
                audio_tag,
                word.kana,
                parts_of_speech.join("·")
            )
        };
        
        // CSV 格式：id:正面:背面:标签
        writeln!(file, "{}:\"{}\":\"{}\":\"单词\"", 
            word.id,
            front.replace("\"", "\"\""),
            word.analysis.replace("\"", "\"\"")
        )?;
    }
    
    println!("✅ 单词卡片已生成：{}", output_file);
    Ok(())
}

// 生成语法 Anki 卡片  
pub fn generate_grammar_cards(grammar: &[JapaneseGrammar], output_file: &str) -> Result<()> {
    let mut file = File::create(output_file)?;
    
    for item in grammar {
        // 生成正面内容：语法表达和假名用｜隔开，如果重复则省略
        let front_content = if item.word == item.kana {
            item.word.clone()
        } else {
            format!("{}｜{}", item.word, item.kana)
        };
        
        // 添加语音文件引用
        let audio_tag = format!("[sound:japanese_word_{}.wav]", item.id);
        let front = format!("{} {}", front_content, audio_tag);
        
        // CSV 格式：id:正面:背面:标签（语法标签为"语法"）
        writeln!(file, "{}:\"{}\":\"{}\":\"语法\"", 
            item.id,
            front.replace("\"", "\"\""),
            item.analysis.replace("\"", "\"\"")
        )?;
    }
    
    println!("✅ 语法卡片已生成：{}", output_file);
    Ok(())
}
