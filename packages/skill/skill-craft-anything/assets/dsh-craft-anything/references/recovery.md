<!-- craft-anything skill reference -->
# Connection and recovery

| Symptom | What to do |
|---|---|
| No `craft-anything` tools in the host | Run `pnpm run build:craft-anything` in the repository root, then restart DSH; the CLI is compiled from the vendored Rust workspace, not registered through `setup` |
| Tools listed but a call is unavailable | Call `list_toolsets`, then `enable_toolset` and `mcp__craft-anything__load_tools` for the returned names; the server starts with only the Core toolset |
| Server fails to start | Run `native/craft-anything/bin/craft-anything-cli doctor` from the repository root; Rust 1.97 or newer is required to rebuild |
| The host asks to approve each tool | That is the host's own policy; do not change it without the user |
| A tool says an engine is missing | Follow its remediation (Settings → Engines in the desktop app) |

The built-in copy never edits agent-app configuration: DSH mounts the MCP server from the Web
`standard` preset and packages this skill itself. Upstream's `setup` and `uninstall` commands are
for other agent apps and must not be used to repair this connection.

Command-line fallback when no app is connected: every tool is a subcommand, for example
`native/craft-anything/bin/craft-anything-cli triage /path/to/program` or
`native/craft-anything/bin/craft-anything-cli core-target-open --json '{"path": "/path/to/program"}'`.
