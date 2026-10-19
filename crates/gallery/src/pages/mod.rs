//! Gallery pages.
//!
//! Later items add a page by adding one file in `crates/gallery/src/pages/` and one line in
//! [`pages`] below. Nothing else in the gallery needs to change.

mod backstage;
mod command_palette;
mod data_grid;
mod drawer;
mod file_dialog;
mod icons;
mod ribbon;
mod tab_strip;
mod terminal;
mod tokens;
mod typography;

pub use backstage::BackstagePage;
pub use command_palette::CommandPalettePage;
pub use data_grid::DataGridPage;
pub use drawer::DrawerPage;
pub use file_dialog::FileDialogPage;
pub use icons::IconsPage;
pub use ribbon::RibbonPage;
pub use tab_strip::TabStripPage;
pub use terminal::TerminalPage;
pub use tokens::TokensPage;
pub use typography::TypographyPage;

/// A page in the gallery's left-hand list.
pub trait Page {
    /// Label shown in the page list.
    fn name(&self) -> &'static str;

    /// Paint the page into the central panel.
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// The pages this gallery ships with.
pub fn pages() -> Vec<Box<dyn Page>> {
    vec![
        Box::new(TokensPage),
        Box::new(TypographyPage),
        Box::new(IconsPage::default()),
        Box::new(DataGridPage::default()),
        Box::new(BackstagePage::default()),
        Box::new(RibbonPage::default()),
        Box::new(TerminalPage::default()),
        Box::new(CommandPalettePage::default()),
        Box::new(TabStripPage::default()),
        Box::new(FileDialogPage::default()),
        Box::new(DrawerPage::default()),
    ]
}
