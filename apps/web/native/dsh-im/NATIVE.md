# Native IM bots (vendored dsh-im)

This directory is the source-vendored copy of [`xmanrui/dsh-im`](https://github.com/xmanrui/dsh-im) that ships with this Super Agent checkout as a built-in capability. It is not installed from npm and it is not a third-party profile plugin.

This native build excludes Slack, Telegram, Discord, and WhatsApp entirely: their Host/Client directories, command wiring, settings tabs, dependencies, and tests were removed. The remaining channels are WeChat, Feishu, DingTalk, WeCom, WeCom app, QQ, iMessage, and email. Matrix and the AI Office connector were removed in a later change; their Host/Client directories, settings tabs, shared codecs, dependencies, and tests are gone too.

Change records: [`logs/00027`](../../../../logs/00027-原生集成dsh-im.md) (integration), [`logs/00028`](../../../../logs/00028-移除四渠道.md) (the four-channel removal and audit fixes), and [`logs/00033`](../../../../logs/00033-移除Matrix与Office.md) (Matrix and AI Office removal).

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
4. `/goal` and `/plan`: delegated from the shared `TextHarnessBridge` and from the Feishu, DingTalk, WeCom, WeCom app, and QQ bridges to the bound Session's Host command gateway. Help text, command catalogs, and the Feishu slash-command manifest include both commands. Upstream already supported them in Weixin only.
5. Removed the standalone installer `bin/`, the Cloudflare badge `worker/`, `poll*.mjs`, `wrangler.jsonc`, and their tests. Tests for the disabled npm updater were removed; behavior tests were extended for the native build and the two new commands.
6. Removed Slack, Telegram, Discord, and WhatsApp end to end (source, Host/Client wiring, command catalogs, delivery/access definitions, i18n dictionaries, Baileys, scripts, and channel-specific tests). The remaining test suite has 3,680 passing cases; helper scripts that referenced the deleted channels were removed or updated.
7. Removed Matrix and the AI Office connector end to end (channel sources, Host/Client entries, settings tabs and RPC, shared conversation codecs, delivery/access/diagnostic definitions, `@matrix-org/olm`, helper scripts, and channel-specific tests). The remaining test suite has 3,548 passing cases.

## Syncing upstream

1. Copy the upstream `src/`, `plugin-src/`, `test/`, `docs/`, `assets/`, `locale/`, README files, `LICENSE`, and `THIRD_PARTY_NOTICES.md` over this directory.
2. Re-apply every local modification above and update the baseline commit in this file.
3. Keep `lib/` out of the synced copy; run `pnpm --filter @xmanrui/dsh-im run build`.
4. Run `node --test test/*.test.mjs test/channels/*/*.test.mjs` inside this directory, then the repository build and a Web boot check.

## Boundaries

- The panel follows the DSH interface language and keeps credentials Host-side; the browser receives only status, QR/manifest data, and masked identifiers.
- Channel connections, permissions, and platform ToS still belong to the upstream implementation. The unofficial WeChat and iMessage paths keep their upstream caveats.
