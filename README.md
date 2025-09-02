# 日语 Anki 卡片生成器

这是一个自动生成日语单词和语法 Anki 卡片的工具，使用 OpenRouter API 和 Google Gemini-2.5-Flash 模型进行文本分析。

## 功能特性

- 📝 分析日语文本，自动提取单词和语法点
- 🔤 将单词转换为辞书形（原形）
- 🗾 提供假名读音和音调标注（0-4）
- 💾 存储到 SQLite 数据库
- 📄 生成可直接导入 Anki 的 CSV 文件
- 🔄 支持更新现有卡片（通过 ID 字段）
- 📚 自动创建两个不同的卡组（单词和语法）

## 安装和配置

### 1. 克隆项目
```bash
cd /path/to/your/projects
git clone <your-repo-url>
cd anki-creator
```

### 2. 获取 OpenRouter API 密钥
1. 访问 [OpenRouter](https://openrouter.ai/)
2. 注册账号并获取 API 密钥
3. 设置环境变量：
```bash
export OPENROUTER_API_KEY=your_api_key_here
```

### 3. 编译项目
```bash
cargo build --release
```

## 使用方法

### 快速开始
```bash
# 设置 API 密钥
export OPENROUTER_API_KEY=your_api_key_here

# 运行程序
cargo run
```

### 自定义文本分析
修改 `src/main.rs` 中的 `sample_text` 变量，替换为你想要分析的日语文本：

```rust
let sample_text = r#"
你的日语文本在这里...
"#;
```

### 输出文件
运行后会生成以下文件：
- `japanese_words.csv` - 单词卡片
- `japanese_grammar.csv` - 语法卡片  
- `anki_cards.db` - SQLite 数据库

## CSV 文件格式

### 单词卡片 (japanese_words.csv)
```csv
id,word,kana,analysis
1,"行く","いく","去，走 [音调: 0] 详细解释..."
2,"今日","きょう","今天 [音调: 1] 详细解释..."
```

### 语法卡片 (japanese_grammar.csv)  
```csv
id,grammar,kana,analysis
1,"ましょう","ましょう","表示邀请或建议的语法形式..."
2,"と思います","とおもいます","表示自己的想法或意见..."
```

## 在 Anki 中导入

1. 打开 Anki
2. 选择"文件" → "导入"
3. 选择 CSV 文件
4. 字段映射：
   - 字段 1：ID（用于更新现有卡片）
   - 字段 2：正面内容
   - 字段 3：读音
   - 字段 4：背面内容
5. 选择合适的卡组（为单词和语法创建不同的卡组）
6. 点击"导入"

## 数据库结构

### words 表
- `id`: 主键，自增
- `word`: 单词（辞书形）
- `kana`: 假名读音
- `analysis`: 解释和分析（包含音调信息）
- `created_at`: 创建时间

### grammar 表  
- `id`: 主键，自增
- `word`: 语法表达
- `kana`: 假名读音
- `analysis`: 语法解释和用法
- `created_at`: 创建时间

## API 调用详情

程序使用 OpenRouter 的 API 调用 Google Gemini-2.5-Flash 模型：
- 模型：`google/gemini-2.5-flash`
- 最大 tokens：4000
- 温度：0.1（确保一致性）

## 自定义和扩展

### 修改提示词
在 `analyze_japanese_text` 方法中修改提示词来调整分析结果。

### 添加新字段
1. 修改数据库结构
2. 更新数据结构定义
3. 调整 CSV 输出格式

### 支持其他语言
修改提示词和数据结构即可支持其他语言的学习卡片生成。

## 故障排除

### 常见问题

**API 密钥错误**
```
⚠️  请设置 OPENROUTER_API_KEY 环境变量
```
解决：确保正确设置了环境变量

**API 请求失败**
- 检查网络连接
- 验证 API 密钥是否有效
- 确认 API 配额是否充足

**数据库连接错误**
- 确保有写入当前目录的权限
- 检查磁盘空间是否充足

**JSON 解析错误**
- API 响应格式可能发生变化
- 检查提示词是否正确
- 查看具体的错误信息和响应内容

## 开发和贡献

### 项目结构
```
src/
  main.rs          # 主要应用逻辑
Cargo.toml         # 依赖配置
README.md          # 说明文档
```

### 依赖项
- `tokio`: 异步运行时
- `reqwest`: HTTP 客户端
- `serde`: 序列化/反序列化
- `sqlx`: 数据库操作
- `anyhow`: 错误处理
- `chrono`: 时间处理

## 许可证

[根据需要添加许可证信息]

## 更新日志

### v0.1.0 (初始版本)
- 基本的日语文本分析功能
- SQLite 数据存储
- Anki CSV 导出
- OpenRouter API 集成
