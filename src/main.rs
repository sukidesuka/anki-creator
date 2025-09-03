use anyhow::Result;
use anki_creator::{Config, AnkiCreator};

// 显示主菜单并获取用户选择
fn show_menu() -> Result<i32> {
    println!("\n🎌 日语 Anki 卡片生成器");
    println!("请选择功能：");
    println!("1. 解析单词");
    println!("2. 解析语法");
    println!("3. 更新所有单词词性");
    println!("4. 重新生成卡片文件");
    println!("5. 更新所有单词解析");
    println!("6. 根据ID更新单词解析");
    println!("0. 退出程序");
    print!("请输入选项 (0-6): ");
    
    use std::io::{self, Write};
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let choice = input.trim().parse::<i32>().unwrap_or(-1);
    Ok(choice)
}

#[tokio::main]
async fn main() -> Result<()> {
    // 加载配置文件
    let config = Config::load().map_err(|e| {
        println!("❌ 配置文件加载失败: {}", e);
        println!("💡 请确保 config.toml 文件存在并包含必要的配置");
        e
    })?;

    println!("✅ 配置文件加载成功");
    println!("   并发请求数: {}", config.processing.concurrent_requests);
    println!("   数据库文件: {}", config.database.db_file);

    // 创建 Anki 卡片生成器
    let creator = AnkiCreator::new(config).await?;

    loop {
        match show_menu()? {
            1 => {
                // 解析单词
                println!("\n📖 读取输入文件: {}", creator.config.input.text_file);
                let text_content = match std::fs::read_to_string(&creator.config.input.text_file) {
                    Ok(content) => {
                        if content.trim().is_empty() {
                            println!("⚠️  警告: 输入文件为空");
                            continue;
                        }
                        content
                    },
                    Err(e) => {
                        println!("❌ 无法读取输入文件 '{}': {}", creator.config.input.text_file, e);
                        println!("💡 请检查文件路径是否正确，文件是否存在");
                        continue;
                    }
                };

                println!("✅ 文件读取成功，内容长度: {} 字符", text_content.chars().count());
                
                // 显示文件内容的前100个字符作为预览
                let preview = if text_content.chars().count() > 100 {
                    format!("{}...", text_content.chars().take(100).collect::<String>())
                } else {
                    text_content.clone()
                };
                println!("📝 文本预览: {}", preview);

                // 只处理单词
                match creator.process_words_only(&text_content).await {
                    Ok(_) => {
                        println!("\n🎉 单词解析完成！生成的文件：");
                        println!("   📄 {} - 单词卡片", creator.config.output.words_file);
                        println!("   🗄️  {} - SQLite 数据库", creator.config.database.db_file);
                    },
                    Err(e) => {
                        println!("❌ 处理单词时出错: {}", e);
                    }
                }
            },
            2 => {
                // 解析语法
                println!("\n📖 读取输入文件: {}", creator.config.input.text_file);
                let text_content = match std::fs::read_to_string(&creator.config.input.text_file) {
                    Ok(content) => {
                        if content.trim().is_empty() {
                            println!("⚠️  警告: 输入文件为空");
                            continue;
                        }
                        content
                    },
                    Err(e) => {
                        println!("❌ 无法读取输入文件 '{}': {}", creator.config.input.text_file, e);
                        println!("💡 请检查文件路径是否正确，文件是否存在");
                        continue;
                    }
                };

                println!("✅ 文件读取成功，内容长度: {} 字符", text_content.chars().count());
                
                // 显示文件内容的前100个字符作为预览
                let preview = if text_content.chars().count() > 100 {
                    format!("{}...", text_content.chars().take(100).collect::<String>())
                } else {
                    text_content.clone()
                };
                println!("📝 文本预览: {}", preview);

                // 只处理语法
                match creator.process_grammar_only(&text_content).await {
                    Ok(_) => {
                        println!("\n🎉 语法解析完成！生成的文件：");
                        println!("   📄 {} - 语法卡片", creator.config.output.grammar_file);
                        println!("   🗄️  {} - SQLite 数据库", creator.config.database.db_file);
                    },
                    Err(e) => {
                        println!("❌ 处理语法时出错: {}", e);
                    }
                }
            },
            3 => {
                // 更新所有单词词性
                println!("\n🔄 开始更新所有单词词性功能...");
                match creator.update_all_word_parts_of_speech().await {
                    Ok(_) => {
                        println!("✅ 词性更新完成");
                        
                        // 询问是否重新生成卡片
                        println!("\n是否重新生成卡片文件？(y/N): ");
                        use std::io::{self, Write};
                        io::stdout().flush().unwrap();
                        
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        
                        if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                            match creator.generate_word_cards().await {
                                Ok(_) => println!("✅ 单词卡片重新生成完成"),
                                Err(e) => println!("❌ 生成单词卡片时出错: {}", e),
                            }
                        }
                    },
                    Err(e) => {
                        println!("❌ 更新词性时出错: {}", e);
                    }
                }
            },
            4 => {
                // 重新生成卡片文件
                println!("\n📄 重新生成卡片文件...");
                match creator.generate_word_cards().await {
                    Ok(_) => {
                        match creator.generate_grammar_cards().await {
                            Ok(_) => {
                                println!("✅ 所有卡片文件重新生成完成");
                                println!("   📄 {} - 单词卡片", creator.config.output.words_file);
                                println!("   📄 {} - 语法卡片", creator.config.output.grammar_file);
                            },
                            Err(e) => println!("❌ 生成语法卡片时出错: {}", e),
                        }
                    },
                    Err(e) => println!("❌ 生成单词卡片时出错: {}", e),
                }
            },
            5 => {
                // 更新所有单词解析
                println!("\n🔄 开始更新所有单词解析功能...");
                match creator.update_all_word_analysis().await {
                    Ok(_) => {
                        println!("✅ 所有单词解析更新完成");
                        
                        // 询问是否重新生成卡片
                        println!("\n是否重新生成卡片文件？(y/N): ");
                        use std::io::{self, Write};
                        io::stdout().flush().unwrap();
                        
                        let mut input = String::new();
                        io::stdin().read_line(&mut input).unwrap();
                        
                        if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                            match creator.generate_word_cards().await {
                                Ok(_) => println!("✅ 单词卡片重新生成完成"),
                                Err(e) => println!("❌ 生成单词卡片时出错: {}", e),
                            }
                        }
                    },
                    Err(e) => {
                        println!("❌ 更新单词解析时出错: {}", e);
                    }
                }
            },
            6 => {
                // 根据ID更新单词解析
                println!("\n🔄 根据ID更新单词解析功能...");
                print!("请输入要更新的单词ID: ");
                use std::io::{self, Write};
                io::stdout().flush().unwrap();
                
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                match input.trim().parse::<i64>() {
                    Ok(id) => {
                        match creator.update_word_analysis_by_id(id).await {
                            Ok(_) => {
                                println!("✅ 单词解析更新完成");
                                
                                // 询问是否重新生成卡片
                                println!("\n是否重新生成卡片文件？(y/N): ");
                                io::stdout().flush().unwrap();
                                
                                let mut input = String::new();
                                io::stdin().read_line(&mut input).unwrap();
                                
                                if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                                    match creator.generate_word_cards().await {
                                        Ok(_) => println!("✅ 单词卡片重新生成完成"),
                                        Err(e) => println!("❌ 生成单词卡片时出错: {}", e),
                                    }
                                }
                            },
                            Err(e) => {
                                println!("❌ 更新单词解析时出错: {}", e);
                            }
                        }
                    },
                    Err(_) => {
                        println!("❌ 无效的ID，请输入一个有效的数字");
                    }
                }
            },
            0 => {
                println!("👋 再见！");
                break;
            },
            _ => {
                println!("❌ 无效选项，请输入 0-6 之间的数字");
            }
        }
        
        println!("\n📋 使用说明：");
        println!("1. 在 Anki 中导入 CSV 文件");
        println!("2. 确保字段映射正确（ID 字段用于更新现有卡片）");
        println!("3. 单词和语法会创建为不同的卡组");
        
        println!("\n按 Enter 键继续...");
        let mut _input = String::new();
        std::io::stdin().read_line(&mut _input)?;
    }
    
    Ok(())
}
