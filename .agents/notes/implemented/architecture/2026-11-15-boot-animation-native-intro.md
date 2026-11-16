# Agent Note: Startup experience as a native client feature

Status: implemented

English | [中文](2026-11-15-boot-animation-native-intro.zh.md)

## Problem

A third-party plugin (`dsh-boot-animation`) demonstrated real demand for a startup intro, but shipping it out of tree had four costs the repository should not keep paying: every launch required an install and a cache-defeating hard refresh, its preferences lived in a private JSON file, its UI copy was untranslated, and its tests asserted its own build artifacts rather than behaviour. The feature also arrived with two defects worth not re-inheriting: an unhandled media-stream error that could terminate the Host process, and a directory-level cache signature that kept publishing stale clip versions after a file was replaced in place.

A second third-party plugin, `dsh-startup-screen`, demonstrated a different startup idea: six interactive acts instead of one clip. Its costs were larger than the first plugin's - a private JSON preference file behind its own HTTP endpoints, 38 untranslated settings, an imperative 1,750-line runtime with no type or coverage story, a desktop titlebar trick built on mutating a preload probe's inline style, and recorded audio taken from a third-party video.

## Decision

Both stages live in this one package, and the default sequence plays the archive stage first and the clip stage second. One package owns them because they share one launch gate, one settings page, one launch record, and one overlay: two packages would need a mutual-exclusion rule, and the only honest rule ("whichever registered first") is invisible to the user. `sequence` selects the stages and their order; `timing`, `enabled`, and the per-conversation override still govern the experience as a whole.

The sequence's preferences are flat `screen*` Config fields beside the clip fields. One table in `src/screen.ts` carries every field's default and bounds, so the Host schema, the settings page, and the runtime normalize against the same facts; the settings page renders from a second table that binds each row to its dictionary labels.

The archive stage is described by one `Frame` object holding everything visible at an instant, driven by one timeline function. A skip rejects every pending wait through one sentinel class, so teardown is a single path instead of a branch after each step, and the sound engine sits behind an injected factory so the same code runs against real media elements and against fakes. Recorded takes ship under `assets/audio/` and the Host serves them from a closed name set through the clip responder's range and validator policy; a missing take falls back to system speech.

The intro is now a first-party package, `@deepseek-ai/dsh-client-ui-boot-animation`, mounted by a row in the `dsh-web-app` bundle. Because Desktop runs the Web profile, one row turns it on for both surfaces; headless, ACP and SDK profiles never load it. Defaults are on: play once per launch, fill the window, brand clip.

Preferences are the Host `Config` fields (`enabled`, `timing`, `fit`, `random`, `selectedClip`), declared volatile and owned by the package's own settings section, which calls `settings.configure({ auto: false })`. The browser reads and writes them through `ctx.configForms`, so they live in the profile patch, survive a port change, and need no custom store. Per-conversation state — replay on every entry, and that conversation's clip — lives in a `boot_animation` storage domain opened through an optional `ctx.inject(['storageDomain'])`, so a composition without storage still plays the intro and only loses that page.

The package replaces the plugin's two overlapping conversation concepts with one record: `{ replay, clipId }`. Asking for a clip in a conversation and asking it to replay were always two questions about one conversation, and one record answers both without the plugin's "pinned but not overridden" state.

Shipped clips are ordinary files under `assets/`, streamed by the Host route, not base64 embedded in the build output: 8.7 MB on disk instead of 12 MB in a JavaScript module, and no decode step on the install path. The media responder is written here (single-range, ETag, version-pinned `immutable`), with the stream's `error`, the response's `close` and an open-before-status ordering, because the only third-party alternative (`serve-static`) would add a dependency for 120 lines.

## Consequences

The six acts stay bilingual art rather than localized copy: a Chinese headline with an English monospace subtitle, the same in every interface language, because that pairing is the design. Interface chrome around it - skip, hints, card labels, welcome line - is dictionary copy in all eight languages.

The desktop caption contract is now explicit: the browser face sets `data-dsh-splash` on the document element while either stage owns the window, and `apps/desktop/src/preload-windows.ts` sends transparent caption colors while the attribute is present. The upstream plugin instead rewrote a preload-owned probe span's inline style, which broke silently whenever that implementation changed.

Some of the upstream behaviour is deliberately not carried over, and the README lists each gap: `screenSfx: 'synth'` still plays the recorded cues, `screenVoiceLang: 'both'` speaks English, and the background blur stays inside the overlay instead of adding `filter` to application nodes (which changes `position: fixed` containing blocks).

The browser's overlay portals to `body` at `z-index: 900`, below the shared Modal band: an approval raised during the intro still wins. Clip versions are computed from each file's size and mtime, and the scan signature covers every entry, so replacing a clip in place invalidates the published version instead of pinning a stale one forever.

The overlay, not the plugin body, reports playback: it calls `markPlaying` on the element's first `playing` event, which is what turns the status line from "Loading video…" into the click-for-sound hint. Every timer it schedules is owned by the component and cleared on unmount, so a stall notice that outlives the overlay cannot stop the next clip, and a first frame that arrives during the notice still plays.

`timing: 'entry'` depends on the blank flag settling after the session binding arrives; the package subscribes to the session face for that reason, but a Host that never settles the flag skips the intro for that entry rather than replaying it.

## Alternatives considered

- **Port the third-party package as-is** (plain JavaScript, private JSON preferences, untranslated copy, tests over its own artifacts): rejected because it fails this repository's type, locale and testing gates and would carry both defects forward.
- **Embed the clips in the build output as base64**, as the plugin did: rejected for 33% size inflation and a decode step on the install path.
- **Keep per-conversation state in browser storage**: rejected because product preferences are host-backed here; a port or profile change would lose them.
- **Add `serve-static` for the media route**: rejected; it still needs version-pinned caching and a per-clip URL, so the dependency would carry little of the work.
- **Keep "pinned conversation" and "conversation clip" as separate concepts**: rejected; one record answers both questions.
- **Ship the sequence as a second package**: rejected; the two stages share a launch gate, a settings page, and a record, so a second package would need an invisible tie-break rule.
- **Port the upstream imperative runtime verbatim**: rejected; it fails the type, locale, and per-file coverage gates, and its 1,750-line closure has no seam for tests.
- **Keep the upstream titlebar probe trick**: rejected; it mutates a node the desktop preload owns, through an implementation detail that is not a contract.
