use std::{io, path::Path};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, FontStyle, Style, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};
use crossterm::style::Color as CColor;
pub enum ColorMode {
    Truecolor,
    Color256,
    Color16,
    Monochrome,
}
#[derive(Clone, Debug)]
pub struct StyledSpan {
    pub style: Style,
    pub text: String,
}

pub struct SyntaxCache {
    syntaxes: SyntaxSet,
    theme: Theme,
    rows: Vec<Vec<StyledSpan>>,
    background: Color,
    syntax_name: String,
}

impl SyntaxCache {
    pub fn new(path: &Path, theme_name: &str) -> Self {
        let syntaxes = SyntaxSet::load_defaults_newlines();
        let themes = ThemeSet::load_defaults();
        let theme = themes
            .themes
            .get(theme_name)
            .or_else(|| themes.themes.get("base16-ocean.dark"))
            .or_else(|| themes.themes.values().next())
            .expect("syntect did not provide any default themes")
            .clone();
        let background = theme.settings.background.unwrap_or(Color {
            r: 30,
            g: 30,
            b: 30,
            a: 255,
        });
        let syntax_name = syntax_for_path(&syntaxes, path).name.to_owned();
        Self {
            syntaxes,
            theme,
            rows: Vec::new(),
            background,
            syntax_name,
        }
    }

    pub fn background(&self) -> Color {
        self.background
    }
    pub fn syntax_name(&self) -> &str {
        &self.syntax_name
    }
    pub fn rows(&self) -> &[Vec<StyledSpan>] {
        &self.rows
    }

    /// Rebuilds highlighting in source order. HighlightLines is stateful, so it must
    /// never be advanced by rendering or by synthetic padding characters.
    pub fn rebuild(&mut self, lines: &[String], path: &Path) -> io::Result<()> {
        let syntax = syntax_for_path(&self.syntaxes, path);
        self.syntax_name = syntax.name.to_owned();
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut rows = Vec::with_capacity(lines.len());
        for source_line in lines {
            let mut with_newline = source_line.clone();
            with_newline.push('\n');
            let regions = highlighter
                .highlight_line(&with_newline, &self.syntaxes)
                .map_err(|e| io::Error::other(e.to_string()))?;
            let mut spans = Vec::with_capacity(regions.len());
            for (style, text) in regions {
                let visible = text.strip_suffix('\n').unwrap_or(text);
                if !visible.is_empty() {
                    spans.push(StyledSpan {
                        style,
                        text: visible.to_owned(),
                    });
                }
            }
            rows.push(spans);
        }
        self.rows = rows;
        Ok(())
    }
}

fn syntax_for_path<'a>(syntaxes: &'a SyntaxSet, path: &Path) -> &'a SyntaxReference {
    path.extension()
        .and_then(|s| s.to_str())
        .and_then(|ext| syntaxes.find_syntax_by_extension(ext))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
}

pub fn style_to_crossterm(style: Style, color_mode: &ColorMode) -> crossterm::style::ContentStyle {
    use crossterm::style::{Attribute, Color as CColor, ContentStyle};
    let mut out = ContentStyle::default();
    match color_mode {
        ColorMode::Truecolor => {
            out.foreground_color = Some(CColor::Rgb {
                r: style.foreground.r,
                g: style.foreground.g,
                b: style.foreground.b,
            });

            out.background_color = Some(CColor::Rgb {
                r: style.background.r,
                g: style.background.g,
                b: style.background.b,
            });
        }
        ColorMode::Color256 => {
            out.foreground_color = Some(rgb_to_256(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
            ));

            out.background_color = Some(rgb_to_256(
                style.background.r,
                style.background.g,
                style.background.b,
            ));
        }
        ColorMode::Color16 => {
            out.foreground_color = Some(rgb_to_16(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
            ));

            out.background_color = Some(rgb_to_16(
                style.background.r,
                style.background.g,
                style.background.b,
            ));
        }
        ColorMode::Monochrome => {
            // nothing is needed, because no colors can be shown
        }
    }
    if style.font_style.contains(FontStyle::BOLD) {
        out.attributes.set(Attribute::Bold);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        out.attributes.set(Attribute::Italic);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        out.attributes.set(Attribute::Underlined);
    }
    out
}

/// Konvertiert RGB in die nächstgelegene Farbe der xterm-256-Palette.
fn rgb_to_256(r: u8, g: u8, b: u8) -> CColor {
    let mut best_index = 0u8;
    let mut best_distance = u32::MAX;

    for index in 0u16..256 {
        let (pr, pg, pb) = xterm_256_rgb(index as u8);

        let dr = r as i32 - pr as i32;
        let dg = g as i32 - pg as i32;
        let db = b as i32 - pb as i32;

        let distance = (dr * dr + dg * dg + db * db) as u32;

        if distance < best_distance {
            best_distance = distance;
            best_index = index as u8;
        }
    }

    CColor::AnsiValue(best_index)
}

/// Map RGB to xterm-256-pallete.
fn xterm_256_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        // Die ersten 16 ANSI-Farben
        0..=15 => {
            const COLORS: [(u8, u8, u8); 16] = [
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            ];

            COLORS[index as usize]
        }

        // 6 × 6 × 6 RGB-Farbwürfel
        16..=231 => {
            const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

            let n = (index - 16) as usize;

            (
                LEVELS[n / 36],
                LEVELS[(n / 6) % 6],
                LEVELS[n % 6],
            )
        }

        // 24 Graustufen
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

/// Map RGB to 16 ANSI-Colors
fn rgb_to_16(r: u8, g: u8, b: u8) -> CColor {
    let mut best_index = 0u8;
    let mut best_distance = u32::MAX;

    for index in 0u8..16 {
        let (pr, pg, pb) = xterm_256_rgb(index);

        let dr = r as i32 - pr as i32;
        let dg = g as i32 - pg as i32;
        let db = b as i32 - pb as i32;

        let distance = (dr * dr + dg * dg + db * db) as u32;

        if distance < best_distance {
            best_distance = distance;
            best_index = index;
        }
    }

    match best_index {
        0 => CColor::Black,
        1 => CColor::DarkRed,
        2 => CColor::DarkGreen,
        3 => CColor::DarkYellow,
        4 => CColor::DarkBlue,
        5 => CColor::DarkMagenta,
        6 => CColor::DarkCyan,
        7 => CColor::Grey,
        8 => CColor::DarkGrey,
        9 => CColor::Red,
        10 => CColor::Green,
        11 => CColor::Yellow,
        12 => CColor::Blue,
        13 => CColor::Magenta,
        14 => CColor::Cyan,
        _ => CColor::White,
    }
}