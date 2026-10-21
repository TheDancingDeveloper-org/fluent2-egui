//! MarkdownView: CommonMark and GFM tables painted with the Fluent type ramp and colour tokens
//! (behind the `markdown` cargo feature).
//!
//! Parsing is `pulldown-cmark` folded into a small block tree ([`Document`]); the app parses
//! once and keeps the document. The view never opens anything. Every link destination goes
//! through the app's [`LinkPolicy`], which classifies it as a web link, a file link or blocked;
//! blocked links paint as plain text, and a click on the others comes back in
//! [`MarkdownResponse::clicked`] for the app to act on. Code blocks go through an optional
//! [`Highlighter`] (the hook a syntax highlighter plugs into) and fall back to plain monospace.
//!
//! Design notes: `egui_commonmark` (MIT OR Apache-2.0) was compared and not used. It renders
//! with egui's own visuals and opens links itself, while this view needs theme tokens and an
//! app-owned link decision. Zed's `markdown` crate (GPL) was not consulted for code.

mod model;

pub use model::{Align, Block, Document, Inline, InlineStyle, ListItem, MAX_DEPTH};

use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Frame, Id, Label, Layout, Margin,
    RichText, Sense, Stroke, Ui, Vec2, WidgetInfo,
};

use crate::foundation::focus;
use crate::foundation::text::{SEMIBOLD_FAMILY, TypeRamp, font_id};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

/// What a link destination is, as decided by the app's [`LinkPolicy`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkTarget {
    /// A link the app may open in a browser.
    Web(String),
    /// A link to a file or path the app resolves itself (for example a workspace file).
    File(String),
    /// Not a link: painted as plain text and never reported.
    Blocked,
}

/// Classifies a raw link destination. Called for every link while painting.
pub type LinkPolicy<'a> = &'a dyn Fn(&str) -> LinkTarget;

/// Turns a code block into a highlighted [`LayoutJob`], or `None` to fall back to plain text.
/// Arguments: language (the fence's info word), code, theme, and the monospace font.
pub type Highlighter<'a> = &'a dyn Fn(Option<&str>, &str, &Theme, &FontId) -> Option<LayoutJob>;

/// The default policy: `http:`, `https:` and `mailto:` are web links, everything else
/// (relative paths, `file:`, `javascript:`, anchors) is blocked.
///
/// ```
/// use fluent2_egui::data::markdown::{LinkTarget, default_link_policy};
/// assert_eq!(default_link_policy("https://example.com"), LinkTarget::Web("https://example.com".into()));
/// assert_eq!(default_link_policy("javascript:alert(1)"), LinkTarget::Blocked);
/// ```
#[must_use]
pub fn default_link_policy(href: &str) -> LinkTarget {
    let lower = href.trim().to_ascii_lowercase();
    if ["http://", "https://", "mailto:"]
        .iter()
        .any(|p| lower.starts_with(p))
    {
        LinkTarget::Web(href.trim().to_owned())
    } else {
        LinkTarget::Blocked
    }
}

/// A link the user activated (click, or Enter or Space while focused).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkClick {
    /// The destination as written in the source.
    pub href: String,
    /// What the policy made of it (never [`LinkTarget::Blocked`]).
    pub target: LinkTarget,
}

/// What [`MarkdownView::show`] returns.
pub struct MarkdownResponse {
    /// The area the document took.
    pub response: egui::Response,
    /// The link activated this frame, if any.
    pub clicked: Option<LinkClick>,
}

/// Paints a [`Document`].
///
/// ```
/// use fluent2_egui::data::markdown::{Document, LinkTarget, MarkdownView};
/// # fn demo(ui: &mut egui::Ui, doc: &Document) {
/// let policy = |href: &str| {
///     if href.starts_with("https://") {
///         LinkTarget::Web(href.to_owned())
///     } else {
///         LinkTarget::Blocked
///     }
/// };
/// let out = MarkdownView::new("notes", doc).link_policy(&policy).show(ui);
/// if let Some(click) = out.clicked {
///     // the app decides what to do with click.target
///     let _ = click;
/// }
/// # }
/// ```
pub struct MarkdownView<'a> {
    id: Id,
    doc: &'a Document,
    policy: LinkPolicy<'a>,
    highlighter: Option<Highlighter<'a>>,
}

impl<'a> MarkdownView<'a> {
    /// A view of `doc`. `id_salt` keeps scroll state of code blocks and table grids apart.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, doc: &'a Document) -> Self {
        Self {
            id: Id::new(id_salt),
            doc,
            policy: &default_link_policy,
            highlighter: None,
        }
    }

    /// Decide what each link destination is. Defaults to [`default_link_policy`].
    #[must_use]
    pub fn link_policy(mut self, policy: LinkPolicy<'a>) -> Self {
        self.policy = policy;
        self
    }

    /// Highlight code blocks. Without one, code is plain monospace.
    #[must_use]
    pub fn highlighter(mut self, highlighter: Highlighter<'a>) -> Self {
        self.highlighter = Some(highlighter);
        self
    }

    /// Paint the document into `ui`.
    pub fn show(self, ui: &mut Ui) -> MarkdownResponse {
        let theme = Theme::get(ui.ctx());
        let semibold = ui.fonts(|f| {
            f.definitions()
                .families
                .contains_key(&FontFamily::Name(SEMIBOLD_FAMILY.into()))
        });
        let mut painter = Painter {
            theme: &theme,
            policy: self.policy,
            highlighter: self.highlighter,
            semibold,
            clicked: None,
            counter: 0,
        };
        let inner = ui.push_id(self.id, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                painter.blocks(ui, &self.doc.blocks, 0);
            })
        });
        MarkdownResponse {
            response: inner.response,
            clicked: painter.clicked,
        }
    }
}

struct Painter<'a, 't> {
    theme: &'t Theme,
    policy: LinkPolicy<'a>,
    highlighter: Option<Highlighter<'a>>,
    semibold: bool,
    clicked: Option<LinkClick>,
    /// Running index for per-block ids (code scroll areas, table grids).
    counter: usize,
}

fn heading_step(level: u8) -> TypeRamp {
    match level {
        1 => TypeRamp::Title2,
        2 => TypeRamp::Title3,
        3 => TypeRamp::Subtitle1,
        4 => TypeRamp::Subtitle2,
        5 => TypeRamp::Body1Strong,
        _ => TypeRamp::Caption1,
    }
}

impl Painter<'_, '_> {
    fn font(&self, step: TypeRamp, strong: bool) -> FontId {
        let mut font = font_id(step);
        if strong {
            font.family = FontFamily::Name(SEMIBOLD_FAMILY.into());
        }
        if !self.semibold && font.family != FontFamily::Monospace {
            font.family = FontFamily::Proportional;
        }
        font
    }

    fn mono(&self, step: TypeRamp) -> FontId {
        FontId::new(step.size(), FontFamily::Monospace)
    }

    fn blocks(&mut self, ui: &mut Ui, blocks: &[Block], depth: usize) {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                ui.add_space(spacing_vertical::M);
            }
            self.block(ui, block, depth);
        }
    }

    fn block(&mut self, ui: &mut Ui, block: &Block, depth: usize) {
        let color = &self.theme.color;
        match block {
            Block::Heading { level, text } => {
                let step = heading_step(*level);
                let plain: String = text.iter().map(|r| r.text.as_str()).collect();
                let rich = RichText::new(plain.clone())
                    .font(self.font(step, step.semibold()))
                    .color(c(color.neutral_foreground_1));
                let response = ui.add(Label::new(rich).wrap());
                let level = usize::from(*level);
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_role(Role::Heading);
                    node.set_level(level);
                    node.set_label(plain);
                });
            }
            Block::Paragraph(text) => {
                self.paragraph(ui, text, TypeRamp::Body1, c(color.neutral_foreground_1));
            }
            Block::List { start, items } => self.list(ui, *start, items, depth),
            Block::Quote(inner) => {
                let out = Frame::new()
                    .inner_margin(Margin {
                        left: spacing_horizontal::M as i8,
                        ..Margin::ZERO
                    })
                    .show(ui, |ui| self.blocks(ui, inner, depth + 1));
                let rect = out.response.rect;
                let x = rect.left() + stroke_width::THICK / 2.0;
                ui.painter().vline(
                    x,
                    rect.y_range(),
                    Stroke::new(stroke_width::THICK, c(color.neutral_stroke_1)),
                );
            }
            Block::Code { lang, text } => self.code(ui, lang.as_deref(), text),
            Block::Table { align, head, rows } => self.table(ui, align, head, rows),
            Block::Rule => {
                let width = ui.available_width().max(1.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, stroke_width::THIN), Sense::hover());
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(stroke_width::THIN, c(color.neutral_stroke_2)),
                );
            }
        }
    }

    fn paragraph(&mut self, ui: &mut Ui, runs: &[Inline], step: TypeRamp, fg: Color32) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for run in runs {
                self.run(ui, run, step, fg, true);
            }
        });
    }

    fn rich(&self, text: &str, run: &Inline, step: TypeRamp, fg: Color32) -> RichText {
        let s = run.style;
        let font = if s.code {
            self.mono(step)
        } else {
            self.font(step, s.strong || step.semibold())
        };
        let mut rich = RichText::new(text).font(font).color(fg);
        if s.code {
            rich = rich.background_color(c(self.theme.color.neutral_background_3));
        }
        if s.emphasis {
            rich = rich.italics();
        }
        if s.strike {
            rich = rich.strikethrough();
        }
        rich
    }

    fn run(&mut self, ui: &mut Ui, run: &Inline, step: TypeRamp, fg: Color32, wrap: bool) {
        let target = run.link.as_deref().map(|href| (href, (self.policy)(href)));
        let mut lines = run.text.split('\n').peekable();
        while let Some(line) = lines.next() {
            if !line.is_empty() {
                match &target {
                    Some((href, t)) if *t != LinkTarget::Blocked => {
                        self.link(ui, line, run, step, wrap, href, t.clone());
                    }
                    _ => {
                        let label = Label::new(self.rich(line, run, step, fg));
                        ui.add(if wrap { label.wrap() } else { label.extend() });
                    }
                }
            }
            if lines.peek().is_some() {
                ui.end_row();
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn link(
        &mut self,
        ui: &mut Ui,
        text: &str,
        run: &Inline,
        step: TypeRamp,
        wrap: bool,
        href: &str,
        target: LinkTarget,
    ) {
        let color = &self.theme.color;
        let rich = self
            .rich(text, run, step, c(color.brand_foreground_link))
            .underline();
        let label = Label::new(rich).sense(Sense::click());
        let response = ui.add(if wrap { label.wrap() } else { label.extend() });
        let enabled = ui.is_enabled();
        let label_text = text.to_owned();
        response.widget_info(|| WidgetInfo::labeled(Role::Link, enabled, label_text.clone()));
        if focus::focus_visible(ui, &response) {
            focus::paint_focus_ring(ui, response.rect, CornerRadius::ZERO);
        }
        let hint = match &target {
            LinkTarget::Web(url) => url.clone(),
            LinkTarget::File(path) => path.clone(),
            LinkTarget::Blocked => String::new(),
        };
        let response = response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(hint);
        if response.clicked() {
            self.clicked = Some(LinkClick {
                href: href.to_owned(),
                target,
            });
        }
    }

    fn list(&mut self, ui: &mut Ui, start: Option<u64>, items: &[ListItem], depth: usize) {
        let color = &self.theme.color;
        let fg = c(color.neutral_foreground_1);
        let body = font_id(TypeRamp::Body1);
        let row = ui.fonts_mut(|f| f.row_height(&body));
        let marker_width = spacing_horizontal::XL;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                ui.add_space(spacing_vertical::XS);
            }
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing_horizontal::XS;
                let (rect, response) = ui.allocate_exact_size(Vec2::new(marker_width, row), Sense::hover());
                let center = rect.center();
                let painter = ui.painter();
                if let Some(checked) = item.task {
                    let side = TypeRamp::Body1.size();
                    let r = egui::Rect::from_center_size(center, Vec2::splat(side));
                    let radius = CornerRadius::same(border_radius::SMALL as u8);
                    if checked {
                        painter.rect_filled(r, radius, c(color.compound_brand_background));
                        let ink = Stroke::new(stroke_width::THICK, c(color.neutral_foreground_on_brand));
                        let pts = [
                            r.left_top() + Vec2::new(side * 0.22, side * 0.52),
                            r.left_top() + Vec2::new(side * 0.42, side * 0.72),
                            r.left_top() + Vec2::new(side * 0.78, side * 0.3),
                        ];
                        painter.line_segment([pts[0], pts[1]], ink);
                        painter.line_segment([pts[1], pts[2]], ink);
                    } else {
                        painter.rect_stroke(
                            r,
                            radius,
                            Stroke::new(stroke_width::THIN, c(color.neutral_stroke_accessible)),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let label = item.blocks.first().map(block_text).unwrap_or_default();
                    response.widget_info(|| {
                        WidgetInfo::selected(Role::CheckBox, false, checked, label.clone())
                    });
                } else if let Some(n) = start {
                    let number = n.saturating_add(i as u64);
                    painter.text(
                        rect.right_center(),
                        Align2::RIGHT_CENTER,
                        format!("{number}."),
                        body.clone(),
                        fg,
                    );
                } else {
                    let radius = TypeRamp::Body1.size() * 0.18;
                    if depth.is_multiple_of(2) {
                        painter.circle_filled(center, radius, fg);
                    } else {
                        painter.circle_stroke(center, radius, Stroke::new(stroke_width::THIN, fg));
                    }
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.blocks(ui, &item.blocks, depth + 1);
                });
            });
        }
    }

    fn code(&mut self, ui: &mut Ui, lang: Option<&str>, text: &str) {
        let theme = self.theme;
        let font = self.mono(TypeRamp::Body1);
        let job = self
            .highlighter
            .and_then(|h| h(lang, text, theme, &font))
            .unwrap_or_else(|| {
                LayoutJob::simple(
                    text.to_owned(),
                    font.clone(),
                    c(theme.color.neutral_foreground_1),
                    f32::INFINITY,
                )
            });
        self.counter += 1;
        let id = ui.id().with(("code", self.counter));
        Frame::new()
            .fill(c(theme.color.neutral_background_3))
            .corner_radius(CornerRadius::same(border_radius::MEDIUM as u8))
            .inner_margin(Margin::same(spacing_horizontal::M as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::horizontal().id_salt(id).show(ui, |ui| {
                    let mut job = job;
                    job.wrap.max_width = f32::INFINITY;
                    ui.add(Label::new(job).extend());
                });
            });
    }

    fn table(&mut self, ui: &mut Ui, align: &[Align], head: &[Vec<Inline>], rows: &[Vec<Vec<Inline>>]) {
        let color = &self.theme.color;
        let fg = c(color.neutral_foreground_1);
        self.counter += 1;
        let id = ui.id().with(("table", self.counter));
        let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
        Frame::new()
            .stroke(Stroke::new(stroke_width::THIN, c(color.neutral_stroke_2)))
            .corner_radius(CornerRadius::same(border_radius::MEDIUM as u8))
            .inner_margin(Margin::same(spacing_horizontal::S as i8))
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt(id.with("scroll"))
                    .show(ui, |ui| {
                        egui::Grid::new(id)
                            .spacing(Vec2::new(spacing_horizontal::L, spacing_vertical::S))
                            .show(ui, |ui| {
                                for (r, cells) in std::iter::once(head)
                                    .chain(rows.iter().map(Vec::as_slice))
                                    .enumerate()
                                {
                                    for col in 0..columns {
                                        let layout = match align.get(col) {
                                            Some(Align::Center) => Layout::top_down(egui::Align::Center),
                                            Some(Align::Right) => Layout::top_down(egui::Align::Max),
                                            _ => Layout::top_down(egui::Align::Min),
                                        };
                                        ui.with_layout(layout, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing = Vec2::ZERO;
                                                for run in cells.get(col).map(Vec::as_slice).unwrap_or(&[]) {
                                                    let step = if r == 0 {
                                                        TypeRamp::Body1Strong
                                                    } else {
                                                        TypeRamp::Body1
                                                    };
                                                    self.run(ui, run, step, fg, false);
                                                }
                                            });
                                        });
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            });
    }
}

fn block_text(block: &Block) -> String {
    match block {
        Block::Paragraph(runs) | Block::Heading { text: runs, .. } => {
            runs.iter().map(|r| r.text.as_str()).collect()
        }
        _ => String::new(),
    }
}
