use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};

fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some('[') = chars.next() {
                while let Some(c) = chars.next() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn parse_message_type(msg: &str) -> (String, Color) {
    if msg.starts_with("[RUST1]") {
        (msg.trim_start_matches("[RUST1]").to_string(), Color::Rgb(204, 85, 0))
    } else if msg.starts_with("[RUST2]") {
        (msg.trim_start_matches("[RUST2]").to_string(), Color::Rgb(255, 102, 0))
    } else if msg.starts_with("[RUST3]") {
        (msg.trim_start_matches("[RUST3]").to_string(), Color::Rgb(255, 136, 0))
    } else if msg.starts_with("[RUST4]") {
        (msg.trim_start_matches("[RUST4]").to_string(), Color::Rgb(204, 102, 0))
    } else if msg.starts_with("[RUST5]") {
        (msg.trim_start_matches("[RUST5]").to_string(), Color::Rgb(170, 85, 0))
    } else if msg.starts_with("[RUST6]") {
        (msg.trim_start_matches("[RUST6]").to_string(), Color::Rgb(136, 68, 0))
    } else if msg.starts_with("[RUST7]") {
        (msg.trim_start_matches("[RUST7]").to_string(), Color::Rgb(119, 51, 0))
    } else if msg.starts_with("[ERROR]") || msg.starts_with("✗") {
        (msg.to_string(), Color::Red)
    } else if msg.starts_with("[✓]") || msg.starts_with("[SUCCESS]") {
        (msg.to_string(), Color::Green)
    } else if msg.starts_with("[INFO]") || msg.starts_with("ℹ") {
        (msg.to_string(), Color::Cyan)
    } else if msg.starts_with("[WARNING]") || msg.starts_with("⚠") {
        (msg.to_string(), Color::Yellow)
    } else if msg.starts_with("[DEBUG]") {
        (msg.to_string(), Color::Magenta)
    } else if msg.starts_with("Username:") || msg.starts_with("UUID:") {
        (msg.to_string(), Color::LightBlue)
    } else if msg.starts_with("Connecting") || msg.starts_with("Starting") {
        (msg.to_string(), Color::LightGreen)
    } else if msg.starts_with("Waiting") || msg.starts_with("Loading") {
        (msg.to_string(), Color::LightYellow)
    } else {
        (msg.to_string(), Color::White)
    }
}

const MAX_MESSAGES: usize = 1000;

pub struct TerminalUI {
    messages: Arc<Mutex<VecDeque<String>>>,
    input: String,
    cursor_position: usize,
    prompt: String,
    scroll_offset: usize,
    history: Vec<String>,
    history_index: usize,
}

impl TerminalUI {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_MESSAGES))),
            input: String::new(),
            cursor_position: 0,
            prompt: String::from("> "),
            scroll_offset: 0,
            history: Vec::new(),
            history_index: 0,
        }
    }

    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    pub fn get_message_logger(&self) -> MessageLogger {
        MessageLogger {
            messages: Arc::clone(&self.messages),
        }
    }

    pub async fn run<FInput, Fut, FTab>(
        &mut self,
        mut on_command: FInput,
        mut on_autocomplete: FTab
    ) -> io::Result<()>
    where
        FInput: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<bool, String>>,
        FTab: FnMut(&str, usize) -> Vec<String>,
    {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Ensure cleanup happens even on panic
        let cleanup = Cleanup;
        let result = self.run_loop(&mut terminal, &mut on_command, &mut on_autocomplete).await;
        drop(cleanup);

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop<FInput, Fut, FTab>(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        on_command: &mut FInput,
        on_autocomplete: &mut FTab
    ) -> io::Result<()>
    where
        FInput: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<bool, String>>,
        FTab: FnMut(&str, usize) -> Vec<String>,
    {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    match self.handle_key(key, on_command, on_autocomplete).await {
                        KeyAction::Exit => return Ok(()),
                        KeyAction::Continue => {}
                    }
                }
            }
        }
    }

    async fn handle_key<FInput, Fut, FTab>(
        &mut self,
        key: KeyEvent,
        on_command: &mut FInput,
        on_autocomplete: &mut FTab
    ) -> KeyAction
    where
        FInput: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<bool, String>>,
        FTab: FnMut(&str, usize) -> Vec<String>,
    {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                KeyAction::Exit
            }
            KeyCode::Enter => {
                let cmd = self.input.clone();

                if !cmd.trim().is_empty() {
                    self.history.push(cmd.clone());
                }
                self.history_index = self.history.len();

                self.input.clear();
                self.cursor_position = 0;
                self.scroll_offset = 0;

                match on_command(cmd).await {
                    Ok(true) => KeyAction::Exit,
                    _ => KeyAction::Continue,
                }
            }
            KeyCode::Up => {
                if self.history_index > 0 {
                    self.history_index -= 1;
                    self.input = self.history[self.history_index].clone();
                    self.cursor_position = self.input.len();
                }
                KeyAction::Continue
            }
            KeyCode::Down => {
                if self.history_index < self.history.len() {
                    self.history_index += 1;
                    if self.history_index < self.history.len() {
                        self.input = self.history[self.history_index].clone();
                    } else {
                        self.input.clear();
                    }
                    self.cursor_position = self.input.len();
                }
                KeyAction::Continue
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                KeyAction::Continue
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
                KeyAction::Continue
            }
            KeyCode::Left => {
                if self.cursor_position > 0 { self.cursor_position -= 1; }
                KeyAction::Continue
            }
            KeyCode::Right => {
                if self.cursor_position < self.input.len() { self.cursor_position += 1; }
                KeyAction::Continue
            }
            KeyCode::Tab => {
                let suggestions = on_autocomplete(&self.input, self.cursor_position);
                if !suggestions.is_empty() {
                    self.input = suggestions[0].clone();
                    self.cursor_position = self.input.len();
                }
                KeyAction::Continue
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(5);
                KeyAction::Continue
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
                KeyAction::Continue
            }
            KeyCode::Home => {
                self.cursor_position = 0;
                KeyAction::Continue
            }
            KeyCode::End => {
                self.cursor_position = self.input.len();
                KeyAction::Continue
            }
            _ => KeyAction::Continue,
        }
    }

    fn draw(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(3),
            ])
            .split(f.area());

        let messages = self.messages.lock().unwrap();

        let available_height = chunks[0].height.saturating_sub(2) as usize;
        let total_messages = messages.len();

        let max_scroll = if total_messages > available_height {
            total_messages - available_height
        } else {
            0
        };

        let clamped_scroll = self.scroll_offset.min(max_scroll);

        let start_index = if total_messages > available_height {
            total_messages - available_height - clamped_scroll
        } else {
            0
        };

        let items: Vec<ListItem> = messages
            .iter()
            .skip(start_index)
            .take(available_height)
            .map(|m| {
                let cleaned = strip_ansi_codes(m);
                let (text, color) = parse_message_type(&cleaned);
                ListItem::new(Line::from(Span::styled(text, Style::default().fg(color))))
            })
            .collect();

        let title = if clamped_scroll > 0 {
            format!("R-Term (↑{})", clamped_scroll)
        } else {
            "R-Term".to_string()
        };

        let messages_list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().fg(Color::Cyan)));

        f.render_widget(messages_list, chunks[0]);

        let input_text = format!("{}{}", self.prompt, self.input);
        let input = Paragraph::new(input_text)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Input")
                .style(Style::default().fg(Color::Green)));

        f.render_widget(input, chunks[1]);

        let prompt_display_width = self.prompt.len() as u16;
        let cursor_x = chunks[1].x + prompt_display_width + self.cursor_position as u16 + 1;
        let cursor_y = chunks[1].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

enum KeyAction {
    Continue,
    Exit,
}

#[derive(Clone)]
pub struct MessageLogger {
    pub messages: Arc<Mutex<VecDeque<String>>>,
}

impl MessageLogger {
    pub fn log(&self, message: String) {
        let mut msgs = self.messages.lock().unwrap();

        // Split multi-line messages into separate entries
        for line in message.lines() {
            if msgs.len() >= MAX_MESSAGES {
                msgs.pop_front();
            }
            msgs.push_back(line.to_string());
        }

        // Handle empty messages (like blank lines)
        if message.is_empty() || message == "\n" {
            if msgs.len() >= MAX_MESSAGES {
                msgs.pop_front();
            }
            msgs.push_back(String::new());
        }
    }

    pub fn info(&self, message: &str) {
        self.log(format!("[INFO] {}", message));
    }

    pub fn error(&self, message: &str) {
        self.log(format!("[ERROR] {}", message));
    }

    pub fn success(&self, message: &str) {
        self.log(format!("[SUCCESS] {}", message));
    }

    pub fn warning(&self, message: &str) {
        self.log(format!("[WARNING] {}", message));
    }

    pub fn debug(&self, message: &str) {
        self.log(format!("[DEBUG] {}", message));
    }
}