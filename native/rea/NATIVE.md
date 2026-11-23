# Native rea (vendored source)

This directory holds the upstream [rea](https://github.com/morluto/rea) source at
release `6.10.1` (commit `65ee0e80abc12b582a8d3158878bf444e2397e8a`,
2026-11-23), MIT licensed. `pnpm run build` installs the vendored npm
dependencies and runs the upstream build, so a checkout needs no published
tarball, no global install and no `rea setup`; the MCP plugin
[`packages/mcp/rea`](../../packages/mcp/rea/README.md) spawns
`scripts/rea.mjs mcp` from this tree, and the Web `standard` preset mounts it
beside [`packages/skill/skill-rea`](../../packages/skill/skill-rea/README.md).

## Baseline

| | |
|---|---|
| Upstream | `https://github.com/morluto/rea` |
| Release | `6.10.1` |
| Commit | `65ee0e80abc12b582a8d3158878bf444e2397e8a` |
| License | MIT (`LICENSE`), third-party notices in `THIRD_PARTY_NOTICES.md` and `licenses/` |
| MCP entry | `node scripts/rea.mjs mcp` (stdio; upstream also serves `--http`) |
| Tool surface | 263 tools in 19 families; default `hot-cold` advertises 5 and loads the rest through `discover_tools` |

## What is vendored

Kept as upstream wrote them: `src/`, `scripts/`, `bridge/`, `native/`, `third_party/`,
`licenses/`, `patches/`, `package.json`, `package-lock.json`, `turbo.json`,
`tsconfig.json`, `tsconfig.build.json`, `.nvmrc`, `.npmrc`, `.agents/skills/reverse-engineer-anything/`,
the release-please configuration and the upstream documentation files.

Removed because only the external distribution, upstream development or the
website needs them: `tests/`, `docs/`, `website/`, `examples/`, the top-level
`assets/` (plugin artwork), `.github/`, `.husky/`, the `.claude-plugin/`,
`.codex-plugin/` and `.cursor-plugin/` manifests, `install.sh`, `server.json`,
`mcp.json`, `CHANGELOG.md`, `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, the
vitest/stryker/knip configuration, and `src/**/*.{test,fixture}.ts`. The upstream
build pipeline does not read any of them, and the MCP server reads none of them
at runtime. `src/generatedPackageMetadata.ts` is kept: the build's metadata
check compares it with `package.json`, `.nvmrc` and the skill manifest.

## Local modifications

The vendored source is unchanged except `package.json`, which adds
`"private": true` because the repository's workspace-constraints gate requires
every `native/` manifest with a `package.json` to be non-publishable. The
built-in adaptation lives in `packages/skill/skill-rea` (renamed skill, on-demand
`load_tools` steps, and this checkout's launcher path) and in
[`scripts/build-rea.ts`](../../scripts/build-rea.ts).

## Build

```sh
pnpm run build:rea   # or pnpm run build
```

1. `npm ci` in this directory, once per `package-lock.json` digest (recorded in
   `node_modules/.dsh-rea-install`; `HUSKY=0`, npm's own `engine-strict` active).
2. `npm run build` on Node.js 24 and 25: upstream `turbo` runs `metadata:check`,
   the `compile` chain (`tsc`, catalog recorders, runtime bundle) and
   `artifacts:skills`. On Node.js 26 it runs `compile` only, because upstream's
   metadata and skill generators need `module.stripTypeScriptTypes` transform
   mode, which Node.js 26 removed; the DSH skill package ships its own skill text,
   so only the rea `search_guidance` tool loses its bundled guidance.
3. The build must produce `dist/mcp/main.js`; the script fails loudly otherwise.

Build outputs stay here and are ignored by git: `node_modules/`, `dist/`,
`skills/`, `.turbo/`, `.cache/`. The first install downloads 748 npm packages
and takes about three minutes; later builds reuse the tree and finish in about
ten seconds.

Node.js requirement: the upstream package requires
`^22.23.4 || ^24.22.1 || >=26.12.1`, and this repository requires
`^24.22.3 || >=26.13.1`, so a supported harness runtime satisfies both. On an
older runtime `npm ci` fails on the engine check and the launcher refuses to
start with an actionable message.

## Mount

- `packages/mcp/rea` mounts one `@deepseek-ai/dsh-mcp-client` stdio instance with
  `serverName: 'rea'`, `args: ['<this>/scripts/rea.mjs', 'mcp']`, catalog profile
  `hot-cold` and tool loading `on-demand`.
- `packages/skill/skill-rea` publishes the adapted
  `reverse-engineer-anything` skill as `dsh-rea`.
- `packages/bundle/web-app/presets/standard.patch.yml` adds both rows to the Web
  `standard` agent; `minimal` and the non-Web profiles are unchanged, so keyless
  recorded-session snapshots do not move.

## Checks

```sh
pnpm run build:rea
node /tmp/rea/mcp-probe.mjs   # optional: initialize + tools/list against the launcher
pnpm vitest run packages/mcp/rea packages/skill/skill-rea scripts/build-rea.spec.ts
pnpm run build:check
pnpm run doc-sync            # or test:docs for the documentation gates
```

## Upstream sync

1. Re-export the new upstream tree at a pinned release (`git archive <tag>`),
   prune the paths listed above, and replace this directory's tracked files.
2. Keep `LICENSE`, `THIRD_PARTY_NOTICES.md`, `licenses/` and `patches/` in step
   with the release, and update the baseline table in this file.
3. If the release changes `package.json`, `.nvmrc`, the authored skill or
   `src/generatedPackageMetadata.ts`, run the full upstream build once on
   Node.js 24 and commit the regenerated `src/generatedPackageMetadata.ts`; on
   Node.js 26 the metadata check is skipped.
4. Run the checks above. The mount suite fails when the initial hot-cold tool
   set changes, which is the signal to update the package README counts.
5. Copy the new `.agents/skills/reverse-engineer-anything/` into
   `packages/skill/skill-rea/assets/dsh-rea/` and re-apply the built-in
   adaptations described in that package's README.
