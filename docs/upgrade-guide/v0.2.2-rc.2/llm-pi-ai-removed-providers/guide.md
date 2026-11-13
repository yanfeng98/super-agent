---
kind: upgrade-guide
description: "The dsh-llm-pi-ai catalog no longer offers amazon-bedrock, azure, cloudflare-ai-gateway, cloudflare-workers-ai, or xiaomi-token-plan-ams."
---

# Removed pi-ai provider routes

English | [中文](guide.zh.md)

## Change

The `dsh-llm-pi-ai` provider catalog no longer offers `amazon-bedrock`, `azure`, `cloudflare-ai-gateway`, `cloudflare-workers-ai`, or `xiaomi-token-plan-ams`. They are absent from the provider picker, model discovery, sign-in, and model resolution. The installed pi-ai package still ships these routes; this build withholds them.

A stored `llm-pi-ai` profile that names one of these routes fails to load with `provider "<id>" is removed from this build; delete the route or select another provider`. Because resolution rejects the whole `providers` dictionary, the plugin's other routes stay unavailable until that entry is removed. The two DeepSeek routes, `deepseek-official` and `deepseek-account`, come from separate plugins and are unaffected.

`bedrock-converse-stream` and `azure-openai-responses` are not custom API protocols, so a hand-declared route cannot select those transports. The Cloudflare routes and `xiaomi-token-plan-ams` use OpenAI-compatible protocols, so an OpenAI-compatible endpoint you operate can still be added as a custom model API. That includes the AMS token-plan endpoint, which serves the same four MiMo models as the China and Singapore token plans.

## Migration

1. Open `$DSH_HOME/profiles/<profile>/cordis.patch.yml` and remove each removed route's entry under `llm-pi-ai.config.providers`. Settings → Models writes the same document; delete the saved row there when the page lists it.
2. If a default model selects a removed route, choose another provider and model in Settings → Models, or update the `agent-default-model` entry.
3. Restart the server if it failed to load the plugin, then confirm Settings → Models lists the expected providers.
4. To keep using the same upstream service, add a custom model API with its endpoint, protocol, credential, and model ids; the catalog cannot restore the removed route.
