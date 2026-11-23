# rea 仓库内集成（源码 vendored，clone 即可用）

> 目标：任何人 clone 后 `pnpm install && pnpm run build && pnpm dsh web`，默认 `standard` preset 的会话就带着 rea 的 MCP 工具与 `dsh-rea` 技能——上游源码在仓库里、由仓库自己安装与构建，不依赖全局安装、`npx` 或 `rea setup`。
> 结论：已完成。上游 `6.10.1`（commit `65ee0e8`）源码落在 [`native/rea/`](native/rea/NATIVE.md)，`pnpm run build` 执行 `npm ci` 与上游构建，插件把 `scripts/rea.mjs mcp` 挂到 Web 的 `standard` preset。

## 1. 现在仓库里有什么

| 组成 | 位置 | 作用 |
| --- | --- | --- |
| 上游源码 | `native/rea/`（`6.10.1`，MIT） | 保留 `src/`、`scripts/`、`bridge/`、`native/`、`third_party/`、`licenses/`、lockfile 与构建配置；剔除只服务于上游开发、文档站、测试与外部分发的 `tests/`、`docs/`、`website/`、`examples/`、`.github/`、插件清单与 `src/**/*.{test,fixture}.ts`。基线、剔除清单与同步流程见 [`NATIVE.md`](native/rea/NATIVE.md) |
| 构建 | `scripts/build-rea.ts` + 根脚本 `build:rea` + `scripts/build.ts` 的 `rea MCP server` gate | 在 `native/rea` 里执行 `npm ci`（按 lockfile 摘要只装一次，约 3 分钟）与 `npm run build`（tsc、catalog 录制、rolldown bundle、技能产物，约 10 秒）；Node 26 上因上游 metadata 脚本依赖已被移除的 `stripTypeScriptTypes` transform 模式，只跑必须的 `compile` 链 |
| 插件包 | `packages/mcp/rea/`（`@deepseek-ai/dsh-rea`） | `src/launcher.ts` 解析 `native/rea/scripts/rea.mjs`，`src/index.ts` 挂一个 `dsh-mcp-client` 实例；默认 `catalogProfile: hot-cold` + `toolLoading: on-demand` |
| 技能 | `packages/skill/skill-rea/assets/dsh-rea/` | 上游 `.agents/skills/reverse-engineer-anything` 的正文与 8 份参考，改名为 `dsh-rea`，并换成内置副本的连接、按需加载与 CLI 说明；随包提供方 `@deepseek-ai/dsh-skill-rea`，菜单里归入 `内置 · morluto/rea` |
| 挂载点 | `packages/bundle/web-app/presets/standard.patch.yml` | `standard` agent 得到 `skill-rea` 与 `mcp-rea` 两行；`minimal`/`ptc` 与 headless/acp/sdk 不挂，keyless 快照不受影响 |
| 类型/清单 | `tsconfig.base.json`、`tsconfig.host.json`、`packages/mcp/README*`、生成的 config catalog / 模块图 / 技能包清单 | 让编译、产品可达性与文档门禁都认这两个包 |
| 本地化元数据 | 两个包的 `locale/{en,zh,ja,es,fr,de,ru,pt}.json` | `verify-locale-metadata` 要求 8 种发布语言的 `meta.title`/`meta.description` |
| 依赖形态 | `@deepseek-ai/dsh-mcp-client` 放在 `peerDependencies`（同时 devDependency） | 插件之间按仓库共享实例约定走 peer |
| 部署闭包 | `python/sdk-runtime/package.json` | `verify-runtime-closure` 要求 shipped preset 的插件与技能出现在部署清单里 |
| CI | 不需要新工具链 | npm 已随 Node 提供；`test`/`test:coverage` 与 `run-gates` 的 coverage/unit 聚合会自动先跑 `build:rea` |

**为什么用 `hot-cold` + `on-demand`**：上游完整目录有 263 个工具、`tools/list` 约 353 KB（~122k tokens）；eager 挂载会把每次请求的上下文直接压垮。`hot-cold` 只先 advertise 5 个工具（约 7.9 KB / ~2.8k tokens），agent 用 `discover_tools` 按任务词加载一页；DSH 的 MCP client 已支持 `notifications/tools/list_changed` 重同步，所以加载后的工具会进入目录，再由 `load_tools` 加入工具列表。

## 2. 实测证据

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 依赖安装 + 源码构建 | `pnpm run build:rea` | 见第 3 节（本轮实测） |
| 工具面 | 对 `scripts/rea.mjs mcp` 发 `initialize` + `tools/list` | 完整目录 263 个工具、353,207 字节 schema；`hot-cold` 初始 5 个；`discover_tools` 触发 `tools/list_changed` |
| 包单测 | `pnpm vitest run packages/mcp/rea packages/skill/skill-rea scripts/build-rea.spec.ts` | 见第 3 节 |
| 组合 | `pnpm dsh --profile web --dump-config` | `standard` preset 含 `mcp-rea` 与 `skill-rea`，`minimal` 不含 |
| 静态门禁 | `pnpm run build:check`、`verify-*` 相关项、`test:docs` | 见第 3 节 |

## 3. 验证记录

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 源码安装 + 构建 | `pnpm run build:rea`（Node 24.22.3） | 通过；`npm ci` 748 个包（冷缓存约 3 分钟，暖缓存 9 秒），上游构建 11.3 秒；生成 `dist/` 69 MB、`node_modules/` 883 MB、`skills/` 276 KB（均不进 git） |
| MCP 工具面 | 直接对 `scripts/rea.mjs mcp` 做 `initialize` + `tools/list` | 默认 `hot-cold` 初始 5 个工具；`full` 263 个工具、353,207 B schema；`discover_tools` 触发 `tools/list_changed` |
| 包单测 | `pnpm vitest run packages/mcp/rea packages/skill/skill-rea scripts/build-rea.spec.ts` | 3 个文件、19 个用例通过 |
| 覆盖率 | `pnpm vitest run --coverage`（include 两个包的 `src`） | 语句/分支/函数/行均 100% |
| 类型检查 | `pnpm run typecheck` | 5/5 通过 |
| 全量构建 | `pnpm run build` | 14/14 通过（29.1 s），含 `rea MCP server` gate（14.19 s）与 Web build |
| 构建后类型检查 | `pnpm run build:check` | 2/2 通过 |
| 静态门禁 | `DSH_GATE_CONCURRENCY=4 pnpm run check:ci:static` | 71/71 通过（219 s） |
| 文档门禁 | `pnpm run test:docs` | 21/21 通过 |
| Lint | `pnpm run lint` | 0 warning / 0 error（7,774 个文件） |
| 仓库卫生 | `pnpm run hygiene` | 27/27 通过（200.7 s） |
| 组合 | `pnpm dsh --profile web --dump-config` | `standard` preset 含 `skill-rea` 与 `mcp-rea` 两行 |
| 公开仓库链接 | `pnpm run verify-public-repository-links` | 通过 |

## 4. 生效方式

- 新 clone：`pnpm install && pnpm run build`（首次在 `native/rea` 里 `npm ci`，需要网络与 Node.js 24.22.3+），然后 `pnpm dsh web`；默认 preset 的会话直接带工具与技能。
- 已有 profile：bundle 与 preset 文件按仓库实时读取，重启 `dsh web` 后新会话生效；会话可 resume，但 rea 的按需目录从新连接重新开始，之前加载的工具需要重新发现。
- 改了 `native/rea/` 的源码：重跑 `pnpm run build:rea`（或全量 `pnpm run build`）。
- 想换运行时或入口：在 preset 行里配 `command`（Node.js 可执行文件）或 `launcher`。
- 验证一句话：重启后问「你现在能用哪些 rea 工具」，或直接说「用 rea 分析这个仓库里某个已发布构建的搜索功能」。

## 5. 已知取舍与可选下一步

- **构建成本**：首次 `npm ci` 下载 748 个包（约 3 分钟、`node_modules/` 约 880 MB），之后按 lockfile 摘要复用；CI 每个会构建/测试的 job 首次都会付这笔成本，后续可用 `actions/cache` 缓存 `native/rea/node_modules`。
- **Node 26 的构建差异**：上游 metadata/技能生成脚本依赖 Node 22–25 的 `stripTypeScriptTypes` transform 模式，Node 26 上 `build:rea` 只跑 `compile`；MCP 工具不受影响，rea 自己的 `search_guidance` 会退化为 `bundle_unavailable`，而 DSH 的 `dsh-rea` 技能随包提供，不依赖该产物。
- **打包运行时（Desktop、Python SDK 可执行文件）**：`native/rea` 在所有包目录之外，打包不会带上；这两处挂载插件后在运行时找不到 launcher，不提供工具（fail-soft），与 rust-anything 的已知回退相同。
- **需要外部分析引擎**：深度原生分析仍要用户自己安装 Hopper、Ghidra、IDA 或 JADX；工具会报告 provider 可用性，不安装引擎。浏览器类工具还需要本机已有的 Chrome/Electron。
- **会执行被分析的目标**：运行时观察、截图、环境录制以 Host 用户权限执行目标进程，并可能采集剪贴板/UI/网络数据；技能与包 README 都要求只在副本上操作。
- **工具目录很大**：完整 263 个工具按需加载；如果未来默认要缩小面，可在 preset 行加 `env: { REA_MCP_TOOL_FAMILIES: "session,application,javascript-runtime" }` 之类的 family 选择。
- **Windows 原生 provider**：上游仓库不提交 `native/windows/build/*.node`，vendored 副本同样没有；Windows 上的原生 Windows API 工具会报告不可用。

## 6. 回滚

```sh
git checkout -- package.json pnpm-lock.yaml .gitignore tsconfig.base.json tsconfig.host.json \
  packages/bundle/web-app/package.json packages/bundle/web-app/presets/standard.patch.yml \
  packages/mcp/README.md packages/mcp/README.zh.md python/sdk-runtime/package.json \
  docs/config-catalog.md docs/config-catalog.zh.md docs/module-graph.md docs/module-graph.zh.md \
  packages/preset/agent-preset/skills/dsh-doc/references/packages.md scripts/build.ts scripts/build.spec.ts \
  scripts/run-gates.ts scripts/run-gates.spec.ts
rm -rf native/rea packages/mcp/rea packages/skill/skill-rea scripts/build-rea.ts scripts/build-rea.spec.ts
pnpm install
```

`native/rea/node_modules/`、`dist/`、`skills/`、`.turbo/`、`.cache/` 都不进 git，删除目录即可清除构建产物。
