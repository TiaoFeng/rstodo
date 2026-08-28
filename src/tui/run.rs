use crate::{error::AppError, io::storage::TaskStore};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Alignment,
    widgets::{Block, Paragraph},
};
use std::io;

static TEST_TITLE: &str = "RSTODO";
static TEST_TEXT: &str = "We'll meet very soon.";

pub fn run(_store: &TaskStore) -> Result<(), AppError> {
    // 测试代码
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let block = Block::default()
                .title(TEST_TITLE)
                .title_alignment(Alignment::Center);
            let paragraph = Paragraph::new(TEST_TEXT)
                .block(block)
                .alignment(Alignment::Center);
            f.render_widget(paragraph, f.area());
        })?;

        if matches!(event::read()?, Event::Key(_)) {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
