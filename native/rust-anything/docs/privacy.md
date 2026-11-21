# Privacy

rust-anything runs on your machine. The CLI, the MCP server and the skill send none of your code,
recordings or results anywhere, collect no telemetry, and need no account.

What does reach the network is what you or your agent run: Cargo downloads crates when a port is
built, `npm`, `pip` and the other package managers install what the original needs, and `gate`
runs `cargo deny` or `cargo audit` when one is installed, which fetch the RustSec advisory
database. Your agent's model provider sees what your agent sends it, under its own policy.

Questions: [open an issue](https://github.com/erkamyaman/rust-anything/issues).
