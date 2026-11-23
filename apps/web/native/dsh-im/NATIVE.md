# Native IM bots (vendored dsh-im)

This directory is the source-vendored copy of [`xmanrui/dsh-im`](https://github.com/xmanrui/dsh-im) that ships with this Super Agent checkout as a built-in capability. It is not installed from npm and it is not a third-party profile plugin.

This native build excludes Slack, Telegram, Discord, and WhatsApp entirely: their Host/Client directories, command wiring, settings tabs, dependencies, and tests were removed. The remaining channels are WeChat, Feishu, DingTalk, WeCom, QQ, iMessage, and email. Matrix and the AI Office connector were removed in a later change, and the WeCom self-built-app channel (enterprise callback, no group chat) in a fourth one; their Host/Client directories, settings tabs, shared codecs, dependencies, and tests are gone too.

Change records: [`logs/00027`](../../../../logs/00027-原生集成dsh-im.md) (integration), [`logs/00028`](../../../../logs/00028-移除四渠道.md) (the four-channel removal and audit fixes), [`logs/00033`](../../../../logs/00033-移除Matrix与Office.md) (Matrix and AI Office removal), and [`logs/00037`](../../../../logs/00037-移除企业微信应用渠道.md) (WeCom self-built-app removal).

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

1. `package.json`: private workspace package, DSH `0.2.2-rc.5` compatibility, updated `dsh.client.inject` package names, build/test scripts, Cordis through the repository's `workspace:~` link (the `ctx.root` fix below covers 4.1), and `pngjs` for its QR reload test.
2. Client and Host entry points: the npm updater is hidden and not mounted when the manifest is private, so the built-in copy cannot be replaced by the upstream npm release.
3. `plugin-src/host/host-scope.mjs`: resolve the Cordis root through `Symbol.for('cordis.root')`; Cordis 4.1 rejects reading the plain `ctx.root` property on contexts that do not inject a service named `root`.
4. `/goal` and `/plan`: delegated from the shared `TextHarnessBridge` and from the Feishu, DingTalk, WeCom, and QQ bridges to the bound Session's Host command gateway. Help text, command catalogs, and the Feishu slash-command manifest include both commands. Upstream already supported them in Weixin only.
5. Removed the standalone installer `bin/`, the Cloudflare badge `worker/`, `poll*.mjs`, `wrangler.jsonc`, and their tests. Tests for the disabled npm updater were removed; behavior tests were extended for the native build and the two new commands.
6. Removed Slack, Telegram, Discord, and WhatsApp end to end (source, Host/Client wiring, command catalogs, delivery/access definitions, i18n dictionaries, Baileys, scripts, and channel-specific tests). The remaining test suite has 3,680 passing cases; helper scripts that referenced the deleted channels were removed or updated.
7. Removed Matrix and the AI Office connector end to end (channel sources, Host/Client entries, settings tabs and RPC, shared conversation codecs, delivery/access/diagnostic definitions, `@matrix-org/olm`, helper scripts, and channel-specific tests). The remaining test suite has 3,548 passing cases.
8. Removed the WeCom self-built-app channel (`wecom-app`) end to end: its callback server, cgi-bin API, Host/Client entries, settings tab and RPC, delivery/access/diagnostic definitions, 51 now-unused client dictionary keys, helper scripts, and channel-specific tests. The remaining test suite has 3,502 passing cases.
9. Added the per-bot Superagent handoff: the Feishu bot stores a `superagent` request separately from `consumerMode`, `describeBot` and the bot status expose `superagent`/`superagentActive`, the General tab has a **交给 Superagent** switch through `bot.superagent.set`, `dshIm.inboundStatus` exposes `superagent`/`superagentActive` without account verification for polling, `consumeInbound` accepts `superagent: true` and refuses `superagent-mode-disabled` when the switch is off, and `#startRuntime` starts standalone when a persisted external-consumer mode has no consumer after a Host restart. The remaining test suite has 3,505 passing cases.
10. Extended the same-Host Feishu reads behind the Superagent handoff: `normalizeExternalText` renders `post` bodies to text with their image keys, admits `image` messages, and admits bot senders when `includeBots` is requested; `dshIm` gains `chatHistoryChecked` (time range, cursor, bots), `threadHistoryChecked`, `messageChecked` (the message's AppLink), `chatMembersChecked`, and `resourceChecked` (bounded image/file bytes); `replyChecked` accepts a Feishu `uuid` so a retried reply deduplicates, and production verification caches the tenant token for its lifetime while still reading the bot identity on every call, so an account change is still detected immediately. The remaining test suite has 3,515 passing cases.
11. Closed the gaps against the removed lark-cli Feishu Source: `replyChecked` takes `format` (`markdown` renders a schema 2.0 card) and `replyInThread` (defaults to whether the source message is already in a topic), and its sender check now accepts bot/app authors through `senderMatches`; `messageChecked` takes `messageId` to read another message of the same conversation (a reply's parent, a topic's root) and still validates the retained route through `anchorMatches`, so a fabricated route reaches no message; a single-message read also returns the message's topic ids; `MESSAGE_TYPES` admits `file`, `audio`, `media`, `sticker`, `interactive`, `share_chat`, `share_user` and `merge_forward`, rendered as one-line placeholders that keep the resource key; history evidence carries `appLink`. The Host forwards the new reply options and `messageId` through the controller, and `replyChecked` on the delivery service validates them. The remaining test suite has 3,521 passing cases.

12. Extended the checked Feishu reads for the Superagent handoff: `resourceChecked` takes `messageId`, so an image or file of another message of the same conversation (a reply's parent, a Thread's root, a neighbour) downloads through the retained route, which is still validated with `anchorMatches` before the target is read; and `chatListChecked` lists the chats the bot has joined (`im.v1.chat.list`, every page) as chat_id plus name, so a caller can name every joined chat without a delivery target for each. `anchorMatches` also tolerates the topic Feishu opens on a message after it was sent, but only when that message became the topic's own root; `replyChecked` uses the same check, so a Ticket whose source message gained a topic keeps its replies and reads. The Host controller, the delivery adapter and the delivery service forward both, and the remaining test suite has 3,524 passing cases.

13. Kept the consumer's inbound stream clean and readable: `readChatHistory` and `readThreadHistory` skip a message whose `app_id` or bot `open_id` is the bot's own (a bot's own replies were being re-received as new work) without counting it in `omitted`, while `historyChecked` and `messageChecked` still show them as conversation context; `merge_forward` bodies are expanded from a second `im.v1.message.get` into `<sender>: <text>` lines on the live path (in `feishu-runtime`) and in history and context reads, keeping the `[合并转发]` placeholder when the sub-messages cannot be read; a body over 16 000 characters is cut to 16 000 characters and ends with `[内容过长，已截断]` instead of being refused as `invalid-inbound`; and rich-text paragraphs are no longer dropped for exceeding the 512-character `identifier` bound. The inbound merged-forward read is bounded to two seconds so the event acknowledgement is not delayed, an item that names another `upper_message_id` is never taken as this forward's content, a cut never splits a surrogate pair, and an aborted read aborts instead of falling back to the placeholder. The remaining test suite has 3,529 passing cases.

14. Root-level third-party disclosure: the repository's generated [`THIRD_PARTY_NOTICES.md`](../../../../THIRD_PARTY_NOTICES.md) lists this plugin's runtime dependencies, and the generator treats the QQ connector's declared `UNLICENSED` terms as an identity-scoped owner authorization that stays visibly non-permissive. The barrier this plugin introduced was `MIT-0` on `nodemailer` (now on the permissive allowlist) and that one identity; the decision and its limits are in [`logs/00120`](../../../../logs/00120-第三方依赖声明恢复与QQ渠道许可决策.md) and [the notices note](../../../../.agents/notes/implemented/process/2026-07-30-generated-third-party-notices.md). The upstream copy's own `THIRD_PARTY_NOTICES.md` is unchanged.


## Syncing upstream

1. Copy the upstream `src/`, `plugin-src/`, `test/`, `docs/`, `assets/`, `locale/`, README files, `LICENSE`, and `THIRD_PARTY_NOTICES.md` over this directory.
2. Re-apply every local modification above and update the baseline commit in this file.
3. Keep `lib/` out of the synced copy; run `pnpm --filter @xmanrui/dsh-im run build`.
4. Run `node --test test/*.test.mjs test/channels/*/*.test.mjs` inside this directory, then the repository build and a Web boot check.

## Boundaries

- The panel follows the DSH interface language and keeps credentials Host-side; the browser receives only status, QR/manifest data, and masked identifiers.
- Channel connections, permissions, and platform ToS still belong to the upstream implementation. The unofficial WeChat and iMessage paths keep their upstream caveats.
