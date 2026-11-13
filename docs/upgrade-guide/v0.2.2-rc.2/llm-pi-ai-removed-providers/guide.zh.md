---
kind: upgrade-guide
description: "dsh-llm-pi-ai 的目录不再提供 amazon-bedrock、azure、cloudflare-ai-gateway、cloudflare-workers-ai 与 xiaomi-token-plan-ams。"
---

# 已移除的 pi-ai 提供方路由

[English](guide.md) | 中文

## 变更

`dsh-llm-pi-ai` 的提供方目录不再提供 `amazon-bedrock`、`azure`、`cloudflare-ai-gateway`、`cloudflare-workers-ai` 与 `xiaomi-token-plan-ams`。它们不会出现在提供方选择器、模型发现、登录与模型解析中。已安装的 pi-ai 包仍随包提供这些路由，是本构建主动移除。

点名这些路由的存量 `llm-pi-ai` profile 会以 `provider "<id>" is removed from this build; delete the route or select another provider` 加载失败。由于解析会拒绝整份 `providers` 字典，删除该条目之前，插件中的其他路由也不可用。两条 DeepSeek 路由 `deepseek-official` 与 `deepseek-account` 来自独立插件，不受影响。

`bedrock-converse-stream` 与 `azure-openai-responses` 不是自定义模型 API 的协议选项，因此手工声明的路由无法选择这两种传输。两条 Cloudflare 路由与 `xiaomi-token-plan-ams` 使用的是 OpenAI 兼容协议，你自有的 OpenAI 兼容端点仍可通过自定义模型 API 接入——AMS token 套餐端点同样如此，它与中国、新加坡套餐提供相同的四个 MiMo 模型。

## 迁移

1. 打开 `$DSH_HOME/profiles/<profile>/cordis.patch.yml`，删除 `llm-pi-ai.config.providers` 下每个被移除路由的条目。设置 → 模型页写入同一文档；页面列出该路由时，也可在页面中删除对应行。
2. 若默认模型指向被移除的路由，在设置 → 模型中选择其他提供方与模型，或更新 `agent-default-model` 条目。
3. 若插件因此加载失败，重启服务器，然后确认设置 → 模型列出了预期的提供方。
4. 若要继续使用同一上游服务，请用其端点、协议、凭据与模型 ID 添加自定义模型 API；目录无法恢复被移除的路由。
