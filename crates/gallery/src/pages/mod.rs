//! Gallery pages.
//!
//! Later items add a page by adding one file in `crates/gallery/src/pages/` and one line in
//! [`pages`] below. Nothing else in the gallery needs to change.

mod icons;
mod tokens;
mod typography;

pub use icons::IconsPage;
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
    ]
}
