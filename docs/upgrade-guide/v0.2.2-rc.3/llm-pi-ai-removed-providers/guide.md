---
kind: upgrade-guide
description: "The dsh-llm-pi-ai catalog no longer offers amazon-bedrock, ant-ling, azure, cerebras, cloudflare-ai-gateway, cloudflare-workers-ai, huggingface, minimax, moonshotai, moonshotai-cn, nvidia, opencode, qwen-token-plan, qwen-token-plan-cn, qwen-token-plan-individual, radius, vercel-ai-gateway, xiaomi, xiaomi-token-plan-sgp, or xiaomi-token-plan-ams."
---

# Removed pi-ai provider routes

English | [中文](guide.zh.md)

## Change

The `dsh-llm-pi-ai` provider catalog no longer offers `amazon-bedrock`, `ant-ling`, `azure`, `cerebras`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, `huggingface`, `minimax`, `moonshotai`, `moonshotai-cn`, `nvidia`, `opencode`, `qwen-token-plan`, `qwen-token-plan-cn`, `qwen-token-plan-individual`, `radius`, `vercel-ai-gateway`, `xiaomi`, `xiaomi-token-plan-sgp`, or `xiaomi-token-plan-ams`. They are absent from the provider picker, discovery, sign-in, and model resolution. pi-ai still ships these routes; this build withholds them.

A stored `llm-pi-ai` profile naming one of these routes fails to load with `provider "<id>" is removed from this build; delete the route or select another provider`. Resolution rejects the whole `providers` dictionary, so the other routes stay unavailable until that entry is removed. The `deepseek-official` and `deepseek-account` plugins are unaffected.

`bedrock-converse-stream` and `azure-openai-responses` are not custom API protocols, so a declared route cannot select them. An endpoint you operate can still be added as a custom model API, including the Xiaomi open-platform endpoint (the same four MiMo models plus two UltraSpeed variants), the SGP, AMS, Moonshot, and Alibaba Cloud endpoints, the Hugging Face router at `router.huggingface.co/v1`, the OpenCode Console endpoint at `opencode.ai/inference/v1`, and the Cerebras endpoint at `api.cerebras.ai/v1`; `vercel-ai-gateway` can be re-added with Anthropic Messages, and `radius`, `nvidia`, and `ant-ling` with OpenAI-compatible chat completions at `radius.pi.dev/v1`, `integrate.api.nvidia.com/v1`, and `api.ant-ling.com/v1`. `xiaomi` is the pay-as-you-go open platform; `xiaomi-token-plan-cn` remains offered. Moonshot's Kimi stays available through `kimi-coding`. `huggingface` routes 78 Hub names: 64 appear on `openrouter` (29 renamed), 14 nowhere else. `cerebras`'s two open-weight models are available elsewhere: `gpt-oss-120b` as `openai/gpt-oss-120b` on `baseten`, `groq`, `openrouter`, and `together`, and Qwen3.8 27B as `qwen/qwen3.8-27b` on `groq` and `openrouter`. `opencode` is OpenCode's pay-as-you-go Console gateway; `opencode-go`, its subscription sibling, remains offered and resolves the same `OPENCODE_API_KEY`. `minimax` is MiniMax's international endpoint; its China sibling `minimax-cn` serves the same three models and remains offered, and the international one can be re-added with Anthropic Messages at `https://api.minimax.io/anthropic`. `radius` resells other vendors' models: 26 of its 29 ids appear unchanged on built-in routes, and three intent aliases (`balanced`, `cheap`, `precise`) name models other routes serve. `nvidia` is the chip vendor's inference cloud: nine of its twenty models appear on built-in routes, while the remaining eleven, including Nemotron and Cosmos, appear nowhere else. `ant-ling` serves the Ling 2.6 pair and the reasoning Ring 2.6 1T, none of which appears on another route.

## Migration

1. Open `$DSH_HOME/profiles/<profile>/cordis.patch.yml` and remove each removed route's entry under `llm-pi-ai.config.providers`. Settings → Models writes the same document; delete its saved row.
2. If a default model selects a removed route, choose another in Settings → Models or update the `agent-default-model` entry.
3. Restart the server if it failed to load the plugin, then confirm Settings → Models lists the expected providers.
4. To keep using the same service, add a custom model API with its endpoint, protocol, and credential under a different provider id; the removed id stays refused.
