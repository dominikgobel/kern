use crate::shortcuts::{self, Action};
use crate::syntax_cache::{SyntaxCache, style_to_crossterm, ColorMode};
use crate::ui_types::{EditorMode, ScreenSize, UiState, Viewport};
use crate::{editor::Editor, file::write_file};

use crossterm::{
    cursor,
    event::{self, Event, KeyEventKind},
    execute, queue,
    style::{Color, ContentStyle, Print, ResetColor, SetStyle},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use unicode_width::UnicodeWidthChar;
use supports_color::Stream;

const _UI_ROWS: u16 = 3;
const THEME: &str = "base16-eighties.dark";

pub fn run(mut editor: Editor, filename: PathBuf) -> io::Result<()> {
    terminal::enable_raw_mode()?;

    let mut stdout = io::stdout();
    let color_mode = detect_color_mode();

    if let Err(e) = execute!(stdout, EnterAlternateScreen, cursor::Hide) {
        let _ = terminal::disable_raw_mode();
        return Err(e);
    }

    let result = event_loop(&mut editor, filename, &color_mode);

    let cleanup = execute!(stdout, cursor::Show, LeaveAlternateScreen);
    let raw = terminal::disable_raw_mode();

    result?;
    cleanup?;
    raw
}

fn event_loop(editor: &mut Editor, filename: PathBuf, color_mode: &ColorMode) -> io::Result<()> {
    // Der Dateipfad bleibt ein PathBuf.
    let mut path = filename.clone();

    let mut viewport = Viewport::default();

    let mut ui = UiState {
        // Der Dateiname im Speichereingabefeld bleibt ein String,
        // weil dort einzelne Zeichen eingefügt und gelöscht werden.
        target_path: filename.to_string_lossy().into_owned(),
        ..UiState::default()
    };

    let mut syntax = SyntaxCache::new(&path, THEME);
    syntax.rebuild(&editor.lines, &path)?;

    let mut dirty = true;

    loop {
        if dirty {
            render(editor, &filename, &ui, &mut viewport, &syntax, color_mode)?;
            dirty = false;
        }

        match event::read()? {
            Event::Resize(_, _) => dirty = true,

            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let action = shortcuts::resolve_key(key, ui.mode);
                let before_cursor = (editor.cursor.row, editor.cursor.col);
                let mut content_changed = false;

                match action {
                    Action::Quit => break,

                    Action::BeginSave => {
                        ui.mode = EditorMode::Saving;
                        ui.target_path = filename.to_string_lossy().into_owned();
                        ui.message = None;
                        dirty = true;
                    }

                    Action::CancelSave => {
                        ui.mode = EditorMode::Writing;
                        ui.message = None;
                        dirty = true;
                    }

                    Action::Save => {
                        let entered_path = ui.target_path.trim();

                        if entered_path.is_empty() {
                            ui.message = Some("File name cannot be empty".into());
                        } else {
                            // Den im Eingabefeld geschriebenen String in einen PathBuf umwandeln.
                            let candidate = PathBuf::from(entered_path);

                            match write_file(&candidate, &editor.lines.join("\n")) {
                                Ok(()) => {
                                    path = candidate.clone();

                                    syntax = SyntaxCache::new(&path, THEME);
                                    syntax.rebuild(&editor.lines, &path)?;

                                    ui.mode = EditorMode::Writing;
                                    ui.modified = false;
                                    ui.message = Some(format!("Saved {}", candidate.display()));
                                }

                                Err(error) => {
                                    ui.message = Some(format!("Could not save: {error}"));
                                }
                            }
                        }

                        dirty = true;
                    }

                    Action::MoveLeft => {
                        if editor.cursor.col > 0 {
                            editor.cursor.col -= 1;
                        } else if editor.cursor.row > 0 {
                            editor.cursor.row -= 1;
                            editor.cursor.col = editor.lines[editor.cursor.row].chars().count();
                        }

                        dirty = true;
                    }

                    Action::MoveRight => {
                        let len = editor
                            .lines
                            .get(editor.cursor.row)
                            .map(|line| line.chars().count())
                            .unwrap_or(0);

                        if editor.cursor.col < len {
                            editor.cursor.col += 1;
                        } else if editor.cursor.row + 1 < editor.lines.len() {
                            editor.cursor.row += 1;
                            editor.cursor.col = 0;
                        }

                        dirty = true;
                    }

                    Action::MoveUp => {
                        if editor.cursor.row > 0 {
                            editor.cursor.row -= 1;
                            editor.cursor.col = editor
                                .cursor
                                .col
                                .min(editor.lines[editor.cursor.row].chars().count());
                        }

                        dirty = true;
                    }

                    Action::MoveDown => {
                        if editor.cursor.row + 1 < editor.lines.len() {
                            editor.cursor.row += 1;
                            editor.cursor.col = editor
                                .cursor
                                .col
                                .min(editor.lines[editor.cursor.row].chars().count());
                        }

                        dirty = true;
                    }

                    Action::Insert(ch) if ui.mode == EditorMode::Saving => {
                        if !ch.is_control() {
                            ui.target_path.push(ch);
                            ui.message = None;
                            dirty = true;
                        }
                    }

                    Action::Insert(ch) => {
                        editor
                            .insert_char(ch)
                            .map_err(|_| io::Error::other("Invalid cursor position"))?;

                        content_changed = true;
                    }

                    Action::Enter if ui.mode == EditorMode::Writing => {
                        editor
                            .insert_char('\n')
                            .map_err(|_| io::Error::other("Invalid cursor position"))?;

                        content_changed = true;
                    }

                    Action::Backspace if ui.mode == EditorMode::Saving => {
                        ui.target_path.pop();
                        ui.message = None;
                        dirty = true;
                    }

                    Action::Backspace => {
                        editor
                            .delete_char()
                            .map_err(|_| io::Error::other("Invalid cursor position"))?;

                        content_changed = true;
                    }

                    Action::Ignore | Action::Enter => {}
                }

                if content_changed {
                    ui.modified = true;
                    syntax.rebuild(&editor.lines, &path)?;
                    dirty = true;
                }

                if before_cursor != (editor.cursor.row, editor.cursor.col) {
                    dirty = true;
                }
            }

            _ => {}
        }
    }

    Ok(())
}

fn char_width(ch: char, cell: usize) -> usize {
    if ch == '\t' {
        4 - cell % 4
    } else {
        UnicodeWidthChar::width(ch).unwrap_or(1)
    }
}

fn render(
    editor: &Editor,
    filename: &Path,
    ui: &UiState,
    viewport: &mut Viewport,
    syntax: &SyntaxCache,
    color_mode: &ColorMode
) -> io::Result<()> {
    let (width, height) = terminal::size()?;

    if width == 0 || height == 0 {
        return Ok(());
    }

    let screen = ScreenSize::new(width, height, editor.lines.len());
    let text_rows = screen.text_rows as usize;
    let text_width = screen.text_width();

    if text_rows == 0 || text_width == 0 {
        return Ok(());
    }

    let cursor_line = editor
        .lines
        .get(editor.cursor.row)
        .map(String::as_str)
        .unwrap_or("");

    let cursor_cell = cursor_line
        .chars()
        .take(editor.cursor.col)
        .fold(0usize, |cell, ch| cell + char_width(ch, cell));

    if editor.cursor.row < viewport.y {
        viewport.y = editor.cursor.row;
    } else if editor.cursor.row >= viewport.y + text_rows {
        viewport.y = editor.cursor.row - text_rows + 1;
    }

    if cursor_cell < viewport.x {
        viewport.x = cursor_cell;
    } else if cursor_cell >= viewport.x + text_width {
        viewport.x = cursor_cell - text_width + 1;
    }

    let mut out = Vec::with_capacity((width as usize * height as usize).min(256 * 1024));

    let bg = syntax.background();

    let base = ContentStyle {
        background_color: Some(Color::Rgb {
            r: bg.r,
            g: bg.g,
            b: bg.b,
        }),
        ..ContentStyle::default()
    };

    let number_style = ContentStyle {
        foreground_color: Some(Color::White),
        ..ContentStyle::default()
    };

    let line_count = editor.lines.len();
    let digits = line_count.max(1).to_string().len();

    for screen_row in 0..text_rows {
        let doc_row = viewport.y + screen_row;

        queue!(
            out,
            cursor::MoveTo(0, screen_row as u16),
            SetStyle(number_style),
            Clear(ClearType::CurrentLine)
        )?;

        if doc_row < line_count {
            write!(out, " {:>width$}  ", doc_row + 1, width = digits)?;
        } else {
            write!(out, " {:>width$}  ", "~", width = digits)?;
        }

        queue!(out, SetStyle(base))?;

        let spans = syntax.rows().get(doc_row);
        let mut cell = 0usize;
        let mut rendered = 0usize;

        if let Some(spans) = spans {
            'spans: for span in spans {
                let style = style_to_crossterm(span.style, color_mode);

                for ch in span.text.chars() {
                    let w = char_width(ch, cell);
                    let start = cell;
                    let end = cell + w;

                    cell = end;

                    if end <= viewport.x {
                        continue;
                    }

                    if start >= viewport.x + text_width {
                        break 'spans;
                    }

                    let left_clip = viewport.x.saturating_sub(start);
                    let right_clip = end.saturating_sub(viewport.x + text_width);

                    let visible_width = w.saturating_sub(left_clip + right_clip);

                    if visible_width == 0 {
                        continue;
                    }

                    queue!(out, SetStyle(style))?;

                    if ch == '\t' || left_clip > 0 || right_clip > 0 {
                        write!(out, "{}", " ".repeat(visible_width))?;
                    } else {
                        write!(out, "{ch}")?;
                    }

                    rendered += visible_width;
                }
            }
        }

        if rendered < text_width {
            queue!(out, SetStyle(base))?;
            write!(out, "{}", " ".repeat(text_width - rendered))?;
        }

        queue!(out, ResetColor)?;
    }

    draw_ui_line(
        &mut out,
        screen.text_rows,
        &status_line(filename, editor, ui),
        width,
        status_style(),
    )?;

    draw_ui_line(
        &mut out,
        screen.text_rows + 1,
        &shortcuts::shortcut_line(ui.mode),
        width,
        ContentStyle::new(),
    )?;

    let message = match ui.mode {
        EditorMode::Writing => {
            format!(" kern | {} ", syntax.syntax_name())
        }
        EditorMode::Saving => format!(
            " {} ",
            ui.message
                .as_deref()
                .unwrap_or("Type a path, then press Enter")
        ),
    };

    draw_ui_line(
        &mut out,
        screen.text_rows + 2,
        &message,
        width,
        ContentStyle::new(),
    )?;

    let gutter = screen.gutter_width;
    let x = gutter as usize + cursor_cell.saturating_sub(viewport.x);
    let y = editor.cursor.row.saturating_sub(viewport.y);

    queue!(
        out,
        cursor::MoveTo(
            x.min(width as usize - 1) as u16,
            y.min(text_rows - 1) as u16
        ),
        cursor::Show
    )?;

    let mut stdout = io::stdout().lock();
    stdout.write_all(&out)?;
    stdout.flush()
}

fn status_line(filename: &Path, editor: &Editor, ui: &UiState) -> String {
    match ui.mode {
        EditorMode::Writing => format!(
            " {} {} | Ln {}, Col {} ",
            filename.display(),
            if ui.modified { "[Modified]" } else { "" },
            editor.cursor.row + 1,
            editor.cursor.col + 1
        ),
        EditorMode::Saving => format!(" Save as: {} ", ui.target_path),
    }
}

fn status_style() -> ContentStyle {
    ContentStyle {
        foreground_color: Some(Color::Black),
        background_color: Some(Color::White),
        ..ContentStyle::default()
    }
}

fn draw_ui_line(
    out: &mut Vec<u8>,
    row: u16,
    content: &str,
    width: u16,
    style: ContentStyle,
) -> io::Result<()> {
    let visible: String = content.chars().take(width as usize).collect();

    let used = visible
        .chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
        .sum::<usize>();

    queue!(
        out,
        cursor::MoveTo(0, row),
        Clear(ClearType::CurrentLine),
        SetStyle(style),
        Print(visible),
        Print(" ".repeat((width as usize).saturating_sub(used))),
        ResetColor
    )?;

    Ok(())
}

fn detect_color_mode() -> ColorMode {
    match supports_color::on(Stream::Stdout) {
        Some(c) if c.has_16m => ColorMode::Truecolor,
        Some(c) if c.has_256 => ColorMode::Color256,
        Some(c) if c.has_basic => ColorMode::Color16,
        _ => ColorMode::Monochrome
    }
}