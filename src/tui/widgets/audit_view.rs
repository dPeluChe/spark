use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::scanner::secret_scanner::{Severity, FindingContext};

/// Render the audit scanning spinner
pub fn render_audit_scanning(frame: &mut Frame, area: Rect, tick: usize) {
    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Scanning for secrets and exposed credentials...", spinner),
            Style::default().fg(RED).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled("  API keys, tokens, passwords, sensitive files", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled("  [ESC] Cancel", Style::default().fg(GRAY))),
    ];
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

/// Render the main audit results list (projects with findings)
pub fn render_audit_list(frame: &mut Frame, area: Rect, model: &AuditModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(5),   // table
        Constraint::Length(2), // help
    ]).split(area);

    // Header with scanned path
    let scan_path_display = model.scan_path.as_ref().map(|p| {
        let s = p.display().to_string();
        let home = std::env::var("HOME").unwrap_or_default();
        if s.starts_with(&home) { format!("~{}", &s[home.len()..]) } else { s }
    }).unwrap_or_else(|| "not configured".into());

    let dep_count = model.dep_vulns.len();
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" SECURITY AUDIT ", Style::default().fg(WHITE).bg(RED).bold()),
            Span::raw("  "),
            if model.total_critical > 0 {
                Span::styled(format!("{} critical  ", model.total_critical), Style::default().fg(RED).bold())
            } else {
                Span::styled("0 critical  ", Style::default().fg(GREEN))
            },
            if model.total_warning > 0 {
                Span::styled(format!("{} warnings  ", model.total_warning), Style::default().fg(YELLOW).bold())
            } else {
                Span::styled("0 warnings  ", Style::default().fg(GRAY))
            },
            Span::styled(format!("{} info  ", model.total_info), Style::default().fg(GRAY)),
            if dep_count > 0 {
                Span::styled(format!("{} dep vulns", dep_count), Style::default().fg(Color::Rgb(255, 140, 0)).bold())
            } else {
                Span::styled("deps clean", Style::default().fg(GREEN))
            },
        ]),
        Line::from(Span::styled(
            format!("{} projects scanned — {}", model.results.len(), scan_path_display),
            Style::default().fg(GRAY),
        )),
    ]);
    frame.render_widget(header, chunks[0]);

    if model.results.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  No security findings detected", Style::default().fg(GREEN).bold())),
            Line::from(""),
            Line::from(Span::styled("  All scanned projects look clean", Style::default().fg(GRAY))),
        ]);
        frame.render_widget(empty, chunks[1]);
    } else {
        // Table header
        let table_header = Row::new(vec![
            Cell::from("  Project").style(Style::default().fg(RED).bold()),
            Cell::from("Critical").style(Style::default().fg(RED).bold()),
            Cell::from("Warning").style(Style::default().fg(RED).bold()),
            Cell::from("Info").style(Style::default().fg(RED).bold()),
            Cell::from("Path").style(Style::default().fg(RED).bold()),
        ]);

        let home = std::env::var("HOME").unwrap_or_default();
        let visible_height = chunks[1].height.saturating_sub(3) as usize;
        let scroll_offset = if model.cursor >= visible_height {
            model.cursor - visible_height + 1
        } else {
            0
        };

        let dep_row_idx = model.results.len(); // deps entry appears after all projects
        let total_rows = dep_row_idx + if dep_count > 0 { 1 } else { 0 };

        let mut rows: Vec<Row> = model.results.iter().enumerate()
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

                let row_style = if is_selected { Style::default().bg(DARK_BG) } else { Style::default() };

                Row::new(vec![
                    Cell::from(format!("{} {}", cursor, result.project_name)),
                    Cell::from(format!("{}", result.critical_count))
                        .style(if result.critical_count > 0 { Style::default().fg(RED).bold() } else { Style::default().fg(GRAY) }),
                    Cell::from(format!("{}", result.warning_count))
                        .style(if result.warning_count > 0 { Style::default().fg(YELLOW) } else { Style::default().fg(GRAY) }),
                    Cell::from(format!("{}", result.info_count))
                        .style(Style::default().fg(GRAY)),
                    Cell::from(display_path).style(Style::default().fg(TERM_GRAY)),
                ]).style(row_style)
            }).collect();

        // Dependency vulnerabilities row
        if dep_count > 0 && dep_row_idx >= scroll_offset {
            let is_selected = model.cursor == dep_row_idx;
            let cursor = if is_selected { ">" } else { " " };
            let row_style = if is_selected { Style::default().bg(DARK_BG) } else { Style::default() };
            rows.push(Row::new(vec![
                Cell::from(format!("{} [dependencies]", cursor))
                    .style(Style::default().fg(Color::Rgb(255, 140, 0))),
                Cell::from(format!("{}", dep_count))
                    .style(Style::default().fg(Color::Rgb(255, 140, 0)).bold()),
                Cell::from(""),
                Cell::from(""),
                Cell::from("OSV.dev scan").style(Style::default().fg(TERM_GRAY)),
            ]).style(row_style));
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
        frame.render_widget(table, chunks[1]);
    }

    let help_text = if model.results.is_empty() && model.dep_vulns.is_empty() && !model.scanning {
        "[r] Scan current directory  [TAB] Next  [q] Back"
    } else {
        "[ENTER] Detail / Deps  [r] Rescan  [TAB] Next  [q] Back"
    };
    let help = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("  Scanning: {}  (run spark from the project you want to audit)", scan_path_display),
            Style::default().fg(TERM_GRAY),
        )),
        Line::from(Span::styled(help_text, Style::default().fg(GRAY))),
    ]);
    frame.render_widget(help, chunks[2]);
}

/// Render detail view for a specific project's findings
pub fn render_audit_detail(frame: &mut Frame, area: Rect, model: &AuditModel) {
    let result = match model.results.get(model.cursor) {
        Some(r) => r,
        None => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(5),   // findings table
        Constraint::Length(2), // help
    ]).split(area);

    // Header
    let home = std::env::var("HOME").unwrap_or_default();
    let path_str = result.project_path.display().to_string();
    let short_path = if path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else {
        path_str
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", result.project_name),
                Style::default().fg(WHITE).bg(RED).bold(),
            ),
            Span::raw("  "),
            Span::styled(format!("{} findings", result.findings.len()), Style::default().fg(WHITE)),
        ]),
        Line::from(Span::styled(short_path, Style::default().fg(GRAY))),
    ]);
    frame.render_widget(header, chunks[0]);

    // Findings table
    let table_header = Row::new(vec![
        Cell::from("  Sev").style(Style::default().fg(RED).bold()),
        Cell::from("Context").style(Style::default().fg(RED).bold()),
        Cell::from("File").style(Style::default().fg(RED).bold()),
        Cell::from("Line").style(Style::default().fg(RED).bold()),
        Cell::from("Description").style(Style::default().fg(RED).bold()),
    ]);

    let visible_height = chunks[1].height.saturating_sub(3) as usize;
    let scroll_offset = if model.detail_cursor >= visible_height {
        model.detail_cursor - visible_height + 1
    } else {
        0
    };

    let rows: Vec<Row> = result.findings.iter().enumerate()
        .skip(scroll_offset)
        .map(|(i, finding)| {
            let is_selected = model.detail_cursor == i;
            let cursor = if is_selected { ">" } else { " " };

            let (sev_icon, sev_style) = match finding.severity {
                Severity::Critical => ("!!", Style::default().fg(RED).bold()),
                Severity::Warning => ("! ", Style::default().fg(YELLOW)),
                Severity::Info => ("i ", Style::default().fg(GRAY)),
            };

            // File path relative to project
            let rel_path = finding.file_path.strip_prefix(&result.project_path)
                .unwrap_or(&finding.file_path);
            let file_str = rel_path.display().to_string();
            let display_file = if file_str.len() > 25 {
                format!("...{}", &file_str[file_str.len() - 22..])
            } else {
                file_str
            };

            let line_str = if finding.line_number > 0 {
                format!("{}", finding.line_number)
            } else {
                "-".into()
            };

            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };

            let ctx_style = match finding.context {
                FindingContext::SourceCode => Style::default().fg(RED),
                FindingContext::Config => Style::default().fg(YELLOW),
                FindingContext::Test => Style::default().fg(GRAY),
                FindingContext::Documentation => Style::default().fg(GRAY),
                FindingContext::BuildArtifact => Style::default().fg(YELLOW),
            };

            Row::new(vec![
                Cell::from(format!("{}{}", cursor, sev_icon)).style(sev_style),
                Cell::from(format!("{}", finding.context)).style(ctx_style),
                Cell::from(display_file),
                Cell::from(line_str).style(Style::default().fg(GRAY)),
                Cell::from(finding.description.clone()),
            ]).style(row_style)
        }).collect();

    let scroll_info = format!(" {}/{} ", model.detail_cursor + 1, result.findings.len());

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(12),
            Constraint::Min(15),
            Constraint::Length(5),
            Constraint::Min(20),
        ],
    )
    .header(table_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(RED))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, chunks[1]);

    // Show redacted match for selected finding
    let detail_line = if let Some(finding) = result.findings.get(model.detail_cursor) {
        Line::from(vec![
            Span::styled("  Match: ", Style::default().fg(PURPLE)),
            Span::styled(&finding.redacted_match, Style::default().fg(YELLOW)),
        ])
    } else {
        Line::from("")
    };

    let help = Paragraph::new(vec![
        detail_line,
        Line::from(Span::styled(
            "  [q] Back  [j/k] Navigate  [PgUp/PgDn] Page",
            Style::default().fg(GRAY),
        )),
    ]);
    frame.render_widget(help, chunks[2]);
}

/// Render dependency vulnerability detail view
pub fn render_audit_deps(frame: &mut Frame, area: Rect, model: &AuditModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
    ]).split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" DEPENDENCY VULNERABILITIES ", Style::default().fg(WHITE).bg(Color::Rgb(255, 140, 0)).bold()),
            Span::raw("  "),
            Span::styled(format!("{} vulnerabilities", model.dep_vulns.len()), Style::default().fg(WHITE)),
        ]),
        Line::from(Span::styled("OSV.dev — open source vulnerability database", Style::default().fg(GRAY))),
    ]);
    frame.render_widget(header, chunks[0]);

    if model.dep_vulns.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  No dependency vulnerabilities found", Style::default().fg(GREEN).bold())),
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
    } else { 0 };

    let rows: Vec<Row> = model.dep_vulns.iter().enumerate()
        .skip(scroll_offset)
        .map(|(i, v)| {
            let is_selected = model.dep_cursor == i;
            let cursor = if is_selected { ">" } else { " " };
            let row_style = if is_selected { Style::default().bg(DARK_BG) } else { Style::default() };
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
            ]).style(row_style)
        }).collect();

    let scroll_info = format!(" {}/{} ", model.dep_cursor + 1, model.dep_vulns.len());
    let table = Table::new(rows, [
        Constraint::Length(12),
        Constraint::Length(20),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Min(20),
    ])
    .header(table_header)
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(255, 140, 0)))
        .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))));
    frame.render_widget(table, chunks[1]);

    let detail_line = if let Some(v) = model.dep_vulns.get(model.dep_cursor) {
        Line::from(vec![
            Span::styled("  CVE: ", Style::default().fg(PURPLE)),
            Span::styled(v.id.clone(), Style::default().fg(YELLOW)),
            Span::raw("  "),
            Span::styled(format!("ecosystem: {}", v.ecosystem), Style::default().fg(GRAY)),
            Span::raw("  "),
            Span::styled("[q] Back  [j/k] Navigate", Style::default().fg(TERM_GRAY)),
        ])
    } else {
        Line::from(Span::styled("  [q] Back  [j/k] Navigate", Style::default().fg(GRAY)))
    };
    frame.render_widget(Paragraph::new(vec![detail_line]), chunks[2]);
}

fn dep_severity_style(severity: &str) -> (&'static str, Style) {
    let s = severity.to_uppercase();
    if s.contains("CRITICAL") { ("CRITICAL", Style::default().fg(RED).bold()) }
    else if s.contains("HIGH") { ("HIGH    ", Style::default().fg(RED)) }
    else if s.contains("MEDIUM") { ("MEDIUM  ", Style::default().fg(YELLOW)) }
    else if s.contains("LOW") { ("LOW     ", Style::default().fg(GRAY)) }
    else { ("UNKNOWN ", Style::default().fg(GRAY)) }
}
