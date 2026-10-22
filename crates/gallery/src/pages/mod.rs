//! Gallery pages.
//!
//! Later items add a page by adding one file in `crates/gallery/src/pages/` and one line in
//! [`pages`] below. Nothing else in the gallery needs to change.

mod accordion;
mod avatars;
mod backstage;
mod breadcrumb;
mod charts;
mod code_view;
mod command_palette;
pub(crate) mod controls;
mod data_grid;
mod date_time;
mod diff;
mod docking;
mod drawer;
mod file_dialog;
mod html_view;
mod icons;
mod markdown;
mod pager;
mod reorder;
mod ribbon;
mod rich_text_editor;
mod shortcuts;
mod skeleton;
mod surfaces;
mod tab_strip;
mod terminal;
mod tokens;
mod typography;
mod variable_list;

pub use accordion::AccordionPage;
pub use avatars::AvatarsPage;
pub use backstage::BackstagePage;
pub use breadcrumb::BreadcrumbPage;
pub use charts::ChartsPage;
pub use code_view::CodeViewPage;
pub use command_palette::CommandPalettePage;
pub use controls::ControlsPage;
pub use data_grid::DataGridPage;
pub use date_time::DateTimePage;
pub use diff::DiffPage;
pub use docking::DockingPage;
pub use drawer::DrawerPage;
pub use file_dialog::FileDialogPage;
pub use html_view::HtmlPage;
pub use icons::IconsPage;
pub use markdown::MarkdownPage;
pub use pager::PagerPage;
pub use reorder::ReorderPage;
pub use ribbon::RibbonPage;
pub use rich_text_editor::RichTextEditorPage;
pub use shortcuts::ShortcutsPage;
pub use skeleton::SkeletonPage;
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
///
/// Keep this list sorted by [`Page::name`]: entries are one per line so two PRs adding a page
/// touch different lines, and the test below keeps the order from drifting.
pub fn pages() -> Vec<Box<dyn Page>> {
    vec![
        Box::new(AccordionPage::default()),
        Box::new(AvatarsPage),
        Box::new(BackstagePage::default()),
        Box::new(ControlsPage::default()),
        Box::new(BreadcrumbPage::default()),
        Box::new(ChartsPage::default()),
        Box::new(CodeViewPage::default()),
        Box::new(CommandPalettePage::default()),
        Box::new(DataGridPage::default()),
        Box::new(DateTimePage::default()),
        Box::new(DiffPage::default()),
        Box::new(DockingPage::default()),
        Box::new(DrawerPage::default()),
        Box::new(FileDialogPage::default()),
        Box::new(HtmlPage::default()),
        Box::new(IconsPage::default()),
        Box::new(VariableListPage::default()),
        Box::new(MarkdownPage::default()),
        Box::new(PagerPage::default()),
        Box::new(ReorderPage::default()),
        Box::new(RibbonPage::default()),
        Box::new(RichTextEditorPage::default()),
        Box::new(ShortcutsPage::default()),
        Box::new(SkeletonPage),
        Box::new(SurfacesPage::default()),
        Box::new(TabStripPage::default()),
        Box::new(TerminalPage::default()),
        Box::new(TokensPage),
        Box::new(TypographyPage),
    ]
}

#[cfg(test)]
mod tests {
    use super::pages;

    #[test]
    fn pages_are_sorted_by_name() {
        let names: Vec<_> = pages().iter().map(|page| page.name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "gallery pages must be sorted by name");
    }
}
