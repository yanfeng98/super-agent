# 00004 — 新增 `make web-bg` 与 `make web-stop`

- **日期**：2026-11-12
- **背景**：`pnpm dsh web` 因 19:59 启动的旧实例仍占用 3080 而启动失败。手工 `setsid` + PID 文件容易留下孤儿进程，改用 tmux 会话托管后台实例。
- **范围**：根 `Makefile`、`README.md`、`docs/development.md` 与 `docs/development.zh.md`（含 `docs/development.i18n.yaml` 哈希）。

## 1. 变更

| 文件 | 变更 |
|---|---|
| `Makefile` | 新增 `web-bg`、`web-stop` 两个目标，以及 `WEB_SESSION`（默认 `dsh-web`）、`WEB_LOG`（默认 `/tmp/dsh-web.log`）两个可覆盖变量；`help` 补充两行 |
| `README.md` | 「日常开发」补充 `make web-bg` / `make web-stop` 一行 |
| `docs/development.md` / `.zh.md` | 「应用命令」一节补充两个目标的一句话说明，中英同步 |
| `docs/development.i18n.yaml` | `verify-translation-pairing --write docs/development.md` 重新记录 |

## 2. 行为

- `make web-bg`：在 `dsh-web` 会话中运行 `pnpm run start:web --no-open $(ARGS)`，输出写入 `WEB_LOG`（权限 600，内容含 token）；最多等待 45 秒从日志中取带 token 的 URL 打印。会话已存在时退出码 1 并提示 attach/stop；启动即退出时打印日志尾部（端口冲突会直接显示 `EADDRINUSE`）。
- `make web-stop`：`tmux kill-session -t dsh-web`，会话不存在时提示并以 0 退出（幂等）。
- 两个目标都先检查 tmux 是否存在；`tmux kill-session` 以 SIGHUP 结束整个会话，不会像只 kill pnpm 父进程那样留下 `sh -c` 与 node 孤儿。

## 3. 校验

| 命令 | 结果 |
|---|---|
| `make web-bg` | 启动成功，打印 `http://127.0.0.1:3080/?token=…`；无 token 请求返回 401 |
| 会话已存在时再次 `make web-bg` | 提示已存在并以退出码 1 结束 |
| `make web-stop` | 会话结束、3080 释放；再次执行提示无会话且退出码 0 |
| 3080 被其他进程占用时 `make web-bg` | 识别启动失败并打印日志尾部，含 `dsh: startup failed` 与 `listen EADDRINUSE` |
| 停止后再次 `make web-bg` | 正常启动并打印新的 token URL |
| `make web-bg ARGS='--port 3084'` | 参数转发到 `dsh web`，日志与端口均为 3084 |
| 停止后用 `ss` 与 `ps` 检查 | 无监听端口、无 `bin.ts web` 残留进程 |
| `pnpm run test:docs` | 全部通过 |

## 4. 复查修正

- 日志含 token，原实现按默认 umask 生成 `0644`，同机其他用户可读。改为 `umask 077` 创建并对既有文件 `chmod 600`，实测 `0644` 的旧日志在下次启动后变为 `0600`。
- 启动失败时 `tail -20` 把首行 `dsh: startup failed` 截掉了，改为 `tail -40`。
- URL 抓取只匹配 `http://`，传 `--tls-cert` 时不会打印；改为 `https?://`。
- 45 秒内日志仍未出现 URL 时，原实现只打印“serving”而没有地址；改为显式提示 `URL not in the log yet; run: tail -f …`。
- `WEB_LOG` 指向不存在的目录时直接失败；补充 `mkdir -p`，并用 `WEB_LOG=/tmp/opencode/nested/dsh-web.log` 验证通过。
- 重复传 `--no-open`（`ARGS='--no-open --port 3082'`）不会报错，仍正常启动。

## 5. 已知限制

- `tmux new-session` 取的是 tmux server 的环境变量，而非调用 `make` 的当前 shell（实测：已有 server 时，新会话拿不到此刻新导出的变量）。本机还有别的 tmux 会话时，`make web-bg` 不会带上新导出的代理或 PATH 变更；API key 保存在 `~/.dsh`，不受影响。`make web-stop` 关掉最后一个会话后 server 退出，下次启动重新取当前环境。
