---
kind: upgrade-guide
description: "The pi-ai model catalog moves to 1.6.0, so a stored model selection that names a removed id fails with UNKNOWN_MODEL."
---

# Removed catalog model ids

English | [中文](guide.zh.md)

## Change

`@earendil-works/pi-ai` moves from 1.3.0 to 1.6.0, and every route the installed catalog serves takes its 1.6.0 model list. Routes lose ids and gain others, so a selection that names a removed id — `agent-default-model`'s `model`, or the model stored for a Session — no longer resolves and fails with `UNKNOWN_MODEL` naming the route and model.

`openai-codex` loses `gpt-5.5` and `gpt-5.3-codex-spark`: the Codex backend rejects both for ChatGPT accounts, which is why upstream removed them. The route now offers `gpt-5.6-luna`, `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-6-astra`, `gpt-6-luna`, `gpt-6-sol`, `gpt-6.1-sol`, `gpt-6.1-terra`, `gpt-6.2-astra`, and `gpt-6.2-sol`.

Other served routes lose ids too: `github-copilot` ten, including `gpt-5.4`, `gpt-5.5`, and the `gemini-3.5-flash` to `gemini-3.7-flash` family; `openai` three (`gpt-5.2-chat-latest`, `gpt-5.3-chat-latest`, `gpt-realtime-2.1`); `opencode-go` six; `google` two preview models; and `xiaomi-token-plan-cn` both MiMo 2.5 ids, which `mimo-v2.6-flash` and `mimo-v2.6-pro` replace. The catalog also adds the `qwen-token-plan-individual-cn` route.

## Migration

1. Collect every stored model selection that names an affected route: `agent-default-model`'s `provider` and `model`, and the selection shown in the model picker for each Session.
2. Replace each removed id with one the route still serves; the lists above name the `openai-codex` ids, and `mimo-v2.6-pro` replaces `mimo-v2.5-pro` on `xiaomi-token-plan-cn`.
3. Send one request on each changed route. It succeeds once no `UNKNOWN_MODEL` failure appears, and the model picker lists the new ids.
