use anyhow::{Context, Result};
use crossterm::cursor::{Hide, Show};
use crossterm::event::EnableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use kurl::app::App;
use kurl::config::Config;
use kurl::events::{Action, Event, EventHandler};
use kurl::ui;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::panic;
use tokio::sync::mpsc;

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)
        .context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("Failed to create Ratatui terminal")
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
        Show
    )
    .context("Failed to restore terminal state")?;
    terminal.show_cursor().context("Failed to show cursor")?;
    Ok(())
}

fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout(),
            LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            Show
        );
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    install_panic_hook();

    let args: Vec<String> = std::env::args().collect();
    let explicit_dir = args.get(1).and_then(|arg| {
        if arg.starts_with('-') {
            None
        } else {
            Some(std::path::PathBuf::from(arg))
        }
    });
    let config = Config::new(explicit_dir);
    let tick_duration = config.tick_duration();

    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
    let mut event_handler = EventHandler::new(tick_duration);
    let mut app = App::new(config, action_tx.clone())?;

    let mut terminal = setup_terminal()?;
    if let Ok(size) = terminal.size() {
        app.term_width = size.width;
    }

    while !app.should_quit {
        terminal
            .draw(|frame| ui::draw(frame, &app))
            .context("Failed to render frame")?;

        tokio::select! {
            Some(action) = action_rx.recv() => {
                app.update(action)?;
            }
            maybe_event = event_handler.next() => {
                if let Some(event) = maybe_event {
                    match event {
                        Event::Key(key) => {
                            if key.kind == crossterm::event::KeyEventKind::Press {
                                if let Some(action) = app.handle_key_event(key) {
                                    app.update(action)?;
                                }
                            }
                        }
                        Event::Mouse(mouse) => {
                            let size = terminal.size().context("Failed to get terminal size")?;
                            let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                            if let Some(action) = app.handle_mouse_event(mouse, area) {
                                app.update(action)?;
                            }
                        }
                        Event::Tick => {
                            app.update(Action::Tick)?;
                        }
                        Event::Resize(w, _) => {
                            app.term_width = w;
                            app.update(Action::Render)?;
                        }
                    }
                }
            }
        }
    }

    let _ = app.save_state();
    restore_terminal(terminal)?;
    Ok(())
}
