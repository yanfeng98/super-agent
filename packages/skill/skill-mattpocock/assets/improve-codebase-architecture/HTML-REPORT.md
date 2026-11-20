# HTML 报告格式

架构评审渲染成操作系统临时目录里的一个自包含 HTML 文件。Tailwind 与 Mermaid 都来自 CDN。Mermaid 可靠地处理图状示意图；手工搭的 div 与内联 SVG 处理更有编辑排版感的视觉（体量图、剖面图）。两者混着用：不要什么都靠 Mermaid，它会开始显得千篇一律。

## 脚手架

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Architecture review for {{repo name}}</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      mermaid.initialize({ startOnLoad: true, theme: "neutral", securityLevel: "loose" });
    </script>
    <style>
      /* small custom layer for things Tailwind doesn't cover cleanly:
         dashed seam lines, hand-drawn-feeling arrow heads, etc. */
      .seam { stroke-dasharray: 4 4; }
      .leak { stroke: #dc2626; }
      .deep { background: linear-gradient(135deg, #0f172a, #1e293b); }
    </style>
  </head>
  <body class="bg-stone-50 text-slate-900 font-sans">
    <main class="max-w-5xl mx-auto px-6 py-12 space-y-12">
      <header>...</header>
      <section id="candidates" class="space-y-10">...</section>
      <section id="top-recommendation">...</section>
    </main>
  </body>
</html>
```

## 页头

仓库名、日期，以及一份紧凑的图例：实心方框 = 模块，虚线 = 接缝，红箭头 = 泄漏，深色粗框 = 深模块。不要引言段落。直接进入候选。

## 候选卡片

扛起分量的是示意图。散文稀疏、平实，用术语表里的词（来自 `/codebase-design` 技能），不加修饰。

每个候选就是一个 `<article>`：

- **Title**：短，点名这次深化（例如「把 Order intake 流水线收拢」）。
- **Badge row**：推荐强度（`Strong` = 翡翠绿，`Worth exploring` = 琥珀色，`Speculative` = 石板灰），再加一个依赖类别标签（`in-process`、`local-substitutable`、`ports & adapters`、`mock`）。
- **Files**：等宽字体列表，`font-mono text-sm`。
- **Before / After diagram**：全篇的中心件。两列，并排。模式见下。
- **Problem**：一句话。哪里疼。
- **Solution**：一句话。改了什么。
- **Wins**：项目符号，每条 ≤6 个词。例如「测试打在一个接口上」、「定价逻辑不再泄漏」、「删掉 4 个浅包装」。
- **ADR callout**（如适用）：一行，放在琥珀色底色的框里。

不要解释性的段落。如果一张示意图需要一段话才能看懂，就重画这张图。

## 示意图模式

挑适合这个候选的模式。混着用。不要让每张图都长一个样。多样性本身就是要点的一部分。

### Mermaid 图（依赖 / 调用流的主力）

当要点是「X 调 Y，Y 调 Z，看看这一团乱」时，用 Mermaid 的 `flowchart` 或 `graph`。把它包进一张 Tailwind 风格的卡片里，免得显得像空投下来的。用 classDef 把泄漏的边染红、把深模块染深。时序图很适合「改前：6 次往返；改后：1 次」。

```html
<div class="rounded-lg border border-slate-200 bg-white p-4">
  <pre class="mermaid">
    flowchart LR
      A[OrderHandler] --> B[OrderValidator]
      B --> C[OrderRepo]
      C -.leak.-> D[PricingClient]
      classDef leak stroke:#dc2626,stroke-width:2px;
      class C,D leak
  </pre>
</div>
```

### 手工搭的方框与箭头（当 Mermaid 的布局跟你对着干时）

模块是带边框与标签的 `<div>`。箭头是内联 SVG 的 `<line>` 或 `<path>` 元素，绝对定位压在相对定位的容器上。当你希望「改后」那张图感觉像一个粗边框的深模块、内部灰掉时，就用这个办法，因为 Mermaid 渲染不出那个恰当的分量。

### 剖面图（适合一层一层都很浅的结构）

把横向色带（`h-12 border-l-4`）叠起来，画出一次调用穿过的那些层。改前：6 层薄片，每层什么也不做。改后：1 条厚色带，标着合并后的职责。

### 体量图（适合「接口和实现一样宽」）

每个模块两个矩形：一个代表接口的表面积，一个代表实现。改前：接口矩形几乎和实现矩形一样高（浅）。改后：接口矩形矮，实现矩形高（深）。

### 调用图折叠

改前：一棵函数调用树，画成层层嵌套的方框。改后：同一棵树折叠进一个方框，如今变成内部调用的那些调用在其中淡显。

## 样式指南

- 偏向编辑化，不要企业仪表盘。留白要慷慨。标题的衬线字体可选（`font-serif` 配 stone/slate 效果不错）。
- 颜色要省着用：一个强调色（翡翠绿或靛蓝），加上表示泄漏的红色与表示警告的琥珀色。
- 示意图保持在 ~320px 高，让前后对比能并排舒服地放下，不用滚动。
- 图里的模块标签用 `text-xs uppercase tracking-wider`，让它们读起来像示意图，而不是 UI。
- 唯一的脚本是 Tailwind CDN 与 Mermaid 的 ESM import。报告除此之外是静态的：没有应用代码，除了 Mermaid 自身的渲染之外没有任何交互。

## Top recommendation 小节

一张更大的卡片。候选名、一句说明为什么、一条指向它那张卡片的锚点链接。就这些。

## 语气

大白话，简洁，但架构名词与动词直接来自 `/codebase-design` 技能。简洁不是漂移的借口。

**Use exactly:** module、interface、implementation、depth、deep、shallow、seam、adapter、leverage、locality（严格使用：模块、接口、实现、深度、深、浅、接缝、适配器、杠杆、局部性）。

**Never substitute:** component、service、unit（指 module 时：组件、服务、单元）· API、signature（指 interface 时：API、签名）· boundary（指 seam 时：边界）· layer、wrapper（指 module 时：层、包装，你想说的就是 module）。

**贴合这套风格的措辞：**

- 「Order intake 模块是浅的：接口几乎赶上实现。」
- 「定价逻辑穿过接缝泄漏。」
- 「深化：一个接口，一处测试点。」
- 「两个适配器才让接缝成立：生产用 HTTP，测试用内存实现。」

**Wins 项目符号**用术语表里的词说出收益：*「局部性：bug 集中在一个模块里」*、*「杠杆：一个接口，N 个调用点」*、*「接口缩小；实现吸收掉那些包装」*。不要写 *「更容易维护」* 或 *「代码更干净」*，因为这些词不在术语表里，赚不到自己的位置。

不要含糊其辞，不要清嗓子，不要「值得一提的是……」。一句话能写成一条项目符号，就写成项目符号。一条项目符号能砍掉，就砍掉。一个词不在 `/codebase-design` 的术语表里，就先去找表里有的，再考虑自己造一个。
