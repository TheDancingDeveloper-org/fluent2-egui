//! The shared syntax highlighter: source text in, per-line token spans out. Pure (no egui
//! state), so CodeView, a diff view and a markdown view can all use it and tests can check it
//! without a frame.
//!
//! Built on `syntect` (MIT) with its bundled syntaxes and the pure-Rust `fancy-regex` engine,
//! so it builds for wasm with no C toolchain. See `docs/specs/code-view.md` for why this is
//! syntect and not tree-sitter.
//!
//! syntect themes map TextMate scopes to colours. Here a private theme maps scopes to
//! [`SyntaxToken`] kinds instead (the kind's index is stored in the red channel), and
//! [`syntax_color`] turns a kind into a Fluent palette colour. Scope matching, including
//! "more specific selector wins", stays syntect's.

use std::ops::Range;
use std::str::FromStr;
use std::sync::OnceLock;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId};
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, ScopeSelectors, StyleModifier, Theme as SynTheme, ThemeItem, ThemeSettings,
};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::theme::{Theme, ThemeKind, c};

/// The kind of a highlighted run. Each kind has one theme colour ([`syntax_color`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SyntaxToken {
    /// Text with no special meaning.
    Plain,
    /// Comments.
    Comment,
    /// Keywords and storage modifiers (`fn`, `let`, `def`, `import`).
    Keyword,
    /// String and character literals.
    String,
    /// Numeric literals.
    Number,
    /// Language constants (`true`, `None`, `null`) and escapes.
    Constant,
    /// Type names.
    Type,
    /// Function names at definition and call.
    Function,
    /// Object keys (JSON, YAML) and markup tags.
    Key,
    /// Attributes, annotations and macros.
    Attribute,
    /// Punctuation and operators.
    Punctuation,
    /// Added lines in a diff.
    Inserted,
    /// Removed lines in a diff.
    Deleted,
    /// Diff headers and hunk ranges.
    Meta,
    /// Markup headings.
    Heading,
}

/// Scope selectors for each token kind. Order is the kind's index; index 0 (`Plain`) has no
/// selector and is the theme default.
const SCOPES: [(SyntaxToken, &str); 15] = [
    (SyntaxToken::Plain, ""),
    (SyntaxToken::Comment, "comment, punctuation.definition.comment"),
    (SyntaxToken::Keyword, "keyword, storage, keyword.operator.word"),
    (
        SyntaxToken::String,
        "string, constant.character, punctuation.definition.string",
    ),
    (SyntaxToken::Number, "constant.numeric"),
    (
        SyntaxToken::Constant,
        "constant.language, constant.other, support.constant, constant.character.escape",
    ),
    (
        SyntaxToken::Type,
        "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, \
         entity.name.trait, support.type, support.class, entity.other.inherited-class",
    ),
    (
        SyntaxToken::Function,
        "entity.name.function, support.function, variable.function",
    ),
    (
        SyntaxToken::Key,
        "meta.mapping.key string, meta.structure.dictionary.key string, meta.mapping.key punctuation.definition.string, meta.structure.dictionary.key punctuation.definition.string, \
         entity.name.tag, support.type.property-name",
    ),
    (
        SyntaxToken::Attribute,
        "meta.annotation, entity.other.attribute-name, support.macro, entity.name.macro, \
         meta.attribute, punctuation.definition.annotation",
    ),
    (
        SyntaxToken::Punctuation,
        "punctuation.separator, punctuation.terminator, punctuation.section, keyword.operator",
    ),
    (SyntaxToken::Inserted, "markup.inserted"),
    (SyntaxToken::Deleted, "markup.deleted"),
    (
        SyntaxToken::Meta,
        "meta.diff.range, meta.diff.header, meta.diff.index, markup.changed",
    ),
    (SyntaxToken::Heading, "markup.heading"),
];

fn token_of(color: Color) -> SyntaxToken {
    SCOPES
        .get(usize::from(color.r))
        .map_or(SyntaxToken::Plain, |(t, _)| *t)
}

/// One coloured run of a line: a byte range into the line text and its kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSpan {
    /// Byte range into the line, without its line ending.
    pub range: Range<usize>,
    /// What the run is.
    pub token: SyntaxToken,
}

/// The syntax set and the scope-to-kind map. Load it once with [`Highlighter::shared`]; loading
/// decompresses the bundled syntaxes (tens of milliseconds).
pub struct Highlighter {
    syntaxes: SyntaxSet,
    theme: SynTheme,
}

impl std::fmt::Debug for Highlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Highlighter")
            .field("syntaxes", &self.syntaxes.syntaxes().len())
            .finish_non_exhaustive()
    }
}

/// Language names that the bundled set lacks, mapped to the closest syntax it has.
const ALIASES: [(&str, &str); 6] = [
    ("typescript", "js"),
    ("ts", "js"),
    ("tsx", "js"),
    ("jsx", "js"),
    ("yml", "yaml"),
    ("patch", "diff"),
];

impl Highlighter {
    /// Load the bundled syntaxes.
    #[must_use]
    pub fn new() -> Self {
        let scopes = SCOPES
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(i, (_, sel))| {
                Some(ThemeItem {
                    scope: ScopeSelectors::from_str(sel).ok()?,
                    style: StyleModifier {
                        foreground: Some(Color {
                            r: u8::try_from(i).ok()?,
                            g: 0,
                            b: 0,
                            a: 0xFF,
                        }),
                        background: None,
                        font_style: None,
                    },
                })
            })
            .collect();
        let theme = SynTheme {
            name: None,
            author: None,
            settings: ThemeSettings {
                foreground: Some(Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0xFF,
                }),
                ..ThemeSettings::default()
            },
            scopes,
        };
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            theme,
        }
    }

    /// The process-wide highlighter, loaded on first use.
    pub fn shared() -> &'static Self {
        static SHARED: OnceLock<Highlighter> = OnceLock::new();
        SHARED.get_or_init(Self::new)
    }

    fn syntax(&self, language: &str) -> Option<&SyntaxReference> {
        let lower = language.trim().to_ascii_lowercase();
        let name = ALIASES
            .iter()
            .find(|(alias, _)| *alias == lower)
            .map_or(lower.as_str(), |(_, to)| to);
        if name.is_empty() {
            return None;
        }
        self.syntaxes.find_syntax_by_token(name)
    }

    /// Whether `language` (a name such as `rust`, or a file extension such as `py`) has a
    /// syntax. Unknown languages highlight as plain text.
    #[must_use]
    pub fn supports(&self, language: &str) -> bool {
        self.syntax(language).is_some()
    }

    /// Highlight `text` as `language`: one entry per line (split on `\n`, a trailing `\r`
    /// dropped), each the line's runs in order, covering the whole line. Never fails: an
    /// unknown language, or a syntax error inside syntect, gives plain runs.
    #[must_use]
    pub fn highlight(&self, language: &str, text: &str) -> Vec<Vec<TokenSpan>> {
        let syntax = self
            .syntax(language)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
        let mut lines = HighlightLines::new(syntax, &self.theme);
        let mut failed = false;
        let mut buf = String::new();
        split_lines(text)
            .map(|line| {
                if failed || line.is_empty() {
                    return plain(line);
                }
                buf.clear();
                buf.push_str(line);
                buf.push('\n');
                match lines.highlight_line(&buf, &self.syntaxes) {
                    Ok(runs) => spans(line.len(), &runs),
                    Err(e) => {
                        log::warn!("syntax highlighting stopped: {e}");
                        failed = true;
                        plain(line)
                    }
                }
            })
            .collect()
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// [`Highlighter::highlight`] on the shared highlighter.
///
/// ```
/// use fluent2_egui::data::code::{SyntaxToken, highlight};
/// let lines = highlight("rust", "let x = 1;");
/// assert_eq!(lines[0][0].token, SyntaxToken::Keyword);
/// ```
#[must_use]
pub fn highlight(language: &str, text: &str) -> Vec<Vec<TokenSpan>> {
    Highlighter::shared().highlight(language, text)
}

/// The lines of `text` as CodeView and [`highlight`] count them.
pub fn split_lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l))
}

fn plain(line: &str) -> Vec<TokenSpan> {
    if line.is_empty() {
        return Vec::new();
    }
    vec![TokenSpan {
        range: 0..line.len(),
        token: SyntaxToken::Plain,
    }]
}

/// Turn syntect's runs (over the line plus `\n`) into merged spans clipped to the line.
fn spans(len: usize, runs: &[(syntect::highlighting::Style, &str)]) -> Vec<TokenSpan> {
    let mut out: Vec<TokenSpan> = Vec::new();
    let mut at = 0;
    for (style, run) in runs {
        let end = (at + run.len()).min(len);
        if end > at {
            let token = token_of(style.foreground);
            match out.last_mut() {
                Some(last) if last.token == token && last.range.end == at => last.range.end = end,
                _ => out.push(TokenSpan {
                    range: at..end,
                    token,
                }),
            }
        }
        at += run.len();
    }
    out
}

/// The theme colour of a token kind, from Fluent palette foreground tokens. High contrast
/// paints every kind in the plain foreground, as the theme's other colours do not apply.
#[must_use]
pub fn syntax_color(token: SyntaxToken, theme: &Theme) -> Color32 {
    let t = &theme.color;
    if theme.kind == ThemeKind::HighContrast {
        return c(t.neutral_foreground_1);
    }
    c(match token {
        SyntaxToken::Plain => t.neutral_foreground_1,
        SyntaxToken::Comment => t.neutral_foreground_3,
        SyntaxToken::Keyword => t.palette_blue_foreground_2,
        SyntaxToken::String => t.palette_cranberry_foreground_2,
        SyntaxToken::Number => t.palette_dark_green_foreground_2,
        SyntaxToken::Constant => t.palette_purple_foreground_2,
        SyntaxToken::Type => t.palette_teal_foreground_2,
        SyntaxToken::Function => t.palette_dark_orange_foreground_2,
        SyntaxToken::Key => t.palette_navy_foreground_2,
        SyntaxToken::Attribute => t.palette_grape_foreground_2,
        SyntaxToken::Punctuation => t.neutral_foreground_2,
        SyntaxToken::Inserted => t.palette_green_foreground_2,
        SyntaxToken::Deleted => t.palette_red_foreground_2,
        SyntaxToken::Meta => t.palette_royal_blue_foreground_2,
        SyntaxToken::Heading => t.brand_foreground_1,
    })
}

/// A LayoutJob for one highlighted line, in theme syntax colours. Spans past the line's end
/// are clipped; gaps are painted plain.
#[must_use]
pub fn line_job(line: &str, spans: &[TokenSpan], font: FontId, theme: &Theme) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut at = 0;
    let push = |job: &mut LayoutJob, range: Range<usize>, token: SyntaxToken| {
        if let Some(text) = line.get(range) {
            job.append(
                text,
                0.0,
                TextFormat::simple(font.clone(), syntax_color(token, theme)),
            );
        }
    };
    for span in spans {
        let start = span.range.start.max(at).min(line.len());
        let end = span.range.end.min(line.len());
        if start > at {
            push(&mut job, at..start, SyntaxToken::Plain);
        }
        if end > start {
            push(&mut job, start..end, span.token);
            at = end;
        }
    }
    if at < line.len() {
        push(&mut job, at..line.len(), SyntaxToken::Plain);
    }
    job
}
