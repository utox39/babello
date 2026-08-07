use std::{
    collections::HashMap,
    error::Error,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, ListState},
};
use ratatui_textarea::TextArea;
use reqwest::blocking::Client;

use crate::{Babello, DeepLTranslation, DeepLWriteImprovement, WRITE_SUPPORTED_LANGUAGES};

/// The label used for the "let DeepL auto-detect" list entry
const AUTO: &str = "Auto";

/// Launch the interactive TUI
pub(crate) fn run(
    client: Client,
    api_key: String,
    languages: HashMap<&'static str, &'static str>,
) -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::try_init()?;
    let mut app = App::new(client, api_key, languages);

    let result = event_loop(&mut terminal, &mut app);

    ratatui::try_restore()?;
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(150))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }

        app.poll_worker();

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Translate,
    Improve,
}

#[derive(PartialEq, Eq)]
enum Mode {
    Normal,
    Editing,
}

#[derive(Clone, Copy)]
enum PopupKind {
    SourceLang,
    TargetLang,
}

/// A floating, filterable language-selection list
struct LangPopup {
    kind: PopupKind,
    /// All (code, name) pairs available for this popup, "Auto" first when applicable
    items: Vec<(String, String)>,
    filter: String,
    /// Indices into `items` matching the current filter
    filtered: Vec<usize>,
    state: ListState,
}

impl LangPopup {
    fn new(kind: PopupKind, items: Vec<(String, String)>) -> Self {
        let filtered = (0..items.len()).collect();
        let mut state = ListState::default();
        state.select(if items.is_empty() { None } else { Some(0) });
        Self {
            kind,
            items,
            filter: String::new(),
            filtered,
            state,
        }
    }

    fn refilter(&mut self) {
        let needle = self.filter.to_lowercase();
        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, (code, name))| {
                needle.is_empty()
                    || code.to_lowercase().contains(&needle)
                    || name.to_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect();
        self.state.select(if self.filtered.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    fn select_next(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let next = match self.state.selected() {
            Some(i) if i + 1 < self.filtered.len() => i + 1,
            _ => 0,
        };
        self.state.select(Some(next));
    }

    fn select_prev(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let prev = match self.state.selected() {
            Some(0) | None => self.filtered.len() - 1,
            Some(i) => i - 1,
        };
        self.state.select(Some(prev));
    }

    fn selected(&self) -> Option<&(String, String)> {
        let i = self.state.selected()?;
        let idx = *self.filtered.get(i)?;
        self.items.get(idx)
    }
}

/// Per-tab textarea pair and chosen languages
struct TabState {
    input: TextArea<'static>,
    /// Populated with the API response; never receives key events
    output: TextArea<'static>,
    /// Translate tab only; `None` means "Auto" (delegate detection to DeepL)
    source_lang: Option<String>,
    /// Translate: required before submit. Improve: `None` means "Auto" (omit target_lang)
    target_lang: Option<String>,
    /// Source language DeepL detected for the last successful request
    detected: Option<String>,
}

impl TabState {
    fn new(input_title: &str, output_title: &str) -> Self {
        let mut input = TextArea::default();
        input.set_block(Block::bordered().title(input_title.to_string()));

        let mut output = TextArea::default();
        output.set_block(Block::bordered().title(output_title.to_string()));

        Self {
            input,
            output,
            source_lang: None,
            target_lang: None,
            detected: None,
        }
    }

    fn input_text(&self) -> String {
        self.input.lines().join("\n")
    }

    fn set_output(&mut self, text: &str, title: &str) {
        let lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
        let mut output = TextArea::new(lines);
        output.set_block(Block::bordered().title(title.to_string()));
        self.output = output;
    }
}

enum WorkerMsg {
    Translate(Result<Vec<DeepLTranslation>, String>),
    Improve(Result<Vec<DeepLWriteImprovement>, String>),
}

struct App {
    tab: Tab,
    mode: Mode,
    translate: TabState,
    improve: TabState,
    popup: Option<LangPopup>,
    loading: bool,
    error: Option<String>,
    should_quit: bool,
    worker_rx: Option<Receiver<WorkerMsg>>,
    client: Client,
    api_key: String,
    languages: HashMap<&'static str, &'static str>,
}

impl App {
    fn new(
        client: Client,
        api_key: String,
        languages: HashMap<&'static str, &'static str>,
    ) -> Self {
        let mut translate = TabState::new("Input", "Translation");
        translate.target_lang = Some("EN-US".to_string());

        Self {
            tab: Tab::Translate,
            mode: Mode::Normal,
            translate,
            improve: TabState::new("Input", "Improved text"),
            popup: None,
            loading: false,
            error: None,
            should_quit: false,
            worker_rx: None,
            client,
            api_key,
            languages,
        }
    }

    fn active_tab_state(&mut self) -> &mut TabState {
        match self.tab {
            Tab::Translate => &mut self.translate,
            Tab::Improve => &mut self.improve,
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        if self.popup.is_some() {
            self.on_popup_key(key);
            return;
        }

        if self.error.is_some() && self.mode == Mode::Normal {
            self.error = None;
        }

        match self.mode {
            Mode::Normal => self.on_normal_key(key),
            Mode::Editing => self.on_editing_key(key),
        }
    }

    fn on_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => {
                self.tab = match self.tab {
                    Tab::Translate => Tab::Improve,
                    Tab::Improve => Tab::Translate,
                };
            }
            KeyCode::Char('i') | KeyCode::Enter => self.mode = Mode::Editing,
            KeyCode::Char('s') if self.tab == Tab::Translate => {
                self.open_popup(PopupKind::SourceLang);
            }
            KeyCode::Char('t') => self.open_popup(PopupKind::TargetLang),
            KeyCode::Char('r') => self.submit(),
            _ => {}
        }
    }

    fn on_editing_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {
                self.active_tab_state().input.input(key);
            }
        }
    }

    fn on_popup_key(&mut self, key: KeyEvent) {
        let Some(popup) = self.popup.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.popup = None,
            KeyCode::Enter => self.apply_popup_selection(),
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                popup.select_prev();
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                popup.select_next();
            }
            KeyCode::Up => popup.select_prev(),
            KeyCode::Down => popup.select_next(),
            KeyCode::Backspace => {
                popup.filter.pop();
                popup.refilter();
            }
            KeyCode::Char(c) => {
                popup.filter.push(c);
                popup.refilter();
            }
            _ => {}
        }
    }

    fn open_popup(&mut self, kind: PopupKind) {
        let items = match kind {
            PopupKind::SourceLang => sorted_lang_items(&self.languages, None, true),
            PopupKind::TargetLang => match self.tab {
                Tab::Translate => sorted_lang_items(&self.languages, None, false),
                Tab::Improve => {
                    sorted_lang_items(&self.languages, Some(WRITE_SUPPORTED_LANGUAGES), true)
                }
            },
        };
        self.popup = Some(LangPopup::new(kind, items));
    }

    fn apply_popup_selection(&mut self) {
        let Some(popup) = self.popup.take() else {
            return;
        };
        let Some((code, _)) = popup.selected() else {
            return;
        };
        let value = if code == AUTO {
            None
        } else {
            Some(code.clone())
        };
        match popup.kind {
            PopupKind::SourceLang => {
                self.translate.source_lang = value;
                self.translate.detected = None;
            }
            PopupKind::TargetLang => match self.tab {
                Tab::Translate => self.translate.target_lang = value,
                Tab::Improve => self.improve.target_lang = value,
            },
        }
    }

    fn submit(&mut self) {
        if self.loading {
            return;
        }

        let text = self.active_tab_state().input_text();
        if text.trim().is_empty() {
            self.error = Some("Input text is empty".to_string());
            return;
        }

        match self.tab {
            Tab::Translate => {
                let Some(target_lang) = self.translate.target_lang.clone() else {
                    self.error = Some("Choose a target language first (press 't')".to_string());
                    return;
                };
                let source_lang = self.translate.source_lang.clone();
                self.worker_rx = Some(spawn_translate(
                    self.client.clone(),
                    self.api_key.clone(),
                    text,
                    source_lang,
                    target_lang,
                ));
            }
            Tab::Improve => {
                let target_lang = self.improve.target_lang.clone();
                self.worker_rx = Some(spawn_improve(
                    self.client.clone(),
                    self.api_key.clone(),
                    text,
                    target_lang,
                ));
            }
        }

        self.loading = true;
        self.error = None;
    }

    fn poll_worker(&mut self) {
        let Some(rx) = &self.worker_rx else {
            return;
        };

        let msg = match rx.try_recv() {
            Ok(msg) => msg,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.loading = false;
                self.worker_rx = None;
                return;
            }
        };

        self.loading = false;
        self.worker_rx = None;

        match msg {
            WorkerMsg::Translate(Ok(translations)) => {
                let detected = translations
                    .first()
                    .map(|t| t.detected_source_language.clone());
                let text = translations
                    .into_iter()
                    .map(|t| t.text)
                    .collect::<Vec<_>>()
                    .join("\n");
                self.translate.set_output(&text, "Translation");
                self.translate.detected = detected;
            }
            WorkerMsg::Translate(Err(e)) => self.error = Some(e),
            WorkerMsg::Improve(Ok(improvements)) => {
                let detected = improvements
                    .first()
                    .map(|i| i.detected_source_language.clone());
                let text = improvements
                    .into_iter()
                    .map(|i| i.text)
                    .collect::<Vec<_>>()
                    .join("\n");
                self.improve.set_output(&text, "Improved text");
                self.improve.detected = detected;
            }
            WorkerMsg::Improve(Err(e)) => self.error = Some(e),
        }
    }
}

/// Build the sorted (code, name) list for a language popup.
///
/// `codes` restricts the list to a specific subset (used for the Improve tab's target
/// language, which only supports `WRITE_SUPPORTED_LANGUAGES`); `None` uses every known language.
fn sorted_lang_items(
    languages: &HashMap<&'static str, &'static str>,
    codes: Option<&'static [&'static str]>,
    include_auto: bool,
) -> Vec<(String, String)> {
    let mut items: Vec<(String, String)> = match codes {
        Some(codes) => codes
            .iter()
            .filter_map(|code| {
                languages
                    .get(code)
                    .map(|name| (code.to_string(), name.to_string()))
            })
            .collect(),
        None => languages
            .iter()
            .map(|(code, name)| (code.to_string(), name.to_string()))
            .collect(),
    };
    items.sort_by(|a, b| a.0.cmp(&b.0));

    if include_auto {
        items.insert(0, (AUTO.to_string(), "Detect automatically".to_string()));
    }

    items
}

fn spawn_translate(
    client: Client,
    api_key: String,
    text: String,
    source_lang: Option<String>,
    target_lang: String,
) -> Receiver<WorkerMsg> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let babello = Babello {
            client: &client,
            api_key: &api_key,
            text: text.lines().collect(),
            source_lang: source_lang.as_deref(),
            target_lang: &target_lang,
        };
        let result = babello.translate().map_err(|e| e.to_string());
        let _ = tx.send(WorkerMsg::Translate(result));
    });
    rx
}

fn spawn_improve(
    client: Client,
    api_key: String,
    text: String,
    target_lang: Option<String>,
) -> Receiver<WorkerMsg> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let babello = Babello {
            client: &client,
            api_key: &api_key,
            text: text.lines().collect(),
            source_lang: None,
            target_lang: target_lang.as_deref().unwrap_or_default(),
        };
        let result = babello.improve().map_err(|e| e.to_string());
        let _ = tx.send(WorkerMsg::Improve(result));
    });
    rx
}

mod ui {
    use ratatui::Frame;

    use super::{App, Constraint, Layout, Mode, Rect, Tab};

    pub(super) fn draw(f: &mut Frame, app: &mut App) {
        let area = f.area();
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

        draw_tab_bar(f, chunks[0], app);

        match app.tab {
            Tab::Translate => draw_tab_body(f, chunks[1], app, true),
            Tab::Improve => draw_tab_body(f, chunks[1], app, false),
        }

        draw_status_bar(f, chunks[2], app);

        if app.popup.is_some() {
            draw_popup(f, app);
        }
    }

    fn draw_tab_bar(f: &mut Frame, area: Rect, app: &App) {
        use ratatui::{
            style::{Modifier, Style},
            widgets::Tabs,
        };

        let selected = match app.tab {
            Tab::Translate => 0,
            Tab::Improve => 1,
        };

        let tabs = Tabs::new(vec!["Translate", "Improve"])
            .select(selected)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_widget(tabs, area);
    }

    fn draw_tab_body(f: &mut Frame, area: Rect, app: &mut App, has_source_lang: bool) {
        use ratatui::{text::Line, widgets::Paragraph};

        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(area);

        let tab_state = match app.tab {
            Tab::Translate => &app.translate,
            Tab::Improve => &app.improve,
        };

        let target = tab_state.target_lang.as_deref().unwrap_or(super::AUTO);
        let mut header = if has_source_lang {
            let source = tab_state.source_lang.as_deref().unwrap_or(super::AUTO);
            format!("Source: {source}   Target: {target}")
        } else {
            format!("Target: {target}")
        };
        if let Some(detected) = &tab_state.detected {
            header.push_str(&format!("   (detected: {detected})"));
        }
        f.render_widget(Paragraph::new(Line::from(header)), chunks[0]);

        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let tab_state = match app.tab {
            Tab::Translate => &app.translate,
            Tab::Improve => &app.improve,
        };
        f.render_widget(&tab_state.input, cols[0]);
        f.render_widget(&tab_state.output, cols[1]);
    }

    fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
        use ratatui::{
            style::{Color, Style},
            text::Line,
            widgets::Paragraph,
        };

        let (text, style) = if let Some(error) = &app.error {
            (format!("Error: {error}"), Style::default().fg(Color::Red))
        } else if app.loading {
            ("Working…".to_string(), Style::default().fg(Color::Yellow))
        } else {
            let hint = match app.mode {
                Mode::Normal => {
                    "i: edit  s: source lang  t: target lang  r: run  Tab: switch tab  q: quit"
                }
                Mode::Editing => "Esc: stop editing",
            };
            (hint.to_string(), Style::default())
        };

        f.render_widget(Paragraph::new(Line::from(text).style(style)), area);
    }

    fn draw_popup(f: &mut Frame, app: &mut App) {
        use ratatui::{
            style::{Modifier, Style},
            widgets::{Block, Clear, List, ListItem},
        };

        let Some(popup) = app.popup.as_mut() else {
            return;
        };

        let area = centered_rect(50, 70, f.area());
        f.render_widget(Clear, area);

        let title = match popup.kind {
            super::PopupKind::SourceLang => "Source Language",
            super::PopupKind::TargetLang => "Target Language",
        };
        let block = Block::bordered().title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);

        f.render_widget(
            ratatui::widgets::Paragraph::new(format!("/{}", popup.filter)),
            chunks[0],
        );

        let items: Vec<ListItem> = popup
            .filtered
            .iter()
            .map(|&i| {
                let (code, name) = &popup.items[i];
                ListItem::new(format!("{code} — {name}"))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        f.render_stateful_widget(list, chunks[1], &mut popup.state);
    }

    fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let vertical = Layout::vertical([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

        Layout::horizontal([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
    }
}
