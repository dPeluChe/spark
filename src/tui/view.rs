use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::tui::model::*;
use crate::tui::styles::*;

/// Main view dispatcher
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Splash screen (full screen, no tab bar)
    if app.mode == AppMode::Updater && app.updater.state == UpdaterState::Splash {
        super::widgets::splash::render_splash(frame, area, app.updater.splash_frame);
        return;
    }

    // Tab bar + content
    let chunks = Layout::vertical([
        Constraint::Length(1), // tab bar
        Constraint::Min(5),   // content
    ])
    .split(area);

    render_tab_bar(frame, chunks[0], &app.mode);

    match app.mode {
        AppMode::Updater => render_updater(frame, chunks[1], app),
        AppMode::Scanner => {
            super::widgets::scanner_view::render_scanner(
                frame,
                chunks[1],
                &app.scanner,
                app.tick_count,
            );
        }
    }
}

fn render_tab_bar(frame: &mut Frame, area: Rect, mode: &AppMode) {
    let updater_style = if *mode == AppMode::Updater {
        Style::default().fg(WHITE).bg(BLUE).bold()
    } else {
        Style::default().fg(GRAY)
    };

    let scanner_style = if *mode == AppMode::Scanner {
        Style::default().fg(WHITE).bg(PURPLE).bold()
    } else {
        Style::default().fg(GRAY)
    };

    let tabs = Line::from(vec![
        Span::styled(" [1] Updater ", updater_style),
        Span::raw("  "),
        Span::styled(" [2] Scanner ", scanner_style),
        Span::raw("  "),
        Span::styled(
            format!(" SPARK {} ", VERSION),
            Style::default().fg(DARK).italic(),
        ),
    ]);

    frame.render_widget(Paragraph::new(tabs), area);
}

fn render_updater(frame: &mut Frame, area: Rect, app: &App) {
    let m = &app.updater;

    match m.state {
        UpdaterState::Preview => {
            super::widgets::dashboard::render_preview(frame, area, m);
        }
        UpdaterState::Updating => {
            // Render dashboard as background
            super::widgets::dashboard::render_dashboard(frame, area, m);
            // Overlay updating modal
            super::widgets::progress::render_updating_overlay(
                frame,
                area,
                m.total_update,
                m.updating_remaining,
                m.current_update
                    .and_then(|i| m.items.get(i))
                    .map(|item| item.tool.name.as_str()),
                &m.current_log,
                app.tick_count,
            );
        }
        UpdaterState::Summary => {
            // Render dashboard as background
            super::widgets::dashboard::render_dashboard(frame, area, m);
            // Overlay summary modal
            super::widgets::progress::render_summary_overlay(frame, area, &m.items);
        }
        UpdaterState::Confirm => {
            // Render dashboard as background
            super::widgets::dashboard::render_dashboard(frame, area, m);
            // Overlay danger modal
            super::widgets::modal::render_danger_modal(frame, area);
        }
        _ => {
            // Main, Search states
            super::widgets::dashboard::render_dashboard(frame, area, m);
        }
    }
}
