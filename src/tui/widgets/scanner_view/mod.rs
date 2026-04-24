//! Scanner tab view — dispatches by ScannerState and renders the matching
//! screen. Sub-screens split into:
//! - config.rs  — scan config picker + in-progress view
//! - results.rs — results table (grouped, with scroll)
//!
//! Small modals and overlays (cleaning, delete confirm, add path, health help)
//! live here.

mod config;
mod results;

use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_scanner(frame: &mut Frame, area: Rect, app: &App, tick: usize) {
    let model = &app.scanner;
    match model.state {
        ScannerState::ScanConfig => config::render_scan_config(frame, area, model),
        ScannerState::ScanAddPath => {
            config::render_scan_config(frame, area, model);
            render_add_path_modal(frame, area, model);
        }
        ScannerState::ContainerLoading => render_container_loading(frame, area, model, tick),
        ScannerState::Scanning => config::render_scanning(frame, area, model, tick),
        ScannerState::ScanResults => results::render_scan_results(frame, area, model),
        ScannerState::RepoDetail => {
            super::detail_panel::render_detail(frame, area, model);
        }
        ScannerState::ContainerChildDetail => {
            super::detail_panel::render_child_detail(frame, area, model);
        }
        ScannerState::ContainerChildDelete => {
            super::detail_panel::render_child_detail(frame, area, model);
            super::detail_panel::render_child_delete_confirm(frame, area, model);
        }
        ScannerState::HealthHelp => {
            results::render_scan_results(frame, area, model);
            render_health_help(frame, area);
        }
        ScannerState::DeleteRepoConfirm => {
            results::render_scan_results(frame, area, model);
            render_delete_repo_confirm(frame, area, model);
        }
        ScannerState::CleanConfirm => {
            results::render_scan_results(frame, area, model);
            let total_size = model
                .checked
                .iter()
                .map(|&i| model.repos.get(i).map(|r| r.artifact_size).unwrap_or(0))
                .sum::<u64>();
            super::modal::render_clean_confirm_modal(
                frame,
                area,
                model.checked.len(),
                &format_size(total_size),
            );
        }
        ScannerState::Cleaning => render_cleaning(frame, area, model, tick),
        ScannerState::RepoManager => {
            super::repo_manager_view::render_repo_manager(frame, area, &app.repo_manager);
        }
        ScannerState::RepoAction => {
            super::repo_manager_view::render_repo_manager(frame, area, &app.repo_manager);
            super::repo_manager_view::render_action_modal(frame, area, &app.repo_manager);
        }
        ScannerState::RepoCloneInput => {
            super::repo_manager_view::render_repo_manager(frame, area, &app.repo_manager);
            super::repo_manager_view::render_clone_input(frame, area, &app.repo_manager);
        }
        ScannerState::RepoCloneSummary => {
            super::repo_manager_view::render_clone_summary(frame, area, &app.repo_manager);
        }
        ScannerState::SystemClean => {
            super::system_view::render_system_cleaner(frame, area, &app.system_cleaner);
        }
        ScannerState::SystemCleanConfirm => {
            super::system_view::render_system_cleaner(frame, area, &app.system_cleaner);
            super::system_view::render_risk_confirm(frame, area, &app.system_cleaner);
        }
        ScannerState::SystemCleanConfirmBulk => {
            super::system_view::render_system_cleaner(frame, area, &app.system_cleaner);
            super::system_view::render_bulk_confirm(frame, area, &app.system_cleaner);
        }
        ScannerState::SecretAuditScanning => {
            super::audit_view::render_audit_scanning(frame, area, tick);
        }
        ScannerState::SecretAudit => {
            super::audit_view::render_audit_list(frame, area, &app.audit);
        }
        ScannerState::SecretAuditDetail => {
            super::audit_view::render_audit_detail(frame, area, &app.audit);
        }
        ScannerState::SecretAuditDeps => {
            super::audit_view::render_audit_deps(frame, area, &app.audit);
        }
        ScannerState::SecretAuditPathInput => {
            super::audit_view::render_audit_list(frame, area, &app.audit);
            super::audit_view::render_audit_path_input(frame, area, &app.audit);
        }
        ScannerState::PortScan => {
            super::port_view::render_ports(frame, area, &app.port_scanner);
        }
        ScannerState::PortAction => {
            super::port_view::render_ports(frame, area, &app.port_scanner);
            super::port_view::render_action_modal(frame, area, &app.port_scanner);
        }
        ScannerState::PortKillConfirm => {
            super::port_view::render_ports(frame, area, &app.port_scanner);
            let ports_str: String = app
                .port_scanner
                .checked
                .iter()
                .filter_map(|&i| {
                    app.port_scanner
                        .ports
                        .get(i)
                        .map(|p| format!(":{}", p.port))
                })
                .collect::<Vec<_>>()
                .join(", ");
            super::port_view::render_kill_confirm(
                frame,
                area,
                app.port_scanner.checked.len(),
                &ports_str,
            );
        }
    }
}

fn render_container_loading(frame: &mut Frame, area: Rect, model: &ScannerModel, tick: usize) {
    let repo_name = model
        .repos
        .get(model.cursor)
        .map(|r| r.name.as_str())
        .unwrap_or("...");
    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let loading = Paragraph::new(vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Loading repos in {}...", spinner, repo_name),
            Style::default().fg(CYAN).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled("  [ESC] Cancel", Style::default().fg(GRAY))),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(loading, area);
}

fn render_cleaning(frame: &mut Frame, area: Rect, model: &ScannerModel, tick: usize) {
    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let completed = model.clean_results.len();
    let total = model.checked.len();

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Cleaning...", spinner),
            Style::default().fg(YELLOW).bold(),
        )),
        Line::from(""),
        Line::from(format!("  Progress: {}/{}", completed, total)),
    ];

    let paragraph = Paragraph::new(lines).block(Block::default().padding(Padding::new(2, 2, 2, 2)));
    frame.render_widget(paragraph, area);
}

fn render_health_help(frame: &mut Frame, area: Rect) {
    let modal_area = center_modal(frame, area, 58, 18);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(PURPLE))
        .style(Style::default().bg(MODAL_BG));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(
            " HEALTH SCORE ",
            Style::default().fg(WHITE).bg(PURPLE).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Score 0-100 based on:",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Recent commits  ", Style::default().fg(WHITE)),
            Span::styled("up to -30 if >12 months old", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Has remote      ", Style::default().fg(WHITE)),
            Span::styled("-15 if no remote configured", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Clean status    ", Style::default().fg(WHITE)),
            Span::styled("-10 if dirty + stale", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Artifact size   ", Style::default().fg(WHITE)),
            Span::styled("-20 if >100MB of artifacts", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Grades:",
            Style::default().fg(YELLOW).bold(),
        )),
        Line::from(vec![
            Span::styled("  A", Style::default().fg(GREEN).bold()),
            Span::styled(" 80-100  ", Style::default().fg(GRAY)),
            Span::styled("B", Style::default().fg(BLUE).bold()),
            Span::styled(" 60-79   ", Style::default().fg(GRAY)),
            Span::styled("C", Style::default().fg(YELLOW).bold()),
            Span::styled(" 40-59", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  D", Style::default().fg(Color::Rgb(255, 165, 0)).bold()),
            Span::styled(" 20-39   ", Style::default().fg(GRAY)),
            Span::styled("F", Style::default().fg(RED).bold()),
            Span::styled("  0-19", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  [q] Close", Style::default().fg(GRAY))),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_delete_repo_confirm(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let repo = match model.repos.get(model.cursor) {
        Some(r) => r,
        None => return,
    };

    let modal_area = center_modal(frame, area, 62, 12);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(RED))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let home = std::env::var("HOME").unwrap_or_default();
    let path_str = repo.path.display().to_string();
    let short_path = if path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else {
        path_str
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "DELETE REPOSITORY",
            Style::default().fg(RED).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Repo: ", Style::default().fg(PURPLE)),
            Span::styled(&repo.name, Style::default().fg(WHITE).bold()),
        ]),
        Line::from(vec![
            Span::styled("  Path: ", Style::default().fg(PURPLE)),
            Span::styled(&short_path, Style::default().fg(GRAY)),
        ]),
    ];

    if repo.is_dirty {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  WARNING: Uncommitted changes will be lost!",
            Style::default().fg(RED).bold(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  This will move the entire folder to trash.",
        Style::default().fg(YELLOW),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Delete? (y/N)",
        Style::default().fg(WHITE),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

fn render_add_path_modal(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let modal_area = center_modal(frame, area, 65, 9);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(PURPLE))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(
            "ADD SCAN DIRECTORY",
            Style::default().fg(PURPLE).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter full path (~ for home):",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}█", model.path_input),
            Style::default().fg(WHITE).bg(DARK),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "e.g.  ~/Projects  or  /opt/code",
            Style::default().fg(TERM_GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[ENTER] Add • [ESC] Cancel",
            Style::default().fg(GRAY),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}
