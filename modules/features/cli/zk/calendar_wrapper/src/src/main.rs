use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::process::Command;

use color_eyre::eyre::{eyre, Context, Result};
use crossterm::event::{self, KeyCode, KeyModifiers};
use nix::unistd::{dup, dup2_stdin, dup2_stdout};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use serde::Deserialize;
use time::{Date, Duration, Month};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Note {
    filename_stem: String,
    abs_path: String,
    raw_content: String,
}

struct NotePreview {
    path: String,
    raw_content: String,
}

struct NoteRepository {
    daily_dir: PathBuf,
}

impl NoteRepository {
    fn new() -> Result<Self> {
        let notebook_dir = env::var("ZK_NOTEBOOK_DIR")
            .context("ZK_NOTEBOOK_DIR is not set")?;

        Ok(Self {
            daily_dir: PathBuf::from(notebook_dir).join("journal/daily"),
        })
    }

    fn create(&self, date: Date) -> Result<NotePreview> {
        let date = date.to_string();

        let args = [
            "new".to_string(),
            "--dry-run".to_string(),
            "--extra".to_string(),
            format!("date={date}"),
            self.daily_dir.display().to_string(),
        ];

        eprintln!("running: zk {}", args.join(" "));

        let output = Command::new("zk")
            .args(&args)
            .output()
            .context("failed to run zk")?;

        if !output.status.success() {
            return Err(eyre!(
                "zk new --dry-run failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        let path = String::from_utf8(output.stderr)
            .context("zk returned a non-UTF-8 note path")?
            .trim()
            .to_owned();

        if path.is_empty() {
            return Err(eyre!("zk did not return a note path on stderr"));
        }

        let raw_content = String::from_utf8(output.stdout)
            .context("zk returned non-UTF-8 note content")?;

        let path = PathBuf::from(path);

        let path = if path.is_absolute() {
            path
        } else {
            self.daily_dir.join(path)
        };

        fs::write(&path, &raw_content)
            .with_context(|| format!("failed to create note {}", path.display()))?;

        Ok(NotePreview {
            path: path.to_string_lossy().into_owned(),
            raw_content,
        })
    }
}

struct App {
    notes: BTreeMap<Date, NotePreview>,
    selected_date: Date,
    calendar_start: Date,
    calendar_columns: u16,
    calendar_rows: u16,
    picked_path: Option<String>,
    repository: NoteRepository,
    confirmation_prompt: bool,
    confirm_create: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let no_confirmation_prompt = env::args()
        .skip(1)
        .any(|arg| arg == "--no-confirmation-prompt");

    // Read the complete JSON array before starting the TUI.
    let notes: Vec<Note> = {
        let stdin = io::stdin();
        serde_json::from_reader(stdin.lock())?
    };

    // Save the real stdout (which may be a pipe), then use /dev/tty
    // for both Ratatui's output and Crossterm's input.
    let stdout = dup(io::stdout().as_fd())?;
    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;

    dup2_stdin(&tty)?;
    dup2_stdout(&tty)?;

    let mut app = App::new(notes, !no_confirmation_prompt)?;

    // Make sure stdout is restored even if the TUI returns an error.
    let run_result = ratatui::run(|terminal| app.run(terminal));

    // Restore the original stdout so the selected path goes down
    // the pipeline rather than to the terminal.
    dup2_stdout(&stdout)?;

    run_result?;

    // Ratatui has restored the terminal at this point.
    if let Some(path) = app.picked_path {
        println!("{path}");
    }

    Ok(())
}

impl App {
    fn new(notes: Vec<Note>, confirmation_prompt: bool) -> Result<Self> {
        let mut notes_by_date = BTreeMap::new();

        for note in notes {
            let date = parse_date(&note.filename_stem)
                .ok_or_else(|| eyre!("invalid daily note date: {}", note.filename_stem))?;

            notes_by_date.insert(
                date,
                NotePreview {
                    path: note.abs_path,
                    raw_content: note.raw_content,
                },
            );
        }

        let selected_date = notes_by_date
            .keys()
            .next_back()
            .copied()
            .ok_or_else(|| eyre!("stdin contained no daily notes"))?;

        Ok(Self {
            notes: notes_by_date,
            selected_date,
            calendar_start: selected_date,
            calendar_columns: 1,
            calendar_rows: 1,
            picked_path: None,
            repository: NoteRepository::new()?,
            confirmation_prompt,
            confirm_create: false,
        })
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if let Some(key) = event::read()?.as_key_press_event() {
                if self.confirm_create {
                    match key.code {
                        KeyCode::Enter | KeyCode::Char('y') => {
                            self.confirm_create = false;

                            match self.create_selected_note() {
                                Ok(path) => {
                                    self.picked_path = Some(path);
                                    return Ok(());
                                }
                                Err(error) => {
                                    return Err(io::Error::other(error));
                                }
                            }
                        }

                        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {
                            self.confirm_create = false;
                        }

                        KeyCode::Char('c')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            return Ok(());
                        }

                        _ => {}
                    }

                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }

                    KeyCode::Enter | KeyCode::Char('e') => {
                        // Existing notes are opened immediately.
                        if let Some(note) = self.notes.get(&self.selected_date) {
                            self.picked_path = Some(note.path.clone());
                            return Ok(());
                        }

                        // Missing notes optionally require confirmation.
                        if self.confirmation_prompt {
                            self.confirm_create = true;
                            continue;
                        }

                        match self.create_selected_note() {
                            Ok(path) => {
                                self.picked_path = Some(path);
                                return Ok(());
                            }
                            Err(error) => {
                                // The current TUI has no error overlay, so
                                // propagate the error and let main restore
                                // stdout before reporting it.
                                return Err(io::Error::other(error));
                            }
                        }
                    }

                    KeyCode::Char('h') | KeyCode::Left => self.move_days(-1, 1),
                    KeyCode::Char('l') | KeyCode::Right => self.move_days(1, 1),

                    KeyCode::Char('j') | KeyCode::Down => {
                        self.move_days(7, self.calendar_columns as i32)
                    }

                    KeyCode::Char('k') | KeyCode::Up => {
                        self.move_days(-7, self.calendar_columns as i32)
                    }

                    KeyCode::Char('n') | KeyCode::PageDown | KeyCode::Tab => {
                        self.move_month(1);
                    }

                    KeyCode::Char('p') | KeyCode::PageUp | KeyCode::BackTab => {
                        self.move_month(-1);
                    }

                    _ => {}
                }
            }
        }
    }

    fn create_selected_note(&mut self) -> Result<String> {
        let note = self.repository.create(self.selected_date)?;
        let path = note.path.clone();

        // Keep the in-memory model consistent after creation.
        self.notes.insert(self.selected_date, note);

        Ok(path)
    }

    fn move_days(&mut self, days: i64, viewport_step: i32) {
        if let Some(date) = self.selected_date.checked_add(Duration::days(days)) {
            self.selected_date = date;
            self.ensure_visible(viewport_step);
        }
    }

    fn move_month(&mut self, months: i32) {
        if let Some(date) = shift_month(self.selected_date, months) {
            self.selected_date = date;
            self.ensure_visible(months);
        }
    }

    fn ensure_visible(&mut self, step: i32) {
        let selected_month = month_start(self.selected_date);
        let start_month = month_start(self.calendar_start);

        let month_offset = month_difference(start_month, selected_month);
        let visible_months =
            (self.calendar_columns * self.calendar_rows) as i32;

        if month_offset < 0 {
            self.calendar_start =
                shift_month(start_month, -step.abs()).unwrap_or(selected_month);
        } else if month_offset >= visible_months {
            self.calendar_start =
                shift_month(start_month, step.abs()).unwrap_or(selected_month);
        }
    }

    fn events(&self) -> CalendarEventStore {
        const NOTE: Style = Style::new()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);

        const SELECTED: Style = Style::new()
            .fg(Color::White)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD);

        let mut events = CalendarEventStore::today(Style::default());

        for date in self.notes.keys() {
            events.add(*date, NOTE);
        }

        // Add this last so the cursor style takes precedence.
        events.add(self.selected_date, SELECTED);

        events
    }

    fn render(&mut self, frame: &mut Frame) {
        const MONTH_WIDTH: u16 = 22;
        const MONTH_HEIGHT: u16 = 9;

        let [calendar_area, preview_area] =
            frame.area().layout(&Layout::horizontal([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ]));

        let calendar_inner =
            calendar_area.inner(ratatui::layout::Margin::new(1, 1));

        let columns = (calendar_inner.width / MONTH_WIDTH).max(1);
        let rows = (calendar_inner.height / MONTH_HEIGHT).max(1);

        self.calendar_columns = columns;
        self.calendar_rows = rows;

        let visible_months = columns * rows;
        let events = self.events();

        frame.render_widget(
            Block::bordered().title(" Calendar "),
            calendar_area,
        );

        for index in 0..visible_months {
            let row = index / columns;
            let column = index % columns;

            let area = Rect {
                x: calendar_inner.x + column * MONTH_WIDTH,
                y: calendar_inner.y + row * MONTH_HEIGHT,
                width: MONTH_WIDTH
                    .min(calendar_inner.width - column * MONTH_WIDTH),
                height: MONTH_HEIGHT
                    .min(calendar_inner.height - row * MONTH_HEIGHT),
            };

            let Some(date) =
                shift_month(self.calendar_start, index as i32)
            else {
                continue;
            };

            let calendar = Monthly::new(date, &events)
                .show_month_header(Style::new().bold())
                .show_weekdays_header(Style::new().fg(Color::Cyan));

            frame.render_widget(calendar, area);
        }

        let preview = self
            .notes
            .get(&self.selected_date)
            .map(|note| note.raw_content.as_str())
            .unwrap_or("(no note for this date)");

        frame.render_widget(
            Paragraph::new(preview)
                .block(Block::bordered().title(" Preview "))
                .wrap(Wrap { trim: false }),
            preview_area,
        );

        if self.confirm_create {
            self.render_confirmation_popup(frame);
        }
    }

    // fn render_confirmation_popup(&self, frame: &mut Frame) {
    //     let area = centered_rect(40, 20, frame.area());

    //     let message = format!(
    //         "No note exists for {}.\nCreate a new note?\n\
    //          [Enter/y] Yes    [Esc/n] No",
    //         self.selected_date
    //     );

    //     frame.render_widget(Clear, area);

    //     frame.render_widget(
    //         Paragraph::new(message)
    //             .alignment(Alignment::Center)
    //             .wrap(Wrap { trim: false })
    //             .block(
    //                 Block::bordered()
    //                     .title(" Create Note ")
    //                     .borders(Borders::ALL)
    //                     .style(Style::default()),
    //             ),
    //         area,
    //     );
    // }
    fn render_confirmation_popup(&self, frame: &mut Frame) {
        let message = format!(
            "No note exists for {}.\nCreate a new note?\n\
             [Enter/y] Yes    [Esc/n] No",
            self.selected_date
        );

        let lines: Vec<&str> = message.lines().collect();

        // Account for the popup's left/right borders and a little horizontal
        // padding so the text isn't touching the border.
        let content_width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);

        let popup_width = (content_width as u16 + 4)
            .max(40)
            .min(frame.area().width);

        // Account for:
        // - top/bottom borders
        // - 3 lines of text
        // - one line of vertical breathing room
        let content_height = lines.len() as u16;
        let popup_height = (content_height + 4)
            .max(7)
            .min(frame.area().height);

        let area = centered_rect_fixed(popup_width, popup_height, frame.area());

        frame.render_widget(Clear, area);

        // Center the text vertically by adding blank lines above it.
        let inner_height = area.height.saturating_sub(2);
        let top_padding = inner_height
            .saturating_sub(content_height)
            / 2;

        let message = format!(
            "{}{}",
            "\n".repeat(top_padding as usize),
            message
        );

        frame.render_widget(
            Paragraph::new(message)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false })
                .block(
                    Block::bordered()
                        .title(" Create Note ")
                        .borders(Borders::ALL)
                        .style(Style::default()),
                ),
            area,
        );
    }
}

fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

fn parse_date(value: &str) -> Option<Date> {
    let mut parts = value.split('-');

    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

fn month_start(date: Date) -> Date {
    Date::from_calendar_date(date.year(), date.month(), 1)
        .expect("first day of month is always valid")
}

fn month_difference(from: Date, to: Date) -> i32 {
    (to.year() - from.year()) * 12
        + (to.month() as i32 - from.month() as i32)
}

fn shift_month(date: Date, months: i32) -> Option<Date> {
    let month_index = date
        .year()
        .checked_mul(12)?
        .checked_add(date.month() as i32 - 1)?
        .checked_add(months)?;

    let year = month_index.div_euclid(12);
    let month_number = month_index.rem_euclid(12) + 1;
    let month = Month::try_from(month_number as u8).ok()?;

    // Clamp dates such as January 31 to the last valid day of February.
    (1..=date.day())
        .rev()
        .find_map(|day| Date::from_calendar_date(year, month, day).ok())
}
