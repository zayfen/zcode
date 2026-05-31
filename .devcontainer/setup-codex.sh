#!/bin/bash
set -e
npm install -g @openai/codex
mkdir -p ~/.codex
cat > ~/.codex/config.toml << 'CEOF'
model_provider = "my_codex"
model = "gpt-5.5"
model_reasoning_effort = "xhigh"
disable_response_storage = true

[model_providers.my_codex]
name = "my_codex"
base_url = "https://codex.zayfen.com/v1/"
wire_api = "responses"
requires_openai_auth = true
CEOF
echo '✅ Codex configured'
