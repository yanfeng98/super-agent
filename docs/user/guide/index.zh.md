# 使用 Web UI

[English](index.md) | 中文

请先按照[根目录 README](../../../README.md#run) 中的说明启动 Web UI；命令会打印其访问地址。本指南从服务器已经运行的状态开始。`dsh` 进程会把启动时所在的目录作为默认文件系统位置；全新的 Web UI 则不会选中任何工作区，你需要添加一个工作区。

## 配置模型

打开**设置 → 模型**，输入 [DeepSeek API 密钥](https://platform.deepseek.com/)并保存。模型路由会立即可用，不需要重启服务器。

[模型配置指南](./providers.zh.md)介绍其他提供方和自定义 OpenAI 兼容端点。

## 选择工作区

点击**选择工作区**，添加启动 `dsh` 时所在的项目目录，然后选中它。选中工作区前，会话输入框不可用。

## 运行任务

启动一个会话并发送：

> Summarize this repository and identify its main packages.

Agent（智能体）可以读取和编辑工作区文件、运行命令、委派工作并维护计划。如果根据当前权限策略，某项操作需要审批，Web UI 会先询问你。

## 召回之前的输入

输入框会为当前工作区文件夹保留最新 100 条被接受的输入。输入框为空时，按 ↑ 召回最新一条；↑ 和 ↓ 在更旧和更新之间移动，↓ 越过最新一条时清空输入框。触发菜单、IME 组合、非空草稿或附件仍保留各自的箭头行为；编辑召回的内容会结束历史导航。历史保存在当前浏览器中，并按工作区的精确路径分桶，因此更换浏览器，或重命名、移动文件夹后，会从空历史开始。

## 继续使用

- [使用内置工程技能](./engineering-skills.zh.md)
- [使用 rust-anything 迁移到 Rust](./rust-anything.zh.md)
- [配置模型](./providers.zh.md)
- [使用 Python SDK](./python-sdk.zh.md)
- [在反向代理之后发布 Web UI](./public-deployments.zh.md)
- [卸载 DSH 桌面端](./desktop-uninstall-cleanup.zh.md)
- [使用其他 CLI 模式](../../../apps/cli/README.zh.md)
- [开发插件](../develop/basic/index.zh.md)
