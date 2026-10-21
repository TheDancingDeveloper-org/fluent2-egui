//! Controls: every basic Fluent control in its main variants (WI-1930).
//!
//! Every control has an accessible name, from its text, a `.label`, or the Field it sits in, so
//! the page doubles as a screen-reader walk (WI-1846, WI-1851).

use fluent2_egui::controls::{
    Appearance, Avatar, Badge, Button, ButtonSize, CheckState, Checkbox, Combobox, CompoundButton, Divider,
    Dropdown, Field, Input, InteractionTag, Link, MenuButton, ProgressBar, RadioGroup, SearchBox, Slider,
    SpinButton, Spinner, SplitButton, Switch, Tag, TagAppearance, TagGroup, TagPicker, TagSize,
    TagSuggestion, Textarea, ToggleButton, ValidationState,
};
use fluent2_egui::surfaces::MenuItem;
use fluent2_icons::Icon;

use super::Page;

pub struct ControlsPage {
    bold: bool,
    italic: bool,
    terms: CheckState,
    all: CheckState,
    size: usize,
    wifi: bool,
    volume: f64,
    copies: f64,
    name: String,
    email: String,
    query: String,
    notes: String,
    colour: Option<usize>,
    font: String,
    status: String,
    recipients: Vec<String>,
    recipient_query: String,
    suggestions: Vec<TagSuggestion>,
    filters: Vec<String>,
}

impl Default for ControlsPage {
    fn default() -> Self {
        Self {
            bold: true,
            italic: false,
            terms: CheckState::Unchecked,
            all: CheckState::Mixed,
            size: 1,
            wifi: true,
            volume: 40.0,
            copies: 2.0,
            name: "Ada Lovelace".to_owned(),
            email: "ada@".to_owned(),
            query: String::new(),
            notes: String::new(),
            colour: Some(0),
            font: "Calibri".to_owned(),
            status: "Nothing clicked yet".to_owned(),
            recipients: vec!["ada@example.com".to_owned(), "not an address".to_owned()],
            recipient_query: String::new(),
            suggestions: Vec::new(),
            filters: vec![
                "Bug".to_owned(),
                "Docs".to_owned(),
                "UI".to_owned(),
                "Release".to_owned(),
            ],
        }
    }
}

impl Page for ControlsPage {
    fn name(&self) -> &'static str {
        "Basic controls"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Basic controls");
        ui.label(format!("Last action: {}", self.status));
        ui.add_space(8.0);

        ui.add(Divider::horizontal().text("Buttons"));
        ui.horizontal_wrapped(|ui| {
            for (label, appearance) in [
                ("Primary", Appearance::Primary),
                ("Secondary", Appearance::Secondary),
                ("Outline", Appearance::Outline),
                ("Subtle", Appearance::Subtle),
                ("Transparent", Appearance::Transparent),
            ] {
                if ui.add(Button::new(label).appearance(appearance)).clicked() {
                    self.status = format!("{label} button");
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (label, size) in [
                ("Small", ButtonSize::Small),
                ("Medium", ButtonSize::Medium),
                ("Large", ButtonSize::Large),
            ] {
                ui.add(Button::new(label).size(size).icon(Icon::Calendar));
            }
            if ui.add(Button::icon_only(Icon::Settings, "Settings")).clicked() {
                self.status = "Settings".to_owned();
            }
            ui.add(ToggleButton::new(&mut self.bold, "Bold"));
            ui.add(ToggleButton::icon_only(
                &mut self.italic,
                Icon::TextItalic,
                "Italic",
            ));
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(
                CompoundButton::new("New meeting")
                    .secondary_content("Starts now")
                    .icon(Icon::Calendar),
            );
            let split = SplitButton::new("Save").icon(Icon::Save).show(ui, |ui| {
                ui.add(MenuItem::new("Save as"));
                ui.add(MenuItem::new("Save a copy"));
            });
            if split.main.clicked() {
                self.status = "Save".to_owned();
            }
            MenuButton::new("Sort").show(ui, |ui| {
                ui.add(MenuItem::new("Name"));
                ui.add(MenuItem::new("Date modified"));
            });
            ui.add(Link::new("Learn more about buttons"));
        });

        ui.add_space(8.0);
        ui.add(Divider::horizontal().text("Tags"));
        ui.horizontal_wrapped(|ui| {
            for appearance in [
                TagAppearance::Filled,
                TagAppearance::Outline,
                TagAppearance::Brand,
            ] {
                for size in [TagSize::ExtraSmall, TagSize::Small, TagSize::Medium] {
                    ui.add(
                        Tag::new(format!("{appearance:?}"))
                            .appearance(appearance)
                            .size(size),
                    );
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(Tag::new("Calendar").icon(Icon::Calendar));
            ui.add(
                Tag::new("Ada Lovelace")
                    .avatar(Avatar::new("Ada Lovelace"))
                    .secondary("Owner"),
            );
            let open = InteractionTag::new(Tag::new("Status: open").dismissible(true))
                .selected(true)
                .show(ui);
            if open.clicked {
                self.status = "Status filter".to_owned();
            }
            if open.dismissed {
                self.status = "Status filter removed".to_owned();
            }
        });
        let tags = self.filters.iter().map(|f| Tag::new(f.clone())).collect();
        let out = TagGroup::new("controls_filters", tags)
            .label("Filters")
            .interactive(true)
            .dismissible(true)
            .size(TagSize::Small)
            .show(ui);
        if let Some(index) = out.clicked
            && let Some(name) = self.filters.get(index)
        {
            self.status = format!("{name} filter");
        }
        if let Some(index) = out.dismissed
            && index < self.filters.len()
        {
            self.filters.remove(index);
        }

        ui.add_space(8.0);
        ui.add(Divider::horizontal().text("Selection"));
        ui.horizontal_wrapped(|ui| {
            ui.add(Checkbox::new(&mut self.terms, "Agree to the terms"));
            ui.add(Checkbox::new(&mut self.all, "Select all (mixed)"));
            ui.add(Switch::new(&mut self.wifi, "Wi-Fi"));
        });
        Field::new("Size").show(ui, |ui| {
            RadioGroup::new(&mut self.size)
                .option("Small")
                .option("Medium")
                .option("Large")
                .horizontal()
                .show(ui);
        });

        ui.add_space(8.0);
        ui.add(Divider::horizontal().text("Input"));
        ui.scope(|ui| {
            ui.set_max_width(420.0);
            Field::new("Name").required(true).show(ui, |ui| {
                ui.add(Input::new(&mut self.name).label("Name"));
            });
            let email_state = if self.email.contains('.') {
                ValidationState::Success("Looks good".to_owned())
            } else {
                ValidationState::Error("Enter a full address, like ada@example.com".to_owned())
            };
            Field::new("Email")
                .hint("Used for sign-in")
                .validation(email_state)
                .show(ui, |ui| {
                    ui.add(Input::new(&mut self.email).label("Email"));
                });
            Field::new("To")
                .hint("Type a name, or paste addresses separated by , or ;")
                .show(ui, |ui| {
                    let valid = |tag: &str| tag.contains('@');
                    let out = TagPicker::new("controls_to", &mut self.recipients, &mut self.recipient_query)
                        .suggestions(&self.suggestions)
                        .validate(&valid)
                        .placeholder("Add people")
                        .show(ui);
                    if out.query_changed {
                        // A real app would fetch these; the gallery filters a fixed list.
                        self.suggestions = contacts(&self.recipient_query);
                    }
                });
            Field::new("Search").show(ui, |ui| {
                ui.add(
                    SearchBox::new(&mut self.query)
                        .placeholder("Search files")
                        .label("Search"),
                );
            });
            Field::new("Notes").show(ui, |ui| {
                ui.add(Textarea::new(&mut self.notes).label("Notes"));
            });
            Field::new("Volume").show(ui, |ui| {
                ui.add(Slider::new(&mut self.volume, 0.0..=100.0).step(Some(5.0)));
            });
            Field::new("Copies").show(ui, |ui| {
                ui.add(SpinButton::new(&mut self.copies).range(1.0..=99.0));
            });
            Field::new("Colour").show(ui, |ui| {
                ui.add(Dropdown::new(
                    "controls_colour",
                    &mut self.colour,
                    &["Red", "Green", "Blue"],
                ));
            });
            Field::new("Font").show(ui, |ui| {
                ui.add(Combobox::new(
                    "controls_font",
                    &mut self.font,
                    &["Arial", "Calibri", "Segoe UI"],
                ));
            });
        });

        ui.add_space(8.0);
        ui.add(Divider::horizontal().text("Status"));
        ui.horizontal_wrapped(|ui| {
            ui.add(Badge::new("New"));
            ui.add(Badge::new("12"));
            ui.add(Spinner::new().label("Loading"));
        });
        ui.scope(|ui| {
            ui.set_max_width(420.0);
            ui.add(ProgressBar::new(Some(0.4)));
            ui.add_space(4.0);
            ui.add(ProgressBar::new(None));
        });
    }
}

/// The gallery's stand-in for a contacts lookup.
fn contacts(query: &str) -> Vec<TagSuggestion> {
    const PEOPLE: [(&str, &str); 5] = [
        ("Ada Lovelace", "ada@example.com"),
        ("Alan Turing", "alan@example.com"),
        ("Grace Hopper", "grace@example.com"),
        ("Edsger Dijkstra", "edsger@example.com"),
        ("Barbara Liskov", "barbara@example.com"),
    ];
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    PEOPLE
        .iter()
        .filter(|(name, address)| name.to_lowercase().contains(&query) || address.contains(&query))
        .map(|(name, address)| TagSuggestion::new(*address).label(*name))
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable, by};
    use fluent2_egui::Theme;

    use super::*;

    /// Tab through the page and collect (role, name) of each stop until focus comes back round.
    pub(crate) fn tab_stops(page: impl Page + 'static) -> Vec<(egui::accesskit::Role, String)> {
        let mut page = page;
        let mut harness = Harness::builder()
            .with_size(egui::vec2(900.0, 1800.0))
            .build_ui(move |ui| {
                fluent2_egui::install(ui.ctx(), &Theme::default());
                page.ui(ui);
            });
        harness.run_steps(2);
        let mut stops = Vec::new();
        for _ in 0..80 {
            harness.key_press(egui::Key::Tab);
            harness.run_steps(2);
            let Some(stop) = harness
                .query_all(by().predicate(|node| node.is_focused()))
                .map(|node| {
                    let node = node.accesskit_node();
                    (node.role(), node.label().unwrap_or_default())
                })
                .next()
            else {
                continue;
            };
            if stops.first() == Some(&stop) {
                break;
            }
            stops.push(stop);
        }
        stops
    }

    #[test]
    fn every_tab_stop_has_a_name() {
        let stops = tab_stops(ControlsPage::default());
        assert!(
            stops.len() > 20,
            "the page has its controls in the Tab order: {stops:#?}"
        );
        for (role, label) in &stops {
            assert!(
                !label.trim().is_empty(),
                "{role:?} stop has no name; stops: {stops:#?}"
            );
        }
        for name in [
            "Primary", "Settings", "Bold", "Italic", "Wi-Fi", "Volume", "Copies", "Email",
        ] {
            assert!(
                stops.iter().any(|(_, label)| label.contains(name)),
                "Tab never reached {name:?}; stops: {stops:#?}"
            );
        }
    }
}
