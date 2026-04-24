//! Security audit views: scanning spinner, project list, detail views, path input.
//!
//! - mod.rs    — scanning + list + path_input (entry views)
//! - detail.rs — render_audit_detail (per-project findings)
//! - deps.rs   — render_audit_deps (OSV.dev dependency vulns)

mod deps;
mod detail;

pub use deps::render_audit_deps;
pub use detail::render_audit_detail;

use crate::tui::model::*;
use crate::tui::styles::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_audit_scanning(frame: &mut Frame, area: Rect, tick: usize) {
    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "{} Scanning for secrets and exposed credentials...",
                spinner
            ),
            Style::default().fg(RED).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  API keys, tokens, passwords, sensitive files",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled("  [ESC] Cancel", Style::default().fg(GRAY))),
    ];
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

pub fn render_audit_list(frame: &mut Frame, area: Rect, model: &AuditModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    let scan_path_display = scan_path_display(model);
    let dep_count = model.dep_vulns.len();

    frame.render_widget(
        render_list_header(model, &scan_path_display, dep_count),
        chunks[0],
    );

    if model.results.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No security findings detected",
                Style::default().fg(GREEN).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  All scanned projects look clean",
                Style::default().fg(GRAY),
            )),
        ]);
        frame.render_widget(empty, chunks[1]);
    } else {
        render_list_table(frame, chunks[1], model, dep_count);
    }

    render_list_help(frame, chunks[2], model, &scan_path_display);
}

fn scan_path_display(model: &AuditModel) -> String {
    model
        .scan_path
        .as_ref()
        .map(|p| {
            let s = p.display().to_string();
            let home = std::env::var("HOME").unwrap_or_default();
            if s.starts_with(&home) {
                format!("~{}", &s[home.len()..])
            } else {
                s
            }
        })
        .unwrap_or_else(|| "not configured".into())
}

fn render_list_header(model: &AuditModel, scan_path: &str, dep_count: usize) -> Paragraph<'static> {
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " SECURITY AUDIT ",
                Style::default().fg(WHITE).bg(RED).bold(),
            ),
            Span::raw("  "),
            if model.total_critical > 0 {
                Span::styled(
                    format!("{} critical  ", model.total_critical),
                    Style::default().fg(RED).bold(),
                )
            } else {
                Span::styled("0 critical  ", Style::default().fg(GREEN))
            },
            if model.total_warning > 0 {
                Span::styled(
                    format!("{} warnings  ", model.total_warning),
                    Style::default().fg(YELLOW).bold(),
                )
            } else {
                Span::styled("0 warnings  ", Style::default().fg(GRAY))
            },
            Span::styled(
                format!("{} info  ", model.total_info),
                Style::default().fg(GRAY),
            ),
            if dep_count > 0 {
                Span::styled(
                    format!("{} dep vulns", dep_count),
                    Style::default().fg(Color::Rgb(255, 140, 0)).bold(),
                )
            } else {
                Span::styled("deps clean", Style::default().fg(GREEN))
            },
        ]),
        Line::from(Span::styled(
            format!("{} projects scanned — {}", model.results.len(), scan_path),
            Style::default().fg(GRAY),
        )),
    ])
}

fn render_list_table(frame: &mut Frame, area: Rect, model: &AuditModel, dep_count: usize) {
    let table_header = Row::new(vec![
        Cell::from("  Project").style(Style::default().fg(RED).bold()),
        Cell::from("Critical").style(Style::default().fg(RED).bold()),
        Cell::from("Warning").style(Style::default().fg(RED).bold()),
        Cell::from("Info").style(Style::default().fg(RED).bold()),
        Cell::from("Path").style(Style::default().fg(RED).bold()),
    ]);

    let home = std::env::var("HOME").unwrap_or_default();
    let visible_height = area.height.saturating_sub(3) as usize;
    let scroll_offset = if model.cursor >= visible_height {
        model.cursor - visible_height + 1
    } else {
        0
    };

    let dep_row_idx = model.results.len();
    let total_rows = dep_row_idx + if dep_count > 0 { 1 } else { 0 };

    let mut rows: Vec<Row> = model
        .results
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .map(|(i, result)| {
            let is_selected = model.cursor == i;
            let cursor = if is_selected { ">" } else { " " };

            let path_str = result.project_path.display().to_string();
            let short_path = if path_str.starts_with(&home) {
                format!("~{}", &path_str[home.len()..])
            } else {
                path_str
            };
            let display_path = if short_path.len() > 35 {
                format!("...{}", &short_path[short_path.len() - 32..])
            } else {
                short_path
            };

            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("{} {}", cursor, result.project_name)),
                Cell::from(format!("{}", result.critical_count)).style(
                    if result.critical_count > 0 {
                        Style::default().fg(RED).bold()
                    } else {
                        Style::default().fg(GRAY)
                    },
                ),
                Cell::from(format!("{}", result.warning_count)).style(
                    if result.warning_count > 0 {
                        Style::default().fg(YELLOW)
                    } else {
                        Style::default().fg(GRAY)
                    },
                ),
                Cell::from(format!("{}", result.info_count)).style(Style::default().fg(GRAY)),
                Cell::from(display_path).style(Style::default().fg(TERM_GRAY)),
            ])
            .style(row_style)
        })
        .collect();

    if dep_count > 0 && dep_row_idx >= scroll_offset {
        let is_selected = model.cursor == dep_row_idx;
        let cursor = if is_selected { ">" } else { " " };
        let row_style = if is_selected {
            Style::default().bg(DARK_BG)
        } else {
            Style::default()
        };
        rows.push(
            Row::new(vec![
                Cell::from(format!("{} [dependencies]", cursor))
                    .style(Style::default().fg(Color::Rgb(255, 140, 0))),
                Cell::from(format!("{}", dep_count))
                    .style(Style::default().fg(Color::Rgb(255, 140, 0)).bold()),
                Cell::from(""),
                Cell::from(""),
                Cell::from("OSV.dev scan").style(Style::default().fg(TERM_GRAY)),
            ])
            .style(row_style),
        );
    }

    let scroll_info = format!(" {}/{} ", model.cursor + 1, total_rows);

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Min(20),
        ],
    )
    .header(table_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(RED))
            .title(Span::styled(" Findings ", Style::default().fg(RED).bold()))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, area);
}

fn render_list_help(frame: &mut Frame, area: Rect, model: &AuditModel, scan_path: &str) {
    let help_text = if model.results.is_empty() && model.dep_vulns.is_empty() && !model.scanning {
        "[a] Set folder  [r] Scan  [TAB] Next  [q] Back"
    } else {
        "[ENTER] Detail / Deps  [a] Set folder  [r] Rescan  [TAB] Next  [q] Back"
    };
    let help = Paragraph::new(vec![
        Line::from(Span::styled(
            format!(
                "  Scanning: {}  (run spark from the project you want to audit)",
                scan_path
            ),
            Style::default().fg(TERM_GRAY),
        )),
        Line::from(Span::styled(help_text, Style::default().fg(GRAY))),
    ]);
    frame.render_widget(help, area);
}

pub fn render_audit_path_input(frame: &mut Frame, area: Rect, model: &AuditModel) {
    let modal_area = center_modal(frame, area, 60, 9);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(RED))
        .style(Style::default().bg(MODAL_BG));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let current = model
        .scan_path
        .as_ref()
        .map(|p| crate::scanner::common::shorten_path(&p.display().to_string()))
        .unwrap_or_else(|| "not set".to_string());

    let lines = vec![
        Line::from(Span::styled(
            " AUDIT FOLDER ",
            Style::default().fg(WHITE).bg(RED).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Current: ", Style::default().fg(GRAY)),
            Span::styled(current, Style::default().fg(TERM_GRAY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Path: ", Style::default().fg(PURPLE)),
            Span::styled(model.path_input.clone(), Style::default().fg(WHITE).bold()),
            Span::styled("_", Style::default().fg(YELLOW)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [ENTER] Scan  [ESC] Cancel",
            Style::default().fg(GRAY),
        )),
    ];

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
}
