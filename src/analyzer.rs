    use anyhow::Result;
use futures::stream::{self, StreamExt};
use std::collections::HashMap;

use crate::api::{ApiClient, OpenRouterRequest, RequestMessage};
use crate::config::Config;
use crate::database::{DatabaseManager, generate_word_cards, generate_grammar_cards};
use crate::models::*;
use crate::tts::{AzureTts, TtsConfig};

pub struct AnkiCreator {
    api_client: ApiClient,
    db_manager: DatabaseManager,
    pub config: Config,
}

impl AnkiCreator {
    pub async fn new(config: Config) -> Result<Self> {
        let api_client = ApiClient::new(config.clone())?;
        let db_manager = DatabaseManager::new(config.clone()).await?;
        
        Ok(AnkiCreator {
            api_client,
            db_manager,
            config,
        })
    }

    // 更新所有单词的词性
    pub async fn update_all_word_parts_of_speech(&self) -> Result<()> {
        println!("🔄 开始更新所有单词的词性...");
        
        // 获取所有单词记录
        let words = self.db_manager.get_all_words().await?;
        
        if words.is_empty() {
            println!("⚠️  数据库中没有找到任何单词");
            return Ok(());
        }
        
        println!("📊 找到 {} 个单词需要更新词性", words.len());
        
        // 使用并发流处理所有单词
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.processing.concurrent_requests));
        
        let total_words = words.len();
        let update_results: Result<Vec<()>, anyhow::Error> = stream::iter(words.into_iter().enumerate())
            .map(|(i, word)| {
                let semaphore = semaphore.clone();
                let api_client = &self.api_client;
                let db_manager = &self.db_manager;
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    println!("  🔍 更新单词 {}/{}: {} ({})", 
                        i + 1, total_words, word.word, word.kana);
                    
                    // 重新分析单词以获取最新的词性
                    let prompt = format!(r#"
请重新分析这个日语单词的词性，只需要返回准确的词性信息：

单词：{}
假名：{}
音调：{}

请用以下JSON格式返回结果（只需要词性信息）：
{{
  "part_of_speech": ["词性1", "词性2"]
}}

重要词性标注规则：
1. 动词必须明确标注为"自动词"或"他动词"，不要只写"动词"
2. 如果一个词既是自动词又是他动词，就标注["自动词", "他动词"]
3. 形容词分为"一类形容词"和"二类形容词"
4. 一律使用简体中文词性标注：名词、自动词、他动词、一类形容词、二类形容词、副词、连词、助词、感叹词等
5. 不要出现"動詞｜自動詞"这种重复标注
6. 只返回JSON格式，不要添加其他内容
"#, word.word, word.kana, word.pitch);

                    let request = OpenRouterRequest {
                        model: self.config.api.models.word_analysis_model.clone(),
                        messages: vec![RequestMessage {
                            role: "user".to_string(),
                            content: prompt,
                        }],
                        max_tokens: 1000,
                        temperature: 0.1,
                    };

                    match api_client.make_request_with_retry(&request).await {
                        Ok(api_response) => {
                            let content = &api_response.choices[0].message.content;
                            
                            // 提取JSON部分
                            let json_start = content.find('{').unwrap_or(0);
                            let json_end = content.rfind('}').map(|i| i + 1).unwrap_or(content.len());
                            let json_content = &content[json_start..json_end];
                            
                            // 解析词性结果
                            match serde_json::from_str::<serde_json::Value>(json_content) {
                                Ok(parsed) => {
                                    if let Some(pos_array) = parsed.get("part_of_speech").and_then(|v| v.as_array()) {
                                        let new_parts_of_speech: Vec<String> = pos_array
                                            .iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect();
                                        
                                        if !new_parts_of_speech.is_empty() {
                                            let new_pos_str = new_parts_of_speech.join("｜");
                                            
                                            // 检查词性是否有变化
                                            if word.part_of_speech != new_pos_str {
                                                println!("    🔄 词性更新: {} -> {}", 
                                                    word.part_of_speech, new_pos_str);
                                                
                                                // 更新数据库中的词性
                                                if let Err(e) = db_manager.update_word_part_of_speech(word.id, &new_pos_str).await {
                                                    println!("    ❌ 更新失败: {}", e);
                                                } else {
                                                    println!("    ✅ 更新成功");
                                                }
                                            } else {
                                                println!("    ✅ 词性无变化，跳过更新");
                                            }
                                        } else {
                                            println!("    ⚠️  未能解析到有效词性");
                                        }
                                    } else {
                                        println!("    ⚠️  响应格式不正确");
                                    }
                                },
                                Err(e) => {
                                    println!("    ❌ JSON解析失败: {}", e);
                                }
                            }
                        },
                        Err(e) => {
                            println!("    ❌ API请求失败: {}", e);
                        }
                    }
                    
                    // 添加延迟以避免过于频繁的请求
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        self.config.processing.request_delay_ms
                    )).await;
                    
                    Ok(())
                }
            })
            .buffer_unordered(self.config.processing.concurrent_requests)
            .collect::<Vec<Result<(), anyhow::Error>>>()
            .await
            .into_iter()
            .collect();
        
        update_results?;
        
        println!("🎉 所有单词词性更新完成！");
        Ok(())
    }

    // 更新所有单词的解析
    pub async fn update_all_word_analysis(&self) -> Result<()> {
        println!("🔄 开始更新所有单词的解析...");
        
        // 获取所有单词记录
        let words = self.db_manager.get_all_words().await?;
        
        if words.is_empty() {
            println!("⚠️  数据库中没有找到任何单词");
            return Ok(());
        }
        
        println!("📊 找到 {} 个单词需要更新解析", words.len());
        
        // 使用并发流处理所有单词
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.processing.concurrent_requests));
        
        let total_words = words.len();
        let update_results: Result<Vec<()>, anyhow::Error> = stream::iter(words.into_iter().enumerate())
            .map(|(i, word)| {
                let semaphore = semaphore.clone();
                let analyzer = self;
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    println!("  🔍 更新单词解析 {}/{}: {} ({})", 
                        i + 1, total_words, word.word, word.kana);
                    
                    // 复用现有的分析逻辑
                    let parts_of_speech: Vec<&str> = word.part_of_speech.split('｜').collect();
                    let parts_of_speech_vec: Vec<String> = parts_of_speech.iter().map(|s| s.to_string()).collect();
                    
                    let basic_word = BasicWordInfo {
                        word: word.word.clone(),
                        kana: word.kana.clone(),
                        pitch: word.pitch.clone(),
                        part_of_speech: parts_of_speech_vec.clone(),
                    };
                    
                    match analyzer.analyze_word_with_multiple_pos(&basic_word, &parts_of_speech_vec).await {
                        Ok(new_analysis) => {
                            // 检查解析是否有变化
                            if word.analysis != new_analysis {
                                println!("    🔄 解析更新: 长度 {} -> {}", 
                                    word.analysis.len(), new_analysis.len());
                                
                                // 更新数据库中的解析
                                if let Err(e) = analyzer.db_manager.update_word_analysis(word.id, &new_analysis).await {
                                    println!("    ❌ 更新失败: {}", e);
                                } else {
                                    println!("    ✅ 更新成功");
                                }
                            } else {
                                println!("    ✅ 解析无变化，跳过更新");
                            }
                        },
                        Err(e) => {
                            println!("    ❌ 分析失败: {}", e);
                        }
                    }
                    
                    // 添加延迟以避免过于频繁的请求
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        analyzer.config.processing.request_delay_ms
                    )).await;
                    
                    Ok(())
                }
            })
            .buffer_unordered(self.config.processing.concurrent_requests)
            .collect::<Vec<Result<(), anyhow::Error>>>()
            .await
            .into_iter()
            .collect();
        
        update_results?;
        
        println!("🎉 所有单词解析更新完成！");
        Ok(())
    }

    // 根据ID更新单词解析
    pub async fn update_word_analysis_by_id(&self, id: i64) -> Result<()> {
        println!("🔄 开始根据ID更新单词解析...");
        
        // 获取指定ID的单词
        let word = match self.db_manager.get_word_by_id(id).await? {
            Some(word) => word,
            None => {
                println!("❌ 未找到ID为 {} 的单词", id);
                return Ok(());
            }
        };
        
        println!("📝 找到单词: {} ({}) - {}", word.word, word.kana, word.part_of_speech);
        
        // 复用现有的分析逻辑
        let parts_of_speech: Vec<&str> = word.part_of_speech.split('｜').collect();
        let parts_of_speech_vec: Vec<String> = parts_of_speech.iter().map(|s| s.to_string()).collect();
        
        let basic_word = BasicWordInfo {
            word: word.word.clone(),
            kana: word.kana.clone(),
            pitch: word.pitch.clone(),
            part_of_speech: parts_of_speech_vec.clone(),
        };
        
        match self.analyze_word_with_multiple_pos(&basic_word, &parts_of_speech_vec).await {
            Ok(new_analysis) => {
                // 检查解析是否有变化
                if word.analysis != new_analysis {
                    println!("🔄 解析更新: 长度 {} -> {}", 
                        word.analysis.len(), new_analysis.len());
                    
                    // 更新数据库中的解析
                    self.db_manager.update_word_analysis(id, &new_analysis).await?;
                    println!("✅ 单词解析更新成功");
                } else {
                    println!("✅ 解析无变化，跳过更新");
                }
            },
            Err(e) => {
                println!("❌ 分析失败: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }

    // 第一步：提取单词和语法的基本信息
    pub async fn extract_words_and_grammar(&self, text: &str) -> Result<ExtractionResult> {
        let prompt = format!(r#"
请分析以下日语文本，提取出所有单词和语法点的基本信息：

1. 单词部分：
   - 将所有单词转换为辞书形（原形）
   - 提供假名读音
   - 提供音调（用0-9数字表示）
   - 确定词性（请精确标注）

2. 语法部分：
   - 识别语法结构和表达方式
   - 提供假名读音

重要词性标注规则：
- 动词必须明确标注为"自动词"或"他动词"，不要只写"动词"
- 如果一个词既是自动词又是他动词，就标注["自动词", "他动词"]
- 形容词分为"一类形容词"和"二类形容词"
- 一律使用简体中文词性：名词、自动词、他动词、一类形容词、二类形容词、副词、连词、助词、感叹词等
- 不要出现重复标注如"動詞｜自動詞"

请用以下JSON格式返回结果（只需要基本信息，不需要详细解释）：
{{
  "words": [
    {{
      "word": "単语辞书形",
      "kana": "かな",
      "pitch": "0",
      "part_of_speech": ["名词", "他动词"]
    }}
  ],
  "grammar": [
    {{
      "grammar": "语法表达",
      "kana": "かな"
    }}
  ]
}}

要分析的文本：
{}
"#, text);

        let request = OpenRouterRequest {
            model: self.config.api.models.extraction_model.clone(),
            messages: vec![RequestMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            max_tokens: 100000,
            temperature: 0.1,
        };

        let api_response = self.api_client.make_request_with_retry(&request).await?;
        let content = &api_response.choices[0].message.content;
        
        // 提取JSON部分
        let json_start = content.find('{').unwrap_or(0);
        let json_end = content.rfind('}').map(|i| i + 1).unwrap_or(content.len());
        let json_content = &content[json_start..json_end];
        
        // 解析提取结果
        let extraction: ExtractionResult = serde_json::from_str(json_content)
            .map_err(|e| anyhow::anyhow!("解析提取结果失败: {}\n响应内容: {}", e, json_content))?;
        
        Ok(extraction)
    }

    // 第二步：详细分析单个单词（支持多词性）
    pub async fn analyze_word_with_multiple_pos(&self, word: &BasicWordInfo, parts_of_speech: &[String]) -> Result<String> {
        let pos_list = parts_of_speech.join("、");
        let prompt = format!(r#"
请分析这个日语单词的用法，以纯HTML格式回复，参考以下示例格式：

示例（单词：帯，假名：おび，音调：obi，词性：名词）：
<div>「帯」（おび、obi）是一个日语名词，意思是<b>"腰带"、"带子"或"地带"</b>。它是一个非常通用的词，根据不同的语境有不同的含义，但核心都与"带状物"或"区域"有关。</div>
<hr>
<div>1. 服饰上的"腰带" 👘<br>
这是最常见、最核心的用法。特指系在和服、浴衣等传统日本服饰上的宽腰带。<br>
例： 帯を締める (obi o shimeru) - 系腰带。<br>
例： 着物と帯 (kimono to obi) - 和服和腰带。<br><br>
2. "地带"、"区域" 🗺️<br>
带有比喻色彩，指某个具有特定特征的带状区域。<br>
例： 台風の帯 (taifū no obi) - 台风带。<br>
例： 火山帯 (kazan tai) - 火山带。</div>
<hr>
<div>「帯」这个汉字本身就带有<b>"束缚"、"捆绑"或"带状"</b>的含义。在日语中，它完美地保留了这些核心概念，从具体的服饰腰带到抽象的地理区域，都用这个词来表达。不同语境下，重点会从具体的物理对象转向抽象的概念性区域。</div>
<hr>
<div>总的来说，「帯」的核心概念是<b>"带状物"或"带状区域"</b>，它可以指实际的物品，也可以指抽象的概念。不同语境下，重点会从具体的物理对象转向抽象的概念性区域。</div>
<hr>
<div><b>词汇对比：</b><br><br>
<b>「帯」vs「ベルト」：</b> 「ベルト」是外来词，多指现代服饰的皮带，而「帯」更偏向传统文化，如和服腰带。<br><br>
<b>「帯」vs「紐」：</b> 「紐」通常指细绳、细带，「帯」则指较宽的带状物，且更正式。<br><br>
<b>「帯」vs「バンド」：</b> 「バンド」多用于技术或医疗领域（如频段、绷带），「帯」更多用于地理和服饰领域。</div>

现在请按照上述格式分析：

单词：{}
假名：{}
音调：{}
词性：{}

重要事项：
1. 直接回复HTML内容，不要使用markdown代码块格式
2. 不要添加```html```标记
3. 粗体使用<b></b>标签，绝不要使用**符号
4. 如果有多个词性，请全面分析所有词性的用法
5. 不要重复模板化的标题
"#, word.word, word.kana, word.pitch, pos_list);

        let request = OpenRouterRequest {
            model: self.config.api.models.word_analysis_model.clone(),
            messages: vec![RequestMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            max_tokens: 100000,
            temperature: 0.1,
        };

        let api_response = self.api_client.make_request_with_retry(&request).await?;
        let analysis = &api_response.choices[0].message.content.trim();
        
        Ok(analysis.to_string())
    }

    // 第二步：详细分析单个语法
    pub async fn analyze_grammar(&self, grammar: &BasicGrammarInfo) -> Result<String> {
        let prompt = format!(r#"
请详细分析这个日语语法点：

语法：{}
假名：{}

请提供：
1. 详细的中文解释
2. 语法功能和意义
3. 使用场合和语境
4. 接续方法（前后可以接什么）
5. 用法例句和注意点
6. 相似语法的区别

请只返回详细的中文分析内容，不需要JSON格式。
"#, grammar.grammar, grammar.kana);

        let request = OpenRouterRequest {
            model: self.config.api.models.grammar_analysis_model.clone(),
            messages: vec![RequestMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            max_tokens: 100000,
            temperature: 0.1,
        };

        let api_response = self.api_client.make_request_with_retry(&request).await?;
        let analysis = &api_response.choices[0].message.content.trim();
        
        Ok(analysis.to_string())
    }

    // 生成单词 Anki 卡片
    pub async fn generate_word_cards(&self) -> Result<()> {
        let words = self.db_manager.get_all_words().await?;
        generate_word_cards(&words, &self.config.output.words_file)?;
        Ok(())
    }

    // 生成语法 Anki 卡片  
    pub async fn generate_grammar_cards(&self) -> Result<()> {
        let grammar = self.db_manager.get_all_grammar().await?;
        generate_grammar_cards(&grammar, &self.config.output.grammar_file)?;
        Ok(())
    }

    // 只处理单词的函数
    pub async fn process_words_only(&self, text: &str) -> Result<()> {
        let text_length = text.chars().count();
        println!("📝 输入文本长度: {} 字符", text_length);
        
        println!("🔄 第一步：提取单词...");
        
        // 直接处理整个文本，不再分块
        let extraction = self.extract_words_and_grammar(text).await?;
        
        println!("📝 找到 {} 个单词", extraction.words.len());

        println!("🔄 第二步：按单词分组并检查重复...");
        
        // 按单词（word+kana+pitch）分组，合并相同单词的不同词性
        let mut word_groups: HashMap<(String, String, String), Vec<String>> = HashMap::new();
        
        for word in extraction.words.iter() {
            let key = (word.word.clone(), word.kana.clone(), word.pitch.clone());
            let group = word_groups.entry(key).or_insert_with(Vec::new);
            
            // 合并词性，避免重复
            for pos in &word.part_of_speech {
                if !group.contains(pos) {
                    group.push(pos.clone());
                }
            }
        }
        
        // 检查哪些单词已存在，哪些需要分析
        let mut words_to_analyze = Vec::new();
        let mut words_to_update: Vec<(i64, String, String)> = Vec::new();
        let mut skipped_count = 0;
        
        for ((word, kana, pitch), parts_of_speech) in word_groups.iter() {
            let exists = self.db_manager.check_word_exists(word, kana).await?;
            
            if exists {
                // 获取已存在的单词信息
                if let Some(existing_word) = self.db_manager.get_existing_word_by_word_kana(word, kana).await? {
                    let new_pos_str = parts_of_speech.join("｜");
                    
                    // 检查是否需要更新 pitch 或词性
                    if existing_word.pitch != *pitch || existing_word.part_of_speech != new_pos_str {
                        println!("  🔄 更新已存在单词: {} ({}) - pitch: {}->{}, pos: {}->{}", 
                            word, kana, 
                            existing_word.pitch, pitch,
                            existing_word.part_of_speech, new_pos_str
                        );
                        
                        words_to_update.push((existing_word.id, pitch.clone(), new_pos_str));
                    } else {
                        skipped_count += 1;
                        println!("  ✅ 跳过已存在的单词（无变化）: {} ({})", word, kana);
                    }
                } else {
                    // 理论上不应该到这里，但为了安全起见
                    skipped_count += 1;
                    println!("  ⚠️不应该到这里，跳过已存在的单词: {} ({})", word, kana);
                }
            } else {
                let basic_word = BasicWordInfo {
                    word: word.clone(),
                    kana: kana.clone(),
                    pitch: pitch.clone(),
                    part_of_speech: parts_of_speech.clone(),
                };
                words_to_analyze.push((basic_word, parts_of_speech.clone()));
            }
        }

        println!("  跳过 {} 个已存在的单词，需要更新 {} 个单词，需要分析 {} 个新单词", 
            skipped_count, 
            words_to_update.len(),
            words_to_analyze.len()
        );

        // 先更新已存在的单词
        if !words_to_update.is_empty() {
            println!("🔄 更新已存在单词的 pitch 和词性...");
            for (id, new_pitch, new_pos) in words_to_update {
                if let Err(e) = self.db_manager.update_word_pitch_and_pos(id, &new_pitch, &new_pos).await {
                    println!("  ❌ 更新失败: ID {} - {}", id, e);
                } else {
                    println!("  ✅ 更新成功: ID {} - pitch: {}, pos: {}", id, new_pitch, new_pos);
                }
            }
        }
        
        // 使用并发流处理所有单词
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.processing.concurrent_requests));
        
        let word_analyses_results: Result<Vec<Vec<WordAnalysis>>, anyhow::Error> = stream::iter(words_to_analyze.into_iter().enumerate())
            .map(|(i, (word, parts_of_speech))| {
                let semaphore = semaphore.clone();
                let analyzer = self;
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    let pos_display = parts_of_speech.join("、");
                    println!("  分析单词 {}: {} ({})", i + 1, word.word, pos_display);
                    let analysis = analyzer.analyze_word_with_multiple_pos(&word, &parts_of_speech).await?;
                    
                    // 为每个单词创建一个WordAnalysis记录，所有词性用｜分隔
                    let merged_parts_of_speech = parts_of_speech.join("｜");
                    let word_analysis = WordAnalysis {
                        word: word.word.clone(),
                        kana: word.kana.clone(),
                        pitch: word.pitch.clone(),
                        part_of_speech: merged_parts_of_speech,
                        analysis: analysis,
                    };
                    
                    Ok(vec![word_analysis])
                }
            })
            .buffer_unordered(10) // 允许最多10个并发任务
            .collect::<Vec<Result<Vec<WordAnalysis>, anyhow::Error>>>()
            .await
            .into_iter()
            .collect();
        
        // 展平结果
        let new_word_analyses: Vec<WordAnalysis> = word_analyses_results?.into_iter().flatten().collect();

        println!("💾 保存分析结果到数据库...");

        // 保存新分析的单词到数据库
        if !new_word_analyses.is_empty() {
            self.db_manager.save_words(&new_word_analyses).await?;
            println!("  ✅ 保存了 {} 个新单词到数据库", new_word_analyses.len());
        } else {
            println!("  ℹ️  没有新单词需要保存");
        }

        println!("📄 生成单词 Anki 卡片文件...");

        // 生成单词 Anki 卡片
        self.generate_word_cards().await?;
        
        Ok(())
    }

    // 只处理语法的函数
    pub async fn process_grammar_only(&self, text: &str) -> Result<()> {
        let text_length = text.chars().count();
        println!("📝 输入文本长度: {} 字符", text_length);
        
        println!("🔄 第一步：提取语法...");
        
        // 直接处理整个文本，不再分块
        let extraction = self.extract_words_and_grammar(text).await?;
        
        println!("📝 找到 {} 个语法点", extraction.grammar.len());
        
        println!("🔄 第二步：并发详细分析每个语法点...");
        
        // 使用并发处理语法分析
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.processing.concurrent_requests));
        
        let grammar_analyses: Result<Vec<GrammarAnalysis>, anyhow::Error> = stream::iter(extraction.grammar.into_iter().enumerate())
            .map(|(i, grammar)| {
                let semaphore = semaphore.clone();
                let analyzer = self;
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    println!("  分析语法 {}: {}", i + 1, grammar.grammar);
                    let analysis = analyzer.analyze_grammar(&grammar).await?;
                    
                    Ok(GrammarAnalysis {
                        grammar: grammar.grammar.clone(),
                        kana: grammar.kana.clone(),
                        analysis,
                    })
                }
            })
            .buffer_unordered(10)
            .collect::<Vec<Result<GrammarAnalysis, anyhow::Error>>>()
            .await
            .into_iter()
            .collect();
        
        let grammar_analyses = grammar_analyses?;

        println!("💾 保存分析结果到数据库...");
        
        self.db_manager.save_grammar(&grammar_analyses).await?;
        
        println!("📄 生成语法 Anki 卡片文件...");

        // 生成语法 Anki 卡片
        self.generate_grammar_cards().await?;
        
        Ok(())
    }

    /// 增量生成音频文件
    pub async fn generate_missing_audio_files(&self) -> Result<()> {
        println!("🎵 开始增量生成音频文件...");
        
        // 确保音频目录存在
        std::fs::create_dir_all(&self.config.output.audio_dir)
            .map_err(|e| anyhow::anyhow!("无法创建音频目录 {}: {}", self.config.output.audio_dir, e))?;
        
        // 获取所有单词
        let words = self.db_manager.get_all_words().await?;
        
        if words.is_empty() {
            println!("⚠️  数据库中没有找到任何单词");
            return Ok(());
        }
        
        println!("📊 找到 {} 个单词，检查缺失的音频文件...", words.len());
        
        // 创建 TTS 客户端
        let tts_config = TtsConfig::from_config(&self.config.tts);
        let tts = AzureTts::new(tts_config);
        
        let mut missing_count = 0;
        let mut generated_count = 0;
        
        // 使用并发流处理所有单词
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.processing.concurrent_requests));
        
        let total_words = words.len();
        let results: Vec<Result<(), anyhow::Error>> = stream::iter(words.into_iter().enumerate())
            .map(|(i, word)| {
                let semaphore = semaphore.clone();
                let tts = &tts;
                let audio_dir = &self.config.output.audio_dir;
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    let audio_filename = format!("japanese_word_{}.wav", word.id);
                    let audio_path = std::path::Path::new(audio_dir).join(&audio_filename);
                    
                    // 检查音频文件是否存在
                    if audio_path.exists() {
                        println!("  ✅ 音频文件已存在: {} ({})", audio_filename, word.word);
                        return Ok(());
                    }
                    
                    println!("  🎵 生成音频文件 {}/{}: {} ({})", 
                        i + 1, total_words, audio_filename, word.kana);
                    
                    // 生成音频文件，使用假名（发音）而不是汉字
                    match tts.synthesize_text_to_file(&word.kana, &audio_path.to_string_lossy()).await {
                        Ok(_) => {
                            println!("  ✅ 音频文件生成成功: {}", audio_filename);
                            Ok(())
                        },
                        Err(e) => {
                            println!("  ❌ 音频文件生成失败: {} - {}", audio_filename, e);
                            Err(e)
                        }
                    }
                }
            })
            .buffer_unordered(self.config.processing.concurrent_requests)
            .collect::<Vec<_>>()
            .await;
        
        // 统计结果
        for result in results {
            match result {
                Ok(_) => generated_count += 1,
                Err(_) => missing_count += 1,
            }
        }
        
        println!("\n🎉 音频文件生成完成！");
        println!("   ✅ 成功生成: {} 个音频文件", generated_count);
        if missing_count > 0 {
            println!("   ❌ 生成失败: {} 个音频文件", missing_count);
        }
        println!("   📁 音频文件目录: {}", self.config.output.audio_dir);
        
        Ok(())
    }


}
