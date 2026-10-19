//! CodeView: read-only highlighted samples in each supported language.

use fluent2_egui::controls::Button;
use fluent2_egui::data::code::{CodeState, CodeView};

use super::Page;

const SAMPLES: [(&str, &str, &str); 6] = [
    (
        "Rust",
        "rust",
        "/// Sum the even numbers.\n#[must_use]\npub fn even_sum(values: &[i64]) -> i64 {\n\tvalues.iter().filter(|v| *v % 2 == 0).sum()\n}\n\nfn main() {\n\tlet total = even_sum(&[1, 2, 3, 4]);\n\tprintln!(\"total = {total}\");\n}\n",
    ),
    (
        "TypeScript",
        "ts",
        "// Fetch a work item.\nexport async function getItem(id: string): Promise<Item> {\n  const res = await fetch(`/api/work/${id}`);\n  if (!res.ok) throw new Error(\"not found\");\n  return res.json();\n}\n",
    ),
    (
        "Python",
        "python",
        "from dataclasses import dataclass\n\n@dataclass\nclass Point:\n    x: float = 0.0\n    y: float = 0.0\n\n    def norm(self) -> float:\n        return (self.x ** 2 + self.y ** 2) ** 0.5  # Euclidean\n",
    ),
    (
        "JSON",
        "json",
        "{\n  \"name\": \"fluent2-egui\",\n  \"version\": \"0.1.0\",\n  \"features\": [\"highlight\", \"terminal\"],\n  \"private\": false,\n  \"license\": null\n}\n",
    ),
    (
        "YAML",
        "yaml",
        "# CI matrix\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v5\n      - run: cargo test --workspace\n    timeout-minutes: 30\n",
    ),
    (
        "Diff",
        "diff",
        "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,4 +1,4 @@\n pub fn greet() -> &'static str {\n-    \"hello\"\n+    \"hello, world\"\n }\n",
    ),
];

#[derive(Default)]
pub struct CodeViewPage {
    sample: usize,
    hide_line_numbers: bool,
    state: CodeState,
    status: String,
    long: Option<String>,
}

impl Page for CodeViewPage {
    fn name(&self) -> &'static str {
        "Code view"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("CodeView");
        ui.horizontal_wrapped(|ui| {
            for (i, (name, _, _)) in SAMPLES.iter().enumerate() {
                if ui
                    .selectable_label(self.sample == i && self.long.is_none(), *name)
                    .clicked()
                {
                    self.sample = i;
                    self.long = None;
                }
            }
            if ui.add(Button::new("10 000 lines")).clicked() {
                self.long = Some(
                    (0..10_000)
                        .map(|i| format!("let value_{i}: u32 = {i}; // line {}\n", i + 1))
                        .collect(),
                );
            }
            ui.separator();
            ui.checkbox(&mut self.hide_line_numbers, "Hide line numbers");
        });
        ui.label("Drag to select, Ctrl+A to select all, Ctrl+C to copy.");
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        ui.add_space(8.0);
        let (name, language, text) = match &self.long {
            Some(text) => ("Long file", "rust", text.as_str()),
            None => SAMPLES.get(self.sample).copied().unwrap_or(SAMPLES[0]),
        };
        let response = CodeView::new("gallery_code")
            .language(language)
            .label(format!("{name} sample"))
            .line_numbers(!self.hide_line_numbers)
            .show(ui, text, &mut self.state);
        if let Some(text) = response.copied {
            self.status = format!("Copied {} characters", text.chars().count());
        }
    }
}
