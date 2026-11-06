#!/usr/bin/env bash
# Membuat sandbox demo agar pengujian UI tidak menyentuh config asli.
set -euo pipefail
ROOT="${1:-/tmp/rk-demo}"
rm -rf "$ROOT"
mkdir -p "$ROOT/pi/agent" "$ROOT/omp/agent" "$ROOT/opencode/config" "$ROOT/opencode/data" "$ROOT/hermes"

cat > "$ROOT/pi/agent/models.json" <<'JSON'
{
  "$schema": "https://pi.dev/schemas/models.schema.json",
  "providers": {
    "deepseek": {
      "name": "DeepSeek",
      "baseUrl": "https://api.deepseek.com",
      "api": "openai-completions",
      "apiKey": "$DEEPSEEK_API_KEY",
      "modelOverrides": { "deepseek-chat": { "contextWindow": 131072 } }
    }
  }
}
JSON

printf '{"defaultProvider":"deepseek","defaultModel":"deepseek/deepseek-chat"}\n' \
  > "$ROOT/pi/agent/settings.json"

cat > "$ROOT/omp/agent/models.yml" <<'YAML'
# demo oh-my-pi
providers:
  opencode-go:
    baseUrl: https://opencode.ai/inference/go/openai/v1
    api: openai-completions
    modelOverrides:
      kimi-k3:
        thinking:
          mode: effort
YAML

printf 'modelRoles:\n  default: opencode-go/kimi-k3\n' > "$ROOT/omp/agent/config.yml"

cat > "$ROOT/opencode/config/opencode.json" <<'JSON'
{
  "$schema": "https://opencode.ai/config.json",
  "model": "deepseek/deepseek-chat",
  "provider": {
    "deepseek": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "DeepSeek",
      "options": { "baseURL": "https://api.deepseek.com" },
      "models": {
        "deepseek-chat": {
          "name": "DeepSeek Chat",
          "limit": { "context": 131072, "output": 8192 }
        }
      }
    }
  }
}
JSON

printf '{"deepseek":{"type":"api","key":"demo-key-opencode"}}\n' \
  > "$ROOT/opencode/data/auth.json"

cat > "$ROOT/hermes/config.yaml" <<'YAML'
model:
  default: kimi-k3
  provider: "custom:commandcode"
providers:
  commandcode:
    name: "CommandCode"
    base_url: "https://api.commandcode.ai/provider/v1"
    key_env: "COMMANDCODE_API_KEY"
    api_mode: "chat_completions"
YAML

printf '# kredensial hermes\nCOMMANDCODE_API_KEY=demo-hermes-key\n' > "$ROOT/hermes/.env"

find "$ROOT" -type f | sort
