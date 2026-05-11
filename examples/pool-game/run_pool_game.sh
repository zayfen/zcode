#!/bin/bash
set -euo pipefail

if [ -z "${ZCODE_API_KEY:-}" ]; then
  echo "ZCODE_API_KEY is required" >&2
  exit 1
fi

ZCODE_BASE_URL="${ZCODE_BASE_URL:-https://open.bigmodel.cn/api/paas/v4}" \
  /Users/riven/Github/zcode/target/debug/zcode run "Execute all tasks in docs/tasks/001-pool-game.tasks.md sequentially to re-execute pool-game." -m "glm-5.1" > zcode_execution.log 2>&1
