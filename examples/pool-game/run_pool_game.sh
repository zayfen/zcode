#!/bin/bash
env ANTHROPIC_API_KEY=b5e665920fb349e7989ce780d245941b.TqOI2itcGmj1cxkw ANTHROPIC_BASE_URL=https://open.bigmodel.cn/api/anthropic /Users/riven/Github/zcode/target/debug/zcode run "Execute all tasks in docs/tasks/001-pool-game.tasks.md sequentially to re-execute pool-game." -m "glm-5.1" > zcode_execution.log 2>&1
