# Native IM bots (vendored dsh-im)

This directory is the source-vendored copy of [`xmanrui/dsh-im`](https://github.com/xmanrui/dsh-im) that ships with this Super Agent checkout as a built-in capability. It is not installed from npm and it is not a third-party profile plugin.

## Baseline

- Upstream version: `5.10.0`
- Upstream commit: `fc2ceab` (2026-11-07)
- License: MIT; upstream `LICENSE` and `THIRD_PARTY_NOTICES.md` are preserved here.

## How it is mounted

- `packages/bundle/web-app/package.json` depends on `@xmanrui/dsh-im` as a workspace package.
- `packages/bundle/web-app/cordis.patch.yml` inserts the `dsh-im` row, which mounts both the Host plugin and the browser client (`dsh.client`).
- `scripts/build.ts` runs the package's own esbuild build before the Web shell build, producing `lib/index.js` and `lib/client.js`.
- No profile edit, plugin-market install, or `dsh plugin add` is needed. A fresh clone needs the usual `pnpm install && pnpm run build`, then `pnpm dsh web` (or Desktop).

## Local modifications from upstream

1. `package.json`: private workspace package, DSH `0.2.2-rc.3` compatibility, updated `dsh.client.inject` package names, build/test scripts, pinned Cordis 4.0.1 for the upstream test suite, and `pngjs` for its QR reload test.
2. Client and Host entry points: the npm updater is hidden and not mounted when the manifest is private, so the built-in copy cannot be replaced by the upstream npm release.
3. `plugin-src/host/host-scope.mjs`: resolve the Cordis root through `Symbol.for('cordis.root')`; Cordis 4.1 rejects reading the plain `ctx.root` property on contexts that do not inject a service named `root`.
4. `/goal` and `/plan`: delegated from the shared `TextHarnessBridge` and from the Feishu, DingTalk, WeCom, WeCom app, and QQ bridges to the bound Session's Host command gateway. Help text, command catalogs, Telegram menus, and the Feishu slash-command manifest include both commands. Upstream already supported them in Weixin only.
5. Removed the standalone installer `bin/`, the Cloudflare badge `worker/`, `poll*.mjs`, `wrangler.jsonc`, and their tests. Tests for the disabled npm updater were removed; behavior tests were extended for the native build and the two new commands.

## Syncing upstream

1. Copy the upstream `src/`, `plugin-src/`, `test/`, `docs/`, `assets/`, `locale/`, README files, `LICENSE`, and `THIRD_PARTY_NOTICES.md` over this directory.
2. Re-apply every local modification above and update the baseline commit in this file.
3. Keep `lib/` out of the synced copy; run `pnpm --filter @xmanrui/dsh-im run build`.
4. Run `node --test test/*.test.mjs test/channels/*/*.test.mjs` inside this directory, then the repository build and a Web boot check.

## Boundaries

- The panel follows the DSH interface language and keeps credentials Host-side; the browser receives only status, QR/manifest data, and masked identifiers.
- Channel connections, permissions, and platform ToS still belong to the upstream implementation. The unofficial WeChat, WhatsApp, Matrix, and iMessage paths keep their upstream caveats.
