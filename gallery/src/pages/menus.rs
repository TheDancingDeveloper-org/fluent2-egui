//! Gallery page for Fluent menus.

use egui::{Response, Sense, Ui};
use fluent2_egui::surfaces::{MenuItem, context_menu, divider, group_header, menu_frame, submenu};
use fluent2_icons::Icon;

/// A menu button, a checkable list, a submenu and a context menu on a card.
pub fn page(ui: &mut Ui) {
    ui.label("Menus");
    let _ = ui.menu_button("File", |ui| {
        let _ = menu_frame(ui);
        group_header(ui, "File");
        let _ = ui.add(MenuItem::new("New").icon(Icon::Add).shortcut("Ctrl+N"));
        let _ = ui.add(MenuItem::new("Save").icon(Icon::Save).shortcut("Ctrl+S"));
        divider(ui);
        submenu(ui, "Share", Some(Icon::Share), |ui| {
            let _ = ui.add(MenuItem::new("Email"));
            let _ = ui.add(MenuItem::new("Copy link"));
        });
        divider(ui);
        let _ = ui.add(
            MenuItem::new("Autosave")
                .checked(true)
                .check_column(true)
                .shortcut(""),
        );
        let _ = ui.add(MenuItem::new("Read only").checked(false).check_column(true));
    });

    let card = card(ui, "Right-click this card");
    context_menu(&card, |ui| {
        let _ = ui.add(MenuItem::new("Cut").shortcut("Ctrl+X"));
        let _ = ui.add(MenuItem::new("Copy").shortcut("Ctrl+C"));
        let _ = ui.add(
            MenuItem::new("Paste")
                .shortcut("Ctrl+V")
                .disabled_reason("Nothing to paste"),
        );
    });
}

fn card(ui: &mut Ui, label: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(220.0, 80.0), Sense::click());
    let theme = fluent2_egui::Theme::get(ui.ctx());
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(4),
        fluent2_egui::theme::c(theme.color.neutral_background_2),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Body.resolve(ui.style()),
        fluent2_egui::theme::c(theme.color.neutral_foreground_1),
    );
    response
}
