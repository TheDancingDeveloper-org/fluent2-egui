//! Sample editor: every piece of the Office-style chrome assembled into one app (WI-1570).
//!
//! Run with `cargo run -p gallery --example editor`. The window is undecorated: a TitleBar with
//! the app icon, the Quick Access Toolbar, a centred title and a search box; resize edges; a
//! ribbon with File, Home, Insert and View tabs plus a "Picture format" contextual tab that
//! appears while the placeholder picture in the document is selected; the backstage behind
//! File; a text document; and a status bar with the word count and zoom.
//!
//! The chrome only reports what happened. Every ribbon event and QAT click becomes a
//! [`Command`], and [`Editor::run`] dispatches it with one `match`: command handling lives in
//! the app, not in the library.
//!
//! Labels are generic. Not affiliated with or endorsed by Microsoft.

#![warn(missing_docs)]

use eframe::egui;
use fluent2_egui::office::backstage::{Backstage, BackstageState};
use fluent2_egui::office::keytips::{self, Region};
use fluent2_egui::office::qat::{Qat, QatItem, QatState};
use fluent2_egui::office::ribbon::{
    ContextualAccent, FileTab, GroupBuilder, Ribbon, RibbonCheckbox, RibbonEventKind, RibbonLarge,
    RibbonSmall, RibbonState, RibbonTab, RibbonToggle, TabBuilder,
};
use fluent2_egui::office::status_bar::StatusBar;
use fluent2_egui::office::window_edges::window_resize_edges;
use fluent2_egui::office::{TitleBar, window_command};
use fluent2_egui::{Icon, IconName, SearchBox, Theme, install};

/// The commands the editor understands. The chrome's keys map onto these in [`Command::from_key`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the backstage (File tab).
    OpenFile,
    /// Save the document.
    Save,
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,
    /// Paste from the clipboard.
    Paste,
    /// Cut the selection.
    Cut,
    /// Copy the selection.
    Copy,
    /// Toggle bold.
    Bold,
    /// Toggle italic.
    Italic,
    /// Toggle underline.
    Underline,
    /// Insert a placeholder picture.
    InsertPicture,
    /// Insert a table.
    InsertTable,
    /// Insert a link.
    InsertLink,
    /// Toggle the ruler.
    Ruler,
    /// Zoom to 100 %.
    ZoomReset,
    /// Zoom in by 10 points.
    ZoomIn,
    /// Remove the selected picture.
    DeletePicture,
    /// Deselect the picture.
    DeselectPicture,
}

impl Command {
    /// The command bound to a ribbon or QAT key, if any.
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "file" => Self::OpenFile,
            "save" => Self::Save,
            "undo" => Self::Undo,
            "redo" => Self::Redo,
            "paste" => Self::Paste,
            "cut" => Self::Cut,
            "copy" => Self::Copy,
            "bold" => Self::Bold,
            "italic" => Self::Italic,
            "underline" => Self::Underline,
            "picture" => Self::InsertPicture,
            "table" => Self::InsertTable,
            "link" => Self::InsertLink,
            "ruler" => Self::Ruler,
            "zoom_reset" => Self::ZoomReset,
            "zoom_in" => Self::ZoomIn,
            "picture_delete" => Self::DeletePicture,
            "picture_deselect" => Self::DeselectPicture,
            _ => return None,
        })
    }
}

/// Commands offered in the Quick Access Toolbar.
const QAT_ITEMS: [QatItem<'static>; 3] = [
    QatItem {
        key: "save",
        icon: IconName::Save,
        label: "Save",
        tooltip: "Save (Ctrl+S)",
        keytip: Some("1"),
    },
    QatItem {
        key: "undo",
        icon: IconName::ArrowCounterclockwise,
        label: "Undo",
        tooltip: "Undo (Ctrl+Z)",
        keytip: Some("2"),
    },
    QatItem {
        key: "redo",
        icon: IconName::ArrowClockwise,
        label: "Redo",
        tooltip: "Redo (Ctrl+Y)",
        keytip: Some("3"),
    },
];

/// The editor's whole state. The chrome widgets hold none of it.
pub struct Editor {
    /// Ribbon display state.
    pub ribbon: RibbonState,
    /// Quick Access Toolbar items and position.
    pub qat: QatState,
    /// Backstage open flag and page.
    pub backstage: BackstageState,
    /// The document text.
    pub text: String,
    /// The title bar's search text.
    pub search: String,
    /// Bold formatting.
    pub bold: bool,
    /// Italic formatting.
    pub italic: bool,
    /// Underline formatting.
    pub underline: bool,
    /// Whether the ruler is shown.
    pub ruler: bool,
    /// Whether the document holds the placeholder picture.
    pub has_picture: bool,
    /// Whether the placeholder picture is selected; shows the contextual tab.
    pub picture_selected: bool,
    /// Zoom in percent.
    pub zoom: f32,
    /// Whether the document has unsaved changes.
    pub dirty: bool,
    /// The last command run, shown in the status bar.
    pub last: Option<Command>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            ribbon: RibbonState::default(),
            qat: QatState {
                items: QAT_ITEMS.iter().map(|item| item.key).collect(),
                ..QatState::default()
            },
            backstage: BackstageState::default(),
            text: "Start typing your document here.".to_owned(),
            search: String::new(),
            bold: false,
            italic: false,
            underline: false,
            ruler: false,
            has_picture: true,
            picture_selected: false,
            zoom: 100.0,
            dirty: false,
            last: None,
        }
    }
}

impl Editor {
    /// Run one command. This is the app's single dispatch point.
    pub fn run(&mut self, command: Command) {
        match command {
            Command::OpenFile => self.backstage.open = true,
            Command::Save => self.dirty = false,
            Command::Undo | Command::Redo | Command::Paste | Command::Cut | Command::Copy => {}
            Command::Bold => self.bold = !self.bold,
            Command::Italic => self.italic = !self.italic,
            Command::Underline => self.underline = !self.underline,
            Command::InsertPicture => {
                self.has_picture = true;
                self.picture_selected = true;
            }
            Command::InsertTable | Command::InsertLink => self.dirty = true,
            Command::Ruler => self.ruler = !self.ruler,
            Command::ZoomReset => self.zoom = 100.0,
            Command::ZoomIn => self.zoom = (self.zoom + 10.0).min(400.0),
            Command::DeletePicture => {
                self.has_picture = false;
                self.picture_selected = false;
            }
            Command::DeselectPicture => self.picture_selected = false,
        }
        self.last = Some(command);
    }

    /// Number of words in the document.
    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }

    /// Draw the whole window into `ui`.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        window_resize_edges(&ctx);
        let (focused, maximized) = ctx.input(|input| {
            let viewport = input.viewport();
            (viewport.focused, viewport.maximized.unwrap_or(false))
        });
        let mut commands = Vec::new();

        self.title_bar(ui, focused, maximized, &mut commands);
        if self.backstage.open {
            self.backstage(ui);
        } else {
            self.ribbon(ui, &mut commands);
            let status_height = Theme::get(&ctx).chrome.status_bar_height;
            let document_height = (ui.available_height() - status_height).max(0.0);
            ui.allocate_ui(egui::vec2(ui.available_width(), document_height), |ui| {
                self.document(ui, &mut commands);
            });
            self.status_bar(ui, &mut commands);
        }

        for command in commands {
            self.run(command);
        }
    }

    fn title_bar(
        &mut self,
        ui: &mut egui::Ui,
        focused: Option<bool>,
        maximized: bool,
        commands: &mut Vec<Command>,
    ) {
        let qat = &mut self.qat;
        let search = &mut self.search;
        let response = TitleBar::new("editor_title", "Document1 - Editor")
            .maximized(maximized)
            .focused(focused)
            .with_search(true)
            .show(ui, |slots| {
                slots.app_icon(Icon::new(IconName::Document));
                slots.left(|ui| {
                    if let Some(key) = Qat::new("editor_qat", qat, &QAT_ITEMS).show(ui).clicked {
                        commands.extend(Command::from_key(key));
                    }
                });
                slots.right(|ui| {
                    ui.add(
                        SearchBox::new(search)
                            .placeholder("Search")
                            .label("Search")
                            .desired_width(200.0),
                    );
                });
            });
        if let Some(action) = response.action {
            ui.ctx().send_viewport_cmd(window_command(action, maximized));
        }
    }

    fn ribbon(&mut self, ui: &mut egui::Ui, commands: &mut Vec<Command>) {
        // The ribbon binds copies; the dispatch in `run` changes the real state.
        let mut bold = self.bold;
        let mut italic = self.italic;
        let mut underline = self.underline;
        let mut ruler = self.ruler;
        let response = Ribbon::new("editor_ribbon")
            .file_tab(FileTab::new("file", "File").keytip("F"))
            .tab(home(&mut bold, &mut italic, &mut underline))
            .tab(insert())
            .tab(view(&mut ruler))
            .tab(picture_format(self.picture_selected))
            .show(ui, &mut self.ribbon);
        for event in response.events {
            if matches!(event.kind, RibbonEventKind::Clicked | RibbonEventKind::Changed) {
                commands.extend(Command::from_key(event.key));
            }
        }
    }

    fn backstage(&mut self, ui: &mut egui::Ui) {
        let words = self.word_count();
        Backstage::new(&mut self.backstage)
            .page("info", "Info", move |ui| {
                ui.label(format!("Document1: {words} words."));
            })
            .page("new", "New", |ui| {
                ui.label("Blank document");
            })
            .page("open", "Open", |ui| {
                ui.label("No recent documents.");
            })
            .bottom_page("options", "Options", |ui| {
                ui.label("Editor options");
            })
            .show(ui);
    }

    fn document(&mut self, ui: &mut egui::Ui, commands: &mut Vec<Command>) {
        let rect = ui.available_rect_before_wrap();
        ui.add_space(8.0);
        if self.ruler {
            ui.label("Ruler");
        }
        if self.has_picture {
            let picture = ui.selectable_label(self.picture_selected, "Placeholder picture");
            if picture.clicked() {
                self.picture_selected = !self.picture_selected;
            }
        }
        let font = egui::FontId::proportional(14.0 * self.zoom / 100.0);
        let mut job_style = egui::TextFormat::simple(font, ui.visuals().text_color());
        job_style.italics = self.italic;
        if self.underline {
            job_style.underline = egui::Stroke::new(1.0, ui.visuals().text_color());
        }
        let bold = self.bold;
        let mut layouter = move |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut format = job_style.clone();
            if bold {
                // egui has no bold weight in its default fonts; a stronger colour stands in.
                format.color = ui.visuals().strong_text_color();
            }
            let mut job = egui::text::LayoutJob::single_section(text.as_str().to_owned(), format);
            job.wrap.max_width = wrap_width;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let editor = ui.add_sized(
            ui.available_size(),
            egui::TextEdit::multiline(&mut self.text)
                .id_salt("editor_document")
                .layouter(&mut layouter),
        );
        if editor.changed() {
            self.dirty = true;
        }
        if editor.clicked() && self.picture_selected {
            commands.push(Command::DeselectPicture);
        }
        keytips::mark_region(ui.ctx(), Region::Content, rect, Some(editor.id));
    }

    fn status_bar(&mut self, ui: &mut egui::Ui, commands: &mut Vec<Command>) {
        let words = self.word_count();
        let bold = self.bold;
        let dirty = self.dirty;
        let response = StatusBar::new("editor_status")
            .left(move |ui| {
                ui.label(format!("{words} words"));
                ui.label(if dirty { "Unsaved changes" } else { "Saved" });
                if bold {
                    ui.label("Bold");
                }
            })
            .zoom(&mut self.zoom)
            .show(ui);
        if response.zoom_reset {
            commands.push(Command::ZoomReset);
        }
    }
}

/// Home: Clipboard, Font, Paragraph and Editing groups.
fn home<'a>(bold: &'a mut bool, italic: &'a mut bool, underline: &'a mut bool) -> RibbonTab<'a> {
    RibbonTab::new("home", "Home", move |tab: &mut TabBuilder<'a>| {
        tab.group("clipboard", "Clipboard", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste").keytip("V"));
            group.add(RibbonSmall::new("cut", IconName::Cut, "Cut").keytip("X"));
            group.add(RibbonSmall::new("copy", IconName::Copy, "Copy").keytip("C"));
        });
        tab.group("font", "Font", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonToggle::new("bold", IconName::TextBold, "Bold", bold).keytip("1"));
            group.add(RibbonToggle::new("italic", IconName::TextItalic, "Italic", italic).keytip("2"));
            group.add(
                RibbonToggle::new("underline", IconName::TextUnderline, "Underline", underline).keytip("3"),
            );
        });
        tab.group("paragraph", "Paragraph", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new("align", IconName::Navigation, "Align text"));
            group.add(RibbonSmall::new("bullets", IconName::MoreHorizontal, "Bullets"));
            group.add(RibbonSmall::new("numbering", IconName::ArrowSort, "Numbering"));
        });
        tab.group("styles", "Styles", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new(
                "style_normal",
                IconName::Document,
                "Normal style",
            ));
            group.add(RibbonLarge::new(
                "style_heading",
                IconName::Bookmark,
                "Heading style",
            ));
            group.add(RibbonLarge::new("style_title", IconName::Star, "Title style"));
        });
        tab.group("editing", "Editing", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new("find", IconName::Search, "Find"));
            group.add(RibbonSmall::new("replace", IconName::Edit, "Replace"));
            group.add(RibbonSmall::new(
                "select_all",
                IconName::SquareMultiple,
                "Select all",
            ));
        });
    })
    .keytip("H")
}

/// Insert: Illustrations, Tables and Links groups.
fn insert<'a>() -> RibbonTab<'a> {
    RibbonTab::new("insert", "Insert", |tab: &mut TabBuilder<'a>| {
        tab.group(
            "illustrations",
            "Illustrations",
            |group: &mut GroupBuilder<'a>| {
                group.add(RibbonLarge::new("picture", IconName::Image, "Picture").keytip("P"));
            },
        );
        tab.group("tables", "Tables", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new("table", IconName::Table, "Table").keytip("T"));
        });
        tab.group("links", "Links", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new("link", IconName::Link, "Link").keytip("K"));
        });
    })
    .keytip("N")
}

/// View: Show and Zoom groups.
fn view<'a>(ruler: &'a mut bool) -> RibbonTab<'a> {
    RibbonTab::new("view", "View", move |tab: &mut TabBuilder<'a>| {
        tab.group("show", "Show", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonCheckbox::new("ruler", "Ruler", ruler));
        });
        tab.group("zoom", "Zoom", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new("zoom_reset", IconName::Search, "100%"));
            group.add(RibbonLarge::new("zoom_in", IconName::ZoomIn, "Zoom in"));
        });
    })
    .keytip("W")
}

/// Picture format: the contextual tab, shown while the picture is selected.
fn picture_format<'a>(visible: bool) -> RibbonTab<'a> {
    RibbonTab::new("picture_format", "Picture format", |tab: &mut TabBuilder<'a>| {
        tab.group("arrange", "Arrange", |group: &mut GroupBuilder<'a>| {
            group.add(RibbonLarge::new(
                "picture_deselect",
                IconName::Dismiss,
                "Deselect",
            ));
            group.add(RibbonLarge::new(
                "picture_delete",
                IconName::Delete,
                "Delete picture",
            ));
        });
    })
    .contextual("picture", "Picture tools", ContextualAccent::Brand, visible)
    .keytip("JP")
}

/// The eframe wrapper around [`Editor`].
struct EditorApp(Editor);

impl eframe::App for EditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.0.ui(ui));
    }
}

#[allow(dead_code)]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "fluent2-egui editor sample",
        options,
        Box::new(|cc| {
            install(&cc.egui_ctx, &Theme::default());
            Ok(Box::new(EditorApp(Editor::default())))
        }),
    )
}
