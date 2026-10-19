//! MarkdownView: an editable source beside its rendering. Links go through a policy that
//! accepts web links and paths under `docs/`; the page only reports what was clicked.

use fluent2_egui::controls::Textarea;
use fluent2_egui::data::markdown::{Document, LinkTarget, MarkdownView, default_link_policy};

use super::Page;

const SAMPLE: &str = "# MarkdownView\n\
\n\
CommonMark with **strong**, *emphasis*, ~~strikethrough~~ and `inline code`, painted with the \
Fluent type ramp.\n\
\n\
## Links\n\
\n\
A [web link](https://fluent2.microsoft.design), a [file link](docs/ARCHITECTURE.md) and a \
[blocked link](javascript:alert(1)) that stays plain text.\n\
\n\
## Lists\n\
\n\
- Bullets\n  - nested\n- [x] a done task\n- [ ] an open task\n\
\n\
1. First\n2. Second\n\
\n\
> A block quote.\n\
\n\
## Table\n\
\n\
| Component | Feature | Status |\n|:--|:--:|--:|\n| TerminalView | `terminal` | landed |\n| MarkdownView | `markdown` | new |\n\
\n\
## Code\n\
\n\
```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n";

pub struct MarkdownPage {
    source: String,
    doc: Document,
    status: String,
}

impl Default for MarkdownPage {
    fn default() -> Self {
        Self {
            source: SAMPLE.to_owned(),
            doc: Document::parse(SAMPLE),
            status: String::new(),
        }
    }
}

fn policy(href: &str) -> LinkTarget {
    if href.starts_with("docs/") && !href.contains("..") {
        LinkTarget::File(href.to_owned())
    } else {
        default_link_policy(href)
    }
}

impl Page for MarkdownPage {
    fn name(&self) -> &'static str {
        "Markdown"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |cols| {
            let [left, right] = cols else { return };
            if left.add(Textarea::new(&mut self.source).rows(30)).changed() {
                self.doc = Document::parse(&self.source);
            }
            egui::ScrollArea::vertical()
                .id_salt("markdown_view")
                .show(right, |ui| {
                    let out = MarkdownView::new("gallery_markdown", &self.doc)
                        .link_policy(&policy)
                        .show(ui);
                    if let Some(click) = out.clicked {
                        self.status = format!("Clicked {:?}", click.target);
                    }
                    if !self.status.is_empty() {
                        ui.separator();
                        ui.label(&self.status);
                    }
                });
        });
    }
}
