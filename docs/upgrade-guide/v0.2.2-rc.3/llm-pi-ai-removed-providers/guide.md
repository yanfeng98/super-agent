---
kind: upgrade-guide
description: "The dsh-llm-pi-ai catalog no longer offers amazon-bedrock, azure, cerebras, cloudflare-ai-gateway, cloudflare-workers-ai, huggingface, moonshotai, moonshotai-cn, qwen-token-plan, qwen-token-plan-cn, qwen-token-plan-individual, vercel-ai-gateway, xiaomi, xiaomi-token-plan-sgp, or xiaomi-token-plan-ams."
---

# Removed pi-ai provider routes

English | [中文](guide.zh.md)

## Change

The `dsh-llm-pi-ai` provider catalog no longer offers `amazon-bedrock`, `azure`, `cerebras`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, `huggingface`, `moonshotai`, `moonshotai-cn`, `qwen-token-plan`, `qwen-token-plan-cn`, `qwen-token-plan-individual`, `vercel-ai-gateway`, `xiaomi`, `xiaomi-token-plan-sgp`, or `xiaomi-token-plan-ams`. They are absent from the provider picker, discovery, sign-in, and model resolution. The installed pi-ai package still ships these routes; this build withholds them.

A stored `llm-pi-ai` profile that names one of these routes fails to load with `provider "<id>" is removed from this build; delete the route or select another provider`. Because resolution rejects the whole `providers` dictionary, the plugin's other routes stay unavailable until that entry is removed. The two DeepSeek routes, `deepseek-official` and `deepseek-account`, come from separate plugins and are unaffected.

`bedrock-converse-stream` and `azure-openai-responses` are not custom API protocols, so a hand-declared route cannot select them. The Cloudflare routes, `cerebras`, `huggingface`, `moonshotai`, `moonshotai-cn`, the three Qwen token-plan routes, and the three Xiaomi routes use OpenAI-compatible protocols, so an OpenAI-compatible endpoint you operate can still be added as a custom model API. That includes the Xiaomi open-platform endpoint (the same four MiMo models as `xiaomi-token-plan-cn`, plus two UltraSpeed variants), the SGP and AMS token-plan endpoints, the Moonshot open-platform endpoints, the Alibaba Cloud token-plan endpoints, the Hugging Face router at `router.huggingface.co/v1`, and the Cerebras endpoint at `api.cerebras.ai/v1`. `vercel-ai-gateway` uses Anthropic Messages and can be re-added as a custom model API. Moonshot's two endpoints are one Kimi service; `kimi-coding` remains offered. `xiaomi` is the pay-as-you-go open platform; `xiaomi-token-plan-cn` remains offered. Two of `xiaomi`'s six models have no equivalent there: MiMo-V2.6-Pro-UltraSpeed is `xiaomi/mimo-v2.6-pro-ultraspeed` on `openrouter`; MiMo-V2.5-Pro-UltraSpeed has no route left. `qwen-token-plan` and `qwen-token-plan-cn` serve the same twenty models from Singapore and Beijing, and `qwen-token-plan-individual` exposes nine. `huggingface` routes 78 Hub model names through third-party inference providers: 64 appear on `openrouter` (29 under a different vendor prefix), one on `nvidia`, and 13 on no other route. `cerebras` served two open-weight models that remain available elsewhere: `gpt-oss-120b` as `openai/gpt-oss-120b` on `baseten`, `groq`, `openrouter`, and `together`, and Qwen3.8 27B as `qwen/qwen3.8-27b` on `groq` and `openrouter`.

## Migration

1. Open `$DSH_HOME/profiles/<profile>/cordis.patch.yml` and remove each removed route's entry under `llm-pi-ai.config.providers`. Settings → Models writes the same document; delete its saved row there.
2. If a default model selects a removed route, choose another provider and model in Settings → Models, or update the `agent-default-model` entry.
3. Restart the server if it failed to load the plugin, then confirm Settings → Models lists the expected providers.
4. To keep using the same upstream service, add a custom model API with its endpoint, protocol, credential, and model ids; the catalog cannot restore the route. Give it a different provider id: the removed one stays refused even for a hand-declared route.
