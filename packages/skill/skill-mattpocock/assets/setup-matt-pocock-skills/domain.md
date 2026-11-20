# Domain Docs

工程类技能在探索代码库时应当如何消费这个仓库的领域文档。

## Before exploring, read these

- 仓库根目录下的 **`GLOSSARY.md`**，或者
- 仓库根目录下的 **`GLOSSARY-MAP.md`**（如果存在）：它为每个上下文指向一个 `GLOSSARY.md`。读其中每个与主题相关的。
- **`docs/adr/`**：读那些触及你即将改动区域的 ADR。在多上下文仓库里，还要看 `src/<context>/docs/adr/`，那里是上下文范围的决定。

如果这些文件里任何一个不存在，就**静默继续**。不要指出它们不存在；不要建议提前创建它们。`/domain-modeling` 技能（经由 `/grill-with-docs` 和 `/improve-codebase-architecture` 抵达）会在术语或决策真正定下来时惰性创建它们。

## File structure

单上下文仓库（大多数仓库）：

```
/
├── GLOSSARY.md
├── docs/adr/
│   ├── 0001-event-sourced-orders.md
│   └── 0002-postgres-for-write-model.md
└── src/
```

多上下文仓库（根目录存在 `GLOSSARY-MAP.md`）：

```
/
├── GLOSSARY-MAP.md
├── docs/adr/                          ← system-wide decisions
└── src/
    ├── ordering/
    │   ├── GLOSSARY.md
    │   └── docs/adr/                  ← context-specific decisions
    └── billing/
        ├── GLOSSARY.md
        └── docs/adr/
```

## Use the glossary's vocabulary

当你的产出要命名一个领域概念时（在 issue 标题、重构提案、假设、测试名字里），用 `GLOSSARY.md` 里定义的术语。不要漂移到术语表明确回避的同义词。

如果你需要的概念还不在术语表里，这是一个信号：要么你在发明项目不用的说法（重新考虑），要么那里真有一个缺口（记下来交给 `/domain-modeling`）。

## Flag ADR conflicts

如果你的产出与某个既有 ADR 相矛盾，就明确地把它摆出来，而不是悄悄地推翻：

> _与 ADR-0007（事件溯源的订单）相矛盾，但值得重新打开，因为……_
