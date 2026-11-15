# Agent Note: Markdown tables fit the reading column

Status: implemented

English | [中文](2026-11-15-markdown-tables-fit-the-column.zh.md)

## Problem

A comparison answer — a plan audit with `#`, 问题, 位置, and 修复 columns — rendered wider than the text it answered. Two rules combined: the renderer kept every table of four or more columns at its natural width, and the chat transcript pushed those tables past the message column out to the transcript edges. Where the natural width exceeded the space the layout offered, the wrapper clipped the remainder, so the last columns stayed invisible until the reader hovered the table; where it fit, the table's right edge still stood outside the paragraph column. Seeded on the assembled app at an 1100px viewport, a four-column table ran 386px past the message column and 240px of it was clipped.

## Decision

A markdown table is never wider than the container its surface gives it. `MarkdownText`'s `.tableScroll table` asks for `width: fit-content` and caps at `max-width: 100%`, so a table that already fits keeps its natural width while a longer one clamps at the available width: cells wrap there down to their existing 100px readable floor. The chat transcript no longer widens `md-table-wide` tables — the container-query breakout (`--dsh-table-spare`, `--dsh-table-lead`, its negative margin, and its lead padding) is gone, so a table begins and ends on the message column's edges like every other block.

Horizontal scrolling remains only where wrapping cannot help: a table whose minimum column widths add up to more than the container (the corpus's fourteen-column table, or a narrow viewport) keeps the wrapper's `overflow-x` fallback, its hover-revealed bar, and the keyboard focus path. `md-table-wide` stays the static column-count hook that selects those wrapper behaviors; it no longer selects a width.

`ChatView`'s `.scroll` keeps `container-type: inline-size`. It is no longer any table's sizing reference, but transcript components without a container of their own (permission presets, job rows, subagent rows, deliverables) resolve their unnamed `@container` rules against the nearest container.

## Alternatives considered

- **Keep the breakout and only cap the width.** The wider view keeps using page space, but a four-column table still stands past the paragraph column on one side — the misalignment this change removes. The reading column is the width every other block uses.
- **Restore the previous always-natural width.** That is the defect: content past the wrapper stays invisible until hover.
- **Measure each table and offer an expand dialog.** The alternative rejected when the wide view was introduced; it adds per-table observers and interaction state for content that now reads in place.

## Consequences

Four-to-six-column tables fit the reading column with wrapped cells and no horizontal scroll at desktop widths, aligned with the paragraphs around them. Many-column tables give up the spare page width, so they wrap tighter and reach their scroll fallback sooner than before. Every `MarkdownText` surface inherits the cap — blockquotes, list items, reasoning rows, tool cards, plugin documents — and so no surface clips a table whose cells can wrap. Nothing is measured: a table's width never depends on hover, focus, or interaction state.

## Testing

[The wide-table e2e](../../../../apps/web/tests/markdown-wide-table.e2e.ts) seeds fill, many-column, long-token, and four-column prose tables and pins, in Chromium at 1680/1100/640px viewports, that the prose, fill, and long-cell tables report no residual horizontal scroll, that the many-column table still does, that every table's content starts on the fill table's left edge, and that each wrapping table grows taller as the column narrows. The [committed golden](../../../../apps/web/tests/expected/markdown-wide-table/geometry.expected.md) records those relations, never pixels. The report above comes from the assembled app with the same table seeded: 240px of clipped overflow at 1100px before the change, none after it.

## Related

- [Web markdown wide-table view](../../archived/feature/2026-08-19-web-markdown-wide-table-view.md) — the archived note whose breakout this decision removes.
