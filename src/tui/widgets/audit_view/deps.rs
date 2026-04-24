//! OSV.dev dependency vulnerability detail view.

use crate::tui::model::*;
use crate::tui::styles::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_audit_deps(frame: &mut Frame, area: Rect, model: &AuditModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " DEPENDENCY VULNERABILITIES ",
                Style::default()
                    .fg(WHITE)
                    .bg(Color::Rgb(255, 140, 0))
                    .bold(),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} vulnerabilities", model.dep_vulns.len()),
                Style::default().fg(WHITE),
            ),
        ]),
        Line::from(Span::styled(
            "OSV.dev — open source vulnerability database",
            Style::default().fg(GRAY),
        )),
    ]);
    frame.render_widget(header, chunks[0]);

    if model.dep_vulns.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No dependency vulnerabilities found",
                Style::default().fg(GREEN).bold(),
            )),
        ]);
        frame.render_widget(msg, chunks[1]);
        return;
    }

    let table_header = Row::new(vec![
        Cell::from("  Severity").style(Style::default().fg(Color::Rgb(255, 140, 0)).bold()),
        Cell::from("Package").style(Style::default().fg(Color::Rgb(255, 140, 0)).bold()),
        Cell::from("Version").style(Style::default().fg(Color::Rgb(255, 140, 0)).bold()),
        Cell::from("Fix").style(Style::default().fg(Color::Rgb(255, 140, 0)).bold()),
        Cell::from("Summary").style(Style::default().fg(Color::Rgb(255, 140, 0)).bold()),
    ]);

    let visible_height = chunks[1].height.saturating_sub(3) as usize;
    let scroll_offset = if model.dep_cursor >= visible_height {
        model.dep_cursor - visible_height + 1
    } else {
        0
    };

    let rows: Vec<Row> = model
        .dep_vulns
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .map(|(i, v)| {
            let is_selected = model.dep_cursor == i;
            let cursor = if is_selected { ">" } else { " " };
            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };
            let (sev_label, sev_style) = dep_severity_style(&v.severity);
            let fix = v.fixed_version.as_deref().unwrap_or("-");
            let summary = if v.summary.len() > 50 {
                format!("{}…", &v.summary[..47])
            } else {
                v.summary.clone()
            };
            Row::new(vec![
                Cell::from(format!("{} {}", cursor, sev_label)).style(sev_style),
                Cell::from(v.dep_name.clone()),
                Cell::from(v.dep_version.clone()).style(Style::default().fg(GRAY)),
                Cell::from(fix).style(Style::default().fg(GREEN)),
                Cell::from(summary).style(Style::default().fg(TERM_GRAY)),
            ])
            .style(row_style)
        })
        .collect();

    let scroll_info = format!(" {}/{} ", model.dep_cursor + 1, model.dep_vulns.len());
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
    )
    .header(table_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(255, 140, 0)))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, chunks[1]);

    let detail_line = if let Some(v) = model.dep_vulns.get(model.dep_cursor) {
        Line::from(vec![
            Span::styled("  CVE: ", Style::default().fg(PURPLE)),
            Span::styled(v.id.clone(), Style::default().fg(YELLOW)),
            Span::raw("  "),
            Span::styled(
                format!("ecosystem: {}", v.ecosystem),
                Style::default().fg(GRAY),
            ),
            Span::raw("  "),
            Span::styled("[q] Back  [j/k] Navigate", Style::default().fg(TERM_GRAY)),
        ])
    } else {
        Line::from(Span::styled(
            "  [q] Back  [j/k] Navigate",
            Style::default().fg(GRAY),
        ))
    };
    frame.render_widget(Paragraph::new(vec![detail_line]), chunks[2]);
}

fn dep_severity_style(severity: &str) -> (&'static str, Style) {
    let s = severity.to_uppercase();
    if s.contains("CRITICAL") {
        ("CRITICAL", Style::default().fg(RED).bold())
    } else if s.contains("HIGH") {
        ("HIGH    ", Style::default().fg(RED))
    } else if s.contains("MEDIUM") {
        ("MEDIUM  ", Style::default().fg(YELLOW))
    } else if s.contains("LOW") {
        ("LOW     ", Style::default().fg(GRAY))
    } else {
        ("UNKNOWN ", Style::default().fg(GRAY))
    }
}
