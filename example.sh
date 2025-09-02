#!/bin/bash

# 日语 Anki 卡片生成器 示例脚本

echo "🎌 日语 Anki 卡片生成器 示例"
echo "================================"

# 检查是否设置了 API 密钥
if [ -z "$OPENROUTER_API_KEY" ]; then
    echo "❌ 错误：请先设置 OPENROUTER_API_KEY 环境变量"
    echo ""
    echo "使用方法："
    echo "export OPENROUTER_API_KEY=your_api_key_here"
    echo "./example.sh"
    exit 1
fi

echo "✅ API 密钥已设置"

# 编译项目
echo "🔨 编译项目..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "❌ 编译失败"
    exit 1
fi

echo "✅ 编译完成"

# 运行程序
echo "🚀 开始分析日语文本..."
cargo run

echo ""
echo "📋 生成的文件："
if [ -f "japanese_words.csv" ]; then
    echo "✅ japanese_words.csv (单词卡片)"
    echo "   记录数: $(tail -n +2 japanese_words.csv | wc -l)"
else
    echo "❌ japanese_words.csv 未找到"
fi

if [ -f "japanese_grammar.csv" ]; then
    echo "✅ japanese_grammar.csv (语法卡片)"  
    echo "   记录数: $(tail -n +2 japanese_grammar.csv | wc -l)"
else
    echo "❌ japanese_grammar.csv 未找到"
fi

if [ -f "anki_cards.db" ]; then
    echo "✅ anki_cards.db (数据库)"
else
    echo "❌ anki_cards.db 未找到"
fi

echo ""
echo "📖 下一步："
echo "1. 在 Anki 中创建新的卡组（如：'日语单词'、'日语语法'）"
echo "2. 导入 CSV 文件到对应的卡组"
echo "3. 设置字段映射：ID -> 第1字段，单词 -> 第2字段，等等"
echo "4. 开始学习！"
