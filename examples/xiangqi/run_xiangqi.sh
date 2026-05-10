#!/bin/bash
set -euo pipefail

cd /Users/riven/Github/zcode/examples/xiangqi
if [ -z "${ZCODE_API_KEY:-}" ]; then
  echo "ZCODE_API_KEY is required" >&2
  exit 1
fi

../../target/debug/zcode task sync
ZCODE_BASE_URL="${ZCODE_BASE_URL:-https://open.bigmodel.cn/api/paas/v4}" \
  ../../target/debug/zcode task run-all -m "glm-5.1" -j 3 > zcode_execution.log 2>&1
