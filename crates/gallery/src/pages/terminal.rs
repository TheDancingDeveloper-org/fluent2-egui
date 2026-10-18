//! TerminalView: a scripted session fed into the VT grid. No PTY, so it runs the same native
//! and on the web; a real app feeds PTY or SSH output to `TerminalGrid::process`.

use fluent2_egui::controls::{Button, SearchBox};
use fluent2_egui::data::terminal::{TerminalGrid, TerminalState, TerminalView};

use super::Page;

pub struct TerminalPage {
    grid: TerminalGrid,
    state: TerminalState,
    command: String,
    status: String,
}

impl Default for TerminalPage {
    fn default() -> Self {
        let mut grid = TerminalGrid::new(80, 24);
        grid.process(&banner());
        Self {
            grid,
            state: TerminalState::default(),
            command: String::new(),
            status: String::new(),
        }
    }
}

/// The welcome text: ANSI colours, attributes, the 256-colour cube and a link.
fn banner() -> Vec<u8> {
    let mut out = String::new();
    out.push_str("\x1b]0;gallery shell\x07");
    out.push_str("\x1b[1mTerminalView\x1b[22m renders a VT grid fed by the app.\r\n\r\n");
    out.push_str("ANSI:   ");
    for i in 0..8 {
        out.push_str(&format!("\x1b[{}m color{i} ", 30 + i));
    }
    out.push_str("\x1b[0m\r\nBright: ");
    for i in 0..8 {
        out.push_str(&format!("\x1b[{}m color{} ", 90 + i, i + 8));
    }
    out.push_str("\x1b[0m\r\nStyles: \x1b[1mbold\x1b[0m \x1b[3mitalic\x1b[0m \x1b[4munderline\x1b[0m \x1b[7minverse\x1b[0m\r\n");
    out.push_str("256:    ");
    for i in (16..232).step_by(6) {
        out.push_str(&format!("\x1b[48;5;{i}m "));
    }
    out.push_str("\x1b[0m\r\nRGB:    ");
    for i in 0..36 {
        let v = i * 7;
        out.push_str(&format!("\x1b[48;2;{v};{};{}m ", 255 - v, 128));
    }
    out.push_str("\x1b[0m\r\n\r\nCtrl+click a link: https://github.com/thedancingdeveloper/fluent2-egui\r\n");
    out.push_str("Drag to select, Ctrl+C to copy, Shift+PageUp for scrollback.\r\n\r\n");
    for i in 1..=40 {
        out.push_str(&format!(
            "\x1b[2m[{i:02}]\x1b[0m build step {i} \x1b[32mok\x1b[0m\r\n"
        ));
    }
    out.push_str(PROMPT);
    out.into_bytes()
}

const PROMPT: &str = "\x1b[36m~/gallery\x1b[0m $ ";

/// Output of the scripted commands.
fn run(command: &str) -> String {
    match command.trim() {
        "" => String::new(),
        "help" => "commands: help, ls, colors, clear, date\r\n".into(),
        "ls" => {
            "\x1b[1;34msrc\x1b[0m  \x1b[1;34mtests\x1b[0m  Cargo.toml  \x1b[32mrun.sh\x1b[0m  README.md\r\n"
                .into()
        }
        "colors" => {
            (0..16)
                .map(|i| format!("\x1b[48;5;{i}m  {i:>2}  "))
                .collect::<String>()
                + "\x1b[0m\r\n"
        }
        "date" => "Sun Oct 18 12:00:00 2026\r\n".into(),
        other => format!("\x1b[31m{other}: command not found\x1b[0m\r\n"),
    }
}

impl Page for TerminalPage {
    fn name(&self) -> &'static str {
        "Terminal"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("TerminalView");
        ui.horizontal(|ui| {
            let run_it = ui
                .add(
                    egui::TextEdit::singleline(&mut self.command)
                        .hint_text("Command (help, ls, colors, clear)"),
                )
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.add(Button::new("Run")).clicked() || run_it {
                let command = std::mem::take(&mut self.command);
                if command.trim() == "clear" {
                    self.grid.process(b"\x1b[H\x1b[2J");
                } else {
                    self.grid
                        .process(format!("{command}\r\n{}", run(&command)).as_bytes());
                }
                self.grid.process(PROMPT.as_bytes());
                self.grid.scroll_to_bottom();
            }
            ui.separator();
            ui.add(SearchBox::new(&mut self.state.search).placeholder("Find"));
            let count = self.state.matches(&self.grid).len();
            if ui.add(Button::new("Previous")).clicked() {
                self.state.find_next(&mut self.grid, false);
            }
            if ui.add(Button::new("Next")).clicked() {
                self.state.find_next(&mut self.grid, true);
            }
            if !self.state.search.trim().is_empty() {
                ui.label(format!("{count} matches"));
            }
        });
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        ui.add_space(8.0);
        let response = TerminalView::new("gallery_terminal")
            .label("Gallery terminal")
            .show(ui, &mut self.grid, &mut self.state);
        let (cols, rows) = response.fit;
        if (cols, rows) != (self.grid.cols(), self.grid.rows()) {
            self.grid.resize(cols, rows);
        }
        if let Some(url) = response.link_clicked {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&url));
            self.status = format!("Opened {url}");
        }
        if let Some(text) = response.copied {
            self.status = format!("Copied {} characters", text.chars().count());
        }
        if let Some(title) = self.grid.take_title() {
            self.status = format!("Title: {title}");
        }
    }
}
