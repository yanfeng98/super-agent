---
kind: upgrade-guide
description: "dsh-llm-pi-ai 的目录不再提供 amazon-bedrock、ant-ling、azure、baseten、cerebras、cloudflare-ai-gateway、cloudflare-workers-ai、fireworks、groq、huggingface、minimax、moonshotai、moonshotai-cn、nvidia、opencode、qwen-token-plan、qwen-token-plan-cn、qwen-token-plan-individual、radius、vercel-ai-gateway、xiaomi、xiaomi-token-plan-sgp、xiaomi-token-plan-ams 或 zai。"
---

# 已移除的 pi-ai 提供方路由

[English](guide.md) | 中文

## 变更

`dsh-llm-pi-ai` 的提供方目录不再提供 `amazon-bedrock`、`ant-ling`、`azure`、`baseten`、`cerebras`、`cloudflare-ai-gateway`、`cloudflare-workers-ai`、`fireworks`、`groq`、`huggingface`、`minimax`、`moonshotai`、`moonshotai-cn`、`nvidia`、`opencode`、`qwen-token-plan`、`qwen-token-plan-cn`、`qwen-token-plan-individual`、`radius`、`vercel-ai-gateway`、`xiaomi`、`xiaomi-token-plan-sgp`、`xiaomi-token-plan-ams` 或 `zai`。它们不会出现在提供方选择器、模型发现、登录与模型解析中。已安装的 pi-ai 包仍随包提供这些路由，是本构建主动移除。

存量 `llm-pi-ai` profile 点名其中一条时会加载失败并报 `provider "<id>" is removed from this build; delete the route or select another provider`；解析会拒绝整个字典，删除该条目后其余路由即恢复。`deepseek-official` 与 `deepseek-account` 插件不受影响。

## 迁移

1. 打开 `$DSH_HOME/profiles/<profile>/cordis.patch.yml`，删除 `llm-pi-ai.config.providers` 下每条被移除路由的条目。设置页「模型」写入的是同一份文档；删除其中保存的行即可。
2. 若默认模型指向被移除的路由，在设置页「模型」改选其他模型，或更新 `agent-default-model` 条目。
3. 若插件因加载失败而不可用，重启服务后确认设置页「模型」列出预期的提供商。
4. 需要继续使用原服务时，以自定义模型 API 接入其端点、协议与凭据，并使用不同的 Provider ID；被移除的 ID 仍会被拒绝。

`bedrock-converse-stream` 与 `azure-openai-responses` 不是自定义模型 API 的协议选项，因此手工声明的路由无法选择它们。其余被移除的端点使用 OpenAI 兼容的对话补全：Xiaomi 开放平台（四个 MiMo 模型，另有两个 UltraSpeed 变体）、SGP、AMS、Moonshot 与阿里云端点、Hugging Face 的路由端点 `router.huggingface.co/v1`、OpenCode Console `opencode.ai/inference/v1`、Baseten `inference.baseten.co/v1`、Cerebras `api.cerebras.ai/v1`、Radius `radius.pi.dev/v1`、NVIDIA `integrate.api.nvidia.com/v1`、Ant Ling `api.ant-ling.com/v1`、Fireworks `api.fireworks.ai/inference/v1`、Groq `api.groq.com/openai/v1` 与 Z.AI `api.z.ai/api/coding/paas/v4`；`vercel-ai-gateway` 与 `minimax` 需使用 Anthropic Messages，端点分别为 `ai-gateway.vercel.sh` 与 `https://api.minimax.io/anthropic`。

`fireworks` 是转售其他厂商开源权重模型并托管自有 Ember 系列的第三方推理云，因此移除它拿掉的是一个托管与计费层。多数移除后该厂商仍由其他内置路由承接。`zai-coding-cn` 承接 `zai` 七个模型中的三个（`glm-5.3`、`glm-5.3-flash`、`glm-5.3-highspeed`），能力与价格一致，`glm-5.2` 另见 `opencode-go`，`glm-4.7` 与 `glm-5-turbo` 另见 `openrouter`；`minimax-cn` 提供与 `minimax` 相同的三个模型；`opencode-go` 取代 `opencode`，解析同一 `OPENCODE_API_KEY`；Kimi 仍可通过 `kimi-coding` 使用；`xiaomi-token-plan-cn` 仍然提供。`cerebras` 的两个模型、`groq` 的七个模型，以及 `vercel-ai-gateway` 的 301 个 id 中的 118 个，都在其他路由上出现，`radius` 的三个意图别名（`balanced`、`cheap`、`precise`）指向的是其他路由已提供的模型。`groq` 是另一家芯片厂商的推理云，因此移除它只拿掉托管层。

部分被移除的路由承载了没有其他内置路由的模型，例如 `zai` 的 `glm-5.2-highspeed`；`baseten` 的两个（DeepSeek V4.1 Flash Fast 与 GLM-5.2 Fast）；`nvidia` 的十一个模型（含 Nemotron 与 Cosmos）；`ant-ling` 的三个模型（Ling 2.6 两个与 Ring 2.6 1T）；`huggingface` 的 78 个 Hub 模型名中的 14 个；`opencode` 的五个付费独有模型（另有八个免费档模型只允许其客户端使用）；`xiaomi` 的两个 UltraSpeed 变体；以及 `vercel-ai-gateway` 的 301 个 id 中的 183 个。需要时按上面的端点以自定义模型 API 接入。
