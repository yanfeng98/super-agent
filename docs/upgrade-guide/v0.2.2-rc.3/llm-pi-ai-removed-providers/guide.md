---
kind: upgrade-guide
description: "The dsh-llm-pi-ai catalog no longer offers amazon-bedrock, ant-ling, azure, baseten, cerebras, cloudflare-ai-gateway, cloudflare-workers-ai, fireworks, google-vertex, groq, huggingface, minimax, mistral, moonshotai, moonshotai-cn, nvidia, opencode, qwen-token-plan, qwen-token-plan-cn, qwen-token-plan-individual, radius, together, vercel-ai-gateway, xiaomi, xiaomi-token-plan-sgp, xiaomi-token-plan-ams, or zai."
---

# Removed pi-ai provider routes

English | [中文](guide.zh.md)

## Change

The `dsh-llm-pi-ai` provider catalog no longer offers `amazon-bedrock`, `ant-ling`, `azure`, `baseten`, `cerebras`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, `fireworks`, `google-vertex`, `groq`, `huggingface`, `minimax`, `mistral`, `moonshotai`, `moonshotai-cn`, `nvidia`, `opencode`, `qwen-token-plan`, `qwen-token-plan-cn`, `qwen-token-plan-individual`, `radius`, `together`, `vercel-ai-gateway`, `xiaomi`, `xiaomi-token-plan-sgp`, `xiaomi-token-plan-ams`, or `zai`. They are absent from the provider picker, discovery, sign-in, and model resolution.

A stored `llm-pi-ai` profile naming one fails to load with `provider "<id>" is removed from this build; delete the route or select another provider`; deleting that entry restores the rest. `deepseek-official` and `deepseek-account` are unaffected.

## Migration

1. Open `$DSH_HOME/profiles/<profile>/cordis.patch.yml` and remove each removed route's entry under `llm-pi-ai.config.providers`.
2. If a default model selects a removed route, choose another in Settings → Models.
3. Restart the server if the plugin failed to load, then confirm Settings → Models lists the expected providers.
4. To keep using a service, add its endpoint as a custom model API under a different provider id; the removed id stays refused.

`bedrock-converse-stream`, `google-vertex`, and `azure-openai-responses` are not selectable custom API protocols. The other removed endpoints accept OpenAI-compatible chat completions: the Xiaomi open platform, the SGP/AMS/Moonshot/Alibaba endpoints, Hugging Face router (`router.huggingface.co/v1`), OpenCode Console (`opencode.ai/inference/v1`), Baseten (`inference.baseten.co/v1`), Cerebras (`api.cerebras.ai/v1`), Radius (`radius.pi.dev/v1`), NVIDIA (`integrate.api.nvidia.com/v1`), Ant Ling (`api.ant-ling.com/v1`), Together (`api.together.ai/v1`), Fireworks (`api.fireworks.ai/inference/v1`), Groq (`api.groq.com/openai/v1`), Mistral (`api.mistral.ai/v1`), and Z.AI (`api.z.ai/api/coding/paas/v4`); `vercel-ai-gateway` and `minimax` need Anthropic Messages, at `ai-gateway.vercel.sh` and `https://api.minimax.io/anthropic`.

Most removals leave the vendor served. `google-vertex`'s fourteen ids all appear on `google`, though one output cap, one cache price, and three tiered entries differ; no custom route replaces it because its protocol is not selectable; `zai-coding-cn` carries three of `zai`'s seven models (`glm-5.3`, `glm-5.3-flash`, `glm-5.3-highspeed`) with identical capabilities and prices; `glm-5.2` also appears on `opencode-go`, `glm-4.7` and `glm-5-turbo` on `openrouter`; `minimax-cn` serves the same three models as `minimax`; `opencode-go` replaces `opencode` and resolves the same `OPENCODE_API_KEY`; Kimi stays available through `kimi-coding`; `xiaomi-token-plan-cn` remains offered. `cerebras`'s two models, `groq`'s seven, and 118 of `vercel-ai-gateway`'s 301 ids appear on other routes, and `radius`'s three intent aliases (`balanced`, `cheap`, `precise`) name models other routes serve. `together`'s seventeen all appear on other routes: fifteen on `openrouter`, its two Turbo ids match `openrouter`'s non-Turbo `meta-llama/llama-3.3-70b-instruct` and `qwen/qwen-2.5-7b-instruct`, so it removes a serving layer.

Some routes carry models no other route serves: eleven of `mistral`'s 40 (the Pixtral pair, Magistral Medium, Leanstral 1.5, Devstral Medium, three Devstral Small ids, Mistral Large 2.1, Mistral 7B, Mixtral 8x7B), the other 29 on `openrouter`; `zai`'s `glm-5.2-highspeed`; `baseten`'s two (DeepSeek V4.1 Flash Fast, GLM-5.2 Fast); `nvidia`'s eleven, Nemotron and Cosmos among them; `ant-ling`'s three (Ling 2.6 pair, Ring 2.6 1T); 14 of `huggingface`'s 78 Hub names; `opencode`'s five paid exclusives and eight client-only free models; `xiaomi`'s two UltraSpeed variants; and 183 of `vercel-ai-gateway`'s 301. Add each endpoint as a custom model API.
