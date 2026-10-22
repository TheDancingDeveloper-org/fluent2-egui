//! Gallery pages.
//!
//! Later items add a page by adding one file in `crates/gallery/src/pages/` and one line in
//! [`pages`] below. Nothing else in the gallery needs to change.

use fluent2_egui::platform::LayoutStore;

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
mod editable_list;
mod file_dialog;
mod find;
mod html_view;
mod icons;
mod markdown;
mod menu_bar;
mod pager;
mod pieces;
mod reorder;
mod ribbon;
mod rich_text_editor;
mod shortcuts;
mod skeleton;
mod surfaces;
mod tab_strip;
mod terminal;
mod tokens;
mod tree_view;
mod typography;
mod url_state;
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
pub use editable_list::EditableListPage;
pub use file_dialog::FileDialogPage;
pub use find::FindPage;
pub use html_view::HtmlPage;
pub use icons::IconsPage;
pub use markdown::MarkdownPage;
pub use menu_bar::MenuBarPage;
pub use pager::PagerPage;
pub use pieces::PiecesPage;
pub use reorder::ReorderPage;
pub use ribbon::RibbonPage;
pub use rich_text_editor::RichTextEditorPage;
pub use shortcuts::ShortcutsPage;
pub use skeleton::SkeletonPage;
pub use surfaces::SurfacesPage;
pub use tab_strip::TabStripPage;
pub use terminal::TerminalPage;
pub use tokens::TokensPage;
pub use tree_view::TreeViewPage;
pub use typography::TypographyPage;
pub use url_state::UrlStatePage;
pub use variable_list::VariableListPage;

/// A page in the gallery's left-hand list.
pub trait Page {
    /// Label shown in the page list.
    fn name(&self) -> &'static str;

    /// Paint the page into the central panel.
    fn ui(&mut self, ui: &mut egui::Ui);

    /// Put the page's layout state (splits, docks, grid columns) into `store` (issue #118).
    /// Pages without layout keep the default, which stores nothing.
    fn save_layout(&self, _store: &mut LayoutStore) {}

    /// Take the page's layout state back from `store` on start. Missing keys keep defaults.
    fn load_layout(&mut self, _store: &LayoutStore) {}
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
        Box::new(EditableListPage::default()),
        Box::new(FileDialogPage::default()),
        Box::new(FindPage::default()),
        Box::new(HtmlPage::default()),
        Box::new(IconsPage::default()),
        Box::new(VariableListPage::default()),
        Box::new(MarkdownPage::default()),
        Box::new(MenuBarPage::default()),
        Box::new(PagerPage::default()),
        Box::new(PiecesPage::default()),
        Box::new(ReorderPage::default()),
        Box::new(RibbonPage::default()),
        Box::new(RichTextEditorPage::default()),
        Box::new(ShortcutsPage::default()),
        Box::new(SkeletonPage),
        Box::new(SurfacesPage::default()),
        Box::new(TabStripPage::default()),
        Box::new(TerminalPage::default()),
        Box::new(TokensPage),
        Box::new(TreeViewPage::default()),
        Box::new(TypographyPage),
        Box::new(UrlStatePage::default()),
    ]
}

#[cfg(test)]
mod tests {
    use super::{LayoutStore, pages};

    #[test]
    fn pages_are_sorted_by_name() {
        let names: Vec<_> = pages().iter().map(|page| page.name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "gallery pages must be sorted by name");
    }

    /// Issue #118: every page's layout survives a save and a fresh start.
    #[test]
    fn page_layouts_survive_a_restart() {
        let first = pages();
        let mut store = LayoutStore::new();
        for page in &first {
            page.save_layout(&mut store);
        }
        assert!(store.contains("docking_tree"));
        assert!(store.contains("data_grid"));
        // A user collapsed the Explorer and switched the grid to compact rows.
        let collapsed = fluent2_egui::surfaces::SideDockState {
            open: false,
            size: 300.0,
        };
        store.put("docking_explorer", &collapsed);
        store.put(
            "data_grid_size",
            &fluent2_egui::data::data_grid::DataGridSize::ExtraSmall,
        );
        let saved = store.to_json();

        let loaded = LayoutStore::from_json(&saved);
        let mut second = pages();
        for page in &mut second {
            page.load_layout(&loaded);
        }
        let mut again = LayoutStore::new();
        for page in &second {
            page.save_layout(&mut again);
        }
        assert_eq!(again.to_json(), saved);
    }
}
