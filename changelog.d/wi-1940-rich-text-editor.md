### Added
- RichTextEditor (`controls::RichTextEditor`, `RichText`, `CharStyle`, `BlockKind`,
  `InlineFormat`, `RichTextEditorResponse`): a compose control for mail (email-rs, WI-1940).
  A toolbar toggles bold, italic and underline (also Ctrl+B, Ctrl+I, Ctrl+U), links, bulleted
  and numbered lists and quotes over egui's multiline text edit. With no selection, a toggled
  format applies to the next characters typed. `RichText::to_html` writes a small escaped
  HTML subset (`p`, `ul`/`ol`/`li`, `blockquote`, `strong`, `em`, `u`, and `a` for `http`,
  `https` and `mailto` links only); `RichText::to_plain` writes the plain-text alternative.
  Gallery page "Rich text editor".
