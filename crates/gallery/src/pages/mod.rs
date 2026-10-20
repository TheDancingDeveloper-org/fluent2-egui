//! Gallery pages.
//!
//! Later items add a page by adding one file in `crates/gallery/src/pages/` and one line in
//! [`pages`] below. Nothing else in the gallery needs to change.

mod backstage;
mod charts;
mod code_view;
mod command_palette;
pub(crate) mod controls;
mod data_grid;
mod diff;
mod docking;
mod drawer;
mod file_dialog;
mod icons;
mod markdown;
mod pager;
mod reorder;
mod ribbon;
mod surfaces;
mod tab_strip;
mod terminal;
mod tokens;
mod typography;
mod variable_list;

pub use backstage::BackstagePage;
pub use charts::ChartsPage;
pub use code_view::CodeViewPage;
pub use command_palette::CommandPalettePage;
pub use controls::ControlsPage;
pub use data_grid::DataGridPage;
pub use diff::DiffPage;
pub use docking::DockingPage;
pub use drawer::DrawerPage;
pub use file_dialog::FileDialogPage;
pub use icons::IconsPage;
pub use markdown::MarkdownPage;
pub use pager::PagerPage;
pub use reorder::ReorderPage;
pub use ribbon::RibbonPage;
pub use surfaces::SurfacesPage;
pub use tab_strip::TabStripPage;
pub use terminal::TerminalPage;
pub use tokens::TokensPage;
pub use typography::TypographyPage;
pub use variable_list::VariableListPage;

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
        Box::new(ControlsPage::default()),
        Box::new(SurfacesPage::default()),
        Box::new(DataGridPage::default()),
        Box::new(VariableListPage::default()),
        Box::new(PagerPage::default()),
        Box::new(BackstagePage::default()),
        Box::new(RibbonPage::default()),
        Box::new(TerminalPage::default()),
        Box::new(DiffPage::default()),
        Box::new(CodeViewPage::default()),
        Box::new(CommandPalettePage::default()),
        Box::new(TabStripPage::default()),
        Box::new(FileDialogPage::default()),
        Box::new(DrawerPage::default()),
        Box::new(ChartsPage::default()),
        Box::new(DockingPage::default()),
        Box::new(ReorderPage::default()),
        Box::new(MarkdownPage::default()),
    ]
}
