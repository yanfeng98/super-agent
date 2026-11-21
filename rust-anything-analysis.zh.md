# rust-anything 项目分析

> 分析对象：<https://github.com/erkamyaman/rust-anything>（v2.1.0，commit `e795d53`，2026-11-20）
> 方法：浅克隆源码通读 + 3 个并行子 agent 分区审阅 + 下载官方 release 二进制实测（本机 Linux x86_64）。
> 结论性判断均标注证据；未能复现的只当作待验证声明。

---

> 本仓库的集成落地记录见 [`logs/00097`](logs/00097-原生集成rust-anything.md)，集成细节见 [`rust-anything-integration.zh.md`](rust-anything-integration.zh.md)。

## 0. 一句话结论

**这是一个把"用编码 agent 把应用逐单元改写成 Rust"这件事工程化的工具：它不翻译代码，而是用「录制原实现行为 → 逐用例差分 → 可执行的完成定义（gate）」把"行为等价"变成可重复的检查。**

方法论上是本仓库最值得学的部分：它把"正确性"从模糊的自然语言要求，换成了一套**可执行、可回放、可阻塞交付**的判定链。工程实现量惊人（4.7 万行 Rust、8 种源语言、16 个 MCP 工具），但**过程成熟度明显滞后于代码体量**：截至分析时 `main` 分支 CI 连续失败、v2.1.0 是从红树上发版的、没有任何覆盖率度量、测试里跑不到的语言有 4 个。宣传语（"prove"、19x、whole app）领先于可实现强度，需要打折扣读。

**适合**：CLI / 简单请求-响应 HTTP 服务 / 纯函数库的"贴身移植"，以及"只把热点循环搬到 Rust"的性能改造。
**不适合**：并发/有状态/长连接服务、依赖真实外部系统语义的系统、需要"语义证明"而非"语料一致"的场合。

---

## 1. 它解决什么问题，和"AI 一把梭翻译"差在哪

LLM 重写整个代码库的失败模式是众所周知的：能编译、能跑 demo，但边角行为（UTF-16 切片、`Math.round` 的半数舍入、`-0`、JSON key 顺序、异常文本）静默漂移，且几乎不可能人工 review 完。

rust-anything 的取舍是把问题拆成两步，并只对第二步负责：

1. **写 Rust**：交给你或你的编码 agent（Claude Code / Codex / Cursor / Gemini CLI / OpenCode）。工具不调模型，因此没有 token 成本和厂商锁定（`DECISIONS.md` D1）。
2. **证明没写歪**：工具提供"原实现此刻到底返回什么"的语料，并提供一个能卡住交付的闸门。

它明确宣称 **"It doesn't translate code; it proves the rewrite"**——这句话一半是准确的（不翻译），一半是营销（不是"证明"，是"在有限语料上的一致"）。见第 7 节。

---

## 2. 核心方法论：五步闭环

```
profile → analyze → bridge → record → 你写 port.rs → build → diff → bench → shadow → gate
```

| 步骤 | 命令 | 本质动作 |
| --- | --- | --- |
| 0 定位热点 | `profile` | 用各语言自带 profiler（`node --cpu-prof`、cProfile、`go test -cpuprofile`+pprof、JFR、Ruby TracePoint）跑真实负载，给出 inclusive time、调用者、可批处理的循环 |
| 1 规划 | `analyze` | 把仓库切成 unit（目录或源根单文件），算依赖图 → **SCC 得到 wave**（叶子先行、环必须一起动），给风险（UTF-16/正则/动态代码/Map 顺序…）、bridge 选型、readiness/priority 评分、依赖→crate 对照 |
| 2 搭桥 | `bridge` | 生成 crate：`src/port.rs`（纯 Rust、`#![forbid(unsafe_code)]`）+ `src/lib.rs`（绑定层）+ 各语言包装（napi/wasm-bindgen/PyO3/C ABI/cgo/JNI/magnus）+ `rust-anything.json`（清单） |
| 3 录制 | `record` | 按参数类型生成边角输入 + 语义陷阱 + 形状采样 + `--from-tests` 捕获真实测试调用，存 `cases.jsonl`；另存原单元指纹 `recorded-from.json` |
| 4 校验 | `diff` / `bench` / `shadow` | 在**调用者视角**（经同一 wrapper/绑定）回放每个用例；bench 比较每调用耗时并给结论（ship / 只搬部分 / 搬更大单元）；shadow 在同一进程双跑，返回原实现结果、只上报差异 |
| 5 交付闸门 | `gate` | fmt、clippy `-D warnings`、cargo test、unsafe 必须有 `// SAFETY:`、无残留 stub、错误路径必须有用例覆盖、无未标注 panic、体积不超源 3x+50 行、构建成功、导出 API 完整、全部录制用例一致 |
| 整应用 | `app record/replay/front-door/wire/lift` | 整个 CLI/HTTP 服务按同一套录制-回放推进；Rust 前门接管入口，未迁移路由回退原实现并计数；fallback=0 且各单元 gate 通过才允许 `lift` 成纯 Rust |

三个设计选择值得单独点出：

- **`port.rs` 与 `lib.rs` 分离**：绑定宏展开必产生 unsafe，隔离后"你的手写逻辑"可以 forever-safe，也能脱离 JS 运行时单测。
- **完成定义是命令，不是文档**：`gate` 同时是给人和给 agent 的接口，失败项自带修复命令（`fix` 字段）——这是典型的 agent 可用性设计。
- **可回退的渐进式切换**：shadow 模式与 front door 的 fallback 计数，让"先上线再逐步吞并"成为默认路径，而不是一次性大爆炸。

---

## 3. 我实测到的（可复现证据）

用官方 release v2.1.0 二进制（本地 rustc 1.96 无法编译，项目要求 1.97，见 §6.2）：

| 实验 | 结果 | 解读 |
| --- | --- | --- |
| `doctor` | 逐项 `ok/missing/outdated`，每条带 fix 命令（如 `rustup update stable`、`sdkmanager "ndk;28.0.13004108"`） | 依赖诊断做得比多数 CLI 都好，直接可交给 agent 执行 |
| 对 11 行 TS 文件 `analyze` + `bridge` | 0.01s 出计划，readiness 96，风险命中"UTF-16 切片"；生成完整 crate（port.rs/lib.rs/wire.rs/utf16.rs/js.rs/index.cjs） | 脚手架产物可读、注释说明下一步，`unimplemented!` 占位 |
| `record` 同一文件 | 0.11s，**703 个用例**，行覆盖率 100%，含 emoji/组合字符/BOM/`-0`/NaN/正则陷阱 | 录制成本极低，这是它最有说服力的部分 |
| `diff` 指向我手写的**错误** JS 实现 | **572/703 匹配**，逐条列出 `slug(":") expected "" got "-"` 等 | 差分器真的能抓行为漂移，输出对 agent 友好 |
| 对 CJS 包 `ms`（npm 本周 2.7k 下载量级的常见依赖）`analyze` | 151 行 1 unit，但 **0 个 export → `start_here` 为空** | TS/ESM 之外的真实世界盲区：`module.exports = fn` 这类 CJS 导出识别不到 |
| `mcp` `tools/list` | 16 个工具，参数 schema 完整；错误返回 `structuredContent {code, message, retryable, next}` | MCP 层是认真设计的，不是壳 |
| 官方 CI | `main` 最新 run **失败**：`macos → Gate every demo port` 与 `demo → Gate the fixture ports`；`check/msrv/windows/actionlint` 绿 | 见 §6.2/§6.3：已定位为**相对路径 bug**（非示例过期）：`build` 相对路径失败，绝对路径下同一 crate **12/12 项全过、900/900 用例一致** |

---

## 4. 代码架构地图

单 crate（无 workspace），`src/` 47,280 行，`tests/` 6,625 行，24 个直接依赖（oxc 解析 TS、tree-sitter 解析其余 8 种语言、clap、serde、ignore）。

| 层 | 文件 | 行数 | 职责 |
| --- | --- | --- | --- |
| 语言前端 | `src/ts/*`（oxc） | 2,134 | 发现包/文件、扫描事实、解析导出与类型、tsconfig 别名 |
| 语言插件 | `src/lang/{python,c,go,ruby,java,kotlin,php,csharp}.rs` | 7,271 | 发现/扫描/解析/导出，全部归一到 TS 风格类型方言 |
| 代码生成 | `src/codegen*.rs` | 9,689 | 4 种绑定（napi/wasm/PyO3/C ABI）+ Go/Java/Ruby/Kotlin 包装 + batch 类 + 二进制 wire |
| 录制/回放 | `harness*.rs`、`recording.rs`、`wire.rs`、`tolerance.rs` | 5,583 | 生成 node/python/go/java/ruby 驱动、比较语义、指纹、known-diffs |
| 规划/度量 | `analyze.rs`、`profile.rs`、`risks.rs`、`port_map.rs` | ~3,000 | 单元图、SCC 波次、评分、风险目录、profiler 解析 |
| 整应用 | `app.rs`、`whole_app.rs`、`front_door*.rs`、`lift.rs`、`shadow.rs`、`effects.rs`、`http.rs` | 8,947 | CLI/HTTP 录制、前门代理、副作用采集、抬升检查表 |
| 交互面 | `main.rs`、`mcp.rs`、`setup.rs`、`doctor.rs`、`gate.rs` | 6,381 | 15 个顶层命令 + `app` 的 5 个子命令、手写 JSON-RPC MCP、5 家 agent 配置写入、依赖诊断 |

数据流：`analyze` 产出 `rust-anything/analyze@1` JSON（版本化 wire contract）→ `bridge` 写 `rust-anything.json` 清单 → `record` 写 `cases.jsonl` + `recorded-from.json` → `diff/gate` 读清单回放 → `map` 汇总"哪些单元已迁移、原实现是否已变更"。**清单是整个工具的事实源**，`refresh` 靠它 + 重解析 `port.rs` 重新生成绑定。

---

## 5. 关键机制拆解（值得学的部分）

### 5.1 输入生成：类型驱动 + 语义陷阱，而非随机 fuzz
`record` 从参数类型生成输入（string/number/array/object/shape），叠加固定边角语料（空串、组合字符、emoji、BOM、`-0`、NaN、int-in-float-slot），再针对"宿主语言与 Rust 的已知语义鸿沟"注入陷阱：字符串长度/大小写折叠、有符号除法、溢出边界、浮点打印、Map/排序顺序、正则非 ASCII 折叠。对结构化返回值做 shape 采样，一半用例让一个字符串参数派生自另一个（让搜索类函数真的命中）。工程判断很对：**语料的边际价值在"边界"而不在"随机"**。

### 5.2 无损编解码（这个细节最能体现功力）
JSON 表达不了 `undefined`、`NaN/Infinity`、`-0`、BigInt、Map/Set、key 顺序、lone surrogate。它定义了一套 marker 编码（`{"$undefined":true}`、`{"$num":"NaN"}`、`{"$utf16":[...]}`…），Node 侧编解码与 Rust 侧对称。这些不是拍脑袋：`DECISIONS.md` X2 明确记录了每个 marker 是被哪个真实 mismatch 逼出来的（`splitKey` 的 `namespace: undefined`、`diffText` 切 emoji…）。

### 5.3 比较语义是分层的，不是 `==`
- 两边都抛异常 = 一致（不比消息；Go 的 error 字符串例外，因为调用者真的比它）。
- 浮点可给相对容差（`--float-tolerance`，逐 case 记录、随 crate 保存）；整数/字符串/结构仍严格。
- 顺序可声明"无序行"；key 顺序可选忽略。
- `known-diffs.jsonl`：允许"故意不同"，但必须写 `actual` + `reason`，且**条目失效（不再出现差异）也会让 gate 失败**，防止豁免清单活过差异本身。
- `recorded-from.json`：记录原单元内容指纹，原实现改了 → diff/gate 直接报"录制过期"，而不是拿旧语料发通行证。

这三条合起来，是在用工程手段对抗"验证者腐化"，思路很对。

### 5.4 性能工程：wire + batch
- **wire**：1 字节 tag + 定宽 LE 的二进制协议（不是 varint），一次 FFI 调用传所有参数；JS 侧用两块注册过的 buffer + `DataView`，字符串以 UTF-16 传输（保真）；形状不匹配则回退 JSON。省掉 napi 的逐属性转换（作者测到原生对象转换比 JSON 还慢 2–3x）。
- **batch**：`--batch fn:param` 生成 `<Fn>Batch` 类，items 只穿越一次、常驻 Rust，`run` 在全核上 map，`rank` 只回传索引；允许用户在 `port.rs` 写 `<Fn>Index` 做"准备一次、按查询增量"的优化。这条直击本质：**跨语言调用贵的不是函数体，是数据穿越和循环位置**。这也是为什么它反复强调"叶子函数别单独搬，搬循环"。

### 5.5 整应用：前门 + fallback 计数 + lift 检查表
- CLI：生成的 `main` 用 clap 解析**从成功用例反推**出的命令/开关，未接管的命令 `exec` 原实现并透传 argv/stdio/env。
- HTTP：axum 前门 + 未接管路由反代到原进程；每条 fallback 追加到日志文件，replay 用"行数增量"折算成每请求是否回退。
- `lift` 前有一张硬检查表：没有未接管的 unit、glue 里不再出现 `original::`、replay 100%、fallback=0、每个 unit gate 通过；然后在**副本**上剥离 cdylib/绑定/前门/回退，跑 fmt/clippy/test + 纯二进制 replay，全绿才回写。删除 `original.rs` 使"偷偷还调用原实现"编译不过——这是防假绿的关键设计。

### 5.6 Agent 集成
16 个 MCP 工具（analyze/bridge/refresh/record/build/diff/bench/profile/shadow/map/gate/app_*），配 205 行 `SKILL.md`（强制"gate 通过才算完成"、要求先给计划并等用户批准）+ 243 行 whole-app 流程；`setup` 会写 5 家 agent 的配置文件（**先备份、保 key 顺序、解析失败拒绝写、幂等**）；`doctor` 检查运行时版本 + MCP 是否应答 + skill 是否安装，并为每项给修复命令。这套"CLI + MCP + skill + doctor"四件套，是当前 agent-native 工具的标准形态，完成度高于大多数同类项目。

---

## 6. 工程质量评估

### 6.1 强项（确实做得好）
- **测试有真实闭环**：325 个测试（125 集成 + 200 单元），`tests/native.rs` 真的编译 napi/PyO3 crate 并加载；`tests/app.rs`/`http.rs` 对 8 种语言跑整应用录制/回放/前门/lift；错误路径、超时、cwd 隔离都有专门用例。没有 `#[ignore]`。
- **文档诚实度罕见**：`docs/how-it-works.md#what-it-cant-catch` 主动写清"通过只代表被录制的用例一致"；`docs/STATUS.md` 标注每项是 Verified/Tested 及最后一次真实运行日期；`docs/real-world-languages.md` 公开撤回并更正了 Java 3x 的结论。
- **子进程纪律**：统一 `process.rs`（stdin 关闭、独立进程组、超时杀组、保留最后 1MiB 输出）；临时目录按 pid 隔离并清理；不跟随符号链接；`--` 之后的用户命令只走 argv，不用 shell（唯一例外见下）。
- **错误处理**：主 crate `unsafe_code = "forbid"`；非测试代码仅约 38 处 `unwrap/expect`；无 `process::exit`（用 `ExitCode` + 单行错误）；MCP 错误带结构化 `code/retryable/next`。
- **发布工程**：crates.io 与 npm 均用 OIDC trusted publishing、tag 与 `Cargo.toml` 版本一致性校验、多平台预编译 + SHA256SUMS。

### 6.2 已核实的问题（按严重度）

| # | 问题 | 证据 | 影响 |
| --- | --- | --- | --- |
| P0 | **`main` CI 是红的** | 最新 run `44739679972`：`macos`（Gate every demo port）与 `demo`（Gate the fixture ports）失败；更早连续多次失败。**已复现并定位根因**：`harness::build` 通过 `run()` 启动 cargo，而 `run()` 无条件把子进程 cwd 换到临时目录（`src/harness.rs:1445-1447`、`src/harness.rs:1436-1443`），却把**相对** `--manifest-path` 传下去（`src/harness.rs:485-490`）；于是 `rust-anything gate demo/fixtures/async-rs` 在 build 检查处报 `manifest path ... does not exist`。换成绝对路径后同一 crate：`12/12` 项通过、`900/900` 用例一致（本机 1.97.0 实测）。CI 里的 gate 全是相对路径 → `demo`/`macos` 两个 job 结构性必红 | 核心卖点"gate 通过"在官方仓库自己的 CI 中不可复现；用户在本机用相对路径也会撞上。另：macOS job 未声明 `components: rustfmt, clippy`，而 gate 必跑 fmt/clippy，该 job 另有独立的必败原因 |
| P0 | **发版不依赖测试** | `release.yml` 仅 `needs: build`；v2.1.0 的 Release run 成功、同 commit 的 CI 失败 | 红树可发版；已发布的 v2.1.0 未经过当次绿色 CI |
| P1 | **零覆盖率度量** | 全仓无 tarpaulin/llvm-cov/codecov；`src/main.rs`（1,286 行 CLI 胶水）无测试 | 4.7 万行无覆盖率信号，回归只能靠 325 个用例的抽样 |
| P1 | **CI 覆盖缺口** | PHP/.NET/Kotlin/Android 测试在 CI 中跳过（`docs/STATUS.md:55-57` 自认）；Windows 只跑 `app::tests::shell_words` 与 `--test native` | 8 语言中的 4 种"支持"在 CI 中从未验证 |
| P1 | **wire 无一致性测试** | JS/Go/Java 三份独立编码器 + Rust 一份解码器（`WIRE_RS`），没有测试断言四方一致；只有 demo 产物间接覆盖 | 协议漂移会静默退化为 JSON 或产生错值 |
| P1 | **结果形状不匹配时会二次执行** | `codegen_tswire.rs:401-404,697-699`：wire 返回 MISMATCH 后改走 JSON 再调一次 | 有副作用的函数被执行两次 |
| P2 | **Java/Go 静默改写 lone surrogate** | Java wire `getBytes(UTF_8)`、Go `from_utf8_lossy`；JS 侧则显式拒绝 | 同一份语料在不同语言路上保真度不一致 |
| P2 | **C ABI 无 panic 兜底** | `codegen_c.rs:146` 直接调用 port；其余 5 种绑定都有 `catch_unwind` | C 侧 panic 会 abort 宿主进程 |
| P2 | **签名漂移静默降级** | `gate` 的 `api` 只比对导出名；wire 在 arity/类型不匹配时自动关闭且无告警 | 改了签名但用例恰好都过 → 性能悄悄退化为 JSON |
| P2 | **豁免权在 agent 手里** | `known-diffs.jsonl`、`// INFALLIBLE:` 都由写移植的一方填写，工具只校验格式与"条目是否失效" | 验证者与被验证者同体，gate 可被"合理的理由"合法绕过 |
| P2 | **`record --from-tests` 会改原仓库源码** | `from_tests.rs:163-210`：备份为 `.rust-anything-backup`、注入包装、`Drop` 恢复 | SIGKILL/断电会留下被改写的源文件；README 的"never touches the original"只针对 `lift`（表述本身没错，但实操需知情） |
| P2 | **replay 执行仓库中持久化的命令** | `rust-anything.json` 里的 `run`、`--db-reset/--db-dump` 会在 replay 时经 `sh -c`/`cmd /C` 重新执行（`effects.rs:206-223`） | 克隆别人的仓库后跑 replay = 执行其配置里的任意命令；需显式信任提示 |
| P3 | **打包三份重复** | `skills/rust-anything/{SKILL.md,whole-app.md}` 在 `plugin/`、`directory/` 下**逐字节相同**；`plugin/.mcp.json` 与根 `.mcp.json` 重复；插件描述漏 Kotlin | 版本漂移风险；已实际发生（SKILL.md 语言列表漏 Kotlin、漏 `map` 工具） |
| P3 | **陈旧元数据** | `SECURITY.md` 仍写"Latest minor (1.x)"；CONTRIBUTING 对 CI 描述不全 | 降低外部贡献者信任 |
| P3 | **advisories 检查永不失败** | `gate.rs:384-424`：`passed: true`，仅 warning | `gate` 的"安全检查"实际是提示 |

### 6.3 过程与历史（客观信号，不做诛心推断）
- 仓库创建于 **2026-11-16 12:10Z**，4 天内 294 次提交、123k 行、15 位贡献者、6,072 star / 482 fork、**0 个 open issue**、10 个 release（1.0.0 → 2.1.0）。
- **前 20 个提交共享同一时间戳**（11-16 14:13:36），单个提交即引入 1.7k–14.6k 行；子 agent 统计约 40 个初始提交时间戳落在同一 3 秒窗口内。→ **公开历史是导入/回填的，不能作为开发过程的证据**。
- 至少 2 个提交署名 `Co-Authored-By: Claude`；代码风格（模块级 `//!`、统一测试脚手架、18 种语言 README 一次性落地 3,753 行）高度一致。
- 结论边界：可以确定"公开历史被批量导入、AI 参与编写程度很高"；**不能**据此判断质量，也不能断言某个 benchmark 造假。热度（6k star / 4 天）与 18 watchers / 0 open issue 的组合不寻常，但可以用"强营销 + 快速响应 issue"解释。
- 采纳率仍然很小：npm 近一周 2,745 次下载、crates.io 累计 1,787 次（对比 star 数）。**Star 是传播信号，下载才是使用信号**。

---

## 7. 方法论批判（最重要的一节）

### 7.1 "prove" 的实际强度 = 有限语料上的差分测试
它证明的是：*在录制到的输入上，Rust 与原实现的输出/异常/副作用一致*。这不是等价性证明，差别在于：
- **语料覆盖**：类型驱动 + 固定陷阱，没有 coverage-guided 反馈（fuzz）。TS 侧测了行覆盖率，但**只报告、不回灌生成**。未走到的分支 = 未验证的分支。
- **输入相关性**：多参数函数靠启发式（"一半用例派生字符串"）产生有意义的组合；结构化参数的"合法而不平凡"取值靠 shape 采样，容易生成原实现直接抛错的语料（对错误路径有用，对主路径无用）。
- **状态与时间**：录制默认在"干净、单次、串行"的假设下进行；全局状态、时间、随机、环境变量、locale、文件系统与网络交互，只被"两次运行取一致"这一层部分吸收。

它自己在 `what-it-can't-catch` 里承认了这些，这是加分项；但 README 顶部"prove the rewrite"、"0 mismatches"这类措辞会把读者的期望拉到"证明"。

### 7.2 差分测试的经典盲区，本项目没有系统性解法
- **共同错误**：如果原实现本身有 bug，你只是把 bug 一起搬走（对"移植"是优点，对"重写"未必是目标）。
- **不可观测行为**：内存布局、日志、性能特征、错误文本（多数语言故意不比）。
- **并发与顺序**：HTTP 录制是**串行、`Connection: close`、整包缓冲**；没有 keep-alive、并发、WebSocket/SSE/upgrade、超时与背压。对"简单请求-响应服务"够用，对真实服务不够。
- **HTTP 保真度**：重复 header 被逗号合并（`Set-Cookie` 会坏）、header 顺序丢失、12 个 header 硬编码跳过、body 里不做 cwd/端口归一化。
- **副作用**：SMTP/HTTP 只拦得住"愿意走 proxy/env 的客户端"；HTTPS 只看到 `CONNECT` 并被拒绝；DB 只支持外部 shell 命令 dump/reset，无事务隔离语义。**在真实项目上跑 `--effects` 有把测试邮件发给真人的风险**（README 有警告，但这是"默认危险、需自觉"的设计）。

### 7.3 自证循环：验证的信任根仍是 agent
gate 的许多"防线"最终由写移植的一方来满足：`known-diffs.jsonl` 的 reason、`// INFALLIBLE:` 的理由、`port.rs` 里的实现、`Cargo.lock` 的选择。工具能做的是"让绕过留下痕迹并使其成本变高"（失效条目必报错、指纹过期必报错、删除 original 使假绿编译失败），但它**不能提供独立于 agent 的第三方保证**。在受监管场景里，这套东西应被视为"高置信度的回归证据"，而不是"审计结论"。

### 7.4 benchmark 的方法学：诚实，但不够
优点：固定机器、两次运行、先比输出再计时、报告里甚至包含 0.48x 的失败结论、并公开撤回错误结论。
不足：n=2、单一 M4 机器、无置信区间/无 CI 化基准、部分对比是"Rust 多核 + mimalloc vs 原实现单线程"（作者标注了这一点，但标题数字不含此限定）；Prettier 的 19x 是 CLI wall time（Node 启动占比高）。**方向性结论可信（批处理确实能快一个量级），具体倍数不可直接引用到别的机器/负载。**

### 7.5 "whole app" 的实现强度
`app record` + 前门 + `lift` 是一条真实工程链（不是 demo 脚本）：两次运行消噪、trycmd 用例、fallback 日志、在副本上先演练、删除 original 使回退不可编译。但"整个应用改写成纯 Rust"当前的真实含义是 **"匹配有限录制语料 + 自报 fallback=0"**；Prettier 这个旗舰案例本身是 **8.1% fallback**（1,387/17,224 仍由原版 Prettier 执行），json-server 的 514 个 HTTP 用例也只是薄薄一层协议切片。`lift` 之后的 CI 检查（trycmd）还比 replay 弱（跳过无序用例、忽略 `.effects` 与浮点容差）。→ **可信的迁移框架，不是"服务等价"的证明装置。**

---

## 8. 同类对比

| 类别 | 代表 | 与本项目的关系 |
| --- | --- | --- |
| 自动转译（C→Rust） | C2Rust、Corrode、DARPA TRACTOR 系 | 它们直接产出（往往 unsafe 的）Rust 并试图语义对齐；本项目**不转译**，把"写"交给 agent，把"验"做成基础设施——这是差异化，也是它能跨 8 种语言的原因 |
| Python→Rust | Depyler 等 | 同上：本项目不声称自动转换，靠录制语料约束手写/agent 写的代码 |
| FFI 绑定生成 | napi-rs、PyO3、wasm-bindgen、uniffi、cbindgen | 它把这些当**下游工具**复用，附加值在"选单元、生成 wrapper、保 API、验证行为、渐进切换"这一层，而不是绑定本身 |
| 性能改造 | 手写扩展、`napi`/`ctypes` 原型 | 它的 `profile → batch → bench 结论` 把"该不该搬、搬哪段、搬完是否值得"变成流程，避免"JIT 语言里逐函数调 Rust 反而更慢"的常见坑 |
| "AI 一把梭重写" | 各类 agent 直接重写仓库 | 本项目的整个存在理由就是反对这种做法的不可验证性；但它同样依赖 agent，只是把 agent 的自由度关进 gate 里 |

**真实竞品可能不是工具，而是"什么都不做"**：对大多数团队，成本最低的路径仍是"改写热点、保留原服务"。本项目给了这条路径一个纪律化的版本——这既是它的机会（低阻力落地），也是它的天花板（愿意整体迁移到 Rust 的团队本来就少）。

---

## 9. 适用 / 不适用

**适合**
- Node/Python/Ruby 的 CLI、纯函数库、格式化/解析类工具（Prettier、ms、Fuse.js 这类正是甜点区）。
- 单进程、请求-响应式 HTTP 服务（json-server 形态）。
- 只想把热点循环搬到 Rust：`profile` → `bridge --batch` → `bench` 给结论。
- 需要"渐进上线 + 可回退"的迁移：shadow / front door / fallback 计数。

**不适合 / 需谨慎**
- 并发、长连接、流式、WebSocket、gRPC streaming、复杂中间件链的服务。
- 依赖数据库事务隔离、分布式一致性、消息队列语义的系统。
- UI、编辑器扩展、动态代码（`eval`/`new Function`/动态 `import`）、重度正则（lookaround/backreference）、依赖 `Intl` 数据的代码——工具会标 `keep`/`blocked`，但别指望它兜底。
- 受监管/需要独立审计的"等价性证明"。这里只能得到"语料内一致"的证据。
- 无法接受"工具会执行你的代码（及副作用）"的环境。

---

## 10. 如果要试用：一条安全路径

1. 在**副本**上做，清空生产凭据（README 明确要求）；先 `doctor`。
2. `analyze --json` 看 `start_here`，人工确认 unit 划分与 `keep/blocked`；**不要跳过"等用户批准计划"这一步**。
3. 选一个纯函数 unit：`bridge` → `record`（补 `--from-tests`）→ 先看 `cases.jsonl` 里有没有你认识的边界，再让 agent 写 `port.rs`。
4. `diff` 看 trend（连续 3 次改动无新增匹配会报 stall，防 agent 瞎改）；`bench` 决定值不值得上线；`gate` 才是完成。
5. 任何 `known-diffs` / `// INFALLIBLE:` 都要人工复核理由——这是 gate 唯一的软点。
6. 上线先 shadow，再前门 + fallback 监控，最后才谈 `lift`。
7. `app replay` 前先审 `rust-anything.json` 里的 `run`/`--db-reset`/`--db-dump`：replay 会执行它们。

---

## 11. 如果要贡献：6 个高杠杆改进

1. **先修相对路径 bug，再谈别的**：`build` 里把 `crate_dir` 先 `std::fs::absolute`（`record`/`bench` 已这么做，`build` 漏了），或者让 `run()` 不替 cargo 换 cwd。两行改动，直接让官方 CI 与所有用相对路径的用户恢复绿色。
2. **让绿成为发版前提**：修 `macos` job 的 toolchain components；把 `release.yml` 改为 `needs:` 一个可复用测试工作流。当前"红树发版"对信任的伤害大于任何功能缺失。
3. **给 wire 加一致性测试**：一个 fixture schema，在 Rust 解码器与 JS/Go/Java 编码器之间做 round-trip 断言；顺便修"结果形状不匹配导致二次执行"（先快照结果再回退）。
4. **把类型系统从"字符串比较"换成解析后的类型**：用 `syn` 解析 `port.rs`，`refresh` 时显式 diff `manifest.exports` 与 port 签名，不一致就报错；顺手消灭 `via_json`（`contains("Value")`）这类子串判定。
5. **补覆盖率并把它接进 gate**：至少对新增代码设下限；把 PHP/.NET/Kotlin/Android 挪到一个带齐工具链的 Linux job（用 `RUST_ANYTHING_REQUIRE` 强制不跳过）。
6. **收敛重复与陈旧**：`skills/plugin/directory` 单一来源生成 + CI 校验；`SECURITY.md`/插件描述/CONTRIBUTING 与版本对齐；`advisories` 要么真失败要么改名"提示"。

---

## 12. 对 agent 工程的借鉴（与本仓库相关的部分）

即使不采用这个工具，它的几个模式值得直接抄：

1. **"完成"必须是一条命令**：`gate` 把"做完了"从 agent 的自我声明变成可执行判定，并且失败项自带 `fix`（下一步该敲什么）。这与 DSH 里"可执行的验收门"思路同源。
2. **给 agent 的 JSON 契约要版本化**：`rust-anything/analyze@1` 明确声明"破坏性变更就升版本"，比"字段随缘"稳定得多。
3. **错误要结构化**：MCP 错误返回 `{code, message, retryable, next}`，agent 才知道该重试、该改参数、还是该问人。
4. **诊断命令即文档**：`doctor` 的"检测 + 修复命令"比 README 里的安装说明更少误导。
5. **落地要可回退**：shadow / front door / fallback 计数，使"先上线一小块"成为默认，而不是一次大爆炸——这对任何"agent 产出的代码要进生产"的系统都适用。
6. **对抗验证者腐化**：指纹过期即失败、豁免条目失效即失败、删掉回退路径使假绿编译不过——这三招成本低、效果好。

---

## 附录：本次分析用到的复现命令

```sh
git clone --depth 1 https://github.com/erkamyaman/rust-anything /tmp/rust-anything
curl -sSL -o ra.tar.gz https://github.com/erkamyaman/rust-anything/releases/download/v2.1.0/rust-anything-v2.1.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf ra.tar.gz && ./rust-anything-v2.1.0-*/rust-anything doctor

# 实测：11 行 TS → 703 用例 → 错误实现被抓
rust-anything analyze /tmp/ra-lab/demo
rust-anything bridge /tmp/ra-lab/demo src/text.ts
rust-anything record /tmp/ra-lab/demo/rust/text-rs
rust-anything diff /tmp/ra-lab/demo/rust/text-rs --module bad2.cjs   # 572/703

# 盲区实测：CJS 包识别不到导出
npm pack ms@2.1.3 && tar xzf ms-2.1.3.tgz
rust-anything analyze package --json   # start_here: []
```

引用来源：
- 仓库：<https://github.com/erkamyaman/rust-anything>
- 决策记录：`DECISIONS.md`（D1–D6、A1–A10、L1–L10、X1–X6、M1–M10、S1–S16）
- 能力边界：`docs/how-it-works.md`、`docs/STATUS.md`、`docs/pitfalls.md`
- 实测与真实案例：`docs/real-world.md`、`docs/real-world-languages.md`、`demo/`
- 公开讨论（"0 mismatches" 争议）：<https://github.com/erkamyaman/rust-anything/issues/118>
