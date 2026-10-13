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
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use serde::Deserialize;
use time::{Date, Duration, Month, OffsetDateTime, UtcOffset};

use chrono::{Datelike, NaiveDate, NaiveTime};
use icalendar::{Calendar, Component, Event, EventLike, Property, DatePerhapsTime};

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

struct EventPreview<'a> {
    events: &'a [&'a Event],
}

impl<'a> EventPreview<'a> {
    fn new(events: &'a [&'a Event]) -> Self {
        Self { events }
    }
    fn render(self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .events
            .iter()
            .map(|event| {
                let start = format_event_time(event);
                let end = format_event_end_time(event);
                let summary = event.get_summary().unwrap_or("(untitled)");

                let recurring = event
                    .properties()
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case("RRULE"));

                let recurring_marker = if recurring { " (Recurring)" } else { "" };

                ListItem::new(format!(
                    "{start} - {end} {summary}{recurring_marker}"
                ))
            })
            .collect();

        let items = if items.is_empty() {
            vec![ListItem::new("No events")]
        } else {
            items
        };

        frame.render_widget(
            List::new(items).block(Block::bordered().title(" Events ")),
            area,
        );
    }
}

struct EventRepository {
    dir: PathBuf,
    calendars: Vec<Calendar>,
}

impl EventRepository {
    fn open(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();

        fs::create_dir_all(&dir)
            .with_context(|| {
                format!("failed to create {}", dir.display())
            })?;

        let mut calendars = Vec::new();

        for entry in fs::read_dir(&dir)
            .with_context(|| {
                format!("failed to read {}", dir.display())
            })?
        {
            let entry = entry?;
            let path = entry.path();

            if !entry.file_type()?.is_file()
                || path.extension().is_none_or(|ext| ext != "ics")
            {
                continue;
            }

            let contents = fs::read_to_string(&path)
                .with_context(|| {
                    format!("failed to read {}", path.display())
                })?;

            let calendar = contents
                .parse::<Calendar>()
                .map_err(|e| {
                    eyre!(
                        "failed to parse {}: {e}",
                        path.display()
                    )
                })?;

            calendars.push(calendar);
        }

        Ok(Self { dir, calendars })
    }

    fn add(&mut self, event: Event) -> Result<()> {
        // Use the event UID as the filename.
        let uid = event
            .properties()
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("UID"))
            .map(|(_, property)| property.value())
            .ok_or_else(|| eyre!("event has no UID"))?;

        // Prevent an arbitrary UID from escaping JOURNAL_ICAL_DIR.
        if uid.is_empty()
            || !uid.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
            })
        {
            return Err(eyre!("event UID is not a valid filename"));
        }

        let path = self.dir.join(format!("{uid}.ics"));

        if path.exists() {
            return Err(eyre!(
                "an event file already exists: {}",
                path.display()
            ));
        }

        let mut calendar = Calendar::new();
        calendar.push(event);

        fs::write(&path, calendar.to_string())
            .with_context(|| {
                format!("failed to write {}", path.display())
            })?;

        // Only update the in-memory state after the write succeeds.
        self.calendars.push(calendar);

        Ok(())
    }

    fn on_date(&self, date: Date) -> Vec<&Event> {
        let Some(date) = NaiveDate::from_ymd_opt(
            date.year(),
            date.month() as u32,
            date.day() as u32,
        ) else {
            return Vec::new();
        };

        self.calendars
            .iter()
            .flat_map(|calendar| calendar.events())
            .filter(|event| event_occurs_on(event, date))
            .collect()
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
    events: EventRepository,
    event_form: Option<EventForm>,
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

    let event_repository =
        EventRepository::open(calendar_dir())?;

    let mut app = App::new(notes, event_repository, !no_confirmation_prompt)?;

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
    fn new(
        notes: Vec<Note>,
        events: EventRepository,
        confirmation_prompt: bool,
    ) -> Result<Self> {
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

         // Always start with today selected, regardless of whether a note
         // exists for today or whether notes exist in the future.
         let selected_date = local_today();
         let calendar_start = month_start(selected_date);


        Ok(Self {
            notes: notes_by_date,
            selected_date,
            calendar_start,
            calendar_columns: 1,
            calendar_rows: 1,
            picked_path: None,
            repository: NoteRepository::new()?,
            events,
            event_form: None,
            confirmation_prompt,
            confirm_create: false,
        })
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                if self.event_form.is_some() {
                    if key.code == KeyCode::Esc {
                        self.event_form = None;
                        continue;
                    }

                    if key.code == KeyCode::Enter {
                        let form = self.event_form.take().unwrap();

                        let result = form
                            .to_event()
                            .and_then(|event| self.events.add(event));

                        if let Err(error) = result {
                            return Err(io::Error::other(error));
                        }

                        continue;
                    }

                    if let Some(form) = self.event_form.as_mut() {
                        form.handle_key(key.code);
                    }

                    continue;
                }
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
                    KeyCode::Char('a') => {
                        self.event_form = Some(EventForm::new(self.selected_date));
                    }
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

         const TODAY: Style = Style::new()
             .fg(Color::Black)
             .bg(Color::Yellow)
             .add_modifier(Modifier::BOLD);

        const SELECTED: Style = Style::new()
            .fg(Color::White)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD);

        let mut events = CalendarEventStore::default();

        for date in self.notes.keys() {
            events.add(*date, NOTE);
        }

         // Add today after note events so it remains visibly different even
         // when today already has a note.
         events.add(local_today(), TODAY);

         // Add the selection last so the red cursor takes precedence over
         // both note and today styles.
        events.add(self.selected_date, SELECTED);

        events
    }

    fn render(&mut self, frame: &mut Frame) {
        const MONTH_WIDTH: u16 = 22;
        const MONTH_HEIGHT: u16 = 9;

        let [calendar_area, details_area] =
            frame.area().layout(&Layout::horizontal([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ]));

        let [note_preview_area, event_preview_area] =
            details_area.layout(&Layout::vertical([
                Constraint::Percentage(75),
                Constraint::Percentage(25),
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

        let note_preview = self
            .notes
            .get(&self.selected_date)
            .map(|note| note.raw_content.as_str())
            .unwrap_or("(no note for this date)");

        frame.render_widget(
            Paragraph::new(note_preview)
                .block(Block::bordered().title(" Note Preview "))
                .wrap(Wrap { trim: false }),
            note_preview_area,
        );

        let events = self.events.on_date(self.selected_date);

        EventPreview::new(&events).render(frame, event_preview_area);

        if let Some(form) = &self.event_form {
            self.render_event_form(frame, form);
        }

        if self.confirm_create {
            self.render_confirmation_popup(frame);
        }
    }

    fn render_event_form(&self, frame: &mut Frame, form: &EventForm) {
        use ratatui::layout::Margin;

        let area = centered_rect_fixed(64, 28, frame.area());

        frame.render_widget(Clear, area);

        frame.render_widget(
            Block::bordered()
                .title(" Add Event ")
                .border_style(Style::default()),
            area,
        );

        let inner = area.inner(Margin::new(1, 1));

        let rows = Layout::vertical([
            Constraint::Length(3), // Summary
            Constraint::Length(3), // Date
            Constraint::Length(3), // Start
            Constraint::Length(3), // End
            Constraint::Length(3), // Location
            Constraint::Length(3), // Recurrence toggle
            Constraint::Length(3), // Frequency
            Constraint::Length(3), // Until
        ])
        .split(inner);

        let labels = [
            ("Summary", form.fields[0].as_str()),
            ("Date (YYYY-MM-DD)", form.fields[1].as_str()),
            ("Start (HH:MM)", form.fields[2].as_str()),
            ("End (HH:MM)", form.fields[3].as_str()),
            ("Location", form.fields[4].as_str()),
            (
                "Repeat",
                if form.recurring { "Yes" } else { "No" },
            ),
            (
                "Frequency (Left/Right)",
                ["Daily", "Weekly", "Monthly", "Yearly"]
                    [form.frequency],
            ),
            ("Repeat until", form.fields[5].as_str()),
        ];

        for (index, (label, value)) in labels.iter().enumerate() {
            let focused = index == form.focus;
            let recurrence_setting = matches!(index, 6 | 7);
            let disabled = recurrence_setting && !form.recurring;

            let style = if disabled {
                Style::default().fg(Color::DarkGray)
            } else if focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let text = if focused && matches!(index, 0..=4 | 7) {
                format!("{value}_")
            } else {
                value.to_string()
            };

            frame.render_widget(
                Paragraph::new(text)
                    .style(style)
                    .block(
                        Block::bordered()
                            .title(*label)
                            .border_style(style),
                    ),
                rows[index],
            );
        }

        frame.render_widget(
            Paragraph::new("Tab: next  Enter: save  Esc: cancel")
                .alignment(Alignment::Center),
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            },
        );
    }

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

fn local_today() -> Date {
    let now = OffsetDateTime::now_utc();

    UtcOffset::current_local_offset()
        .map(|offset| now.to_offset(offset).date())
        .unwrap_or_else(|_| now.date())
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

struct EventForm {
    fields: Vec<String>,
    focus: usize,
    recurring: bool,
    frequency: usize,
}

impl Default for EventForm {
    fn default() -> Self {
        Self {
            fields: vec![
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            focus: 0,
            recurring: false,
            frequency: 1, // Weekly
        }
    }
}

impl EventForm {
    const FIELD_COUNT: usize = 8;

    fn new(date: Date) -> Self {
        let mut form = Self::default();
        form.fields[1] = date.to_string();
        form
    }

    fn move_focus(&mut self, direction: i32) {
    loop {
        self.focus = if direction > 0 {
            (self.focus + 1) % Self::FIELD_COUNT
        } else {
            (self.focus + Self::FIELD_COUNT - 1) % Self::FIELD_COUNT
        };

        // Frequency (6) and Repeat until (7) require recurrence.
        if self.recurring || !matches!(self.focus, 6 | 7) {
            break;
        }
    }
}

    fn handle_key(&mut self, key: KeyCode) {
        const FREQUENCIES: [&str; 4] = [
            "Daily", "Weekly", "Monthly", "Yearly",
        ];

        match key {
            KeyCode::Tab | KeyCode::Down => {
                self.move_focus(1);
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.move_focus(-1);
            }
            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right if self.focus == 5 => {
                self.recurring = !self.recurring;
            }
            KeyCode::Char('y') if self.focus == 5 => {
                self.recurring = true;
            }
            KeyCode::Char('n') if self.focus == 5 => {
                self.recurring = false;
            }
            KeyCode::Left | KeyCode::Right
                if self.focus == 6 && self.recurring =>
            {
                if key == KeyCode::Right {
                    self.frequency = (self.frequency + 1) % 4;
                } else {
                    self.frequency = (self.frequency + 3) % 4;
                }
            }
            KeyCode::Backspace => {
                if let Some(field) = self.editable_field_mut() {
                    field.pop();
                }
            }
            KeyCode::Char(c) if !c.is_control() => {
                if self.focus == 6 {
                    let current = FREQUENCIES[self.frequency];
                    let _ = current;
                } else if let Some(field) = self.editable_field_mut() {
                    field.push(c);
                }
            }
            _ => {}
        }
    }

    fn editable_field_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            0..=4 => self.fields.get_mut(self.focus),
            7 => self.fields.get_mut(5),
            _ => None,
        }
    }

    fn to_event(&self) -> Result<Event> {
        let summary = self.fields[0].trim();
        if summary.is_empty() {
            return Err(eyre!("event summary cannot be empty"));
        }

        let date = NaiveDate::parse_from_str(
            &self.fields[1],
            "%Y-%m-%d",
        )
        .context("date must be YYYY-MM-DD")?;

        let start_input = self.fields[2].trim();
        let end_input = self.fields[3].trim();

        let start_time = if start_input.is_empty() {
            None
        } else {
            Some(
                NaiveTime::parse_from_str(start_input, "%H:%M")
                    .context("start time must be HH:MM")?,
            )
        };

        let end_time = if end_input.is_empty() {
            None
        } else {
            Some(
                NaiveTime::parse_from_str(end_input, "%H:%M")
                    .context("end time must be HH:MM")?,
            )
        };

        let uid = uuid::Uuid::new_v4().to_string();
        let mut event = Event::with_uid(&uid);

        event.summary(summary);

        match (start_time, end_time) {
            (Some(start_time), Some(end_time)) => {
                let start = date.and_time(start_time);
                let end = date.and_time(end_time);

                if end <= start {
                    return Err(eyre!(
                        "end time must be after start time"
                    ));
                }

                event.starts(start).ends(end);
            }

            (Some(start_time), None) => {
                // A timed event without DTEND.
                event.starts(date.and_time(start_time));
            }

            (None, Some(_)) => {
                return Err(eyre!(
                    "an end time requires a start time"
                ));
            }

            (None, None) => {
                // All-day event. DTEND is exclusive.
                let next_day = date
                    .checked_add_signed(chrono::Duration::days(1))
                    .ok_or_else(|| eyre!("event date is out of range"))?;

                event.starts(date).ends(next_day);
            }
        }

        let location = self.fields[4].trim();
        if !location.is_empty() {
            event.location(location);
        }

        if self.recurring {
            let until = NaiveDate::parse_from_str(
                &self.fields[5],
                "%Y-%m-%d",
            )
            .context("repeat-until date must be YYYY-MM-DD")?;

            if until < date {
                return Err(eyre!(
                    "repeat-until date cannot precede the event"
                ));
            }

            let frequency = [
                "DAILY", "WEEKLY", "MONTHLY", "YEARLY",
            ][self.frequency];

            // DATE DTSTART requires a DATE UNTIL value; timed events
            // use a floating DATE-TIME UNTIL value here.
            let until_value = if start_time.is_some() {
                format!("{}T235959", until.format("%Y%m%d"))
            } else {
                until.format("%Y%m%d").to_string()
            };

            event.append_property(Property::new(
                "RRULE",
                format!("FREQ={frequency};UNTIL={until_value}"),
            ));
        }

        Ok(event)
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

fn calendar_dir() -> PathBuf {
    if let Some(dir) = env::var_os("JOURNAL_ICAL_DIR") {
        return PathBuf::from(dir);
    }

    let notebook_dir = env::var_os("ZK_NOTEBOOK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Notes")
        });

    let events_dir = env::var_os("ZK_NOTEBOOK_EVENTS_DIR_NAME")
        .unwrap_or_else(|| "events".into());

    notebook_dir.join(events_dir)
}

fn format_event_time(event: &Event) -> String {
    match event.get_start() {
        Some(DatePerhapsTime::DateTime(start)) => match start {
            icalendar::CalendarDateTime::Floating(date_time) => {
                date_time.format("%H:%M").to_string()
            }
            icalendar::CalendarDateTime::Utc(date_time) => {
                date_time.format("%H:%M").to_string()
            }
            icalendar::CalendarDateTime::WithTimezone {
                date_time, ..
            } => date_time.format("%H:%M").to_string(),
        },
        Some(DatePerhapsTime::Date(_)) => "--:--".to_string(),
        None => "--:--".to_string(),
    }
}

fn format_event_start_date(event: &Event) -> Option<NaiveDate> {
    match event.get_start()? {
        DatePerhapsTime::Date(date) => Some(date),

        DatePerhapsTime::DateTime(
            icalendar::CalendarDateTime::Floating(date_time),
        ) => Some(date_time.date()),

        DatePerhapsTime::DateTime(
            icalendar::CalendarDateTime::Utc(date_time),
        ) => Some(date_time.naive_utc().date()),

        DatePerhapsTime::DateTime(
            icalendar::CalendarDateTime::WithTimezone {
                date_time, ..
            },
        ) => Some(date_time.date()),
    }
}

fn format_event_end_time(event: &Event) -> String {
    match event.get_end() {
        Some(DatePerhapsTime::DateTime(end)) => match end {
            icalendar::CalendarDateTime::Floating(date_time) => {
                date_time.format("%H:%M").to_string()
            }
            icalendar::CalendarDateTime::Utc(date_time) => {
                date_time.format("%H:%M").to_string()
            }
            icalendar::CalendarDateTime::WithTimezone {
                date_time, ..
            } => date_time.format("%H:%M").to_string(),
        },
        Some(DatePerhapsTime::Date(_)) => "--:--".to_string(),
        None => "--:--".to_string(),
    }
}

fn event_occurs_on(event: &Event, date: NaiveDate) -> bool {
    let Some(start) = format_event_start_date(event) else {
        return false;
    };

    // An event's DTSTART is always an occurrence.
    if date == start {
        return true;
    }

    // Never show an event before its DTSTART.
    if date < start {
        return false;
    }

    let Some(rrule) = event
        .properties()
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("RRULE"))
        .map(|(_, property)| property.value())
    else {
        return false;
    };

    let mut frequency = None;
    let mut interval = 1_i64;
    let mut until = None;

    for part in rrule.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };

        match key {
            "FREQ" => frequency = Some(value),
            "INTERVAL" => {
                let Ok(value) = value.parse::<i64>() else {
                    return false;
                };

                if value < 1 {
                    return false;
                }

                interval = value;
            }
            "UNTIL" => {
                // DTSTART is a DATE for all-day events and a DATE-TIME
                // for timed events. We compare the recurrence by date.
                until = NaiveDate::parse_from_str(value, "%Y%m%d")
                    .ok()
                    .or_else(|| {
                        NaiveDate::parse_from_str(
                            value.get(..8)?,
                            "%Y%m%d",
                        )
                        .ok()
                    });
            }
            _ => {}
        }
    }

    if until.is_some_and(|until| date > until) {
        return false;
    }

    let days = (date - start).num_days();
    let months = i64::from(date.year() - start.year()) * 12
        + i64::from(date.month() as i32 - start.month() as i32);
    let years = i64::from(date.year() - start.year());

    match frequency {
        Some("DAILY") => days % interval == 0,

        Some("WEEKLY") => {
            days % (7 * interval) == 0
        }

        Some("MONTHLY") => {
            months >= 0
                && months % interval == 0
                && date.day() == start.day()
        }

        Some("YEARLY") => {
            years >= 0
                && years % interval == 0
                && date.month() == start.month()
                && date.day() == start.day()
        }

        _ => false,
    }
}
