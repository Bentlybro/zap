use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

pub struct TransferUI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    should_quit: bool,
}

pub struct TransferState {
    pub code: String,
    pub filename: String,
    pub total_size: u64,
    pub transferred: u64,
    pub speed: f64, // bytes per second
    pub encrypted: bool,
    pub status: String,
}

impl TransferUI {
    /// Initialize the TUI
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        
        Ok(Self {
            terminal,
            should_quit: false,
        })
    }
    
    /// Run the TUI with the given transfer state
    pub fn run<F>(&mut self, mut get_state: F) -> Result<()>
    where
        F: FnMut() -> TransferState,
    {
        loop {
            let state = get_state();
            self.terminal.draw(|f| Self::render_ui(f, &state))?;
            
            if self.should_quit || state.status.contains("complete") || state.status.contains("error") {
                break;
            }
            
            // Check for user input (q to quit)
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('q') {
                        self.should_quit = true;
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Render the UI
    fn render_ui(f: &mut Frame, state: &TransferState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(f.area());
        
        // Title
        let title = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("⚡ ", Style::default().fg(Color::Yellow)),
                Span::styled("Zap Transfer", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);
        
        // Code
        let code_text = format!("Transfer Code: {}", state.code);
        let code = Paragraph::new(code_text)
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Code"));
        f.render_widget(code, chunks[1]);
        
        // File info
        let size_mb = state.total_size as f64 / 1_048_576.0;
        let transferred_mb = state.transferred as f64 / 1_048_576.0;
        let speed_mbps = state.speed / 1_048_576.0;
        
        let file_info = format!(
            "{} | {:.2} MB / {:.2} MB | {:.2} MB/s",
            state.filename, transferred_mb, size_mb, speed_mbps
        );
        let file = Paragraph::new(file_info)
            .block(Block::default().borders(Borders::ALL).title("File"));
        f.render_widget(file, chunks[2]);
        
        // Progress bar
        let progress = if state.total_size > 0 {
            (state.transferred as f64 / state.total_size as f64).min(1.0)
        } else {
            0.0
        };
        
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent((progress * 100.0) as u16)
            .label(format!("{:.1}%", progress * 100.0));
        f.render_widget(gauge, chunks[3]);
        
        // Status
        let encryption_icon = if state.encrypted { "🔒" } else { "🔓" };
        let status_text = format!("{} {}", encryption_icon, state.status);
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, chunks[4]);
    }
    
    /// Clean up the TUI
    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TransferUI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Simple progress bar for non-TUI mode
pub fn print_progress(filename: &str, transferred: u64, total: u64, speed: f64) {
    let progress = if total > 0 {
        (transferred as f64 / total as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    
    let speed_mbps = speed / 1_048_576.0;
    let transferred_mb = transferred as f64 / 1_048_576.0;
    let total_mb = total as f64 / 1_048_576.0;
    
    print!(
        "\r{}: {:.1}% ({:.2}/{:.2} MB) @ {:.2} MB/s   ",
        filename, progress, transferred_mb, total_mb, speed_mbps
    );
    
    use std::io::Write;
    io::stdout().flush().unwrap();
}
