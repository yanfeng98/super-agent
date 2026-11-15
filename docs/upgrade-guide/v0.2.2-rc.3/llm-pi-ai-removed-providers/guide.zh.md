---
kind: upgrade-guide
description: "dsh-llm-pi-ai 的目录不再提供 amazon-bedrock、azure、cerebras、cloudflare-ai-gateway、cloudflare-workers-ai、huggingface、minimax、moonshotai、moonshotai-cn、opencode、qwen-token-plan、qwen-token-plan-cn、qwen-token-plan-individual、vercel-ai-gateway、xiaomi、xiaomi-token-plan-sgp 与 xiaomi-token-plan-ams。"
---

# 已移除的 pi-ai 提供方路由

[English](guide.md) | 中文

## 变更

`dsh-llm-pi-ai` 的提供方目录不再提供 `amazon-bedrock`、`azure`、`cerebras`、`cloudflare-ai-gateway`、`cloudflare-workers-ai`、`huggingface`、`minimax`、`moonshotai`、`moonshotai-cn`、`opencode`、`qwen-token-plan`、`qwen-token-plan-cn`、`qwen-token-plan-individual`、`vercel-ai-gateway`、`xiaomi`、`xiaomi-token-plan-sgp` 与 `xiaomi-token-plan-ams`。它们不会出现在提供方选择器、模型发现、登录与模型解析中。已安装的 pi-ai 包仍随包提供这些路由，是本构建主动移除。

点名这些路由的存量 `llm-pi-ai` profile 会以 `provider "<id>" is removed from this build; delete the route or select another provider` 加载失败。由于解析会拒绝整份 `providers` 字典，删除该条目之前，插件中的其他路由也不可用。两条 DeepSeek 路由 `deepseek-official` 与 `deepseek-account` 来自独立插件，不受影响。

`bedrock-converse-stream` 与 `azure-openai-responses` 不是自定义模型 API 的协议选项，因此手工声明的路由无法选择这两种传输。你自有的端点仍可通过自定义模型 API 接入：Xiaomi 开放平台端点（与 `xiaomi-token-plan-cn` 相同的四个 MiMo 模型，另有两个 UltraSpeed 变体），SGP 与 AMS token 套餐端点、Moonshot 开放平台端点、阿里云 token 套餐端点、Hugging Face 的路由端点 `router.huggingface.co/v1`、OpenCode Console 端点 `opencode.ai/inference/v1` 与 Cerebras 端点 `api.cerebras.ai/v1` 同样适用；`vercel-ai-gateway` 可用 Anthropic Messages 重新接入。Moonshot 的两个端点属于同一个 Kimi 服务；`kimi-coding` 仍然提供。`xiaomi` 是按量计费的开放平台；`xiaomi-token-plan-cn` 仍然提供。它的 MiMo-V2.5-Pro-UltraSpeed 已无任何路由提供；MiMo-V2.6-Pro-UltraSpeed 在 `openrouter` 上为 `xiaomi/mimo-v2.6-pro-ultraspeed`。三条 Qwen token 套餐路由提供相同的二十个模型，个人版套餐开放其中的九个。`huggingface` 把 78 个 Hub 名称路由给第三方推理服务商，其中 64 个出现在 `openrouter` 上（29 个换了厂商前缀）、1 个出现在 `nvidia` 上，其余 13 个没有其他内置路由。`cerebras` 提供两个开源权重模型，二者在其他路由上仍然可用：`gpt-oss-120b` 为 `baseten`、`groq`、`openrouter` 与 `together` 上的 `openai/gpt-oss-120b`，Qwen3.8 27B 为 `groq` 与 `openrouter` 上的 `qwen/qwen3.8-27b`。`opencode` 是 OpenCode 的按量付费 Console 网关；其订阅路由 `opencode-go` 仍然提供，并解析同一 `OPENCODE_API_KEY`。`minimax` 是 MiniMax 的国际站端点；其中文站 `minimax-cn` 仍然提供同样的三个模型，国际站端点可用 Anthropic Messages 重新接入（`https://api.minimax.io/anthropic`）。

## 迁移

1. 打开 `$DSH_HOME/profiles/<profile>/cordis.patch.yml`，删除 `llm-pi-ai.config.providers` 下每个被移除路由的条目。设置 → 模型页写入同一文档，也可在页面中删除对应的行。
2. 若默认模型指向被移除的路由，在设置 → 模型中选择其他模型，或更新 `agent-default-model` 条目。
3. 若插件因此加载失败，重启服务器，然后确认设置 → 模型列出了预期的提供方。
4. 若要继续使用同一服务，请用其端点、协议、凭据与模型 ID 以不同的 Provider ID 添加自定义模型 API；目录无法恢复该路由，被移除的 ID 即使作为手工声明的路由仍会被拒绝。
