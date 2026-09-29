use std::collections::BTreeMap;
use std::io;

use color_eyre::eyre::{eyre, Result};
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use ratatui::{DefaultTerminal, Frame};
use serde::Deserialize;
use time::{Date, Duration, Month};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Note {
    filename_stem: String,
    path: String,
}

struct App {
    notes: BTreeMap<Date, String>,
    selected_date: Date,
    picked_path: Option<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    // Read the complete JSON array before starting the TUI.
    let notes: Vec<Note> = {
        let stdin = io::stdin();
        serde_json::from_reader(stdin.lock())?
    };

    let mut app = App::new(notes)?;

    ratatui::run(|terminal| app.run(terminal))?;

    // Ratatui has restored the terminal at this point.
    if let Some(path) = app.picked_path {
        println!("{path}");
    }

    Ok(())
}

impl App {
    fn new(notes: Vec<Note>) -> Result<Self> {
        let mut notes_by_date = BTreeMap::new();

        for note in notes {
            let date = parse_date(&note.filename_stem)
                .ok_or_else(|| eyre!("invalid daily note date: {}", note.filename_stem))?;

            notes_by_date.insert(date, note.path);
        }

        // Begin on the newest available note.
        let selected_date = notes_by_date
            .keys()
            .next_back()
            .copied()
            .ok_or_else(|| eyre!("stdin contained no daily notes"))?;

        Ok(Self {
            notes: notes_by_date,
            selected_date,
            picked_path: None,
        })
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if let Some(key) = event::read()?.as_key_press_event() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),

                    KeyCode::Enter => {
                        if let Some(path) = self.notes.get(&self.selected_date) {
                            self.picked_path = Some(path.clone());
                            return Ok(());
                        }
                    }

                    KeyCode::Char('h') | KeyCode::Left => self.move_days(-1),
                    KeyCode::Char('j') | KeyCode::Down => self.move_days(7),
                    KeyCode::Char('k') | KeyCode::Up => self.move_days(-7),
                    KeyCode::Char('l') | KeyCode::Right => self.move_days(1),

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

    fn move_days(&mut self, days: i64) {
        if let Some(date) = self.selected_date.checked_add(Duration::days(days)) {
            self.selected_date = date;
        }
    }

    fn move_month(&mut self, months: i32) {
        if let Some(date) = shift_month(self.selected_date, months) {
            self.selected_date = date;
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

    fn render(&self, frame: &mut Frame) {
        let selected_note = self
            .notes
            .get(&self.selected_date)
            .map(String::as_str)
            .unwrap_or("(no note for this date)");

        let header = Text::from_iter([
            Line::from("Daily notes — green has a note, red is selected".bold()),
            Line::from(format!(
                "Selected: {} | {}",
                self.selected_date, selected_note
            )),
            Line::from(
                "<Enter> select | <q/Esc> quit | <hjkl/arrows> move | <n/p> month",
            ),
        ]);

        let [header_area, calendar_area] =
            frame.area().layout(&Layout::vertical([
                Constraint::Length(3),
                Constraint::Fill(1),
            ]));

        frame.render_widget(header, header_area);

        let events = self.events();
        let calendar = Monthly::new(self.selected_date, &events)
            .show_month_header(Style::new().bold())
            .show_weekdays_header(Style::new().fg(Color::Cyan))
            .show_surrounding(Style::new().dim());

        frame.render_widget(calendar, calendar_area);
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
