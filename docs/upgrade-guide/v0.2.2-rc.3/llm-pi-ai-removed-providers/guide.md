---
kind: upgrade-guide
description: "The dsh-llm-pi-ai catalog no longer offers amazon-bedrock, ant-ling, azure, baseten, cerebras, cloudflare-ai-gateway, cloudflare-workers-ai, huggingface, minimax, moonshotai, moonshotai-cn, nvidia, opencode, qwen-token-plan, qwen-token-plan-cn, qwen-token-plan-individual, radius, vercel-ai-gateway, xiaomi, xiaomi-token-plan-sgp, xiaomi-token-plan-ams, or zai."
---

# Removed pi-ai provider routes

English | [中文](guide.zh.md)

## Change

The `dsh-llm-pi-ai` provider catalog no longer offers `amazon-bedrock`, `ant-ling`, `azure`, `baseten`, `cerebras`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, `huggingface`, `minimax`, `moonshotai`, `moonshotai-cn`, `nvidia`, `opencode`, `qwen-token-plan`, `qwen-token-plan-cn`, `qwen-token-plan-individual`, `radius`, `vercel-ai-gateway`, `xiaomi`, `xiaomi-token-plan-sgp`, `xiaomi-token-plan-ams`, or `zai`. They are absent from the provider picker, discovery, sign-in, and model resolution. pi-ai still ships these routes; this build withholds them.

A stored `llm-pi-ai` profile naming one fails to load with `provider "<id>" is removed from this build; delete the route or select another provider`; resolution rejects the whole dictionary, so deleting that entry restores the rest. The `deepseek-official` and `deepseek-account` plugins are unaffected.

## Migration

1. Open `$DSH_HOME/profiles/<profile>/cordis.patch.yml` and remove each removed route's entry under `llm-pi-ai.config.providers`. Settings → Models writes the same document; delete its saved row.
2. If a default model selects a removed route, choose another in Settings → Models or update the `agent-default-model` entry.
3. Restart the server if it failed to load the plugin, then confirm Settings → Models lists the expected providers.
4. To keep using the same service, add a custom model API with its endpoint, protocol, and credential under a different provider id; the removed id stays refused.

`bedrock-converse-stream` and `azure-openai-responses` are not custom API protocols, so a declared route cannot select them. The other removed endpoints accept OpenAI-compatible chat completions: the Xiaomi open platform (four MiMo models plus two UltraSpeed variants), the SGP, AMS, Moonshot, and Alibaba Cloud endpoints, the Hugging Face router (`router.huggingface.co/v1`), the OpenCode Console (`opencode.ai/inference/v1`), Baseten (`inference.baseten.co/v1`), Cerebras (`api.cerebras.ai/v1`), Radius (`radius.pi.dev/v1`), NVIDIA (`integrate.api.nvidia.com/v1`), Ant Ling (`api.ant-ling.com/v1`), and Z.AI (`api.z.ai/api/coding/paas/v4`); `vercel-ai-gateway` and `minimax` need Anthropic Messages, at `ai-gateway.vercel.sh` and `https://api.minimax.io/anthropic`.

Most removals leave the vendor served. `zai-coding-cn` carries three of `zai`'s seven models (`glm-5.3`, `glm-5.3-flash`, `glm-5.3-highspeed`) with identical capabilities and prices; `glm-5.2` also appears on `opencode-go`, `glm-4.7` and `glm-5-turbo` on `openrouter`; `minimax-cn` serves the same three models as `minimax`; `opencode-go` replaces `opencode` and resolves the same `OPENCODE_API_KEY`; Kimi stays available through `kimi-coding`; `xiaomi-token-plan-cn` remains offered. `cerebras`'s two models and 118 of `vercel-ai-gateway`'s 301 ids appear unchanged on other routes, and `radius`'s three intent aliases (`balanced`, `cheap`, `precise`) name models other routes serve.

Some removed routes carry models that no other built-in route serves, for example `zai`'s `glm-5.2-highspeed`; `baseten`'s two (DeepSeek V4.1 Flash Fast and GLM-5.2 Fast); `nvidia`'s eleven, including Nemotron and Cosmos; `ant-ling`'s three (the Ling 2.6 pair and Ring 2.6 1T); 14 of `huggingface`'s 78 Hub names; `opencode`'s five paid exclusives, with its eight free-tier models reachable only through the OpenCode client; `xiaomi`'s two UltraSpeed variants; and 183 of `vercel-ai-gateway`'s 301 ids. Add each route's endpoint above as a custom model API to reach them.
