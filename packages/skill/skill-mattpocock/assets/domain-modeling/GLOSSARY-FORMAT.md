# GLOSSARY.md 的格式

## 结构

```md
# {Context Name}

{一到两句话，说明这个上下文是什么、为什么存在。}

## Language

**Order**:
{对这个术语一到两句话的描述}
_Avoid_: Purchase, transaction（采购、交易）

**Invoice**:
交付后发给客户的付款请求。
_Avoid_: Bill, payment request（账单、付款请求）

**Customer**:
下单的个人或组织。
_Avoid_: Client, buyer, account（客户、买家、账户）
```

## 规则

- **要有主见。** 同一个概念有多个词时，挑最好的那个，把其余的列在 `_Avoid_` 下。
- **定义要收紧。** 最多一到两句。定义它*是*什么，不是它做*什么*。
- **只收这个项目上下文特有的术语。** 通用编程概念（超时、错误类型、工具模式）不属于这里，哪怕项目大量使用它们。加一个术语之前先问：这是本上下文独有的概念，还是通用编程概念？只有前者才属于这里。
- 当自然成簇时，**把术语归到小标题下**。如果所有术语都属于同一个内聚的领域，一张扁平列表就够了。

## 单上下文与多上下文仓库

**单上下文（多数仓库）：** 仓库根目录下一个 `GLOSSARY.md`。

**多上下文：** 仓库根目录下的 `GLOSSARY-MAP.md` 列出各个上下文、它们的所在，以及彼此的关系：

```md
# Glossary Map

## Contexts

- [Ordering](./src/ordering/GLOSSARY.md): 接收并跟踪客户订单
- [Billing](./src/billing/GLOSSARY.md): 生成发票并处理付款
- [Fulfillment](./src/fulfillment/GLOSSARY.md): 管理仓库拣货与发货

## Relationships

- **Ordering → Fulfillment**: Ordering 发出 `OrderPlaced` 事件；Fulfillment 消费它们以开始拣货
- **Fulfillment → Billing**: Fulfillment 发出 `ShipmentDispatched` 事件；Billing 消费它们以生成发票
- **Ordering ↔ Billing**: 共享 `CustomerId` 与 `Money` 类型
```

这个技能会推断该用哪种结构：

- 如果存在 `GLOSSARY-MAP.md`，读它来找出各个上下文
- 如果只有根目录的 `GLOSSARY.md`，就是单上下文
- 如果两者都没有，就在第一个术语定下来时按需创建根目录的 `GLOSSARY.md`

存在多个上下文时，推断当前话题与哪一个相关。分辨不清就问。
