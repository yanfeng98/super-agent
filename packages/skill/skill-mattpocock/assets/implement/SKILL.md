---
name: implement
description: "根据 spec 或一组工单实现一项工作。"
disable-model-invocation: true
---

实现用户在 spec 或工单里描述的工作。

如果用户传了工单引用，先从问题跟踪器取回它，并在开始前说出它的标题。引用有歧义就问。

在可能的地方用 `skill` 工具调用 "tdd"，落在事先约定好的接缝（seam）上。

经常跑类型检查，经常跑单个测试文件，最后完整跑一遍测试套件。

做完之后，用 `skill` 工具调用 "code-review" 评审这项工作。

把工作提交到当前分支。
