---
kind: upgrade-guide
description: "The dsh-llm-pi-ai catalog no longer offers amazon-bedrock, azure, cloudflare-ai-gateway, cloudflare-workers-ai, moonshotai, moonshotai-cn, qwen-token-plan, qwen-token-plan-cn, qwen-token-plan-individual, xiaomi-token-plan-sgp, or xiaomi-token-plan-ams."
---

# Removed pi-ai provider routes

English | [中文](guide.zh.md)

## Change

The `dsh-llm-pi-ai` provider catalog no longer offers `amazon-bedrock`, `azure`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, `moonshotai`, `moonshotai-cn`, `qwen-token-plan`, `qwen-token-plan-cn`, `qwen-token-plan-individual`, `xiaomi-token-plan-sgp`, or `xiaomi-token-plan-ams`. They are absent from the provider picker, model discovery, sign-in, and model resolution. The installed pi-ai package still ships these routes; this build withholds them.

A stored `llm-pi-ai` profile that names one of these routes fails to load with `provider "<id>" is removed from this build; delete the route or select another provider`. Because resolution rejects the whole `providers` dictionary, the plugin's other routes stay unavailable until that entry is removed. The two DeepSeek routes, `deepseek-official` and `deepseek-account`, come from separate plugins and are unaffected.

`bedrock-converse-stream` and `azure-openai-responses` are not custom API protocols, so a hand-declared route cannot select those transports. The Cloudflare routes, `moonshotai`, `moonshotai-cn`, the three Qwen token-plan routes, `xiaomi-token-plan-sgp`, and `xiaomi-token-plan-ams` use OpenAI-compatible protocols, so an OpenAI-compatible endpoint you operate can still be added as a custom model API. That includes the SGP and AMS token-plan endpoints, which serve the same four MiMo models as `xiaomi-token-plan-cn`, the Moonshot open-platform endpoints, and the Alibaba Cloud token-plan endpoints. `moonshotai` and `moonshotai-cn` are one Kimi service's global and China endpoints; the Kimi coding plan (`kimi-coding`) remains offered. `qwen-token-plan` and `qwen-token-plan-cn` serve the same twenty models from the Singapore and Beijing endpoints, and `qwen-token-plan-individual` exposes nine of them.

## Migration

1. Open `$DSH_HOME/profiles/<profile>/cordis.patch.yml` and remove each removed route's entry under `llm-pi-ai.config.providers`. Settings → Models writes the same document; delete the saved row there when the page lists it.
2. If a default model selects a removed route, choose another provider and model in Settings → Models, or update the `agent-default-model` entry.
3. Restart the server if it failed to load the plugin, then confirm Settings → Models lists the expected providers.
4. To keep using the same upstream service, add a custom model API with its endpoint, protocol, credential, and model ids; the catalog cannot restore the removed route. Give the custom route a provider id other than the removed one: that id stays refused even for a hand-declared route.
