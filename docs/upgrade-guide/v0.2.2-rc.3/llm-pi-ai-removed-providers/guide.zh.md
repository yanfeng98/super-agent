---
kind: upgrade-guide
description: "dsh-llm-pi-ai 的目录不再提供 amazon-bedrock、ant-ling、azure、baseten、cerebras、cloudflare-ai-gateway、cloudflare-workers-ai、fireworks、google-vertex、groq、huggingface、meta、minimax、mistral、moonshotai、moonshotai-cn、nvidia、opencode、openrouter、qwen-token-plan、qwen-token-plan-cn、qwen-token-plan-individual、radius、together、vercel-ai-gateway、xai、xiaomi、xiaomi-token-plan-sgp、xiaomi-token-plan-ams 或 zai。"
---

# 已移除的 pi-ai 提供方路由

[English](guide.md) | 中文

## 变更

`dsh-llm-pi-ai` 的提供方目录不再提供 `amazon-bedrock`、`ant-ling`、`azure`、`baseten`、`cerebras`、`cloudflare-ai-gateway`、`cloudflare-workers-ai`、`fireworks`、`google-vertex`、`groq`、`huggingface`、`meta`、`minimax`、`mistral`、`moonshotai`、`moonshotai-cn`、`nvidia`、`opencode`、`openrouter`、`qwen-token-plan`、`qwen-token-plan-cn`、`qwen-token-plan-individual`、`radius`、`together`、`vercel-ai-gateway`、`xai`、`xiaomi`、`xiaomi-token-plan-sgp`、`xiaomi-token-plan-ams` 或 `zai`。它们不会出现在提供方选择器、模型发现、登录与模型解析中。已安装的 pi-ai 包仍随包提供这些路由，是本构建主动移除。

存量 `llm-pi-ai` profile 点名其中一条时会加载失败并报 `provider "<id>" is removed from this build; delete the route or select another provider`；删除该条目后其余路由即恢复。`deepseek-official` 与 `deepseek-account` 插件不受影响。

## 迁移

1. 打开 `$DSH_HOME/profiles/<profile>/cordis.patch.yml`，删除 `llm-pi-ai.config.providers` 下每条被移除路由的条目。设置页「模型」写入的是同一份文档；删除其中保存的行即可。
2. 若默认模型指向被移除的路由，在设置页「模型」改选其他模型，或更新 `agent-default-model` 条目。
3. 若插件因加载失败而不可用，重启服务后确认设置页「模型」列出预期的提供商。
4. 需要继续使用原服务时，以自定义模型 API 接入其端点、协议与凭据，并使用不同的 Provider ID；被移除的 ID 仍会被拒绝。

`bedrock-converse-stream`、`google-vertex` 与 `azure-openai-responses` 不是自定义模型 API 的协议选项，因此手工声明的路由无法选择它们。其余被移除的端点使用 OpenAI 兼容的对话补全：Xiaomi 开放平台、SGP/AMS/Moonshot/阿里云端点、Hugging Face 路由端点 `router.huggingface.co/v1`、OpenCode Console `opencode.ai/inference/v1`、Baseten `inference.baseten.co/v1`、Cerebras `api.cerebras.ai/v1`、Radius `radius.pi.dev/v1`、NVIDIA `integrate.api.nvidia.com/v1`、Ant Ling `api.ant-ling.com/v1`、Together `api.together.ai/v1`、Fireworks `api.fireworks.ai/inference/v1`、Groq `api.groq.com/openai/v1`、Mistral `api.mistral.ai/v1`、OpenRouter（`openrouter.ai/api/v1`，Anthropic Messages 用 `openrouter.ai/api`）与 Z.AI `api.z.ai/api/coding/paas/v4`；`vercel-ai-gateway` 与 `minimax` 需使用 Anthropic Messages，端点分别为 `ai-gateway.vercel.sh` 与 `https://api.minimax.io/anthropic`；Meta 与 xAI 需使用 OpenAI Responses，端点分别为 `https://api.meta.ai/v1` 与 `https://api.x.ai/v1`。

多数移除后该厂商仍由其他内置路由承接。`google-vertex` 的 14 个 id 全部出现在 `google` 上（三个目录字段不同），它的协议不可选。`azure` 的 45 个 id 出现在 `openai` 上，`minimax` 的三个出现在 `minimax-cn` 上，`zai-coding-cn` 承接 `zai` 七个模型中的三个（`glm-5.2` 另见 `opencode-go`）。按模型 id 核对，`opencode` 的 84 个 id 中 58 个仍在已提供路由上，`cloudflare-ai-gateway` 为 57 中的 44 个、`amazon-bedrock` 为 193 中的 26 个、`huggingface` 为 78 中的 15 个、`vercel-ai-gateway` 为 301 中的 87 个、`baseten` 为 22 中的 7 个、`together` 为 17 中的 7 个、`fireworks` 为 22 中的 6 个、`mistral` 为 40 中的 3 个、`nvidia` 为 20 中的 3 个、`qwen-token-plan` 与 `-cn` 为 20 中的 8 个（`-individual` 为 9 中的 5 个），`radius` 为 29 中的 26 个。`meta` 的两个 `-contributor` id 也出现在 `opencode-go` 上，六个模型中剩四个没有其他内置路由。`xai` 的 `grok-4.5`、`grok-4.6` 与 `grok-4.7` 也在 `github-copilot` 上以相同 id 提供（该路由的输出上限为 128,000），`grok-4.7` 还在 `opencode-go` 上以相同 id、能力与目录价格提供，只有 `grok-4.3` 没有其他内置路由。

`openrouter` 是其中移除面最宽的一条：396 个 id（除裸 `auto` 外都带厂商前缀）中，95 个在去掉前缀并忽略标点后与已提供路由上的模型 id 相同，2 个按显示名对应，其余 299 个没有其他路由，因此它拿掉的是模型覆盖。

其余 id 没有其他内置路由：`ant-ling` 的三个、`cerebras` 的两个、`groq` 的七个，以及上文统计的未覆盖条目——其中包括 `zai` 的 `glm-4.7`、`glm-5-turbo` 与 `glm-5.2-highspeed`，`baseten` 的两个 `-Fast` id，NVIDIA 的 Nemotron 与 Cosmos 条目，Hugging Face 的 63 个 Hub 模型名，以及 OpenCode 的付费独有与免费档条目。需要时按上面的端点以自定义模型 API 接入。
