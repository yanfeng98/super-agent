---
description: "Full-window startup experience: a six-act archive sequence, then an intro clip, with one settings page and a Host-owned media route."
kind: "package-reference"
---

# @deepseek-ai/dsh-client-ui-boot-animation

English | [中文](README.zh.md)

## Summary

`@deepseek-ai/dsh-client-ui-boot-animation` plays a two-stage startup experience over the client at launch, or when you enter a new conversation: a six-act archive sequence, then a short intro clip. Its Startup intro page owns the stage order, every sequence preference, the clip library, and the moment it plays. Four clips and six recorded lines ship with the package; MP4, M4V, MOV, and WebM files in the user video directory join the library. Each conversation can replay the experience or pin its own clip, and Skip or Esc ends the whole experience. It is on by default for Web and Desktop.

## Table of Contents

- [Use this package](#use-this-package)
- [Understand the implementation](#understand-the-implementation)
- [Further Exploration](#further-exploration)
- [Model Experience](#model-experience)
- [Known Limitations and Deferred Work](#known-limitations-and-deferred-work)
- [Dev Note](#dev-note)

-----

<a id="use-this-package"></a>
## Use this package

The shipped Web composition mounts the intro as `ui-boot-animation`, so the Web GUI and Desktop both play it by default; everything a user can change lives on the Startup intro page in Settings, and its copy follows the active UI language.

### The archive sequence

The first stage is six acts of full-window authorization theatre: a black prologue with the whale mark and the DeepSeek wordmark, an overexposed wash, a warm paper plinth, five authorization lines, the welcome act, and a chromatic reveal that hands off to the clip. A reticle turns slowly behind the text, the corner counter tracks `[ 01 / 06 ]` through the acts, and a progress bar fills with them. The whole sequence follows the light or dark theme, and either theme can render as a translucent frosted plinth over the app instead of a solid surface.

Each of the five lines is a Chinese headline with an English monospace subtitle that types itself in, plus a bottom log line. Two of them wait for you: the first asks to grant access, the second to confirm the identity card that appears beside the button, and both continue on the button, Enter, or Space. The other three advance on their own. Every line is also spoken: the package ships six recorded takes, and a line waits for its take to finish before advancing, so answering quickly never cuts the voice off. When a take is missing, or `screenVoiceSource` is `'tts'`, the same line is spoken by the system voice instead. Interface cues (a tick, a two-tone confirm, a closing chime) accompany the acts; they play only while `screenSound` is on.

`screenSpeed` multiplies every fixed duration, and a line still waits for its spoken take before advancing. `screenRequireInteraction` turns the two confirmation gates off, and `screenAllowSkip` controls the skip control and Escape; skipping ends the whole experience rather than continuing with the clip. The letters, the brand block, the prologue wordmark, the closing box, the footer, the identity, and the three colors are all settings rather than constants.

### The clip stage

The clip stage fills the window and leaves only a Skip button and a status line over the video. It starts muted, because browsers block audible autoplay; clicking the picture turns the sound on and asks for full screen, which the browser grants only during that click. Skip and Esc close the intro immediately, and it also closes when the clip ends or fails to decode, with a short notice in place of the video. The status line reports the first frame: it reads "Loading video…" until the element paints one, then becomes the click-for-sound hint. A clip that produces no playable frame within 25 seconds shows a timeout notice and then closes, and a late first frame arriving during that notice still plays.

The overlay sits above the rest of the client but below dialogs, so an approval or a modal raised during playback stays visible and usable. Preview on the settings page plays through the same overlay with a preview badge and never changes the selection.

### When the intro plays

`timing` picks the automatic moment. `app-start` plays once per launch. `entry` plays when you enter a conversation that still has no turns, once per conversation per launch. In both modes the launch is the scope: a reload plays again, a second tab keeps its own record, and returning to a conversation does not replay it unless that conversation's own override asks for a replay. Turning `enabled` off stops automatic playback; previews keep working.

The launch and per-conversation records live in sessionStorage, with an in-memory fallback when the browser refuses storage.

### Clip sources

Four clips ship inside the package under `assets/`; they are listed first and need no configuration. Your own videos join the library when you copy them into `$DSH_HOME/boot-animation/videos/` — the plugin creates that directory at startup — and press Refresh on the settings page. MP4, M4V, MOV, and WebM files are listed; empty files and hidden files are skipped.

A clip is identified as `builtin:<key>` when it ships with the package and as `file:<stem>-<hash>` when it is your own file, where the hash covers the absolute path. Renaming or moving a file therefore yields a new id, and a selection that no longer exists falls back to the first listed clip instead of playing different bytes under the old name. Replacing a file's contents keeps its id but changes its version, so the browser fetches the new bytes.

### Preferences

| Field | Default | Meaning |
|---|---|---|
| `enabled` | `true` | Play the intro automatically |
| `timing` | `'app-start'` | `app-start` plays once per launch; `entry` plays on entering an empty conversation |
| `fit` | `'cover'` | `cover` fills the window and crops the edges; `contain` shows the whole frame |
| `random` | `false` | Play a different clip each time instead of `selectedClip` |
| `selectedClip` | `'builtin:brand'` | The clip id played when random playback is off |
| `sequence` | `'archive-then-video'` | Which stages run and in what order: `archive-then-video`, `video-then-archive`, `archive-only`, or `video-only` |

The sequence's own preferences are the `screen*` fields: `screenTheme`, `screenSpeed`, `screenRequireInteraction`, `screenAllowSkip`, the identity trio, the voice and cue fields, the glass fields, the three colors, and the nine text slots. The Startup intro page groups them under playback, identity, voice and sound, appearance, and on-screen text.

These are volatile Config fields: the settings page writes them into the running entry, so a change applies without editing a profile. The generated [configuration catalog](../../../docs/config-catalog.md#deepseek-aidsh-client-ui-boot-animation) is the exhaustive field reference.

### Per-conversation override

The third group on the settings page applies to the conversation that is open. Replay on every entry makes the intro play each time you enter that conversation, outranking both timing modes. The clip selector pins one clip for that conversation and defaults to the global selection. The Host stores both fields per conversation, so they hold across restarts and across browsers pointed at the same Host. When the Host has no durable storage, the group reports that it is unavailable and the rest of the page keeps working.

<a id="understand-the-implementation"></a>
## Understand the implementation

A browser page cannot read the filesystem or remember a choice across browsers, so the Host owns clip discovery, clip bytes, the "which clip now" answer, and the durable per-conversation override. The browser owns when an intro plays and what the user sees.

<details>
<summary>Implementation details — click to expand</summary>

Every route lives under `/dsh-boot-animation` and is owner-only: the registration requires authentication, the handler rejects a non-owner principal with 403, and a clip id is validated against the discovered catalog rather than used as a path. `catalog.json` lists the clips together with the asking conversation's override and states whether overrides are available; `active.json` answers the single "which clip plays now" question and carries the stage plan (`stages`) the browser will run; `audio/<name>` streams one of the six recorded takes or the two interface cues, from a closed name set, under the same range and validator policy as a clip; `POST session` merges one conversation's override. `media/<clipId>` streams the bytes, with the content type taken from the file extension, single-range `Range` support answering 206, an ETag for `If-None-Match` revalidation, and a cache policy that is `public, max-age=31536000, immutable` only when `?v=` matches this clip's current version and `no-cache` otherwise. Every failure, including 403, 404, 416, and 500, is `no-store`, so one transient error cannot stick in the browser.

Discovery is cached under a signature built from every entry's size and modification time. A directory-level signature would not notice a file overwritten in place, which would keep publishing a stale version and let `?v=` serve permanently cached old bytes. That same size and modification time is the clip's version, which is what makes the long cache lifetime safe.

One resolver owns the priority chain: the conversation override, then random playback, then `selectedClip`, then the first listed clip. Random playback remembers the previous pick in process memory and excludes it while another clip exists, so two intros in a row differ. That memory advances only when a random answer is actually requested, so a catalog refresh or a status read never consumes it.

The archive stage renders from one `Frame` object describing everything visible at that instant, driven by a single timeline function; a skip rejects every pending wait through one sentinel, so teardown is one path rather than a branch at each step. Its sound engine sits behind an injected factory, so the same engine runs against real media elements in the browser and against plain fakes in tests, and one `stop()` releases every element and cancels the synthesizer. The state that outlives a frame - the identity, the copy, the colors, the glass - arrives from the settings projection, never from the Host wire.

While either stage owns the window, the browser face sets `data-dsh-splash` on the document element. The Desktop shell reads that attribute to hide its Windows caption buttons with the chrome the sequence covers, and removes the transparency when the attribute goes away; a browser ignores it.

The sequence's art stylesheet, `src/client/archive/archive.css`, is a plain global sheet scoped under `.dsh-boot-archive-root` with `--dba-*` custom properties. It carries roughly a hundred elements and eleven keyframes; CSS Modules references would add a name table without enabling reuse, and the class names are prefixed so the sheet cannot touch the rest of the app.

Overrides live in a `@deepseek-ai/dsh-storage-domain` domain named `boot_animation`, table `sessions`, with one `{ replay, clipId }` record per conversation; a record with neither field set is deleted instead of stored empty. The plugin opens the domain through an optional `storageDomain` injection, so a Host without storage still plays intros: only the override group is unavailable.

The browser face mirrors the Host settings projection, follows the current conversation binding, and asks the pure `decidePlay` function on each relevant change. That function takes the settings, the conversation identity, its blank flag, and the two records, and returns either a play with a reason or no play with a reason. Only a play the Host answered with a clip records itself, so a failed resolve is retried on the next evaluation instead of consuming the launch. Two registrations publish the result: `shell.overlay` with id `boot-animation` renders the player, and `settings.section` with id `boot-animation` and order 40 renders the Startup intro page. Both receive bare observables through their inject `hooks` compartment; the components render and call back, and hold no lifecycle of their own.

Copy lives in `src/client/locales.ts` under the `bootAnimation` namespace, with eight languages; `locale/*.json` carries only the plugin's display metadata.

The plugin declares `settings.configure({ auto: false })` for its own entry, so the generic settings form does not claim fields that the Startup intro page already edits.

</details>

<a id="further-exploration"></a>
## Further Exploration

- [Web client architecture](../../../docs/subsystems/web-client.md) — the loading chain, object layer, and where a browser plugin sits.
- [Slots](../../../docs/subsystems/slots.md) — how the `shell.overlay` and `settings.section` contributions are declared and rendered.
- [Settings](../../../docs/subsystems/settings.md) — how volatile Config fields reach a browser form.
- [Storage domain](../../storage/storage-domain/README.md) — the durable domain tables the per-conversation overrides use.
- [Web server](../../host/webserver/README.md) — the authenticated route surface these endpoints extend.

<a id="model-experience"></a>
## Model Experience

None, as this browser package registers no model tools, prompt sections, or Session events; clip choice, playback, and preferences stay in browser and Host presentation state.

#### KV Cache effect

None. Nothing the intro stores — the selected clip, the timing, or a conversation's override — enters a model request, so no cached prefix changes.

## Known Limitations and Deferred Work

<a id="known-limitations-and-deferred-work"></a>

- **No system file picker** — the settings page cannot open an operating-system dialog, so a user copies a video into `$DSH_HOME/boot-animation/videos/` and presses Refresh. Files outside that directory are never listed.
- **Built-in clip rights** — the four shipped clips are published as-is; their branding and licensing are the publishing party's responsibility, and the package verifies neither.
- **`timing` = `entry` can miss a slow binding** — the mode needs the conversation's blank flag to settle after the binding appears, so a binding that takes very long can let that entry pass without an intro.
- **`screenSfx` = `'synth'` still plays the recorded cues** — the synthesized cue set is not implemented; the setting is stored and offered, and both values currently play the shipped cue files.
- **`screenVoiceLang` = `'both'` speaks English** — the recorded takes are English only, and the synthesized path speaks one language per line, so the "both" choice currently behaves like `'en'`.
- **Background blur stays inside the overlay** — `screenGlassBackdropBlur` blurs what is behind the overlay's own layers; the package does not add `filter` to application nodes, so an app element that repaints itself during playback is not blurred.
- **Recorded voice rights** — the six recorded lines come from the upstream startup-screen plugin, whose audio was taken from a third-party video; they are published as-is, and their rights are the publishing party's responsibility.
- **Mobile full-screen differences are untested** — full-screen requests and viewport units follow desktop browser behavior; mobile browser chrome and mobile full-screen APIs are not handled separately.

<a id="dev-note"></a>
### Dev Note

<details>
<summary>Maintenance context — click to expand</summary>

None.

</details>
