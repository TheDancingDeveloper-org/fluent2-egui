//! HtmlView: a sanitised subset of HTML painted with the Fluent type ramp and colour tokens
//! (behind the `html` cargo feature). Built for HTML mail.
//!
//! [`HtmlDocument::parse`] tokenises with `html5gum` (MIT, a WHATWG tokenizer, no unsafe, no
//! dependencies) and folds the tokens into a small, lossy block model. Nothing is executed or
//! fetched, so there is no separate sanitiser: what the model doesn't keep never reaches the
//! screen. Kept are block flow, headings, paragraphs, lists, blockquotes, `pre`, rules, tables
//! as layout grids, links, images, and a few inline styles (colour, highlight, weight, italic,
//! underline, strike-through, size, alignment). There is no CSS engine. `script`, `style`,
//! `head`, forms, embedded content, tracking pixels and text hidden with `display: none` (and
//! the other preheader tricks) are dropped.
//!
//! The app decides what leaves the view:
//! - every link goes through a [`LinkPolicy`]; a click comes back in [`HtmlResponse::clicked`];
//! - every image goes through an [`ImagePolicy`]: show it, show a "blocked" placeholder, or
//!   hide it. The default shows inline `data:` images (decoded here and handed to egui's image
//!   loaders, so the app installs `egui_extras::install_image_loaders`) and blocks everything
//!   else, so remote images and tracking never load without the app's consent.
//!
//! Colours from the document are kept in the light theme (unless unreadable), remapped in the
//! dark theme (light backgrounds darken, dark text lightens, by inverting HSL lightness until
//! text reaches 4.5:1), and dropped in high contrast, which uses only the theme's colours.
//! Each text block is one selectable label; [`HtmlDocument::to_plain_text`] is the text
//! fallback and what an app's "copy all" copies.
//!
//! Design reference: fluent-gpui's `RichDocument::from_html_fragment_lossy` (document viewer in
//! email-rs's GPUI client); no code was copied.

mod css;
mod data_uri;
mod model;

#[rustfmt::skip]
pub use css::{
    Rgb,
    TextAlign,
    parse_color,
};
#[rustfmt::skip]
pub use model::{
    BASE_SIZE,
    Block,
    Cell,
    HtmlDocument,
    ImageRef,
    Inline,
    Length,
    MAX_DEPTH,
    Run,
    RunStyle,
    Table,
    TextKind,
};

use std::sync::Arc;

use egui::accesskit::Role;
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CornerRadius, CursorIcon, FontFamily, FontId, Frame, Galley, Id, Label, Layout, Margin, Rect,
    Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType,
};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::foundation::focus;
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::{SEMIBOLD_FAMILY, TypeRamp, font_id};
use crate::theme::{Theme, ThemeKind, c};

/// What a link is, as decided by the app's [`LinkPolicy`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkTarget {
    /// A link the app may open (in a browser, or a `mailto:` composer).
    Web(String),
    /// Not a link: painted as text and never reported.
    Blocked,
}

/// Classifies a raw `href`. Called for every link while painting.
pub type LinkPolicy<'a> = &'a dyn Fn(&str) -> LinkTarget;

/// The default link policy: `http:`, `https:` and `mailto:` links are web links; everything
/// else (`javascript:`, `file:`, relative links, anchors) is blocked.
///
/// ```
/// use fluent2_egui::data::html::{LinkTarget, default_link_policy};
/// assert_eq!(default_link_policy("javascript:x()"), LinkTarget::Blocked);
/// ```
#[must_use]
pub fn default_link_policy(href: &str) -> LinkTarget {
    let trimmed = href.trim();
    let lower = trimmed.to_ascii_lowercase();
    if ["http://", "https://", "mailto:"]
        .iter()
        .any(|p| lower.starts_with(p))
    {
        LinkTarget::Web(trimmed.to_owned())
    } else {
        LinkTarget::Blocked
    }
}

/// What to do with one image, as decided by the app's [`ImagePolicy`].
#[derive(Clone, Debug)]
pub enum ImageDecision {
    /// Show this image source (a URI the app's egui image loaders handle, a texture, or bytes).
    /// A `cid:` image resolved from the message's attachments comes back this way.
    Show(egui::ImageSource<'static>),
    /// Show the image's own `data:` URI, decoded here. Only valid for `data:image/…;base64`.
    Embedded,
    /// Paint a placeholder with the alt text, and count it in
    /// [`HtmlResponse::blocked_images`] (so the app can offer "show images").
    Blocked,
    /// Paint nothing.
    Hidden,
}

/// Decides what to do with each image. Called for every image while painting.
pub type ImagePolicy<'a> = &'a dyn Fn(&ImageRef) -> ImageDecision;

/// The default image policy: inline `data:` images are shown; everything else (remote `http:`
/// images, `cid:` attachments) is blocked until the app decides otherwise.
#[must_use]
pub fn default_image_policy(image: &ImageRef) -> ImageDecision {
    if image
        .src
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("data:image/")
    {
        ImageDecision::Embedded
    } else {
        ImageDecision::Blocked
    }
}

/// A link the user activated (click, or Enter or Space while focused).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkClick {
    /// The `href` as written.
    pub href: String,
    /// What the policy made of it (never [`LinkTarget::Blocked`]).
    pub target: LinkTarget,
}

/// What [`HtmlView::show`] returns.
pub struct HtmlResponse {
    /// The area the document took.
    pub response: egui::Response,
    /// The link activated this frame, if any.
    pub clicked: Option<LinkClick>,
    /// How many images were painted as blocked placeholders.
    pub blocked_images: usize,
}

/// Paints an [`HtmlDocument`].
///
/// ```
/// use fluent2_egui::data::html::{HtmlDocument, HtmlView};
/// # fn demo(ui: &mut egui::Ui, doc: &HtmlDocument) {
/// let out = HtmlView::new("message", doc).show(ui);
/// if let Some(click) = out.clicked {
///     // the app opens click.target, if it wants to
///     let _ = click;
/// }
/// if out.blocked_images > 0 {
///     // offer "Show images"
/// }
/// # }
/// ```
pub struct HtmlView<'a> {
    id: Id,
    doc: &'a HtmlDocument,
    links: LinkPolicy<'a>,
    images: ImagePolicy<'a>,
}

impl<'a> HtmlView<'a> {
    /// A view of `doc`. `id_salt` keeps the ids of its links and scroll areas apart.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, doc: &'a HtmlDocument) -> Self {
        Self {
            id: Id::new(id_salt),
            doc,
            links: &default_link_policy,
            images: &default_image_policy,
        }
    }

    /// Decide what each link is. Defaults to [`default_link_policy`].
    #[must_use]
    pub fn link_policy(mut self, policy: LinkPolicy<'a>) -> Self {
        self.links = policy;
        self
    }

    /// Decide what each image does. Defaults to [`default_image_policy`].
    #[must_use]
    pub fn image_policy(mut self, policy: ImagePolicy<'a>) -> Self {
        self.images = policy;
        self
    }

    /// Paint the document into `ui`.
    pub fn show(self, ui: &mut Ui) -> HtmlResponse {
        crate::controls::ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let semibold = ui.fonts(|f| {
            f.definitions()
                .families
                .contains_key(&FontFamily::Name(SEMIBOLD_FAMILY.into()))
        });
        let page = c(theme.color.neutral_background_1);
        let mut painter = Painter {
            theme: &theme,
            links: self.links,
            images: self.images,
            semibold,
            clicked: None,
            blocked: 0,
            counter: 0,
            backgrounds: vec![page],
        };
        let inner = ui.push_id(self.id, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                painter.blocks(ui, &self.doc.blocks, 0);
            })
        });
        HtmlResponse {
            response: inner.response,
            clicked: painter.clicked,
            blocked_images: painter.blocked,
        }
    }
}

struct Painter<'a, 't> {
    theme: &'t Theme,
    links: LinkPolicy<'a>,
    images: ImagePolicy<'a>,
    semibold: bool,
    clicked: Option<LinkClick>,
    blocked: usize,
    /// Running index for ids of links, images and scroll areas.
    counter: usize,
    /// The background under the text being painted, innermost last.
    backgrounds: Vec<Color32>,
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

/// Whether the block takes vertical space around itself, as `<p>` margins do.
fn spaced(block: &Block) -> bool {
    !matches!(
        block,
        Block::Text {
            kind: TextKind::Flow,
            ..
        } | Block::Table(_)
    )
}

impl Painter<'_, '_> {
    fn next_id(&mut self, ui: &Ui, what: &str) -> Id {
        self.counter += 1;
        ui.id().with((what, self.counter))
    }

    fn page(&self) -> Color32 {
        self.backgrounds.last().copied().unwrap_or(Color32::WHITE)
    }

    fn blocks(&mut self, ui: &mut Ui, blocks: &[Block], depth: usize) {
        let mut previous: Option<&Block> = None;
        for block in blocks {
            if let Some(prev) = previous
                && (spaced(prev) || spaced(block))
            {
                ui.add_space(spacing_vertical::M);
            }
            self.block(ui, block, depth);
            previous = Some(block);
        }
    }

    fn block(&mut self, ui: &mut Ui, block: &Block, depth: usize) {
        let color = &self.theme.color;
        match block {
            Block::Text { kind, align, inlines } => self.text(ui, *kind, *align, inlines),
            Block::Pre(inlines) => {
                let id = self.next_id(ui, "pre");
                let fill = c(color.neutral_background_3);
                self.backgrounds.push(fill);
                Frame::new()
                    .fill(fill)
                    .corner_radius(CornerRadius::same(border_radius::MEDIUM as u8))
                    .inner_margin(Margin::same(spacing_horizontal::M as i8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::horizontal().id_salt(id).show(ui, |ui| {
                            let (job, links) =
                                self.job(inlines, TypeRamp::Body1, true, f32::INFINITY, TextAlign::Left);
                            self.paint_job(ui, job, &links, None);
                        });
                    });
                self.backgrounds.pop();
            }
            Block::Rule => {
                let width = ui.available_width().max(1.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(width, stroke_width::THIN), Sense::hover());
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(stroke_width::THIN, c(color.neutral_stroke_2)),
                );
            }
            Block::Quote(inner) => {
                let out = Frame::new()
                    .inner_margin(Margin {
                        left: spacing_horizontal::M as i8,
                        ..Margin::ZERO
                    })
                    .show(ui, |ui| self.blocks(ui, inner, depth + 1));
                let rect = out.response.rect;
                ui.painter().vline(
                    rect.left() + stroke_width::THICK / 2.0,
                    rect.y_range(),
                    Stroke::new(stroke_width::THICK, c(color.neutral_stroke_1)),
                );
            }
            Block::List { start, items } => self.list(ui, *start, items, depth),
            Block::Table(table) => self.table(ui, table, depth),
        }
    }

    /// One text block: a single label when it is all text, or a wrapped row of labels and
    /// images when it has images.
    fn text(&mut self, ui: &mut Ui, kind: TextKind, align: TextAlign, inlines: &[Inline]) {
        let step = match kind {
            TextKind::Heading(level) => heading_step(level),
            TextKind::Paragraph | TextKind::Flow => TypeRamp::Body1,
        };
        let layout = match align {
            TextAlign::Left => Layout::top_down(egui::Align::Min),
            TextAlign::Center => Layout::top_down(egui::Align::Center),
            TextAlign::Right => Layout::top_down(egui::Align::Max),
        };
        let has_images = inlines.iter().any(|i| matches!(i, Inline::Image(_)));
        ui.with_layout(layout, |ui| {
            if !has_images {
                let width = ui.available_width();
                let (job, links) = self.job(inlines, step, false, width, align);
                let heading = match kind {
                    TextKind::Heading(level) => Some(level),
                    _ => None,
                };
                self.paint_job(ui, job, &links, heading);
                return;
            }
            let main = match align {
                TextAlign::Left => egui::Align::Min,
                TextAlign::Center => egui::Align::Center,
                TextAlign::Right => egui::Align::Max,
            };
            ui.with_layout(
                Layout::left_to_right(egui::Align::BOTTOM)
                    .with_main_wrap(true)
                    .with_main_align(main),
                |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let mut start = 0;
                    for (i, inline) in inlines.iter().enumerate() {
                        if let Inline::Image(image) = inline {
                            if start < i {
                                let width = ui.available_width();
                                let (job, links) =
                                    self.job(&inlines[start..i], step, false, width, TextAlign::Left);
                                self.paint_job(ui, job, &links, None);
                            }
                            self.image(ui, image);
                            start = i + 1;
                        }
                    }
                    if start < inlines.len() {
                        let width = ui.available_width();
                        let (job, links) = self.job(&inlines[start..], step, false, width, TextAlign::Left);
                        self.paint_job(ui, job, &links, None);
                    }
                },
            );
        });
    }

    fn font(&self, size: f32, bold: bool, code: bool) -> FontId {
        if code {
            return FontId::new(size, FontFamily::Monospace);
        }
        let family = if bold && self.semibold {
            FontFamily::Name(SEMIBOLD_FAMILY.into())
        } else {
            FontFamily::Proportional
        };
        FontId::new(size, family)
    }

    /// A layout job for `inlines` (text runs only), and the char ranges of its links.
    fn job(
        &self,
        inlines: &[Inline],
        step: TypeRamp,
        pre: bool,
        width: f32,
        align: TextAlign,
    ) -> (LayoutJob, Vec<LinkSpan>) {
        let color = &self.theme.color;
        let mut job = LayoutJob::default();
        job.wrap.max_width = if pre { f32::INFINITY } else { width.max(1.0) };
        job.halign = match align {
            TextAlign::Left => egui::Align::LEFT,
            TextAlign::Center => egui::Align::Center,
            TextAlign::Right => egui::Align::RIGHT,
        };
        let mut links: Vec<LinkSpan> = Vec::new();
        let mut chars = 0;
        let base = font_id(step).size;
        let bold_step = step.semibold();
        for inline in inlines {
            let Inline::Text(run) = inline else {
                continue;
            };
            let target = run
                .link
                .as_deref()
                .map(|href| (href, (self.links)(href)))
                .filter(|(_, t)| *t != LinkTarget::Blocked);
            let style = run.style;
            let size = style.size.unwrap_or(base);
            let mut format = TextFormat {
                font_id: self.font(size, style.bold || bold_step, style.code || pre),
                color: if target.is_some() {
                    c(color.brand_foreground_link)
                } else {
                    self.ink(style.color)
                },
                italics: style.italic,
                ..TextFormat::default()
            };
            if let Some(bg) = style.background.and_then(|bg| self.fill(bg)) {
                format.background = bg;
                // On its own background (a link styled as a button), text keeps the author's
                // colour, checked against that background rather than the page.
                format.color = if target.is_some() && style.color.is_none() {
                    self.ink_on(None, bg)
                } else {
                    self.ink_on(style.color, bg)
                };
            }
            if style.underline || target.is_some() {
                format.underline = Stroke::new(stroke_width::THIN, format.color);
            }
            if style.strike {
                format.strikethrough = Stroke::new(stroke_width::THIN, format.color);
            }
            let text = if pre {
                run.text.clone()
            } else {
                run.text.replace('\t', " ")
            };
            let count = text.chars().count();
            if let Some((href, target)) = target {
                match links.last_mut() {
                    Some(last) if last.href == href && last.end == chars => {
                        last.end += count;
                        last.text.push_str(&text);
                    }
                    _ => links.push(LinkSpan {
                        start: chars,
                        end: chars + count,
                        href: href.to_owned(),
                        target,
                        text: text.clone(),
                    }),
                }
            }
            chars += count;
            job.append(&text, 0.0, format);
        }
        (job, links)
    }

    /// Paint a job as one selectable label, with a focusable, clickable link area over each
    /// link (which is also the link's AccessKit node).
    fn paint_job(&mut self, ui: &mut Ui, job: LayoutJob, links: &[LinkSpan], heading: Option<u8>) {
        if job.text.trim().is_empty() && job.text.chars().all(|ch| ch != '\n') {
            return;
        }
        let galley: Arc<Galley> = ui.fonts_mut(|f| f.layout_job(job));
        let response = ui.add(Label::new(galley.clone()).selectable(true));
        let origin = match galley.job.halign {
            egui::Align::LEFT => response.rect.left_top(),
            egui::Align::Center => response.rect.center_top(),
            egui::Align::RIGHT => response.rect.right_top(),
        };
        if let Some(level) = heading {
            let text = galley.text().to_owned();
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(Role::Heading);
                node.set_level(usize::from(level.max(1)));
                node.set_label(text);
            });
        }
        for link in links {
            let rects = link_rects(&galley, link.start, link.end);
            let id = self.next_id(ui, "link");
            for (n, rect) in rects.iter().enumerate() {
                let rect = rect.translate(origin.to_vec2());
                // The first piece is the Tab stop; the rest only take clicks.
                let sense = if n == 0 { Sense::click() } else { Sense::CLICK };
                let piece = ui
                    .interact(rect, id.with(n), sense)
                    .on_hover_cursor(CursorIcon::PointingHand);
                if n == 0 {
                    let label = link.text.trim().to_owned();
                    let enabled = ui.is_enabled();
                    piece.widget_info(|| WidgetInfo::labeled(WidgetType::Link, enabled, label.clone()));
                    if focus::focus_visible(ui, &piece) {
                        focus::paint_focus_ring(ui, rect, CornerRadius::ZERO);
                    }
                }
                let piece = if let LinkTarget::Web(url) = &link.target {
                    piece.on_hover_text(url.clone())
                } else {
                    piece
                };
                if piece.clicked() {
                    self.clicked = Some(LinkClick {
                        href: link.href.clone(),
                        target: link.target.clone(),
                    });
                }
            }
        }
    }

    fn image(&mut self, ui: &mut Ui, image: &ImageRef) {
        let available = ui.available_width().max(1.0);
        let decision = (self.images)(image);
        let source = match decision {
            ImageDecision::Hidden => return,
            ImageDecision::Blocked => None,
            ImageDecision::Show(source) => Some(source),
            ImageDecision::Embedded => {
                data_uri::register(ui.ctx(), &image.src).map(|uri| egui::ImageSource::Uri(uri.into()))
            }
        };
        let size = image_size(image, available);
        let id = self.next_id(ui, "image");
        let label = if image.alt.is_empty() {
            "Image".to_owned()
        } else {
            image.alt.clone()
        };
        let response = if let Some(source) = source {
            let mut widget = egui::Image::new(source).alt_text(label.clone());
            widget = match size {
                Some(size) => widget.fit_to_exact_size(size),
                None => widget.max_width(available).max_height(600.0),
            };
            ui.add(widget)
        } else {
            self.blocked += 1;
            let size = size
                .unwrap_or(Vec2::new(available.min(160.0), 48.0))
                .max(Vec2::splat(24.0));
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            let color = &self.theme.color;
            let painter = ui.painter();
            painter.rect(
                rect,
                CornerRadius::same(border_radius::MEDIUM as u8),
                c(color.neutral_background_3),
                Stroke::new(stroke_width::THIN, c(color.neutral_stroke_2)),
                egui::StrokeKind::Inside,
            );
            if rect.width() >= 24.0 && rect.height() >= 24.0 {
                let icon = Rect::from_min_size(rect.left_top() + Vec2::splat(4.0), Vec2::splat(16.0));
                paint_icon(
                    ui,
                    icon,
                    IconName::Image,
                    IconSize::S16,
                    IconStyle::Regular,
                    Some(c(color.neutral_foreground_3)),
                );
            }
            if !image.alt.is_empty() && rect.width() > 40.0 {
                let text_rect = rect.shrink2(Vec2::new(24.0, 4.0));
                let galley = ui.fonts_mut(|f| {
                    f.layout(
                        image.alt.clone(),
                        font_id(TypeRamp::Caption1),
                        c(color.neutral_foreground_3),
                        text_rect.width().max(1.0),
                    )
                });
                ui.painter().with_clip_rect(rect).galley(
                    text_rect.left_top(),
                    galley,
                    c(color.neutral_foreground_3),
                );
            }
            let blocked = format!("Blocked image: {label}");
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(Role::Image);
                node.set_label(blocked);
            });
            response
        };
        let _ = id;
        // A linked image is a link.
        if let Some(href) = &image.link
            && let LinkTarget::Web(url) = (self.links)(href)
        {
            let link = ui
                .interact(response.rect, self.next_id(ui, "image-link"), Sense::click())
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text(url.clone());
            let enabled = ui.is_enabled();
            link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, enabled, label.clone()));
            if focus::focus_visible(ui, &link) {
                focus::paint_focus_ring(ui, response.rect, CornerRadius::ZERO);
            }
            if link.clicked() {
                self.clicked = Some(LinkClick {
                    href: href.clone(),
                    target: LinkTarget::Web(url),
                });
            }
        }
    }

    fn list(&mut self, ui: &mut Ui, start: Option<u64>, items: &[Vec<Block>], depth: usize) {
        let fg = c(self.theme.color.neutral_foreground_1);
        let body = font_id(TypeRamp::Body1);
        let row = ui.fonts_mut(|f| f.row_height(&body));
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                ui.add_space(spacing_vertical::XS);
            }
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = spacing_horizontal::XS;
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(spacing_horizontal::XL, row), Sense::hover());
                match start {
                    Some(n) => {
                        ui.painter().text(
                            rect.right_center(),
                            egui::Align2::RIGHT_CENTER,
                            format!("{}.", n.saturating_add(i as u64)),
                            body.clone(),
                            fg,
                        );
                    }
                    None => {
                        let radius = TypeRamp::Body1.size() * 0.18;
                        if depth.is_multiple_of(2) {
                            ui.painter().circle_filled(rect.center(), radius, fg);
                        } else {
                            ui.painter().circle_stroke(
                                rect.center(),
                                radius,
                                Stroke::new(stroke_width::THIN, fg),
                            );
                        }
                    }
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    self.blocks(ui, item, depth + 1);
                });
            });
        }
    }

    fn table(&mut self, ui: &mut Ui, table: &Table, depth: usize) {
        let available = ui.available_width().max(1.0);
        let width = match table.width {
            Some(Length::Points(p)) => p.min(available),
            Some(Length::Percent(pct)) => (available * pct / 100.0).min(available),
            None => available,
        };
        let widths = column_widths(table, width);
        let pad = spacing_horizontal::XS;
        // A table narrower than its container sits by its alignment.
        let offset = match table.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => (available - width) / 2.0,
            TextAlign::Right => available - width,
        };
        if offset > 0.5 {
            let rect = Rect::from_min_size(ui.cursor().min + Vec2::new(offset, 0.0), Vec2::new(width, 0.0));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(Layout::top_down(egui::Align::Min)),
            );
            let mut centred = table.clone();
            centred.align = TextAlign::Left;
            centred.width = Some(Length::Points(width));
            self.table(&mut child, &centred, depth);
            let used = child.min_rect();
            ui.allocate_rect(
                Rect::from_min_max(
                    ui.cursor().min,
                    egui::pos2(ui.cursor().min.x + available, used.bottom()),
                ),
                Sense::hover(),
            );
            return;
        }
        let table_bg = table.background.and_then(|bg| self.fill(bg));
        let table_slot = ui.painter().add(egui::Shape::Noop);
        let top = ui.cursor().top();
        let left = ui.cursor().left();
        if let Some(bg) = table_bg {
            self.backgrounds.push(bg);
        }
        for row in &table.rows {
            let row_top = ui.cursor().top();
            let mut slots = Vec::new();
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let mut column = 0;
                for cell in row {
                    let span = cell.colspan.max(1);
                    let w: f32 = widths
                        .iter()
                        .skip(column)
                        .take(span)
                        .sum::<f32>()
                        .max(pad * 2.0 + 1.0);
                    column += span;
                    let fill = cell.background.and_then(|bg| self.fill(bg));
                    let slot = ui.painter().add(egui::Shape::Noop);
                    let x = ui.cursor().left();
                    if let Some(bg) = fill {
                        self.backgrounds.push(bg);
                    }
                    ui.allocate_ui_with_layout(Vec2::new(w, 0.0), Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(w);
                        Frame::new().inner_margin(Margin::same(pad as i8)).show(ui, |ui| {
                            ui.set_width((w - pad * 2.0).max(1.0));
                            ui.spacing_mut().item_spacing.y = 0.0;
                            self.blocks(ui, &cell.blocks, depth + 1);
                        });
                    });
                    if fill.is_some() {
                        self.backgrounds.pop();
                    }
                    slots.push((slot, x, w, fill));
                }
            });
            let row_bottom = ui.min_rect().bottom().max(row_top);
            for (slot, x, w, fill) in slots {
                if let Some(fill) = fill {
                    let rect = Rect::from_min_max(egui::pos2(x, row_top), egui::pos2(x + w, row_bottom));
                    ui.painter().set(slot, egui::Shape::rect_filled(rect, 0.0, fill));
                }
            }
        }
        if let Some(bg) = table_bg {
            self.backgrounds.pop();
            let rect = Rect::from_min_max(
                egui::pos2(left, top),
                egui::pos2(left + width, ui.min_rect().bottom()),
            );
            ui.painter()
                .set(table_slot, egui::Shape::rect_filled(rect, 0.0, bg));
        }
    }

    /// The colour for document text `color` on the current background.
    fn ink(&self, color: Option<Rgb>) -> Color32 {
        self.ink_on(color, self.page())
    }

    fn ink_on(&self, color: Option<Rgb>, background: Color32) -> Color32 {
        let fg = c(self.theme.color.neutral_foreground_1);
        let Some(Rgb(r, g, b)) = color else {
            return fg;
        };
        let authored = Color32::from_rgb(r, g, b);
        match self.theme.kind {
            ThemeKind::HighContrast => fg,
            ThemeKind::Light => {
                // Keep the author's colour unless it is unreadable (white on white).
                if contrast(authored, background) < 2.0 {
                    fg
                } else {
                    authored
                }
            }
            ThemeKind::Dark => {
                if contrast(authored, background) >= 4.5 {
                    return authored;
                }
                let flipped = invert_lightness(authored);
                if contrast(flipped, background) >= 4.5 {
                    flipped
                } else {
                    fg
                }
            }
        }
    }

    /// The fill for document background `color`, or `None` to leave the page showing.
    fn fill(&self, Rgb(r, g, b): Rgb) -> Option<Color32> {
        let authored = Color32::from_rgb(r, g, b);
        match self.theme.kind {
            ThemeKind::HighContrast => None,
            ThemeKind::Light => Some(authored),
            ThemeKind::Dark => Some(if luminance(authored) > 0.18 {
                invert_lightness(authored)
            } else {
                authored
            }),
        }
    }
}

/// A link inside a laid-out job.
struct LinkSpan {
    start: usize,
    end: usize,
    href: String,
    target: LinkTarget,
    text: String,
}

/// Galley-relative rectangles covering chars `start..end`, one per row.
fn link_rects(galley: &Galley, start: usize, end: usize) -> Vec<Rect> {
    let mut out = Vec::new();
    let mut row_start = 0;
    for row in &galley.rows {
        let count = row.char_count_including_newline().0;
        let row_end = row_start + row.char_count_excluding_newline().0;
        let from = start.max(row_start);
        let to = end.min(row_end);
        if from < to {
            let x0 = row.x_offset(epaint_char(from - row_start));
            let x1 = row.x_offset(epaint_char(to - row_start));
            let rect = row.rect();
            out.push(Rect::from_x_y_ranges(
                (rect.left() + x0)..=(rect.left() + x1),
                rect.y_range(),
            ));
        }
        row_start += count;
        if row_start >= end {
            break;
        }
    }
    out
}

fn epaint_char(index: usize) -> egui::epaint::text::CharIndex {
    egui::epaint::text::CharIndex(index)
}

/// The size to paint `image` at, from its attributes, scaled down to fit; `None` when unknown.
fn image_size(image: &ImageRef, available: f32) -> Option<Vec2> {
    let (w, h) = match (image.width, image.height) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => (w, h),
        _ => return None,
    };
    let scale = (available / w).min(1.0);
    Some(Vec2::new(w * scale, h * scale))
}

/// Column widths in points: explicit widths from single-column cells, the rest shared equally,
/// everything scaled to fit `total`.
fn column_widths(table: &Table, total: f32) -> Vec<f32> {
    let columns = table
        .rows
        .iter()
        .map(|row| row.iter().map(|cell| cell.colspan.max(1)).sum::<usize>())
        .max()
        .unwrap_or(1)
        .max(1);
    let mut widths: Vec<Option<f32>> = vec![None; columns];
    for row in &table.rows {
        let mut column = 0;
        for cell in row {
            let span = cell.colspan.max(1);
            if span == 1
                && column < columns
                && widths[column].is_none()
                && let Some(width) = cell.width
            {
                widths[column] = Some(match width {
                    Length::Points(p) => p,
                    Length::Percent(pct) => total * pct / 100.0,
                });
            }
            column += span;
        }
    }
    let known: f32 = widths.iter().flatten().sum();
    let unknown = widths.iter().filter(|w| w.is_none()).count();
    let share = if unknown > 0 {
        ((total - known) / unknown as f32).max(total / columns as f32 * 0.25)
    } else {
        0.0
    };
    let mut out: Vec<f32> = widths.iter().map(|w| w.unwrap_or(share)).collect();
    let sum: f32 = out.iter().sum();
    if sum > total && sum > 0.0 {
        for w in &mut out {
            *w *= total / sum;
        }
    }
    out
}

fn linear(channel: u8) -> f32 {
    let v = f32::from(channel) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(color: Color32) -> f32 {
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

/// WCAG contrast ratio.
fn contrast(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Mirror a colour's HSL lightness (keeping hue and saturation), so dark text becomes light
/// and light backgrounds become dark.
fn invert_lightness(color: Color32) -> Color32 {
    let (r, g, b) = (
        f32::from(color.r()) / 255.0,
        f32::from(color.g()) / 255.0,
        f32::from(color.b()) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    let (h, s) = if d == 0.0 {
        (0.0, 0.0)
    } else {
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if max == r {
            ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h / 6.0, s)
    };
    let l = 1.0 - l;
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let channel = |t: f32| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lightness_inversion_keeps_hue() {
        assert_eq!(invert_lightness(Color32::WHITE), Color32::BLACK);
        assert_eq!(invert_lightness(Color32::BLACK), Color32::WHITE);
        let navy = invert_lightness(Color32::from_rgb(0, 0, 128));
        assert!(navy.b() > navy.r() && navy.b() > navy.g(), "{navy:?}");
        assert!(luminance(navy) > luminance(Color32::from_rgb(0, 0, 128)));
    }

    #[test]
    fn widths_fit_the_table() {
        let cell = |width| Cell {
            blocks: Vec::new(),
            colspan: 1,
            width,
            background: None,
            header: false,
        };
        let table = Table {
            rows: vec![vec![
                cell(Some(Length::Points(400.0))),
                cell(None),
                cell(Some(Length::Percent(25.0))),
            ]],
            width: None,
            background: None,
            align: TextAlign::Left,
        };
        let widths = column_widths(&table, 600.0);
        assert!((widths.iter().sum::<f32>() - 600.0).abs() < 0.5, "{widths:?}");
        assert!(widths[0] > widths[1]);
    }
}
