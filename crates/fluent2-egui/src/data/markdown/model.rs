//! The parsed document: CommonMark events from `pulldown-cmark` folded into a small block
//! tree that the view paints and tests compare as text ([`Document::outline`]).

use std::fmt::Write as _;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Containers nested deeper than this are flattened into their parent, so hostile input such
/// as ten thousand `>` cannot make painting recurse without bound.
pub const MAX_DEPTH: usize = 16;

/// A parsed Markdown document. Parse once and keep it beside the source text.
///
/// ```
/// use fluent2_egui::data::markdown::Document;
/// let doc = Document::parse("# Title\n\nSome *text*.");
/// assert_eq!(doc.outline(), "h1 Title\np Some *text*.\n");
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    /// Top-level blocks, in source order.
    pub blocks: Vec<Block>,
}

/// One block of a [`Document`].
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// `#` to `######`. `level` is 1 to 6.
    Heading {
        /// 1 to 6.
        level: u8,
        /// Heading text.
        text: Vec<Inline>,
    },
    /// A paragraph, or the text of a tight list item.
    Paragraph(Vec<Inline>),
    /// A bullet (`start` is `None`) or ordered list.
    List {
        /// First number of an ordered list.
        start: Option<u64>,
        /// The items.
        items: Vec<ListItem>,
    },
    /// A `>` block quote.
    Quote(Vec<Block>),
    /// A fenced or indented code block.
    Code {
        /// The info string's first word, if any (`rust` in ```` ```rust ````).
        lang: Option<String>,
        /// The code, without the final newline.
        text: String,
    },
    /// A GFM table.
    Table {
        /// Column alignment from the delimiter row.
        align: Vec<Align>,
        /// Header cells.
        head: Vec<Vec<Inline>>,
        /// Body rows of cells.
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// A thematic break (`---`).
    Rule,
}

/// One item of a [`Block::List`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListItem {
    /// `Some(checked)` for a GFM task item (`- [x]`).
    pub task: Option<bool>,
    /// The item's content.
    pub blocks: Vec<Block>,
}

/// Column alignment of a [`Block::Table`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Align {
    /// No colon in the delimiter row.
    None,
    /// `:--`.
    Left,
    /// `:-:`.
    Center,
    /// `--:`.
    Right,
}

/// A run of text with one style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inline {
    /// The text. A hard line break is a `\n`.
    pub text: String,
    /// Emphasis, strong, strikethrough or code.
    pub style: InlineStyle,
    /// The raw link destination when the run is inside a link. Whether and how it is followed
    /// is decided by the view's link policy, never here.
    pub link: Option<String>,
}

/// Character style of an [`Inline`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InlineStyle {
    /// `**strong**`.
    pub strong: bool,
    /// `*emphasis*`.
    pub emphasis: bool,
    /// `~~strikethrough~~`.
    pub strike: bool,
    /// `` `code` ``.
    pub code: bool,
}

enum Frame {
    Quote(Vec<Block>),
    List {
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Item(ListItem),
}

#[derive(Default)]
struct Table {
    align: Vec<Align>,
    head: Vec<Vec<Inline>>,
    rows: Vec<Vec<Vec<Inline>>>,
    row: Vec<Vec<Inline>>,
}

#[derive(Default)]
struct Builder {
    root: Vec<Block>,
    frames: Vec<Frame>,
    /// Container starts not pushed because of [`MAX_DEPTH`]; their ends are ignored.
    flattened: Vec<bool>,
    inlines: Vec<Inline>,
    strong: u32,
    emphasis: u32,
    strike: u32,
    links: Vec<String>,
    heading: Option<u8>,
    code: Option<(Option<String>, String)>,
    table: Option<Table>,
}

impl Builder {
    fn blocks(&mut self) -> &mut Vec<Block> {
        match self.frames.last_mut() {
            Some(Frame::Quote(blocks)) => blocks,
            Some(Frame::Item(item)) => &mut item.blocks,
            // Inline content directly in a list (never produced by pulldown-cmark) and the
            // document itself both land at the root.
            Some(Frame::List { .. }) | None => &mut self.root,
        }
    }

    fn push_block(&mut self, block: Block) {
        self.flush();
        if let Some(Frame::List { items, .. }) = self.frames.last_mut() {
            items.push(ListItem {
                task: None,
                blocks: vec![block],
            });
        } else {
            self.blocks().push(block);
        }
    }

    /// Turn pending inline runs into a paragraph (tight list items have no paragraph tags).
    fn flush(&mut self) {
        if self.inlines.is_empty() || self.table.is_some() {
            return;
        }
        let text = std::mem::take(&mut self.inlines);
        self.blocks().push(Block::Paragraph(text));
    }

    fn style(&self) -> InlineStyle {
        InlineStyle {
            strong: self.strong > 0,
            emphasis: self.emphasis > 0,
            strike: self.strike > 0,
            code: false,
        }
    }

    fn text(&mut self, text: &str, code: bool) {
        if let Some((_, buf)) = &mut self.code {
            buf.push_str(text);
            return;
        }
        let style = InlineStyle { code, ..self.style() };
        let link = self.links.last().cloned();
        if let Some(last) = self.inlines.last_mut()
            && last.style == style
            && last.link == link
        {
            last.text.push_str(text);
            return;
        }
        self.inlines.push(Inline {
            text: text.to_owned(),
            style,
            link,
        });
    }

    fn open(&mut self, frame: Frame) {
        let deep = self.frames.len() >= MAX_DEPTH;
        self.flattened.push(deep);
        if !deep {
            self.flush();
            self.frames.push(frame);
        }
    }

    fn close(&mut self) -> Option<Frame> {
        if self.flattened.pop().unwrap_or(true) {
            return None;
        }
        self.flush();
        self.frames.pop()
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.flush(),
            Tag::Heading { level, .. } => {
                self.flush();
                self.heading = Some(level as u8);
            }
            Tag::BlockQuote(_) => self.open(Frame::Quote(Vec::new())),
            Tag::CodeBlock(kind) => {
                self.flush();
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().map(str::to_owned),
                    CodeBlockKind::Indented => None,
                };
                self.code = Some((lang, String::new()));
            }
            Tag::List(start) => self.open(Frame::List {
                start,
                items: Vec::new(),
            }),
            Tag::Item => self.open(Frame::Item(ListItem::default())),
            Tag::Table(align) => {
                self.flush();
                self.table = Some(Table {
                    align: align
                        .iter()
                        .map(|a| match a {
                            Alignment::None => Align::None,
                            Alignment::Left => Align::Left,
                            Alignment::Center => Align::Center,
                            Alignment::Right => Align::Right,
                        })
                        .collect(),
                    ..Table::default()
                });
            }
            Tag::Emphasis => self.emphasis += 1,
            Tag::Strong => self.strong += 1,
            Tag::Strikethrough => self.strike += 1,
            Tag::Link { dest_url, .. } => self.links.push(dest_url.into_string()),
            // Images are never fetched: the alt text is shown as plain text.
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush(),
            TagEnd::Heading(_) => {
                let text = std::mem::take(&mut self.inlines);
                let level = self.heading.take().unwrap_or(1);
                self.push_block(Block::Heading { level, text });
            }
            TagEnd::BlockQuote(_) => {
                if let Some(Frame::Quote(blocks)) = self.close() {
                    self.push_block(Block::Quote(blocks));
                }
            }
            TagEnd::CodeBlock => {
                if let Some((lang, mut text)) = self.code.take() {
                    if text.ends_with('\n') {
                        text.pop();
                    }
                    self.push_block(Block::Code { lang, text });
                }
            }
            TagEnd::List(_) => {
                if let Some(Frame::List { start, items }) = self.close() {
                    self.push_block(Block::List { start, items });
                }
            }
            TagEnd::Item => {
                if let Some(Frame::Item(item)) = self.close() {
                    if let Some(Frame::List { items, .. }) = self.frames.last_mut() {
                        items.push(item);
                    } else {
                        self.blocks().extend(item.blocks);
                    }
                }
            }
            TagEnd::TableHead => {
                if let Some(t) = &mut self.table {
                    t.head = std::mem::take(&mut t.row);
                }
            }
            TagEnd::TableRow => {
                if let Some(t) = &mut self.table {
                    let row = std::mem::take(&mut t.row);
                    t.rows.push(row);
                }
            }
            TagEnd::TableCell => {
                let cell = std::mem::take(&mut self.inlines);
                if let Some(t) = &mut self.table {
                    t.row.push(cell);
                }
            }
            TagEnd::Table => {
                if let Some(t) = self.table.take() {
                    self.push_block(Block::Table {
                        align: t.align,
                        head: t.head,
                        rows: t.rows,
                    });
                }
            }
            TagEnd::Emphasis => self.emphasis = self.emphasis.saturating_sub(1),
            TagEnd::Strong => self.strong = self.strong.saturating_sub(1),
            TagEnd::Strikethrough => self.strike = self.strike.saturating_sub(1),
            TagEnd::Link => {
                self.links.pop();
            }
            _ => {}
        }
    }
}

impl Document {
    /// Parse CommonMark with the GFM table, strikethrough and task-list extensions.
    ///
    /// Raw HTML is kept as literal text and images show their alt text; nothing is fetched or
    /// interpreted.
    pub fn parse(source: &str) -> Self {
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
        let mut b = Builder::default();
        for event in Parser::new_ext(source, options) {
            match event {
                Event::Start(tag) => b.start(tag),
                Event::End(tag) => b.end(tag),
                Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) => b.text(&t, false),
                Event::Code(t) => b.text(&t, true),
                Event::SoftBreak => b.text(" ", false),
                Event::HardBreak => b.text("\n", false),
                Event::Rule => b.push_block(Block::Rule),
                Event::TaskListMarker(checked) => {
                    if let Some(Frame::Item(item)) = b.frames.last_mut() {
                        item.task = Some(checked);
                    }
                }
                _ => {}
            }
        }
        // Close anything left open so no content is lost.
        while !b.flattened.is_empty() {
            match b.close() {
                Some(Frame::Quote(blocks)) => b.push_block(Block::Quote(blocks)),
                Some(Frame::List { start, items }) => b.push_block(Block::List { start, items }),
                Some(Frame::Item(item)) => b.blocks().extend(item.blocks),
                None => {}
            }
        }
        b.flush();
        Self { blocks: b.root }
    }

    /// A plain-text outline of the block tree, one line per block, used by snapshot tests.
    ///
    /// Inline runs are written back in Markdown form (`**strong**`, `[text](href)`), so a
    /// change in parsing shows up as a readable diff.
    #[must_use]
    pub fn outline(&self) -> String {
        let mut out = String::new();
        outline_blocks(&mut out, &self.blocks, 0);
        out
    }
}

fn outline_blocks(out: &mut String, blocks: &[Block], depth: usize) {
    let pad = "  ".repeat(depth);
    for block in blocks {
        match block {
            Block::Heading { level, text } => {
                let _ = writeln!(out, "{pad}h{level} {}", inline_text(text));
            }
            Block::Paragraph(text) => {
                let _ = writeln!(out, "{pad}p {}", inline_text(text));
            }
            Block::List { start, items } => {
                match start {
                    Some(n) => {
                        let _ = writeln!(out, "{pad}ol start={n}");
                    }
                    None => {
                        let _ = writeln!(out, "{pad}ul");
                    }
                }
                for item in items {
                    let mark = match item.task {
                        Some(true) => " [x]",
                        Some(false) => " [ ]",
                        None => "",
                    };
                    let _ = writeln!(out, "{pad}  li{mark}");
                    outline_blocks(out, &item.blocks, depth + 2);
                }
            }
            Block::Quote(inner) => {
                let _ = writeln!(out, "{pad}quote");
                outline_blocks(out, inner, depth + 1);
            }
            Block::Code { lang, text } => {
                let _ = writeln!(out, "{pad}code {}", lang.as_deref().unwrap_or("-"));
                for line in text.lines() {
                    let _ = writeln!(out, "{pad}  | {line}");
                }
            }
            Block::Table { align, head, rows } => {
                let align: Vec<&str> = align
                    .iter()
                    .map(|a| match a {
                        Align::None => "-",
                        Align::Left => "l",
                        Align::Center => "c",
                        Align::Right => "r",
                    })
                    .collect();
                let _ = writeln!(out, "{pad}table {}", align.join(""));
                let row = |cells: &[Vec<Inline>]| -> String {
                    cells
                        .iter()
                        .map(|c| inline_text(c))
                        .collect::<Vec<_>>()
                        .join(" | ")
                };
                let _ = writeln!(out, "{pad}  th {}", row(head));
                for r in rows {
                    let _ = writeln!(out, "{pad}  td {}", row(r));
                }
            }
            Block::Rule => {
                let _ = writeln!(out, "{pad}hr");
            }
        }
    }
}

fn inline_text(runs: &[Inline]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while let Some(run) = runs.get(i) {
        if let Some(href) = &run.link {
            out.push('[');
            while let Some(r) = runs.get(i).filter(|r| r.link.as_ref() == Some(href)) {
                out.push_str(&styled(r));
                i += 1;
            }
            let _ = write!(out, "]({href})");
        } else {
            out.push_str(&styled(run));
            i += 1;
        }
    }
    out.replace('\n', "\\n")
}

fn styled(run: &Inline) -> String {
    let s = &run.style;
    let mut text = run.text.clone();
    if s.code {
        text = format!("`{text}`");
    }
    if s.strike {
        text = format!("~~{text}~~");
    }
    if s.emphasis {
        text = format!("*{text}*");
    }
    if s.strong {
        text = format!("**{text}**");
    }
    text
}
