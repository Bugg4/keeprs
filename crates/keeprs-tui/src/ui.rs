//! UI rendering with Ratatui.

use crate::app::{App, AppState, Focus, InputMode, TreeItemKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Main render function.
pub fn render(frame: &mut Frame, app: &App) {
    match app.state {
        AppState::Locked => render_locked(frame, app),
        AppState::Unlocked => render_unlocked(frame, app),
        AppState::Quit => {}
    }
}

/// Render the password entry screen.
fn render_locked(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Center the dialog
    let dialog_width = 50;
    let dialog_height = 7;
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    // Clear the background
    frame.render_widget(Clear, dialog_area);

    // Main dialog box
    let block = Block::default()
        .title(" 🔐 Keeprs - Unlock Database ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Layout for prompt and input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // Password prompt
    let prompt = Paragraph::new("Enter master password:")
        .style(Style::default().fg(Color::White));
    frame.render_widget(prompt, chunks[0]);

    // Password input (masked)
    let masked: String = "*".repeat(app.password_input.len());
    let input = Paragraph::new(format!("▸ {}_", masked))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[1]);

    // Error message
    if let Some(ref error) = app.error_message {
        let error_msg = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red));
        frame.render_widget(error_msg, chunks[2]);
    }
}

/// Render the main unlocked view with sidebar and entry detail.
fn render_unlocked(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Split into sidebar (30%) and main content (70%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_sidebar(frame, app, chunks[0]);
    render_entry_view(frame, app, chunks[1]);

    // Render search overlay if in search mode
    if app.input_mode == InputMode::Search {
        render_search_overlay(frame, app, area);
    }
}

/// Render the sidebar tree view.
fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Sidebar && app.input_mode == InputMode::Normal;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(" 📁 Database ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build list items from tree
    let items: Vec<ListItem> = app
        .tree_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let indent = "  ".repeat(item.depth.saturating_sub(1));
            
            let (icon, icon_color) = match item.kind {
                TreeItemKind::Group => {
                    if item.is_expanded {
                        ("▾ 📂", Color::Yellow)
                    } else if item.has_children {
                        ("▸ 📁", Color::Yellow)
                    } else {
                        ("  📁", Color::Yellow)
                    }
                }
                TreeItemKind::Entry => ("  🔑", Color::Cyan),
            };

            let content = format!("{}{} {}", indent, icon, item.name);
            
            let style = if i == app.sidebar_selected_index {
                Style::default()
                    .bg(Color::Rgb(60, 60, 80))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(icon_color)
            };

            ListItem::new(Line::from(content)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

/// Render the entry detail view.
fn render_entry_view(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::EntryView && app.input_mode == InputMode::Normal;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let block = Block::default()
        .title(" 📋 Entry Details ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(ref entry) = app.selected_entry {
        // Split into fields
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(2), // Username
                Constraint::Length(2), // Password
                Constraint::Length(2), // URL
                Constraint::Min(3),    // Notes
                Constraint::Length(1), // Help line
            ])
            .split(inner);

        // Title
        render_field(frame, "Title", &entry.title, chunks[0], Color::White);

        // Username
        render_field(frame, "Username", &entry.username, chunks[1], Color::Green);

        // Password (masked)
        let masked = "••••••••••••";
        render_field(frame, "Password", masked, chunks[2], Color::Yellow);

        // URL
        render_field(frame, "URL", &entry.url, chunks[3], Color::Blue);

        // Notes
        let notes_block = Block::default()
            .title(Span::styled(" Notes ", Style::default().fg(Color::DarkGray)));
        let notes_inner = notes_block.inner(chunks[4]);
        frame.render_widget(notes_block, chunks[4]);

        let notes = Paragraph::new(entry.notes.as_str())
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        frame.render_widget(notes, notes_inner);

        // Help line
        let help = Paragraph::new("Tab: switch focus | /: search | q: quit")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[5]);
    } else {
        // No entry selected
        let message = Paragraph::new("Select an entry from the sidebar")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        
        let centered = centered_rect(50, 3, inner);
        frame.render_widget(message, centered);
    }
}

fn render_field(frame: &mut Frame, label: &str, value: &str, area: Rect, color: Color) {
    let line = Line::from(vec![
        Span::styled(format!("{}: ", label), Style::default().fg(Color::DarkGray)),
        Span::styled(value, Style::default().fg(color)),
    ]);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

/// Render the search overlay.
fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    // Center the search dialog
    let dialog_width = 60.min(area.width.saturating_sub(4));
    let dialog_height = 15.min(area.height.saturating_sub(4));
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    // Clear background
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" 🔍 Search ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Layout for search input and results
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    // Search input
    let input_line = Line::from(vec![
        Span::styled("▸ ", Style::default().fg(Color::Magenta)),
        Span::styled(&app.search_query, Style::default().fg(Color::White)),
        Span::styled("_", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
    ]);
    let input = Paragraph::new(input_line);
    frame.render_widget(input, chunks[0]);

    // Search results
    if !app.search_results.is_empty() {
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let style = if i == app.search_selected_index {
                    Style::default()
                        .bg(Color::Rgb(60, 40, 80))
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let line = Line::from(vec![
                    Span::styled("🔑 ", Style::default()),
                    Span::styled(&result.entry.title, style),
                    Span::styled(" ", Style::default()),
                    Span::styled(&result.path, Style::default().fg(Color::DarkGray)),
                ]);

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, chunks[1]);
    } else if !app.search_query.is_empty() {
        let no_results = Paragraph::new("No results found")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(no_results, chunks[1]);
    }
}

/// Helper to create a centered rectangle.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
