mod database;
mod disk;
mod settings;
mod ui;
mod utils;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::Terminal;
use std::io;
use std::panic;

fn main() -> io::Result<()> {
    panic::set_hook(Box::new(|info| {
        let _ = execute!(io::stderr(), LeaveAlternateScreen, Clear(ClearType::All));
        eprintln!("Panic: {:?}", info);
    }));

    let backend = CrosstermBackend::new(io::stdout());

    enable_raw_mode()?;
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = ui::App::new();
    let res = app.run(&mut terminal);

    disable_raw_mode()?;

    execute!(io::stdout(), LeaveAlternateScreen, Clear(ClearType::All))?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}
