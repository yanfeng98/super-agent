---
name: handoff
description: 把当前对话压缩成一份交接文档，好让另一个 agent 接着做。
argument-hint: "下一个会话用来做什么？"
disable-model-invocation: true
---

写一份交接文档，总结当前对话，好让一个全新的 agent 能接着做这件事。保存到用户操作系统的临时目录（`$TMPDIR`，没有就用 `/tmp`；Windows 上是 `%TEMP%`），而不是当前工作区。

文档里要有一个 "suggested skills" 小节，写明下一个 agent 该为哪些技能调用 `skill` 工具。

不要重复其他产物里已经记录过的内容（spec、计划、ADR、issue、提交、diff）。改用路径或 URL 引用它们。

把任何敏感信息脱敏，例如 API 密钥、密码，或可识别个人身份的信息。

如果用户传了参数，就把它们当作对下一次会话要聚焦什么的描述，并据此调整这份文档。
