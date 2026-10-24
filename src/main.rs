mod aa;
mod aihot;
mod aitracker;
mod board;
mod coding_agents;
mod frontiercode;
mod rsc;
mod slopalytics;
mod ui;

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use base64::Engine;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use board::{Board, Status};
use ui::{AppState, Click};

type Msg = (Board, anyhow::Result<board::Data>);

/// Fetch threads carry this name prefix so the panic hook can tell them apart.
const FETCH_THREAD_PREFIX: &str = "fetch-";

fn main() -> Result<()> {
    install_panic_hook();
    let mut terminal = setup_terminal().context("setting up terminal")?;
    let result = run(&mut terminal);
    teardown_terminal(&mut terminal).ok();
    result
}

type Term = Terminal<CrosstermBackend<io::Stdout>>;

fn setup_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn teardown_terminal(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// A fetch-thread panic is caught in `spawn_fetch` and shown on its board, so
/// it must not print over the TUI. Any other panic restores the terminal
/// first, so the message lands on a usable screen.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let is_fetch_thread = thread::current()
            .name()
            .is_some_and(|name| name.starts_with(FETCH_THREAD_PREFIX));
        if is_fetch_thread {
            return;
        }
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));
}

/// Fetch threads are detached: quitting does not wait for a slow source to
/// time out, and a late result is dropped with the closed channel.
fn run(terminal: &mut Term) -> Result<()> {
    let mut app = AppState::new();
    let (tx, rx) = mpsc::channel::<Msg>();

    for &board in app.boards {
        app.set_status(board, Status::Loading);
        spawn_fetch(board, &tx);
    }

    let tick = Duration::from_millis(120);
    // `None` forces a draw on the next loop pass.
    let mut last_draw: Option<Instant> = None;

    loop {
        while let Ok((board, result)) = rx.try_recv() {
            match result {
                Ok(data) => app.set_status(board, Status::Loaded(data)),
                Err(e) => app.set_status(board, Status::Error(format!("{e:#}"))),
            }
        }

        if last_draw.is_none_or(|t| t.elapsed() >= tick) {
            terminal.draw(|f| ui::render(f, &mut app))?;
            last_draw = Some(Instant::now());
        }

        if !event::poll(Duration::from_millis(80))? {
            continue;
        }
        match event::read()? {
            Event::Key(k) if k.kind != KeyEventKind::Press => {}
            Event::Key(k) if handle_key(k, &mut app, &tx) => return Ok(()),
            Event::Key(_) => {}
            Event::Mouse(m) => handle_mouse(m, &mut app, &tx),
            Event::Resize(_, _) => last_draw = None,
            _ => {}
        }
    }
}

fn handle_key(k: KeyEvent, app: &mut AppState, tx: &mpsc::Sender<Msg>) -> bool {
    if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c')) {
        return true;
    }
    if app.is_help_open() {
        match k.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Esc => app.close_help(),
            KeyCode::Down | KeyCode::Char('j') => app.scroll_help(1),
            KeyCode::Up | KeyCode::Char('k') => app.scroll_help(-1),
            KeyCode::PageDown => app.scroll_help(10),
            KeyCode::PageUp => app.scroll_help(-10),
            KeyCode::Home | KeyCode::Char('g') => app.scroll_help(i32::MIN),
            KeyCode::End | KeyCode::Char('G') => app.scroll_help(i32::MAX),
            _ => {}
        }
        return false;
    }
    if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('u')) {
        app.clear_current_filter();
        return false;
    }
    if app.is_filter_editing() {
        handle_filter_key(k, app);
        return false;
    }

    match k.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('?') | KeyCode::Char('h') => app.toggle_help(),
        KeyCode::Char('r') => {
            let b = app.current_board();
            if !matches!(app.status.get(&b), Some(Status::Loading)) {
                app.set_status(b, Status::Loading);
                spawn_fetch(b, tx);
            }
        }
        KeyCode::Char('[') => {
            app.cycle_board(-1);
            ensure_loaded(app, tx);
        }
        KeyCode::Char(']') => {
            app.cycle_board(1);
            ensure_loaded(app, tx);
        }
        KeyCode::Char('o') => app.toggle_dir(),
        KeyCode::Char('m') => {
            app.toggle_view();
        }
        KeyCode::Char('z') => app.compact = !app.compact,
        KeyCode::Char('y') => {
            if let Some(name) = app.selected_name() {
                copy_to_clipboard(&name);
                app.copy_feedback_at = Some(Instant::now());
            }
        }
        KeyCode::Char('/') => app.begin_filter(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::PageDown => app.move_page_down(),
        KeyCode::PageUp => app.move_page_up(),
        KeyCode::Home => app.move_to_top(),
        KeyCode::End => app.move_to_bottom(),
        KeyCode::Char('g') => app.move_to_top(),
        KeyCode::Char('G') => app.move_to_bottom(),
        KeyCode::Char(c @ ('1'..='9' | '0' | '-' | '=')) => {
            if let Some(idx) = Board::from_shortcut(c) {
                app.select_board(idx);
                ensure_loaded(app, tx);
            }
        }
        KeyCode::Char(c @ ('a' | 'c' | 'd' | 'i' | 'p' | 's' | 't' | 'u')) => {
            app.cycle_sort(c);
        }
        KeyCode::Char(_) => {}
        _ => {}
    }
    false
}

fn handle_filter_key(k: KeyEvent, app: &mut AppState) {
    match k.code {
        KeyCode::Enter | KeyCode::Esc => app.finish_filter(),
        KeyCode::Backspace => app.pop_filter_char(),
        KeyCode::Char(c) if k.modifiers.is_empty() || k.modifiers == KeyModifiers::SHIFT => {
            app.push_filter_char(c)
        }
        _ => {}
    }
}

fn handle_mouse(m: MouseEvent, app: &mut AppState, tx: &mpsc::Sender<Msg>) {
    if app.is_help_open() {
        match m.kind {
            MouseEventKind::ScrollUp => app.scroll_help(-1),
            MouseEventKind::ScrollDown => app.scroll_help(1),
            _ => {}
        }
        return;
    }
    match m.kind {
        MouseEventKind::ScrollUp => app.move_up(),
        MouseEventKind::ScrollDown => app.move_down(),
        MouseEventKind::Down(MouseButton::Left) => match app.hit_test(m.column, m.row) {
            Click::Tab(idx) => {
                app.select_board(idx);
                ensure_loaded(app, tx);
            }
            // Same as pressing the column's sort key: a second click flips
            // the direction.
            Click::Header(key) => app.cycle_sort(key),
            Click::Row(idx) => app.select_row(idx),
            Click::Nothing => {}
        },
        // Dragging along the scrollbar (or down the rows) scrubs the table.
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Click::Row(idx) = app.hit_test(m.column, m.row) {
                app.select_row(idx);
            }
        }
        _ => {}
    }
}

fn ensure_loaded(app: &mut AppState, tx: &mpsc::Sender<Msg>) {
    let b = app.current_board();
    if matches!(
        app.status.get(&b),
        Some(Status::Loading) | Some(Status::Loaded(_))
    ) {
        return;
    }
    app.set_status(b, Status::Loading);
    spawn_fetch(b, tx);
}

fn spawn_fetch(board: Board, tx: &mpsc::Sender<Msg>) {
    let thread_tx = tx.clone();
    let spawned = thread::Builder::new()
        .name(format!("{FETCH_THREAD_PREFIX}{}", board.label()))
        .spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| board::fetch(board)))
                    .unwrap_or_else(|payload| {
                        Err(anyhow::anyhow!("parser bug: {}", panic_message(&*payload)))
                    });
            // The receiver is gone once the user quits; nothing to report then.
            let _ = thread_tx.send((board, result));
        });
    if let Err(e) = spawned {
        let _ = tx.send((board, Err(anyhow::anyhow!("starting fetch thread: {e}"))));
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic with a non-string payload")
}

fn copy_to_clipboard(text: &str) {
    use std::io::Write;
    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    let _ = write!(io::stdout(), "\x1b]52;c;{encoded}\x07");
    let _ = io::stdout().flush();
}
