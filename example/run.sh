#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# zcode example runner
# 使用 BigModel GLM API（Anthropic 兼容模式）运行 zcode
# ─────────────────────────────────────────────────────────────
set -e

ZCODE_BIN="$(dirname "$0")/../target/release/zcode"

# API 参数
export ANTHROPIC_AUTH_TOKEN="811e7c65bfe54ce3aa82cff62c83dd69.7dS9rMab8ewVu3J8"
export ANTHROPIC_BASE_URL="https://open.bigmodel.cn/api/anthropic"
export API_TIMEOUT_MS="3000000"
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1"
export ANTHROPIC_DEFAULT_HAIKU_MODEL="glm-4.5-air"
export ANTHROPIC_DEFAULT_SONNET_MODEL="glm-4.7"
export ANTHROPIC_DEFAULT_OPUS_MODEL="glm-5"

# 进入 example 目录
cd "$(dirname "$0")"
echo "📁 Working directory: $(pwd)"
echo "🔑 API base: $ANTHROPIC_BASE_URL"
echo ""

# 1. 初始化 docs（如果还没有）
if [ ! -f "docs/validation.md" ]; then
  echo "📚 Initializing docs scaffold..."
  "$ZCODE_BIN" docs init
  echo ""
fi

# 2. 验证 docs
echo "🔍 Checking docs..."
"$ZCODE_BIN" docs check
echo ""

# 3. 执行任务
TASK="${1:-create a hello world Python script}"
echo "🚀 Running task: $TASK"
echo ""
"$ZCODE_BIN" run "$TASK"
