# HtmlView (WI-1939)

`data::html`, behind the `html` cargo feature. It renders a sanitised subset of HTML, built for
HTML mail (email-rs L1). The design reference is fluent-gpui's
`RichDocument::from_html_fragment_lossy`, a lossy block model; no code was copied.

## Model

`HtmlDocument::parse` tokenises with `html5gum` 0.8.4 (MIT; a WHATWG tokenizer with no unsafe
and, with default features off, no dependencies). It never fails. Tokens fold into blocks:

| HTML | model |
|---|---|
| `p`, `h1`–`h6` | `Block::Text` with `TextKind::Paragraph` or `Heading(n)`; spaced from neighbours |
| `div`, `td` text, body text, `center`, `section`… | `Block::Text` with `TextKind::Flow`; no spacing of its own |
| `ul`, `ol` (`start`), `li` | `Block::List` |
| `blockquote` | `Block::Quote` |
| `pre` | `Block::Pre` (whitespace kept, monospace) |
| `hr` | `Block::Rule` |
| `table`, `tr`, `td`, `th` (`colspan`, `width`, `bgcolor`) | `Block::Table`. A table with one cell per row and no colours is flattened into its content |
| `b`/`strong`, `i`/`em`, `u`, `s`/`del`, `code`/`kbd`, `small`/`big`, `mark`, `font` | `RunStyle` |
| `a href` | `Run::link` (raw; the view's policy decides) |
| `img` | `Inline::Image` (`src`, `alt`, `width`, `height`); images of 2 pt or less (tracking pixels) are dropped |
| `br` | `\n` in the run |

Inline `style` is read for `color`, `background(-color)`, `font-weight`, `font-style`,
`text-decoration`, `font-size` (px, pt, em, %, keywords; clamped to 8–40 pt) and `text-align`.
`display:none`, `visibility:hidden`, `mso-hide:all`, `font-size:0`, `opacity:0` and
`max-height:0` with `overflow:hidden` hide an element (the preheader tricks).

Dropped with their content: `script`, `style`, `head`, `title`, `template`, `noscript`,
`iframe`, `object`, `embed`, `svg`, `math`, `select`, `textarea`, `canvas`, `audio`, `video`, and
elements with the `hidden` attribute. Comments (including Outlook conditional comments),
doctype, unknown tags (`o:p`) and every attribute not listed above are ignored. Whitespace
collapses outside `pre`; a no-break space stays. `<p>`, `<li>`, `<td>` and `<tr>` close an open
element of the same kind. Containers nest at most `MAX_DEPTH` (12) deep; deeper content flows
into the innermost container.

Nothing is executed or fetched, so the allow-list is the sanitiser.

## View

`HtmlView::new(id, &doc).link_policy(..).image_policy(..).show(ui) -> HtmlResponse`.

- **Text:** each text block is one selectable label built from a `LayoutJob` (Fluent type
  ramp: body1, headings as in MarkdownView). Headings are AccessKit headings.
- **Links:** each link the policy allows (default: `http`, `https`, `mailto`) gets a focusable
  click area over its glyphs, one per row. It is an AccessKit Link named by its text, with a
  focus ring and the URL as a tooltip. Activation is returned in `clicked`. Blocked links are
  plain text.
- **Images:** the policy returns `Show(source)`, `Embedded` (decode the image's own
  `data:image/…;base64` and pass it to egui's loaders as `bytes://`), `Blocked` (a placeholder
  with an icon and the alt text, an AccessKit Image "Blocked image: alt", counted in
  `blocked_images`) or `Hidden`. The default shows `data:` images and blocks the rest. A linked
  image is a link.
- **Tables:** laid out as rows of fixed-width cells. Widths come from single-column cells'
  `width` (points or percent), the rest are shared, and everything is scaled to the available
  width. Cell and table backgrounds fill the whole row height.
- **Colour:** light keeps the author's colours unless text is under 2:1 on its background.
  Dark mirrors HSL lightness of light backgrounds, and of text under 4.5:1, falling back to
  `neutral_foreground_1`. High contrast drops all document colours.
- **Text fallback:** `HtmlDocument::to_plain_text` gives blocks with blank lines, bullets,
  `>` quotes, tab-separated data rows, `[alt]` images and `<url>` after link text when it
  differs. No-break spaces become spaces. `outline()` is a structure dump for tests.

## Not in v1

CSS classes and `<style>` rules, floats, positioning, margins and padding (except
paragraph spacing), `rowspan`, borders, background images, web fonts, `font-family`, `dir=rtl`,
forms. Selection across blocks, find-in-message, link hover status and the
AccessKit document tree are in the next section.

## Reading the message

- **Selection.** Dragging across blocks selects one range of the document's
  selectable text ([`HtmlDocument::text_blocks`]). The range is highlighted, and
  Copy (while a block of the view is focused) places it on the clipboard with a
  newline between blocks. Copy-all stays [`HtmlDocument::to_plain_text`].
- **Find.** [`HtmlView::find`] takes a case-insensitive query and a match index.
  Matches stay inside one block. Every match is highlighted; the current one is
  stronger and scrolled into view. [`HtmlResponse::find`] is the "n of m" status.
  [`find_text`] is the same search without painting.
- **Link hover.** A link the policy allows sets [`HtmlResponse::hovered`] (href,
  target, text) while the pointer is over it, and still shows the URL as a tooltip.
  A blocked link sets neither.
- **AccessKit.** Each text block is a `Paragraph`, or a `Heading` at its level. Each
  link is a `Link` and each image an `Image`, as before. They stay in egui's own tree:
  parenting a node egui already placed makes AccessKit reject the update as a
  duplicate child.

Nothing is executed. Remote images still load only when the app's image policy
says so.

## Fixtures

`crates/fluent2-egui/tests/fixtures/html/` holds synthetic samples: a newsletter, an Outlook
reply, plain `div` mail and hostile input. email-rs has no HTML fixtures yet
(`fixtures/fluentgui-phase0` holds only SQLite seeds), so real-world samples are a follow-up.
