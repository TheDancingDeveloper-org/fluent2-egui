//! URL state: the tab and filter live in the URL fragment, so a view can be bookmarked and
//! shared on the web build (issue #119).

use fluent2_egui::platform::hash_state::{self, HashState};
use fluent2_egui::{SearchBox, TabList};

use super::Page;

/// Tab keys as they appear in the fragment, with the labels shown on the tabs.
const TABS: [(&str, &str); 3] = [("all", "All"), ("controls", "Controls"), ("surfaces", "Surfaces")];

/// Components listed on the page, each with the tab key it belongs to.
const ITEMS: [(&str, &str); 10] = [
    ("Button", "controls"),
    ("Checkbox", "controls"),
    ("Combobox", "controls"),
    ("Slider", "controls"),
    ("Switch", "controls"),
    ("Card", "surfaces"),
    ("Dialog", "surfaces"),
    ("Popover", "surfaces"),
    ("Tab list", "surfaces"),
    ("Toast", "surfaces"),
];

/// A filtered list whose tab and filter text round-trip through the URL fragment.
#[derive(Default)]
pub struct UrlStatePage {
    state: HashState,
    tab: usize,
    filter: String,
}

impl UrlStatePage {
    /// Take the tab and filter from a fragment. A missing or unknown value falls back to the
    /// default, so the whole view always matches the address bar.
    fn apply(&mut self, fragment: &str) {
        let pairs = hash_state::decode(fragment);
        self.tab = hash_state::value(&pairs, "tab")
            .and_then(|key| TABS.iter().position(|(tab_key, _)| *tab_key == key))
            .unwrap_or(0);
        self.filter = hash_state::value(&pairs, "q").unwrap_or_default().to_owned();
    }
}

impl Page for UrlStatePage {
    fn name(&self) -> &'static str {
        "URL state"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(fragment) = self.state.read_external() {
            self.apply(&fragment);
        }

        ui.heading("URL state");
        ui.label(if cfg!(target_arch = "wasm32") {
            "The tab and filter are in the address bar. Copy the URL to share this view."
        } else {
            "The web build keeps the tab and filter in the URL. This native build has no URL, \
             so the page keeps its defaults."
        });
        ui.add_space(8.0);

        TabList::new("url_state_tabs", &mut self.tab)
            .tab(TABS[0].1)
            .tab(TABS[1].1)
            .tab(TABS[2].1)
            .show(ui);
        ui.add_space(8.0);
        ui.add(
            SearchBox::new(&mut self.filter)
                .placeholder("Filter components")
                .label("Filter components"),
        );
        ui.add_space(8.0);

        let key = TABS.get(self.tab).map_or(TABS[0].0, |(key, _)| key);
        let needle = self.filter.to_lowercase();
        let mut shown = 0;
        for (name, category) in ITEMS {
            if (key == "all" || key == category) && name.to_lowercase().contains(&needle) {
                ui.label(name);
                shown += 1;
            }
        }
        if shown == 0 {
            ui.label("No components match.");
        }

        let mut pairs = vec![("tab", key)];
        if !self.filter.is_empty() {
            pairs.push(("q", self.filter.as_str()));
        }
        self.state.write(&hash_state::encode(pairs));
    }
}
