---
kind: upgrade-guide
description: "pi-ai 模型目录升级到 1.6.0，已保存的、指向被移除 id 的模型选择会以 UNKNOWN_MODEL 失败。"
---

# 被移除的目录模型 id

[English](guide.md) | 中文

## 变更

`@earendil-works/pi-ai` 由 1.3.0 升到 1.6.0，安装目录提供的每条路由都改用 1.6.0 的模型清单。路由既有移除也有新增，因此指向被移除 id 的选择——`agent-default-model` 的 `model`，或某个会话保存的模型——不再解析，并报出 `UNKNOWN_MODEL` 且指明路由与模型。

`openai-codex` 移除 `gpt-5.5` 与 `gpt-5.3-codex-spark`：Codex 后端对 ChatGPT 账号拒绝这两个模型，上游因此删除了它们。该路由现在提供 `gpt-5.6-luna`、`gpt-5.6-sol`、`gpt-5.6-terra`、`gpt-6-astra`、`gpt-6-luna`、`gpt-6-sol`、`gpt-6.1-sol`、`gpt-6.1-terra`、`gpt-6.2-astra` 和 `gpt-6.2-sol`。

其他对外路由同样有移除：`github-copilot` 十个，包括 `gpt-5.4`、`gpt-5.5` 以及 `gemini-3.5-flash` 到 `gemini-3.7-flash` 系列；`openai` 三个（`gpt-5.2-chat-latest`、`gpt-5.3-chat-latest`、`gpt-realtime-2.1`）；`opencode-go` 六个；`google` 两个预览模型；`xiaomi-token-plan-cn` 两个 MiMo 2.5 id，由 `mimo-v2.6-flash` 与 `mimo-v2.6-pro` 取代。目录同时新增 `qwen-token-plan-individual-cn` 路由。

## 迁移

1. 收集所有指向受影响路由的已保存模型选择：`agent-default-model` 的 `provider` 与 `model`，以及模型选择器为每个会话显示的选项。
2. 把每个被移除的 id 换成该路由仍在提供的模型；上文列出了 `openai-codex` 的 id，`xiaomi-token-plan-cn` 上用 `mimo-v2.6-pro` 取代 `mimo-v2.5-pro`。
3. 在每条改动过的路由上发送一次请求。不再出现 `UNKNOWN_MODEL` 失败、且模型选择器列出新 id 时即完成。
