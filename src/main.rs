use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame, Terminal,
};

struct App<'a> {
    state: TableState,
    items: Vec<Vec<&'a str>>,
}

impl<'a> App<'a> {
    fn new() -> App<'a> {
        App {
            state: TableState::default(),
            items: vec![
                vec![
                    "58467808fc5",
                    "LOTUS-2268 Do not add () after AND, OR and NOT in autocomplete",
                    "84 minutes ago",
                ],
                vec![
                    "58467808fc5",
                    "LOTUS-2268 Refactor getAnaplanLanguageObject for Hyperblock vs Polaris cores",
                    "68 minutes ago",
                ],
                vec![
                    "d53434e6200",
                    "LOTUS-2280 No . after list names in autocompletion (#36674)",
                    "24 hours ago",
                ],
                vec![
                    "7febc16b4a9",
                    "LOTUS-2281 Rename modal wrapper (#36620)",
                    "2 days ago",
                ],
                vec!["0f35fd83b58", "LOTUS-2211 Remove unused type", "2 days ago"],
                vec![
                    "1c322a45977",
                    "LOTUS-2210 Use renderWrappedHook in useGlobalNavFader tests",
                    "52 days ago",
                ],
                vec![
                    "ee1e5fdc922",
                    "LOTUS-2010 Fade gnav when modal shows",
                    "7 days ago",
                ],
                vec![
                    "7a71891e734",
                    "LOTUS-2010 Remove enabled checkbox",
                    "7 days ago",
                ],
            ],
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let rects = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .margin(5)
        .split(f.size());

    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default().bg(Color::Blue);
    let header_cells =
        ["/Users/edward.andrewshodgson/Developer/work/dashboards-and-visualisations"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Red)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1)
        .bottom_margin(1);
    let rows = app.items.iter().map(|item| {
        let height = item
            .iter()
            .map(|content| content.chars().filter(|c| *c == '\n').count())
            .max()
            .unwrap_or(0)
            + 1;
        let cells = item.iter().map(|c| Cell::from(*c));
        Row::new(cells).height(height as u16).bottom_margin(1)
    });
    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Today"))
        .highlight_style(selected_style)
        .widths(&[
            Constraint::Length(10),
            Constraint::Min(80),
            Constraint::Min(10),
        ]);
    f.render_stateful_widget(t, rects[0], &mut app.state);
}
