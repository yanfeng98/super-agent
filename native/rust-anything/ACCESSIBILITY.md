# Accessibility

rust-anything is a command-line tool, so its accessibility is mostly about its output and its docs.

## What we aim for

- **Plain text output.** Results are plain lines of text that read in order. Status is spelled
  out (`pass`, `fail`, `warn`) rather than shown only by color, so it works with screen readers
  and in terminals without color.
- **Machine-readable output.** `analyze`, `diff` and `gate` take `--json`, and the MCP
  server returns the same data. A coding agent or your own tooling can read the results without
  parsing text.
- **Docs that work without images.** Every image and animation in the README has alt text that
  says what it shows, and the numbers in images also appear as text or tables.

## Known limitations

- The README's animated GIFs have no pause control. The same content is in the text and tables
  around them.
- Tables in the README and reports can be long; they are standard Markdown tables, which most
  screen readers can step through.

## Reporting a barrier

If something in rust-anything or its docs gets in your way, please open an
[issue](https://github.com/erkamyaman/rust-anything/issues/new/choose) and describe what you were
trying to do and what you use (screen reader, terminal, OS). Accessibility issues are treated as
bugs.
