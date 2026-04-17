use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::tui::model::*;
use crate::tui::styles::*;

/// Main view dispatcher
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Welcome screen (full screen, no tab bar)
    if app.show_welcome {
        super::widgets::splash::render_splash(frame, area, app.tick_count);
        return;
    }

    // Tab bar + content
    let chunks = Layout::vertical([
        Constraint::Length(1), // tab bar
        Constraint::Min(5),   // content
    ])
    .split(area);

    render_tab_bar(frame, chunks[0], app);

    match app.mode {
        AppMode::Updater => render_updater(frame, chunks[1], app),
        AppMode::Scanner => {
            super::widgets::scanner_view::render_scanner(
                frame,
                chunks[1],
                app,
                app.tick_count,
            );
        }
    }

    // Toast notification overlay (bottom right)
    if let Some(toast) = &app.toast {
        let age = app.tick_count.saturating_sub(toast.created_at);
        if age < 30 {
            render_toast(frame, area, toast);
        }
    }
}

fn render_toast(frame: &mut Frame, area: Rect, toast: &crate::tui::model::Toast) {
    let msg_len = toast.message.len() as u16 + 4;
    let width = msg_len.min(area.width.saturating_sub(4));
    let x = area.width.saturating_sub(width + 2);
    let y = area.height.saturating_sub(3);

    let toast_area = Rect::new(x, y, width, 1);

    let (fg, bg) = if toast.is_error {
        (WHITE, Color::Rgb(180, 40, 40))
    } else {
        (WHITE, Color::Rgb(30, 120, 70))
    };

    let text = Paragraph::new(Span::styled(
        format!(" {} ", toast.message),
        Style::default().fg(fg).bg(bg).bold(),
    ));
    frame.render_widget(text, toast_area);
}

fn render_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let is_updater = app.mode == AppMode::Updater;
    let is_scanner_base = app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::ScanConfig
                | ScannerState::Scanning
                | ScannerState::ScanResults
                | ScannerState::RepoDetail
                | ScannerState::CleanConfirm
                | ScannerState::Cleaning
                | ScannerState::DeleteRepoConfirm
                | ScannerState::HealthHelp
        );
    let is_repo_mgr = app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::RepoManager | ScannerState::RepoAction
                | ScannerState::RepoCloneInput | ScannerState::RepoCloneSummary
        );
    let is_ports = app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::PortScan | ScannerState::PortAction | ScannerState::PortKillConfirm
        );
    let is_system = app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::SystemClean | ScannerState::SystemCleanConfirm
        );
    let is_audit = app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::SecretAudit | ScannerState::SecretAuditScanning
                | ScannerState::SecretAuditDetail | ScannerState::SecretAuditDeps
        );

    let active = |on: bool, color: Color| -> Style {
        if on {
            Style::default().fg(WHITE).bg(color).bold()
        } else {
            Style::default().fg(GRAY)
        }
    };

    let tabs = Line::from(vec![
        Span::styled(" Scanner ", active(is_scanner_base, PURPLE)),
        Span::styled(" Repos ", active(is_repo_mgr, GREEN)),
        Span::styled(" Ports ", active(is_ports, YELLOW)),
        Span::styled(" System ", active(is_system, Color::Rgb(180, 80, 40))),
        Span::styled(" Audit ", active(is_audit, RED)),
        Span::styled(" Updater ", active(is_updater, BLUE)),
        Span::raw("  "),
        Span::styled("TAB ", Style::default().fg(GRAY).bold()),
        Span::styled("switch", Style::default().fg(GRAY)),
        Span::raw("  "),
        Span::styled(
            format!("SPARK {} ", VERSION),
            Style::default().fg(DARK_BG).italic(),
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
