//! Keyboard event handling.

use crate::app::{App, AppState, Focus, InputMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event. Returns true if the app should quit.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match app.input_mode {
        InputMode::PasswordEntry => handle_password_key(app, key),
        InputMode::Normal => handle_normal_key(app, key),
        InputMode::Search => handle_search_key(app, key),
    }
}

fn handle_password_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => {
            app.try_unlock();
            false
        }
        KeyCode::Char(c) => {
            app.password_input.push(c);
            false
        }
        KeyCode::Backspace => {
            app.password_input.pop();
            false
        }
        KeyCode::Esc => {
            app.state = AppState::Quit;
            true
        }
        _ => false,
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent) -> bool {
    // Ctrl+C to quit
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.state = AppState::Quit;
        return true;
    }

    // Ctrl+P to search
    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.start_search();
        return false;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.state = AppState::Quit;
            true
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.focus == Focus::Sidebar {
                app.move_down();
            }
            false
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.focus == Focus::Sidebar {
                app.move_up();
            }
            false
        }
        KeyCode::Left => {
            // Collapse current group or go to parent
            if app.focus == Focus::Sidebar {
                app.collapse_current();
            }
            false
        }
        KeyCode::Right => {
            // Expand current group or select entry
            if app.focus == Focus::Sidebar {
                app.expand_current();
            }
            false
        }
        KeyCode::Enter => {
            if app.focus == Focus::Sidebar {
                app.select_current_item();
            }
            false
        }
        KeyCode::Tab => {
            // Toggle focus between sidebar and entry view
            app.focus = match app.focus {
                Focus::Sidebar => Focus::EntryView,
                Focus::EntryView => Focus::Sidebar,
            };
            false
        }
        KeyCode::Char('h') => {
            // Vim-style: collapse in sidebar
            if app.focus == Focus::Sidebar {
                app.collapse_current();
            }
            false
        }
        KeyCode::Char('l') => {
            // Vim-style: expand in sidebar
            if app.focus == Focus::Sidebar {
                app.expand_current();
            }
            false
        }
        KeyCode::Esc => {
            // Go back to sidebar from entry view
            if app.focus == Focus::EntryView {
                app.focus = Focus::Sidebar;
            }
            false
        }
        _ => false,
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.exit_search();
            false
        }
        KeyCode::Enter => {
            app.select_search_result();
            false
        }
        KeyCode::Up => {
            if app.search_selected_index > 0 {
                app.search_selected_index -= 1;
            }
            false
        }
        KeyCode::Down => {
            if app.search_selected_index + 1 < app.search_results.len() {
                app.search_selected_index += 1;
            }
            false
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.perform_search();
            false
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.perform_search();
            false
        }
        _ => false,
    }
}
