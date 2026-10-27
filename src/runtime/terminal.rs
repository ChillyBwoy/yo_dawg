use crate::{app::App, ui};
use anyhow::Result;
use crossbeam_channel::Receiver;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static KEYBOARD_ENHANCED: AtomicBool = AtomicBool::new(false);

pub(super) fn run(app: &mut App, errors: Receiver<String>) -> Result<()> {
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        old_hook(info);
    }));
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        EnableMouseCapture,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )?;
    KEYBOARD_ENHANCED.store(true, Ordering::Relaxed);
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut last_frame = Instant::now() - Duration::from_secs(1);
    while !app.quit {
        app.poll_engine();
        while let Ok(error) = errors.try_recv() {
            app.status = error;
        }
        if last_frame.elapsed() >= Duration::from_millis(33) {
            terminal.draw(|frame| ui::draw(frame, app))?;
            last_frame = Instant::now();
        }
        if event::poll(Duration::from_millis(4))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => app.key_event(key),
                Event::Mouse(mouse) => app.mouse(mouse),
                _ => {}
            }
        }
    }
    Ok(())
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let _ = disable_raw_mode();
    // Both the panic hook and the guard may restore the terminal during unwinding.
    if KEYBOARD_ENHANCED.swap(false, Ordering::Relaxed) {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    let _ = execute!(
        io::stdout(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}
