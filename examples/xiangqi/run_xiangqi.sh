#!/bin/bash
cd /Users/riven/Github/zcode/examples/xiangqi
../../target/debug/zcode task sync
env ANTHROPIC_API_KEY=b5e665920fb349e7989ce780d245941b.TqOI2itcGmj1cxkw ANTHROPIC_BASE_URL=https://open.bigmodel.cn/api/anthropic ../../target/debug/zcode task run-all -m "glm-5.1" -j 3 > zcode_execution.log 2>&1
