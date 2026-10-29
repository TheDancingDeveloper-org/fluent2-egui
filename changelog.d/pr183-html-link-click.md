### Fixed

- `HtmlView` link clicks reach the link again. The view's drag-selection widget covers the whole document; registered after a link, egui drops the click underneath it. The drag is registered first, using the previous frame's block rectangles, and each text block only senses hover.
