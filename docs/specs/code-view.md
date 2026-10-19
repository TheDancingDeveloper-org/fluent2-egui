# CodeView and the shared highlighter

Gap G15 in [`vogt-port-gap-analysis.md`](../vogt-port-gap-analysis.md), work item WI-1798.
`data::code`, behind the off-by-default `highlight` cargo feature. Zed's `crates/editor`,
`language`, `rope` and `text` (GPL-3.0) were design references only; no code was copied.

## Spike: tree-sitter or syntect (2026-10-19)

The item asked for tree-sitter, provided it works on native and wasm. A scratch crate tried
it, and the result decided the choice:

| Build | Native | `wasm32-unknown-unknown` |
|---|---|---|
| tree-sitter 0.25 + highlight + grammars | builds and highlights | core fails: `stdio.h` not found |
| tree-sitter 0.26.13 | builds | fails: tree-sitter-language asks for 0.27 |
| tree-sitter =0.27.0, tree-sitter-language =0.1.8 | builds | core builds; `tree-sitter-rust` 0.24.2 fails: `parser.h` needs `stdlib.h` |

The grammars are C with no wasm libc. Pointing them at headers through `CFLAGS` would depend on
registry paths and on each grammar's build script, and CI builds the gallery for wasm with
trunk. **Decision (operator, 2026-10-19): syntect 5.3** with `default-syntaxes`, `parsing` and
`regex-fancy`. That is pure Rust and needs no C toolchain. The bundled syntaxes cover Rust,
Python, JSON, YAML and Diff. TypeScript uses the JavaScript syntax (`ts`, `tsx`, `typescript`
and `jsx` are aliases), which covers ordinary code but not type-only syntax such as annotations.
If grammars gain a wasm build later, a tree-sitter backend could sit behind the same
`SyntaxToken` API.

## Highlighter

- `highlight(language, text) -> Vec<Vec<TokenSpan>>` is pure: one entry per line (split on
  `\n`, a trailing `\r` dropped), with byte ranges that cover the line in order. An unknown
  language, or a syntect error part-way through, gives plain runs; it never panics.
- `language` is a syntect token: a name or a file extension (`rust`, `rs`, `py`, `json`,
  `yaml`/`yml`, `diff`/`patch`, `ts`).
- Kinds come from TextMate scopes through a private syntect theme (one `ThemeItem` per kind), so
  syntect's rule that the more specific selector wins decides overlaps (a JSON key is `Key`,
  not `String`).
- `Highlighter::shared()` loads the syntax set once (a `OnceLock`).
- `syntax_color(kind, theme)` uses palette foreground-2 tokens, `neutralForeground3` for
  comments and `neutralForeground2` for punctuation. Every kind is 4.5:1 or better on
  `neutralBackground1` in light and dark (tested). High contrast paints all kinds in
  `neutralForeground1`.
- `line_job(line, spans, font, theme)` builds one line's LayoutJob, for DiffView (G16) and
  MarkdownView (G14).

## CodeView

- Read-only. Editing is out of scope.
- Tabs expand to `TAB_WIDTH` (4) spaces before highlighting. Columns are counted in chars on a
  monospace grid.
- A line-number gutter (`neutralForeground3`, `neutralStroke3` rule), on by default.
- `ScrollArea::both().show_viewport`: the content is sized for every line, but only the rows in
  the viewport are laid out and painted. A text change is re-highlighted once, and the result
  is cached in `CodeState` by a hash of language and text. A change also drops the selection.
- Drag selects (in chars), Ctrl+A selects all, Copy copies the selection (and returns it in
  `CodeResponse::copied`), Escape clears it. A click focuses the view. Tab is not captured.
- AccessKit: `MultilineTextInput`, read-only, labelled (`label`, default "Code"), with the
  text as its value.
