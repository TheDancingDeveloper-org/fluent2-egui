//! The parsed document: HTML tokens from `html5gum` folded into a small, lossy block tree.
//!
//! Nothing is fetched, executed or trusted. Only the elements and the few inline styles below
//! are kept; everything else contributes its text, or nothing (`script`, `style`, `head`,
//! forms, embedded content, and anything hidden with `display: none` and the other preheader
//! tricks).

use std::fmt::Write as _;

use html5gum::{HtmlString, Token, Tokenizer};

use super::css::{self, Declared, Rgb, Size, TextAlign};

/// Containers (quotes, lists, tables, cells) nested deeper than this are flattened into their
/// parent, so hostile input cannot make painting recurse without bound.
pub const MAX_DEPTH: usize = 12;

/// Body text size, in points, that relative sizes (`em`, `%`, `<small>`) start from.
pub const BASE_SIZE: f32 = 14.0;

/// A parsed HTML document. Parse once, keep it with the message.
///
/// ```
/// use fluent2_egui::data::html::HtmlDocument;
/// let doc = HtmlDocument::parse("<h1>Hi</h1><p>Some <b>bold</b> text.<script>x()</script></p>");
/// assert_eq!(doc.outline(), "h1 Hi\np Some *bold* text.\n");
/// assert_eq!(doc.to_plain_text(), "Hi\n\nSome bold text.");
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HtmlDocument {
    /// Top-level blocks, in source order.
    pub blocks: Vec<Block>,
}

/// One block of an [`HtmlDocument`].
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// A run of text: a paragraph, a heading, or text in a `div` or table cell.
    Text {
        /// Paragraph, heading or plain flow.
        kind: TextKind,
        /// Horizontal alignment.
        align: TextAlign,
        /// The text and inline images.
        inlines: Vec<Inline>,
    },
    /// `<ul>` (`start` is `None`) or `<ol>`.
    List {
        /// First number of an ordered list.
        start: Option<u64>,
        /// Each item's blocks.
        items: Vec<Vec<Block>>,
    },
    /// `<blockquote>`.
    Quote(Vec<Block>),
    /// `<pre>`: whitespace kept, monospace.
    Pre(Vec<Inline>),
    /// `<hr>`.
    Rule,
    /// A `<table>`, painted as a layout grid.
    Table(Table),
}

/// What kind of text a [`Block::Text`] is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextKind {
    /// `<p>`: spaced from its neighbours.
    Paragraph,
    /// `<h1>` to `<h6>`.
    Heading(u8),
    /// Text in a `div`, a table cell or the body: no spacing of its own.
    Flow,
}

/// A table, used by most HTML mail for layout.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    /// Rows of cells.
    pub rows: Vec<Vec<Cell>>,
    /// The `width` attribute or style.
    pub width: Option<Length>,
    /// The `bgcolor` attribute or background style.
    pub background: Option<Rgb>,
    /// Where the table sits when narrower than its container (`align`, or the enclosing
    /// cell's alignment).
    pub align: TextAlign,
}

/// One table cell.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// The cell's content.
    pub blocks: Vec<Block>,
    /// Columns spanned, at least 1.
    pub colspan: usize,
    /// The `width` attribute or style.
    pub width: Option<Length>,
    /// The `bgcolor` attribute or background style.
    pub background: Option<Rgb>,
    /// `<th>`.
    pub header: bool,
}

/// A width from the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    /// Points (CSS pixels).
    Points(f32),
    /// Percent of the available width.
    Percent(f32),
}

/// A piece of a text block.
#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    /// Text with one style. A line break is a `\n`.
    Text(Run),
    /// An image.
    Image(ImageRef),
}

/// A run of text with one style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    /// The text, with whitespace collapsed outside `<pre>`.
    pub text: String,
    /// Character style.
    pub style: RunStyle,
    /// The raw `href` when the run is inside a link. The view's link policy decides what it is.
    pub link: Option<String>,
}

/// Character style of a [`Run`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RunStyle {
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underlined.
    pub underline: bool,
    /// Struck through.
    pub strike: bool,
    /// Monospace (`code`, `kbd`, `tt`, `samp`).
    pub code: bool,
    /// Text colour from the document.
    pub color: Option<Rgb>,
    /// Highlight colour from the document.
    pub background: Option<Rgb>,
    /// Size in points; `None` is body size.
    pub size: Option<f32>,
}

/// An `<img>`. The view's image policy decides whether and how it is shown.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImageRef {
    /// The `src` attribute, as written (`https:`, `cid:`, `data:`…).
    pub src: String,
    /// The `alt` attribute.
    pub alt: String,
    /// The `width` attribute or style, in points.
    pub width: Option<f32>,
    /// The `height` attribute or style, in points.
    pub height: Option<f32>,
    /// The `href` of the link around the image.
    pub link: Option<String>,
}

impl HtmlDocument {
    /// Parse `html`. Never fails: anything unreadable is dropped.
    #[must_use]
    pub fn parse(html: &str) -> Self {
        let mut emitter = html5gum::DefaultEmitter::default();
        // Script, style and title content arrive as text, which the hidden frames then drop.
        emitter.naively_switch_states(true);
        let mut builder = Builder::default();
        for token in Tokenizer::new_with_emitter(html, emitter).flatten() {
            match token {
                Token::StartTag(tag) => {
                    let name = text(&tag.name);
                    let attrs = Attrs(
                        tag.attributes
                            .iter()
                            .map(|(k, v)| (text(k), text(&v.value)))
                            .collect(),
                    );
                    builder.start(&name, &attrs, tag.self_closing);
                }
                Token::EndTag(tag) => builder.end(&text(&tag.name)),
                Token::String(s) => builder.text(&text(&s.value)),
                _ => {}
            }
        }
        Self {
            blocks: builder.finish(),
        }
    }

    /// The document as plain text: blocks separated by blank lines (flow text by single line
    /// breaks), list items bulleted, table cells separated by tabs, images as `[alt]`, and a
    /// link's address after its text when it differs. The text fallback, and what "copy all"
    /// copies.
    #[must_use]
    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        plain_blocks(&self.blocks, &mut out, "");
        // No-break spaces keep words together on screen; copied text wants plain spaces.
        out.trim().replace('\u{a0}', " ")
    }

    /// Text runs in reading order, one string per selectable block (paragraphs, headings,
    /// flow, pre, list items, quotes and table cells). Find-in-message and cross-block copy
    /// walk this, not the painted widgets.
    #[must_use]
    pub fn text_blocks(&self) -> Vec<String> {
        let mut out = Vec::new();
        collect_text(&self.blocks, &mut out);
        out
    }

    /// A compact text form of the structure, for tests: one line per block (`p`, `h2`, `div`,
    /// `pre`, `hr`, `ul`/`ol` items as `-`, `quote`, table rows as `|`), runs inline, bold as
    /// `*x*`, links as `[x](href)`, images as `![alt](src)`.
    #[must_use]
    pub fn outline(&self) -> String {
        let mut out = String::new();
        outline_blocks(&self.blocks, &mut out, 0);
        out
    }

    /// Every image the document refers to, in order, including images in links and tables.
    #[must_use]
    pub fn images(&self) -> Vec<&ImageRef> {
        let mut out = Vec::new();
        collect_images(&self.blocks, &mut out);
        out
    }
}

fn text(s: &HtmlString) -> String {
    String::from_utf8_lossy(s).into_owned()
}

struct Attrs(Vec<(String, String)>);

impl Attrs {
    fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Elements whose content is never shown.
const SKIP: [&str; 17] = [
    "script", "style", "head", "title", "template", "noscript", "iframe", "object", "embed", "svg", "math",
    "select", "textarea", "canvas", "audio", "video", "xml",
];

/// Elements with no end tag.
const VOID: [&str; 15] = [
    "br", "img", "hr", "meta", "link", "input", "col", "area", "base", "wbr", "source", "track", "param",
    "keygen", "embed",
];

/// Elements that end the current text run and start a new one.
const BLOCKS: [&str; 22] = [
    "p",
    "div",
    "section",
    "article",
    "header",
    "footer",
    "main",
    "aside",
    "nav",
    "center",
    "address",
    "figure",
    "figcaption",
    "form",
    "fieldset",
    "dl",
    "dt",
    "dd",
    "details",
    "summary",
    "body",
    "html",
];

#[derive(Clone, Copy, PartialEq)]
enum ContainerKind {
    Root,
    Quote,
    List(Option<u64>),
    Item,
    Table,
    Cell,
}

struct Container {
    kind: ContainerKind,
    blocks: Vec<Block>,
    inlines: Vec<Inline>,
    /// Kind and alignment of the text block being gathered, fixed by its first inline.
    pending: Option<(FlowKind, TextAlign)>,
    /// List items, for list containers.
    items: Vec<Vec<Block>>,
    /// Table state, for table containers.
    table: Table,
    /// Cell metadata, for cell containers.
    cell: Option<Cell>,
}

impl Container {
    fn new(kind: ContainerKind) -> Self {
        Self {
            kind,
            blocks: Vec::new(),
            inlines: Vec::new(),
            pending: None,
            items: Vec::new(),
            table: Table::default(),
            cell: None,
        }
    }

    /// Turn the gathered inlines into a text block.
    fn flush(&mut self) {
        if let Some(Inline::Text(run)) = self.inlines.last_mut() {
            let trimmed = run.text.trim_end_matches(' ').len();
            run.text.truncate(trimmed);
        }
        let inlines = std::mem::take(&mut self.inlines);
        let (kind, align) = self
            .pending
            .take()
            .unwrap_or((FlowKind::Text(TextKind::Flow), TextAlign::Left));
        let empty = inlines.iter().all(|inline| match inline {
            Inline::Text(run) => run.text.trim().is_empty(),
            Inline::Image(_) => false,
        });
        if empty {
            return;
        }
        match kind {
            FlowKind::Pre => self.blocks.push(Block::Pre(inlines)),
            FlowKind::Text(kind) => self.blocks.push(Block::Text { kind, align, inlines }),
        }
    }
}

/// The kind of text being gathered: a public [`TextKind`], or `<pre>` (which becomes
/// [`Block::Pre`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FlowKind {
    Text(TextKind),
    Pre,
}

#[derive(Clone)]
struct Frame {
    tag: String,
    style: RunStyle,
    link: Option<String>,
    hidden: bool,
    align: Option<TextAlign>,
    kind: Option<FlowKind>,
    /// A block element: its start and end break the text.
    block: bool,
    container: bool,
}

#[derive(Default)]
struct Builder {
    frames: Vec<Frame>,
    containers: Vec<Container>,
}

impl Builder {
    fn top(&mut self) -> &mut Container {
        if self.containers.is_empty() {
            self.containers.push(Container::new(ContainerKind::Root));
        }
        let last = self.containers.len() - 1;
        &mut self.containers[last]
    }

    fn current(&self) -> Frame {
        self.frames.last().cloned().unwrap_or(Frame {
            tag: String::new(),
            style: RunStyle::default(),
            link: None,
            hidden: false,
            align: None,
            kind: None,
            block: false,
            container: false,
        })
    }

    fn depth(&self) -> usize {
        self.containers.len()
    }

    fn start(&mut self, name: &str, attrs: &Attrs, self_closing: bool) {
        let parent = self.current();
        // Implied ends: a new <p>, <li>, <td> or <tr> closes the previous one.
        match name {
            "p" if self.open("p") && !self.open_above("p", &["div", "td", "th", "li", "blockquote"]) => {
                self.end("p")
            }
            "li" if self.open("li") && !self.open_above("li", &["ul", "ol"]) => self.end("li"),
            "td" | "th" => {
                for tag in ["td", "th"] {
                    if self.open(tag) && !self.open_above(tag, &["table"]) {
                        self.end(tag);
                    }
                }
            }
            "tr" if self.open("tr") && !self.open_above("tr", &["table"]) => self.end("tr"),
            _ => {}
        }

        let declared = attrs.get("style").map(css::parse_style).unwrap_or_default();
        let hidden =
            parent.hidden || declared.hidden || SKIP.contains(&name) || attrs.get("hidden").is_some();

        if !hidden {
            match name {
                "br" => self.push_text("\n", &parent, true),
                "hr" => {
                    self.top().flush();
                    self.top().blocks.push(Block::Rule);
                }
                "img" => self.image(attrs, &declared, &parent),
                _ => {}
            }
        }
        if VOID.contains(&name) || (self_closing && !BLOCKS.contains(&name)) {
            return;
        }

        let mut style = parent.style;
        apply_tag_style(name, attrs, &mut style);
        apply_declared(&declared, &mut style);
        let link = if name == "a" {
            attrs
                .get("href")
                .map(str::trim)
                .filter(|href| !href.is_empty())
                .map(str::to_owned)
                .or(parent.link.clone())
        } else {
            parent.link.clone()
        };
        let align = declared
            .align
            .or_else(|| attrs.get("align").and_then(css::parse_align))
            .or_else(|| (name == "center").then_some(TextAlign::Center));

        let block_kind = match name {
            "p" => Some(FlowKind::Text(TextKind::Paragraph)),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some(FlowKind::Text(TextKind::Heading(
                name.as_bytes()
                    .get(1)
                    .map_or(1, |digit| digit.saturating_sub(b'0')),
            ))),
            "pre" => Some(FlowKind::Pre),
            _ => None,
        };
        let is_block = block_kind.is_some()
            || BLOCKS.contains(&name)
            || matches!(
                name,
                "blockquote"
                    | "ul"
                    | "ol"
                    | "li"
                    | "table"
                    | "tr"
                    | "td"
                    | "th"
                    | "tbody"
                    | "thead"
                    | "tfoot"
            );
        if is_block && !hidden {
            self.top().flush();
        }

        let mut container = false;
        if !hidden && self.depth() <= MAX_DEPTH {
            let kind = match name {
                "blockquote" => Some(ContainerKind::Quote),
                "ul" => Some(ContainerKind::List(None)),
                "ol" => Some(ContainerKind::List(Some(
                    attrs
                        .get("start")
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(1),
                ))),
                "li" => Some(ContainerKind::Item),
                "table" => Some(ContainerKind::Table),
                "td" | "th" if self.top().kind == ContainerKind::Table => Some(ContainerKind::Cell),
                _ => None,
            };
            if let Some(kind) = kind {
                let mut next = Container::new(kind);
                if kind == ContainerKind::Table {
                    next.table.width = length(attrs, &declared);
                    next.table.background = declared
                        .background
                        .or_else(|| attrs.get("bgcolor").and_then(css::parse_color));
                    next.table.align = attrs
                        .get("align")
                        .and_then(css::parse_align)
                        .or(parent.align)
                        .unwrap_or_default();
                }
                if kind == ContainerKind::Cell {
                    next.cell = Some(Cell {
                        blocks: Vec::new(),
                        colspan: attrs
                            .get("colspan")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                            .unwrap_or(1)
                            .clamp(1, 64),
                        width: length(attrs, &declared),
                        background: declared
                            .background
                            .or_else(|| attrs.get("bgcolor").and_then(css::parse_color)),
                        header: name == "th",
                    });
                }
                self.containers.push(next);
                container = true;
            }
            if name == "tr" && self.top().kind == ContainerKind::Table {
                self.top().table.rows.push(Vec::new());
            }
        }

        if name == "th" {
            style.bold = true;
        }
        // A background on an inline element (a link styled as a button, a highlighted span)
        // colours its text; on a block it belongs to the cell or table, if any.
        if !is_block && let Some(background) = declared.background {
            style.background = Some(background);
        }
        self.frames.push(Frame {
            tag: name.to_owned(),
            style,
            link,
            hidden,
            align: align.or(parent.align),
            kind: block_kind.or(if is_block { None } else { parent.kind }),
            block: is_block,
            container,
        });
    }

    /// Whether `tag` is open.
    fn open(&self, tag: &str) -> bool {
        self.frames.iter().any(|f| f.tag == tag)
    }

    /// Whether one of `barriers` was opened after the innermost `tag` (so `tag` is not the
    /// element an implied end would close).
    fn open_above(&self, tag: &str, barriers: &[&str]) -> bool {
        for frame in self.frames.iter().rev() {
            if frame.tag == tag {
                return false;
            }
            if barriers.contains(&frame.tag.as_str()) {
                return true;
            }
        }
        false
    }

    fn end(&mut self, name: &str) {
        let Some(index) = self.frames.iter().rposition(|f| f.tag == name) else {
            // `</p>` with no open `<p>` is an empty paragraph break; others are ignored.
            if name == "p" {
                self.top().flush();
            }
            return;
        };
        while self.frames.len() > index {
            let Some(frame) = self.frames.pop() else {
                break;
            };
            self.close(&frame);
        }
    }

    fn close(&mut self, frame: &Frame) {
        if frame.block && !frame.hidden {
            self.top().flush();
        }
        if !frame.container || self.containers.len() < 2 {
            return;
        }
        let Some(mut done) = self.containers.pop() else {
            return;
        };
        done.flush();
        let parent = self.top();
        match done.kind {
            ContainerKind::Quote => {
                if !done.blocks.is_empty() {
                    parent.blocks.push(Block::Quote(done.blocks));
                }
            }
            ContainerKind::List(start) => {
                let mut items = done.items;
                if !done.blocks.is_empty() {
                    items.push(done.blocks);
                }
                if !items.is_empty() {
                    parent.blocks.push(Block::List { start, items });
                }
            }
            ContainerKind::Item => {
                if matches!(parent.kind, ContainerKind::List(_)) {
                    parent.items.push(done.blocks);
                } else {
                    parent.blocks.extend(done.blocks);
                }
            }
            ContainerKind::Table => {
                let mut table = done.table;
                table.rows.retain(|row| !row.is_empty());
                // Text written straight into the table (outside cells) still shows, before it.
                parent.blocks.extend(done.blocks);
                if let Some(blocks) = flatten_layout_table(&table) {
                    parent.blocks.extend(blocks);
                } else if !table.rows.is_empty() {
                    parent.blocks.push(Block::Table(table));
                }
            }
            ContainerKind::Cell => {
                let Some(mut cell) = done.cell else {
                    return;
                };
                cell.blocks = done.blocks;
                if parent.table.rows.is_empty() {
                    parent.table.rows.push(Vec::new());
                }
                if let Some(row) = parent.table.rows.last_mut() {
                    row.push(cell);
                }
            }
            ContainerKind::Root => {}
        }
    }

    fn text(&mut self, raw: &str) {
        let frame = self.current();
        if frame.hidden {
            return;
        }
        let pre = frame.kind == Some(FlowKind::Pre);
        self.push_text(raw, &frame, pre);
    }

    fn push_text(&mut self, raw: &str, frame: &Frame, keep_space: bool) {
        let kind = frame.kind.unwrap_or(FlowKind::Text(TextKind::Flow));
        let align = frame.align.unwrap_or_default();
        let container = self.top();
        let text = if keep_space {
            raw.to_owned()
        } else {
            let mut out = String::with_capacity(raw.len());
            let mut space = container.inlines.is_empty()
                || matches!(container.inlines.last(), Some(Inline::Text(run)) if run.text.ends_with([' ', '\n']));
            for ch in raw.chars() {
                // HTML collapses only ASCII whitespace: a no-break space stays.
                if ch.is_ascii_whitespace() {
                    if !space {
                        out.push(' ');
                        space = true;
                    }
                } else {
                    out.push(ch);
                    space = false;
                }
            }
            out
        };
        if text.is_empty() {
            return;
        }
        if container.inlines.is_empty() {
            container.pending = Some((kind, align));
        }
        if let Some(Inline::Text(last)) = container.inlines.last_mut()
            && last.style == frame.style
            && last.link == frame.link
        {
            last.text.push_str(&text);
            return;
        }
        container.inlines.push(Inline::Text(Run {
            text,
            style: frame.style,
            link: frame.link.clone(),
        }));
    }

    fn image(&mut self, attrs: &Attrs, declared: &Declared, frame: &Frame) {
        let src = attrs.get("src").unwrap_or("").trim().to_owned();
        let dimension = |attr: &str| {
            attrs
                .get(attr)
                .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
                .filter(|v| v.is_finite() && *v >= 0.0)
        };
        let width = dimension("width");
        let height = dimension("height");
        // Tracking pixels.
        if src.is_empty() || width.is_some_and(|w| w <= 2.0) || height.is_some_and(|h| h <= 2.0) {
            return;
        }
        let _ = declared;
        let container = self.top();
        if container.inlines.is_empty() {
            container.pending = Some((
                frame.kind.unwrap_or(FlowKind::Text(TextKind::Flow)),
                frame.align.unwrap_or_default(),
            ));
        }
        container.inlines.push(Inline::Image(ImageRef {
            src,
            alt: attrs.get("alt").unwrap_or("").trim().to_owned(),
            width,
            height,
            link: frame.link.clone(),
        }));
    }

    fn finish(mut self) -> Vec<Block> {
        while let Some(frame) = self.frames.pop() {
            self.close(&frame);
        }
        while self.containers.len() > 1 {
            let frame = Frame {
                tag: String::new(),
                style: RunStyle::default(),
                link: None,
                hidden: false,
                align: None,
                kind: None,
                block: true,
                container: true,
            };
            self.close(&frame);
        }
        let root = self.top();
        root.flush();
        std::mem::take(&mut root.blocks)
    }
}

fn length(attrs: &Attrs, _declared: &Declared) -> Option<Length> {
    let value = attrs.get("width")?.trim();
    if let Some(pct) = value.strip_suffix('%') {
        pct.trim()
            .parse::<f32>()
            .ok()
            .filter(|p| p.is_finite() && *p > 0.0)
            .map(Length::Percent)
    } else {
        value
            .trim_end_matches("px")
            .parse::<f32>()
            .ok()
            .filter(|p| p.is_finite() && *p > 0.0)
            .map(Length::Points)
    }
}

/// A table with one cell per row and no colours is only stacking its content: return the
/// content, so it reflows like any other text.
fn flatten_layout_table(table: &Table) -> Option<Vec<Block>> {
    if table.background.is_some()
        || table
            .rows
            .iter()
            .any(|row| row.len() > 1 || row.iter().any(|cell| cell.background.is_some() || cell.header))
    {
        return None;
    }
    Some(
        table
            .rows
            .iter()
            .flat_map(|row| row.iter().flat_map(|cell| cell.blocks.iter().cloned()))
            .collect(),
    )
}

fn apply_tag_style(name: &str, attrs: &Attrs, style: &mut RunStyle) {
    match name {
        "b" | "strong" => style.bold = true,
        "i" | "em" | "cite" | "var" | "dfn" => style.italic = true,
        "u" | "ins" => style.underline = true,
        "s" | "strike" | "del" => style.strike = true,
        "code" | "tt" | "kbd" | "samp" | "pre" => style.code = true,
        "small" | "sub" | "sup" => scale(style, 0.85),
        "big" => scale(style, 1.2),
        "mark" => style.background = Some(Rgb(255, 241, 118)),
        "a" => style.underline = true,
        "font" => {
            if let Some(color) = attrs.get("color").and_then(css::parse_color) {
                style.color = Some(color);
            }
            if let Some(size) = attrs.get("size").and_then(css::font_tag_size) {
                apply_size(style, size);
            }
        }
        _ => {}
    }
}

fn apply_declared(declared: &Declared, style: &mut RunStyle) {
    if let Some(color) = declared.color {
        style.color = Some(color);
    }
    if let Some(bold) = declared.bold {
        style.bold = bold;
    }
    if let Some(italic) = declared.italic {
        style.italic = italic;
    }
    if let Some(underline) = declared.underline {
        style.underline = underline;
    }
    if let Some(strike) = declared.strike {
        style.strike = strike;
    }
    if let Some(size) = declared.size {
        apply_size(style, size);
    }
}

fn scale(style: &mut RunStyle, factor: f32) {
    style.size = Some(clamp_size(style.size.unwrap_or(BASE_SIZE) * factor));
}

fn apply_size(style: &mut RunStyle, size: Size) {
    match size {
        Size::Points(points) => style.size = Some(clamp_size(points)),
        Size::Factor(factor) => scale(style, factor),
    }
}

/// Text sizes from mail range from hidden 1 px preheaders to 60 px banners; keep them readable.
fn clamp_size(points: f32) -> f32 {
    points.clamp(8.0, 40.0)
}

fn plain_blocks(blocks: &[Block], out: &mut String, indent: &str) {
    for block in blocks {
        match block {
            Block::Text { kind, inlines, .. } => {
                let _ = write!(
                    out,
                    "{indent}{}",
                    plain_inlines(inlines).replace('\n', &format!("\n{indent}"))
                );
                out.push('\n');
                if *kind != TextKind::Flow {
                    out.push('\n');
                }
            }
            Block::Pre(inlines) => {
                out.push_str(&plain_inlines(inlines));
                out.push_str("\n\n");
            }
            Block::List { start, items } => {
                for (i, item) in items.iter().enumerate() {
                    let marker = match start {
                        Some(n) => format!("{}. ", n.saturating_add(i as u64)),
                        None => "• ".to_owned(),
                    };
                    let mut inner = String::new();
                    plain_blocks(item, &mut inner, "");
                    let _ = writeln!(
                        out,
                        "{indent}{marker}{}",
                        inner.trim().replace('\n', &format!("\n{indent}  "))
                    );
                }
                out.push('\n');
            }
            Block::Quote(inner) => {
                let mut quoted = String::new();
                plain_blocks(inner, &mut quoted, "");
                for line in quoted.trim().lines() {
                    let _ = writeln!(out, "{indent}> {line}");
                }
                out.push('\n');
            }
            Block::Rule => out.push_str("---\n\n"),
            Block::Table(table) => {
                for row in &table.rows {
                    let cells: Vec<String> = row
                        .iter()
                        .map(|cell| {
                            let mut inner = String::new();
                            plain_blocks(&cell.blocks, &mut inner, indent);
                            inner.trim().to_owned()
                        })
                        .filter(|text| !text.is_empty())
                        .collect();
                    // A data row of short cells reads as one tab-separated line; layout rows
                    // (one cell, or cells holding several lines) read as their blocks.
                    if cells.len() > 1 && cells.iter().all(|cell| !cell.contains('\n')) {
                        let _ = writeln!(out, "{indent}{}", cells.join("\t"));
                    } else {
                        for cell in cells {
                            let _ = writeln!(out, "{cell}\n");
                        }
                    }
                }
                out.push('\n');
            }
        }
    }
}

fn plain_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < inlines.len() {
        match &inlines[i] {
            Inline::Text(run) => {
                // Gather the whole link, which may span several runs.
                if let Some(href) = &run.link {
                    let mut label = String::new();
                    while let Some(Inline::Text(next)) = inlines.get(i) {
                        if next.link.as_ref() != Some(href) {
                            break;
                        }
                        label.push_str(&next.text);
                        i += 1;
                    }
                    out.push_str(&label);
                    let shown = label.trim().trim_end_matches('/');
                    let target = href.trim_start_matches("mailto:").trim_end_matches('/');
                    let lower = href.to_ascii_lowercase();
                    let shareable = ["http://", "https://", "mailto:"]
                        .iter()
                        .any(|p| lower.starts_with(p));
                    if shareable && !shown.ends_with(target) && !target.ends_with(shown) {
                        let _ = write!(out, " <{href}>");
                    }
                    continue;
                }
                out.push_str(&run.text);
            }
            Inline::Image(image) => {
                if !image.alt.is_empty() {
                    let _ = write!(out, "[{}]", image.alt);
                }
            }
        }
        i += 1;
    }
    out
}

/// Selectable text of each text-bearing block, in reading order. Empty blocks are skipped.
fn collect_text(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Text { inlines, .. } | Block::Pre(inlines) => {
                let text = inline_plain(inlines);
                if !text.trim().is_empty() {
                    out.push(text);
                }
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_text(item, out);
                }
            }
            Block::Quote(inner) => collect_text(inner, out),
            Block::Rule => {}
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in row {
                        collect_text(&cell.blocks, out);
                    }
                }
            }
        }
    }
}

fn inline_plain(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        if let Inline::Text(run) = inline {
            text.push_str(&run.text);
        }
    }
    text
}

fn outline_blocks(blocks: &[Block], out: &mut String, depth: usize) {
    let pad = "  ".repeat(depth);
    for block in blocks {
        match block {
            Block::Text { kind, align, inlines } => {
                let tag = match kind {
                    TextKind::Paragraph => "p".to_owned(),
                    TextKind::Heading(level) => format!("h{level}"),
                    TextKind::Flow => "div".to_owned(),
                };
                let align = match align {
                    TextAlign::Left => "",
                    TextAlign::Center => "^",
                    TextAlign::Right => ">",
                };
                let _ = writeln!(out, "{pad}{tag}{align} {}", outline_inlines(inlines));
            }
            Block::Pre(inlines) => {
                let _ = writeln!(out, "{pad}pre {}", outline_inlines(inlines).replace('\n', "\\n"));
            }
            Block::Rule => {
                let _ = writeln!(out, "{pad}hr");
            }
            Block::Quote(inner) => {
                let _ = writeln!(out, "{pad}quote");
                outline_blocks(inner, out, depth + 1);
            }
            Block::List { start, items } => {
                let _ = writeln!(out, "{pad}{}", if start.is_some() { "ol" } else { "ul" });
                for item in items {
                    let _ = writeln!(out, "{pad}-");
                    outline_blocks(item, out, depth + 1);
                }
            }
            Block::Table(table) => {
                let _ = writeln!(out, "{pad}table");
                for row in &table.rows {
                    let _ = writeln!(out, "{pad}|");
                    for cell in row {
                        let bg = cell
                            .background
                            .map(|Rgb(r, g, b)| format!(" bg#{r:02x}{g:02x}{b:02x}"))
                            .unwrap_or_default();
                        let _ = writeln!(
                            out,
                            "{pad} cell{}{bg}",
                            if cell.colspan > 1 {
                                format!(" span{}", cell.colspan)
                            } else {
                                String::new()
                            }
                        );
                        outline_blocks(&cell.blocks, out, depth + 2);
                    }
                }
            }
        }
    }
}

fn outline_inlines(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(run) => {
                let mut text = run.text.clone();
                if run.style.bold {
                    text = format!("*{text}*");
                }
                if run.style.italic {
                    text = format!("_{text}_");
                }
                if let Some(href) = &run.link {
                    text = format!("[{text}]({href})");
                }
                out.push_str(&text);
            }
            Inline::Image(image) => {
                let _ = write!(out, "![{}]({})", image.alt, image.src);
            }
        }
    }
    out
}

fn collect_images<'a>(blocks: &'a [Block], out: &mut Vec<&'a ImageRef>) {
    for block in blocks {
        match block {
            Block::Text { inlines, .. } | Block::Pre(inlines) => {
                out.extend(inlines.iter().filter_map(|inline| match inline {
                    Inline::Image(image) => Some(image),
                    Inline::Text(_) => None,
                }));
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_images(item, out);
                }
            }
            Block::Quote(inner) => collect_images(inner, out),
            Block::Table(table) => {
                for cell in table.rows.iter().flatten() {
                    collect_images(&cell.blocks, out);
                }
            }
            Block::Rule => {}
        }
    }
}
