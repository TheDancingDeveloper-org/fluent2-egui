//! FileCard: an attachment card with a type icon, name, size or progress, and actions
//! (WI-1945).
//!
//! The icon comes from the MIME type, or from the file extension when there is none. Long
//! names are cut with an ellipsis (the full name is the tooltip and the accessible name). The
//! size reads "1.2 MB" ([`format_size`]); while an upload or download runs, a progress bar
//! takes its place. Remove, download and open are icon buttons named "Remove report.pdf" and so
//! on; when the card is openable, clicking it (or Enter on it) opens too. [`file_card_row`] lays
//! cards out in wrapping rows.

use egui::{CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, WidgetType};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::controls::{ProgressBar, ProgressBarSize};
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::TypeRamp;
use crate::surfaces::TooltipExt;
use crate::theme::{Theme, c};

/// What a [`FileCard`] did this frame.
#[derive(Debug)]
pub struct FileCardResponse {
    /// The card.
    pub response: Response,
    /// The card or its open button was activated.
    pub open: bool,
    /// The download button was activated.
    pub download: bool,
    /// The remove button was activated.
    pub remove: bool,
}

/// An attachment card. See the module docs.
///
/// ```
/// # use fluent2_egui::controls::{FileCard, file_card_row};
/// # fn demo(ui: &mut egui::Ui) {
/// file_card_row(ui, |ui| {
///     let out = FileCard::new("report.pdf").mime("application/pdf").size(1_250_000).removable(true).show(ui);
///     if out.remove {
///         // drop the attachment
///     }
///     FileCard::new("photo.jpg").progress(Some(0.4)).show(ui);
/// });
/// # }
/// ```
pub struct FileCard {
    name: String,
    mime: Option<String>,
    size: Option<u64>,
    progress: Option<Option<f32>>,
    removable: bool,
    downloadable: bool,
    openable: bool,
    width: f32,
    id_salt: Option<Id>,
}

impl FileCard {
    /// A card for the file `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            mime: None,
            size: None,
            progress: None,
            removable: false,
            downloadable: false,
            openable: true,
            width: 240.0,
            id_salt: None,
        }
    }

    /// The MIME type, which picks the icon (`image/png`, `application/pdf`…).
    #[must_use]
    pub fn mime(mut self, mime: impl Into<String>) -> Self {
        self.mime = Some(mime.into());
        self
    }

    /// The size in bytes, shown as "1.2 MB".
    #[must_use]
    pub fn size(mut self, bytes: u64) -> Self {
        self.size = Some(bytes);
        self
    }

    /// A transfer in progress: `Some(fraction)` from 0 to 1, or `None` while the total is
    /// unknown. It replaces the size line.
    #[must_use]
    pub fn progress(mut self, fraction: Option<f32>) -> Self {
        self.progress = Some(fraction);
        self
    }

    /// Show a remove button.
    #[must_use]
    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    /// Show a download button.
    #[must_use]
    pub fn downloadable(mut self, downloadable: bool) -> Self {
        self.downloadable = downloadable;
        self
    }

    /// Whether activating the card opens the file. The default is true.
    #[must_use]
    pub fn openable(mut self, openable: bool) -> Self {
        self.openable = openable;
        self
    }

    /// Card width in points. The default is 240.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.clamp(120.0, 600.0);
        }
        self
    }

    /// Salt for the card's id, when two cards could share a name.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// The icon for this card's MIME type or extension.
    #[must_use]
    pub fn icon(&self) -> IconName {
        file_icon(self.mime.as_deref(), &self.name)
    }

    /// Show the card.
    pub fn show(self, ui: &mut Ui) -> FileCardResponse {
        crate::controls::ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let color = &theme.color;
        let enabled = ui.is_enabled();
        let height = 52.0;
        let salt = self.id_salt.unwrap_or_else(|| Id::new(&self.name));
        let id = ui.make_persistent_id(("fluent2_file_card", salt));
        let sense = if self.openable {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, card) = ui.allocate_exact_size(Vec2::new(self.width, height), sense);
        let radius = CornerRadius::same(border_radius::MEDIUM as u8);
        let pad = spacing_horizontal::M;

        let fill = if self.openable && card.is_pointer_button_down_on() {
            color.neutral_background_1_pressed
        } else if self.openable && card.hovered() {
            color.neutral_background_1_hover
        } else {
            color.neutral_background_1
        };
        ui.painter().rect(
            rect,
            radius,
            c(fill),
            Stroke::new(stroke_width::THIN, c(color.neutral_stroke_1)),
            StrokeKind::Inside,
        );
        let ink = if self.openable && card.hovered() {
            color.neutral_foreground_1_hover
        } else {
            color.neutral_foreground_1
        };

        // Type icon.
        let icon_rect = Rect::from_center_size(
            egui::pos2(rect.left() + pad + 12.0, rect.center().y),
            Vec2::splat(24.0),
        );
        paint_icon(
            ui,
            icon_rect,
            self.icon(),
            IconSize::S24,
            IconStyle::Regular,
            Some(c(color.neutral_foreground_3)),
        );

        // Action buttons, right to left.
        let button = 28.0;
        let mut right = rect.right() - spacing_horizontal::S;
        let mut actions: Vec<(&str, IconName)> = Vec::new();
        if self.removable {
            actions.push(("Remove", IconName::Dismiss));
        }
        if self.downloadable {
            actions.push(("Download", IconName::ArrowDownload));
        }
        let mut pressed = [false; 2];
        let mut action_rects = Vec::new();
        for (index, (verb, icon)) in actions.iter().enumerate() {
            let b = Rect::from_center_size(
                egui::pos2(right - button / 2.0, rect.center().y),
                Vec2::splat(button),
            );
            right -= button + spacing_horizontal::XXS;
            action_rects.push(b);
            let response = ui.interact(b, id.with(*verb), Sense::click());
            if response.hovered() && enabled {
                ui.painter()
                    .rect_filled(b, radius, c(color.subtle_background_hover));
            }
            paint_icon(
                ui,
                b.shrink(6.0),
                *icon,
                IconSize::S16,
                IconStyle::Regular,
                Some(c(color.neutral_foreground_2)),
            );
            if focus_visible(ui, &response) {
                paint_focus_ring(ui, b, radius);
            }
            let name = format!("{verb} {}", self.name);
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, name.clone()));
            let response = response.fluent_tooltip(verb.to_string());
            if enabled && response.clicked() {
                pressed[usize::from(index > 0 && self.removable)] = true;
            }
        }
        let (remove, download) = if self.removable {
            (pressed[0], pressed[1])
        } else {
            (false, pressed[0])
        };

        // Name and size or progress.
        let text_left = icon_rect.right() + spacing_horizontal::S;
        let text_width = (right - text_left - spacing_horizontal::XS).max(20.0);
        let name_galley = egui::WidgetText::from(self.name.clone()).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            text_width,
            egui::TextStyle::Name(TypeRamp::Body1Strong.name().into()),
        );
        let truncated = name_galley.elided;
        let name_top = rect.top() + spacing_vertical::S + 1.0;
        ui.painter()
            .galley(egui::pos2(text_left, name_top), name_galley, c(ink));
        let second = egui::pos2(text_left, rect.bottom() - spacing_vertical::S - 14.0);
        if let Some(fraction) = self.progress {
            let bar = Rect::from_min_size(second + Vec2::new(0.0, 6.0), Vec2::new(text_width, 4.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar));
            child.add(ProgressBar::new(fraction).size(ProgressBarSize::Medium));
        } else if let Some(bytes) = self.size {
            let galley = egui::WidgetText::from(format_size(bytes)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                text_width,
                egui::TextStyle::Name(TypeRamp::Caption1.name().into()),
            );
            ui.painter().galley(second, galley, c(color.neutral_foreground_3));
        }

        let card = if truncated {
            card.fluent_tooltip(self.name.clone())
        } else {
            card
        };
        if self.openable && focus_visible(ui, &card) {
            paint_focus_ring(ui, rect, radius);
        }
        let mut label = self.name.clone();
        match (self.progress, self.size) {
            (Some(Some(fraction)), _) => label.push_str(&format!(
                ", {}% transferred",
                (fraction.clamp(0.0, 1.0) * 100.0).round()
            )),
            (Some(None), _) => label.push_str(", transferring"),
            (None, Some(bytes)) => label.push_str(&format!(", {}", format_size(bytes))),
            (None, None) => {}
        }
        let kind = if self.openable {
            WidgetType::Button
        } else {
            WidgetType::Label
        };
        card.widget_info(|| WidgetInfo::labeled(kind, enabled, label.clone()));
        let pointer_on_action = ui
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|p| action_rects.iter().any(|r| r.contains(p)));
        let open = self.openable && enabled && card.clicked() && !pointer_on_action;
        FileCardResponse {
            response: card,
            open,
            download,
            remove,
        }
    }
}

/// Lay cards (or anything) out in rows that wrap, with Fluent gaps.
pub fn file_card_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(spacing_horizontal::S, spacing_vertical::S);
        add(ui)
    })
    .inner
}

/// "512 B", "1.2 KB", "34 MB", "1.5 GB": 1024-based, one decimal below 10.
#[must_use]
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

/// The icon for a MIME type, or for the file name's extension when the type is missing or
/// generic (`application/octet-stream`).
#[must_use]
pub fn file_icon(mime: Option<&str>, name: &str) -> IconName {
    let mime = mime
        .map(str::to_ascii_lowercase)
        .filter(|m| m != "application/octet-stream");
    if let Some(mime) = mime {
        let (kind, sub) = mime.split_once('/').unwrap_or((mime.as_str(), ""));
        return match (kind, sub) {
            ("image", _) => IconName::Image,
            ("message", _) => IconName::Mail,
            (_, s)
                if s.contains("zip")
                    || s.contains("compressed")
                    || s.contains("tar")
                    || s.contains("7z")
                    || s.contains("rar") =>
            {
                IconName::Archive
            }
            (_, s) if s.contains("spreadsheet") || s.contains("excel") || s == "csv" => IconName::Table,
            ("calendar", _) | (_, "ics") => IconName::Calendar,
            ("text", _) | ("application", _) => IconName::Document,
            _ => IconName::Attach,
        };
    }
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "heic" | "tif" | "tiff" => IconName::Image,
        "eml" | "msg" => IconName::Mail,
        "zip" | "gz" | "tgz" | "tar" | "7z" | "rar" | "bz2" | "xz" => IconName::Archive,
        "xls" | "xlsx" | "csv" | "ods" | "tsv" => IconName::Table,
        "ics" => IconName::Calendar,
        "pdf" | "doc" | "docx" | "txt" | "md" | "rtf" | "odt" | "ppt" | "pptx" | "html" | "json" | "xml" => {
            IconName::Document
        }
        _ => IconName::Attach,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1_250_000), "1.2 MB");
        assert_eq!(format_size(34 * 1024 * 1024), "34 MB");
        assert_eq!(format_size(u64::MAX), "16384 PB");
    }

    #[test]
    fn icons() {
        assert_eq!(file_icon(Some("image/png"), "x"), IconName::Image);
        assert_eq!(file_icon(Some("application/pdf"), "x"), IconName::Document);
        assert_eq!(file_icon(Some("application/zip"), "x"), IconName::Archive);
        assert_eq!(file_icon(Some("text/csv"), "x"), IconName::Table);
        assert_eq!(file_icon(Some("message/rfc822"), "x"), IconName::Mail);
        assert_eq!(
            file_icon(Some("application/octet-stream"), "a.JPG"),
            IconName::Image
        );
        assert_eq!(file_icon(None, "budget.xlsx"), IconName::Table);
        assert_eq!(file_icon(None, "invite.ics"), IconName::Calendar);
        assert_eq!(file_icon(None, "noext"), IconName::Attach);
    }
}
