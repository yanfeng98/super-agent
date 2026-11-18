---
kind: upgrade-guide
description: "The dsh-llm-pi-ai catalog no longer offers amazon-bedrock, ant-ling, azure, baseten, cerebras, cloudflare-ai-gateway, cloudflare-workers-ai, fireworks, google-vertex, groq, huggingface, meta, minimax, mistral, moonshotai, moonshotai-cn, nvidia, opencode, openrouter, qwen-token-plan, qwen-token-plan-cn, qwen-token-plan-individual, radius, together, vercel-ai-gateway, xai, xiaomi, xiaomi-token-plan-sgp, xiaomi-token-plan-ams, or zai."
---

# Removed pi-ai provider routes

English | [中文](guide.zh.md)

## Change

The `dsh-llm-pi-ai` provider catalog no longer offers `amazon-bedrock`, `ant-ling`, `azure`, `baseten`, `cerebras`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, `fireworks`, `google-vertex`, `groq`, `huggingface`, `meta`, `minimax`, `mistral`, `moonshotai`, `moonshotai-cn`, `nvidia`, `opencode`, `openrouter`, `qwen-token-plan`, `qwen-token-plan-cn`, `qwen-token-plan-individual`, `radius`, `together`, `vercel-ai-gateway`, `xai`, `xiaomi`, `xiaomi-token-plan-sgp`, `xiaomi-token-plan-ams`, or `zai`. They are absent from the picker, discovery, sign-in, and model resolution.

A stored profile naming one fails to load with `provider "<id>" is removed from this build; delete the route or select another provider`; deleting that entry restores the rest. `deepseek-official` and `deepseek-account` are unaffected.

## Migration

1. Open `$DSH_HOME/profiles/<profile>/cordis.patch.yml` and remove each removed route's entry under `llm-pi-ai.config.providers`.
2. If a default model selects a removed route, choose another in Settings → Models.
3. Restart the server if the plugin failed to load, then confirm Settings → Models lists the expected providers.
4. To keep using a service, add its endpoint as a custom model API under a different provider id; the removed id stays refused.

`bedrock-converse-stream`, `google-vertex`, and `azure-openai-responses` are not selectable custom API protocols. The others accept OpenAI-compatible chat completions: the Xiaomi open platform, the SGP/AMS/Moonshot/Alibaba endpoints, Hugging Face router (`router.huggingface.co/v1`), OpenCode Console (`opencode.ai/inference/v1`), Baseten (`inference.baseten.co/v1`), Cerebras (`api.cerebras.ai/v1`), Radius (`radius.pi.dev/v1`), NVIDIA (`integrate.api.nvidia.com/v1`), Ant Ling (`api.ant-ling.com/v1`), Together (`api.together.ai/v1`), Fireworks (`api.fireworks.ai/inference/v1`), Groq (`api.groq.com/openai/v1`), Mistral (`api.mistral.ai/v1`), OpenRouter (`openrouter.ai/api/v1`; `openrouter.ai/api` for Anthropic Messages), and Z.AI (`api.z.ai/api/coding/paas/v4`); `vercel-ai-gateway` and `minimax` need Anthropic Messages, at `ai-gateway.vercel.sh` and `https://api.minimax.io/anthropic`; Meta and xAI need OpenAI Responses, at `https://api.meta.ai/v1` and `https://api.x.ai/v1`.

`google-vertex`'s fourteen ids all appear on `google`, though three fields differ; its protocol is not selectable. `azure`'s 45 ids appear on `openai`, `minimax`'s three on `minimax-cn`, and `zai-coding-cn` carries three of `zai`'s seven models, with `glm-5.2` on `opencode-go`. `opencode` keeps 58 of 84 ids, `cloudflare-ai-gateway` 44 of 57, `amazon-bedrock` 26 of 193, `huggingface` 15 of 78, `vercel-ai-gateway` 87 of 301, `baseten` 7 of 22, `together` 7 of 17, `fireworks` 6 of 22, `mistral` 3 of 40, `nvidia` 3 of 20, `qwen-token-plan` and `-cn` 8 of 20 (`-individual` 5 of 9), and `radius` 26 of 29. `meta`'s two `-contributor` ids also appear on `opencode-go`, leaving four of six. `xai`'s `grok-4.5`, `grok-4.6`, and `grok-4.7` also appear on `github-copilot` (128,000-token cap) and `grok-4.7` on `opencode-go`, leaving `grok-4.3` without another route.

`openrouter` is the widest removal: its 396 ids, all but the bare `auto` router vendor-prefixed, 95 name a model another served route carries once the prefix is dropped and punctuation ignored, two match by display name, and 299 have no other route, so it drops model coverage.

The rest appear on no other route: `ant-ling`'s three, `cerebras`'s two, `groq`'s seven, and the uncovered entries — including `zai`'s `glm-4.7`, `glm-5-turbo`, and `glm-5.2-highspeed`, `baseten`'s two `-Fast` ids, NVIDIA's Nemotron and Cosmos entries, 63 Hugging Face Hub names, and OpenCode's exclusives.
