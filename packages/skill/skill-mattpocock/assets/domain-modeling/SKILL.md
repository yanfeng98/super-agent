---
name: domain-modeling
description: "建立并磨利项目的领域模型。当讨论代码库术语、撰写或编辑 GLOSSARY.md、记录或编辑 ADR 时使用。"
---

# 领域建模

在设计的过程中主动建立并磨利项目的领域模型。这是一门*主动*的纪律：质疑术语、编出边缘场景，并在术语表与决策一旦成形的当下就把它们写下来。（仅仅为了取词汇而*读* `GLOSSARY.md` 不算这个技能：那是一行就能做完的习惯，任何技能都能做。这个技能处理的是你在改变模型的时候，而不只是消费它。）

## 文件结构

多数仓库只有一个上下文：

```
/
├── GLOSSARY.md
├── docs/
│   └── adr/
│       ├── 0001-event-sourced-orders.md
│       └── 0002-postgres-for-write-model.md
└── src/
```

如果根目录下存在 `GLOSSARY-MAP.md`，这个仓库就有多个上下文。上下文地图指向每一个的所在：

```
/
├── GLOSSARY-MAP.md
├── docs/
│   └── adr/                          ← system-wide decisions
├── src/
│   ├── ordering/
│   │   ├── GLOSSARY.md
│   │   └── docs/adr/                 ← context-specific decisions
│   └── billing/
│       ├── GLOSSARY.md
│       └── docs/adr/
```

按需创建文件：只有真的有东西要写时才创建。如果还没有 `GLOSSARY.md`，就在第一个术语定下来时创建它。如果还没有 `docs/adr/`，就在第一次需要 ADR 时创建它。

## 会话进行中

### 用术语表检验用词

当用户用的术语与 `GLOSSARY.md` 里既有的语言冲突时，立刻指出来。「你的术语表把 'cancellation' 定义成 X，但你想说的似乎是 Y。到底是哪个？」

### 把模糊的说法磨利

当用户用了含糊或一词多义的术语时，提一个精确的规范术语。「你说的是 'account'：你指的是 Customer 还是 User？这是两个不同的东西。」

### 讨论具体场景

讨论领域关系时，用具体场景去压力测试它们。编一些能探到边缘情况的场景，逼用户把概念之间的界线说清楚。

### 与代码交叉核对

当用户断言某个东西是怎么工作的，就去核对代码是否同意。发现矛盾就摆出来：「你的代码会取消整张 Order，但你刚说可以部分取消。哪个是对的？」

### 就地更新 GLOSSARY.md

一个术语定下来，就地更新 `GLOSSARY.md`。不要攒起来一起写：发生的时候就记下。使用 [GLOSSARY-FORMAT.md](./GLOSSARY-FORMAT.md) 里的格式。

`GLOSSARY.md` 里不应有任何实现细节。不要把 `GLOSSARY.md` 当成 spec、草稿本，或实现决策的存放处。它是术语表，仅此而已。

### 克制地提议写 ADR

只有这三条同时成立时，才提议创建 ADR：

1. **难以回退**：将来改主意要有可观的代价
2. **脱离上下文会显得费解**：将来的读者会想「他们为什么这么做？」
3. **真做过取舍**：确实存在多个选项，而你出于具体的理由选了其中一个

三条里缺任何一条，就不要写 ADR。使用 [ADR-FORMAT.md](./ADR-FORMAT.md) 里的格式。
