# ⚙️ 配置指南

## 📋 概述

本指南详细说明了 `config.toml` 配置文件的各个选项，帮助你根据需求优化程序性能和行为。

## 📁 配置文件位置

- **配置文件**：`config.toml`
- **模板文件**：`config.example`
- **默认位置**：项目根目录

## 🔧 配置项详解

### 🌐 API 配置

```toml
[api]
# OpenRouter API 密钥
openrouter_key = "sk-or-v1-your-api-key-here"
```

#### openrouter_key
- **类型**：字符串
- **必需**：是
- **说明**：OpenRouter API 密钥，用于访问 Google Gemini-2.5-Flash 模型
- **获取方式**：
  1. 访问 [OpenRouter](https://openrouter.ai/)
  2. 注册账号并登录
  3. 在 API Keys 页面创建新的密钥
  4. 复制密钥并粘贴到配置文件中

**安全提示**：
- ⚠️ 不要将包含真实 API 密钥的配置文件提交到版本控制系统
- 🔒 建议使用环境变量覆盖：`export OPENROUTER_API_KEY=your_key`
- 🛡️ 定期轮换 API 密钥以确保安全

### 🚀 处理配置

```toml
[processing]
# 并发处理数量
concurrent_requests = 20
# 请求之间的延迟（毫秒）
request_delay_ms = 100
# 最大重试次数
max_retries = 3
# 请求超时时间（秒）
request_timeout_seconds = 180
```

#### concurrent_requests
- **类型**：整数
- **默认值**：20
- **范围**：1-50
- **说明**：同时处理的并发请求数量
- **影响**：
  - 数值越大，处理速度越快
  - 数值过大可能导致 API 限制或网络拥塞
  - 建议根据网络状况和 API 限制调整

**推荐值**：
- 🐌 慢速网络：5-10
- 🚀 快速网络：15-25
- ⚡ 企业网络：20-30

#### request_delay_ms
- **类型**：整数
- **默认值**：100
- **范围**：50-1000
- **单位**：毫秒
- **说明**：请求之间的延迟时间
- **作用**：
  - 避免触发 API 频率限制
  - 减少服务器负载
  - 提高请求成功率

**推荐值**：
- 🆓 免费 API：200-500ms
- 💰 付费 API：50-200ms
- 🏢 企业 API：100-300ms

#### max_retries
- **类型**：整数
- **默认值**：3
- **范围**：1-10
- **说明**：请求失败时的最大重试次数
- **重试条件**：
  - 网络超时
  - 服务器错误（5xx）
  - 临时性错误

**推荐值**：
- 🌐 稳定网络：2-3
- 📶 不稳定网络：5-7
- 🔄 高可靠性需求：3-5

#### request_timeout_seconds
- **类型**：整数
- **默认值**：180
- **范围**：30-600
- **单位**：秒
- **说明**：单个请求的超时时间
- **影响**：
  - 超时时间过短可能导致请求失败
  - 超时时间过长可能导致程序卡死

**推荐值**：
- 🚀 快速 API：60-120s
- 🐌 慢速 API：180-300s
- 🔄 批量处理：120-240s

### 🗄️ 数据库配置

```toml
[database]
# 数据库文件名
db_file = "anki_cards.db"
```

#### db_file
- **类型**：字符串
- **默认值**：`"anki_cards.db"`
- **说明**：SQLite 数据库文件路径
- **支持**：
  - 相对路径：相对于项目根目录
  - 绝对路径：完整的文件系统路径
  - 文件名：在当前目录创建

**示例**：
```toml
# 相对路径
db_file = "data/anki_cards.db"

# 绝对路径
db_file = "/home/user/anki_data/cards.db"

# 文件名（推荐）
db_file = "anki_cards.db"
```

### 📥 输入配置

```toml
[input]
# 输入文件路径
text_file = "input.txt"
```

#### text_file
- **类型**：字符串
- **默认值**：`"input.txt"`
- **说明**：包含日语文本的输入文件路径
- **格式要求**：
  - 纯文本文件（.txt）
  - UTF-8 编码
  - 支持换行和特殊字符

**示例**：
```toml
# 相对路径
text_file = "data/japanese_text.txt"

# 绝对路径
text_file = "/home/user/documents/lesson1.txt"

# 文件名（推荐）
text_file = "input.txt"
```

### 📤 输出配置

```toml
[output]
# 输出文件名
words_file = "japanese_words.csv"
grammar_file = "japanese_grammar.csv"
```

#### words_file
- **类型**：字符串
- **默认值**：`"japanese_words.csv"`
- **说明**：单词卡片的输出 CSV 文件路径
- **格式**：CSV 格式，包含 id, word, kana, analysis 字段

#### grammar_file
- **类型**：字符串
- **默认值**：`"japanese_grammar.csv"`
- **说明**：语法卡片的输出 CSV 文件路径
- **格式**：CSV 格式，包含 id, grammar, kana, analysis 字段

## 🎯 配置优化建议

### 🚀 性能优化

#### 高并发配置
```toml
[processing]
concurrent_requests = 30
request_delay_ms = 50
max_retries = 2
request_timeout_seconds = 120
```

**适用场景**：
- 大批量文本处理
- 稳定的网络环境
- 付费 API 服务

#### 稳定优先配置
```toml
[processing]
concurrent_requests = 10
request_delay_ms = 200
max_retries = 5
request_timeout_seconds = 300
```

**适用场景**：
- 不稳定的网络环境
- 免费 API 服务
- 对成功率要求高

#### 平衡配置（推荐）
```toml
[processing]
concurrent_requests = 20
request_delay_ms = 100
max_retries = 3
request_timeout_seconds = 180
```

**适用场景**：
- 大多数使用场景
- 中等规模的文本处理
- 平衡速度和稳定性

### 🔒 安全配置

#### 环境变量覆盖
```bash
# 设置环境变量
export OPENROUTER_API_KEY="your-secret-key"

# 配置文件中的占位符
[api]
openrouter_key = "your-api-key-here"
```

#### 配置文件权限
```bash
# 设置配置文件权限（仅所有者可读写）
chmod 600 config.toml

# 检查权限
ls -l config.toml
```

### 📊 监控配置

#### 调试模式
```toml
[processing]
concurrent_requests = 5
request_delay_ms = 500
max_retries = 1
request_timeout_seconds = 60
```

**特点**：
- 降低并发数便于调试
- 增加延迟便于观察
- 减少重试次数快速失败

## 🛠️ 配置验证

### 自动验证
程序启动时会自动验证配置：
- ✅ 检查必需字段是否存在
- ✅ 验证数值范围是否合理
- ✅ 确认文件路径是否有效
- ✅ 测试 API 密钥是否可用

### 手动验证
```bash
# 检查配置文件语法
toml-cli validate config.toml

# 检查文件权限
ls -l config.toml

# 测试 API 连接
curl -H "Authorization: Bearer $OPENROUTER_API_KEY" \
     https://openrouter.ai/api/v1/models
```

## 🔄 配置更新

### 热重载
程序支持配置热重载：
1. 修改配置文件
2. 程序会自动检测变化
3. 重新加载配置（部分功能）

### 重启生效
某些配置需要重启程序：
- API 密钥更改
- 数据库文件路径更改
- 输出文件路径更改

## 📝 配置示例

### 完整配置示例
```toml
# Anki Creator 配置文件
# 请保护好你的API密钥，不要将此文件提交到版本控制系统

[api]
# OpenRouter API 密钥
openrouter_key = "sk-or-v1-your-api-key-here"

[processing]
# 并发处理数量（建议 10-30）
concurrent_requests = 20
# 请求间延迟（毫秒）
request_delay_ms = 100
# 最大重试次数
max_retries = 3
# 请求超时时间（秒）
request_timeout_seconds = 180

[database]
# 数据库文件名
db_file = "anki_cards.db"

[input]
# 输入文件路径
text_file = "input.txt"

[output]
# 输出文件名
words_file = "japanese_words.csv"
grammar_file = "japanese_grammar.csv"
```

### 开发环境配置
```toml
[api]
openrouter_key = "sk-or-v1-dev-key"

[processing]
concurrent_requests = 5
request_delay_ms = 500
max_retries = 1
request_timeout_seconds = 60

[database]
db_file = "dev_anki_cards.db"

[input]
text_file = "test_input.txt"

[output]
words_file = "dev_words.csv"
grammar_file = "dev_grammar.csv"
```

### 生产环境配置
```toml
[api]
openrouter_key = "sk-or-v1-prod-key"

[processing]
concurrent_requests = 25
request_delay_ms = 80
max_retries = 3
request_timeout_seconds = 240

[database]
db_file = "/data/anki_cards.db"

[input]
text_file = "/data/input.txt"

[output]
words_file = "/data/japanese_words.csv"
grammar_file = "/data/japanese_grammar.csv"
```

## 🚨 常见配置错误

### 1. API 密钥错误
```
❌ 错误：API key not found
✅ 解决：检查 openrouter_key 配置项
```

### 2. 并发数过高
```
❌ 错误：Too many concurrent requests
✅ 解决：降低 concurrent_requests 值
```

### 3. 文件路径错误
```
❌ 错误：File not found
✅ 解决：检查文件路径是否正确
```

### 4. 权限问题
```
❌ 错误：Permission denied
✅ 解决：检查文件读写权限
```

## 📚 相关文档

- [README.md](./README.md) - 项目概述和快速开始
- [PROJECT_STRUCTURE.md](./PROJECT_STRUCTURE.md) - 项目结构说明
- [USAGE_GUIDE.md](./USAGE_GUIDE.md) - 详细使用指南

---

通过合理配置这些选项，你可以根据具体需求优化程序的性能、稳定性和安全性。建议从默认配置开始，根据实际使用情况逐步调整。
