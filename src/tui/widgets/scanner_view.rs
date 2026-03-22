use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;

/// Render the scanner mode
pub fn render_scanner(frame: &mut Frame, area: Rect, model: &ScannerModel, tick: usize) {
    match model.state {
        ScannerState::ScanConfig => render_scan_config(frame, area, model),
        ScannerState::Scanning => render_scanning(frame, area, model, tick),
        ScannerState::ScanResults => render_scan_results(frame, area, model),
        ScannerState::RepoDetail => {
            super::detail_panel::render_detail(frame, area, model);
        }
        ScannerState::CleanConfirm => {
            render_scan_results(frame, area, model);
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
        ScannerState::CleanSummary => render_clean_summary(frame, area, model),
    }
}

fn render_scan_config(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Min(5),   // directory list
        Constraint::Length(2), // help
    ])
    .split(area);

    // Title
    let title = Paragraph::new(vec![
        Line::from(Span::styled(
            " 🔍 REPOSITORY SCANNER ",
            Style::default().fg(WHITE).bg(PURPLE).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Select directories to scan for repositories:",
            Style::default().fg(GRAY),
        )),
    ]);
    frame.render_widget(title, chunks[0]);

    // Directory list
    let mut lines = Vec::new();
    for (i, dir) in model.discovered_dirs.iter().enumerate() {
        let is_selected = model.cursor == i;
        let is_checked = model.selected_scan_dirs.contains(&i);

        let cursor = if is_selected {
            Span::styled("❯ ", Style::default().fg(GREEN).bold())
        } else {
            Span::raw("  ")
        };

        let checkbox = render_checkbox(is_checked);

        let dir_style = if is_selected {
            Style::default().fg(WHITE).bold()
        } else {
            Style::default().fg(GRAY)
        };

        lines.push(Line::from(vec![
            cursor,
            checkbox,
            Span::styled(dir.display().to_string(), dir_style),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No project directories found. Press 'a' to add a custom path.",
            Style::default().fg(YELLOW),
        )));
    }

    let list = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(PURPLE))
            .title(Span::styled(
                " Directories ",
                Style::default().fg(PURPLE).bold(),
            )),
    );
    frame.render_widget(list, chunks[1]);

    // Help
    let help = Paragraph::new(Span::styled(
        "[SPACE] Toggle • [ENTER] Start Scan • [TAB] Switch to Updater • [Q] Quit",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[2]);
}

fn render_scanning(frame: &mut Frame, area: Rect, model: &ScannerModel, tick: usize) {
    let spinner = crate::tui::styles::SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Scanning directories...", spinner),
            Style::default().fg(BLUE).bold(),
        )),
        Line::from(""),
        Line::from(format!(
            "  Directories scanned: {}",
            model.scan_progress_dirs
        )),
        Line::from(format!(
            "  Repositories found:  {}",
            model.scan_progress_repos
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Current: {}", model.scan_progress_current),
            Style::default().fg(GRAY),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .padding(Padding::new(2, 2, 2, 2)),
    );
    frame.render_widget(paragraph, area);
}

fn render_scan_results(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // summary bar
        Constraint::Min(5),   // table
        Constraint::Length(2), // help
    ])
    .split(area);

    // Summary bar
    let total_size: u64 = model.repos.iter().map(|r| r.total_size).sum();
    let total_artifact: u64 = model.repos.iter().map(|r| r.artifact_size).sum();

    let summary = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} repos ", model.repos.len()),
            Style::default().fg(WHITE).bg(PURPLE).bold(),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Total: {} ", format_size(total_size)),
            Style::default().fg(GRAY),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Recoverable: {} ", format_size(total_artifact)),
            Style::default().fg(YELLOW).bold(),
        ),
    ]));
    frame.render_widget(summary, chunks[0]);

    // Table
    let header = Row::new(vec![
        Cell::from("  Name").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Health").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Last Commit").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Size").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Artifacts").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Remote").style(Style::default().fg(PURPLE).bold()),
    ]);

    let rows: Vec<Row> = model
        .repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let is_selected = model.cursor == i;
            let is_checked = model.checked.contains(&i);

            let cursor = if is_selected { "❯" } else { " " };
            let checkbox = if is_checked { "✔" } else { " " };
            let name = format!("{} [{}] {}", cursor, checkbox, repo.name);

            let grade_style = health_grade_style(&repo.health_grade);

            let last_commit = repo
                .last_commit_date
                .map(|d| {
                    let days = (chrono::Utc::now() - d).num_days();
                    if days < 1 {
                        "today".into()
                    } else if days < 30 {
                        format!("{}d ago", days)
                    } else if days < 365 {
                        format!("{}mo ago", days / 30)
                    } else {
                        format!("{}y ago", days / 365)
                    }
                })
                .unwrap_or_else(|| "never".into());

            let remote = if repo.has_remote { "✔" } else { "✘" };

            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(name),
                Cell::from(format!("{} ({})", repo.health_grade, repo.health_score))
                    .style(grade_style),
                Cell::from(last_commit),
                Cell::from(format_size(repo.total_size)),
                Cell::from(format_size(repo.artifact_size))
                    .style(if repo.artifact_size > 100_000_000 {
                        Style::default().fg(RED)
                    } else if repo.artifact_size > 10_000_000 {
                        Style::default().fg(YELLOW)
                    } else {
                        Style::default()
                    }),
                Cell::from(remote),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(25),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(7),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(PURPLE)),
    );
    frame.render_widget(table, chunks[1]);

    // Help bar
    let help = Paragraph::new(Span::styled(
        "[SPACE] Select • [ENTER] Detail • [d] Clean Artifacts • [D] Trash Repo • [s] Sort • [TAB] Mode • [Q] Quit",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[2]);
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

    let paragraph = Paragraph::new(lines)
        .block(Block::default().padding(Padding::new(2, 2, 2, 2)));
    frame.render_widget(paragraph, area);
}

fn render_clean_summary(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let total_recovered: u64 = model.clean_results.iter().map(|(_, bytes, _, _)| *bytes).sum();
    let success_count = model.clean_results.iter().filter(|(_, _, ok, _)| *ok).count();
    let fail_count = model.clean_results.len() - success_count;

    let mut lines = vec![
        Line::from(Span::styled(
            " 🧹 CLEANUP COMPLETE ",
            Style::default().fg(WHITE).bg(GREEN).bold(),
        )),
        Line::from(""),
        Line::from(format!("Space recovered: {}", format_size(total_recovered))),
        Line::from(format!(
            "Successful: {}  |  Failed: {}",
            success_count, fail_count
        )),
        Line::from(""),
    ];

    for (_, _, success, error) in &model.clean_results {
        if !success {
            if let Some(err) = error {
                lines.push(Line::from(Span::styled(
                    format!("  ✘ {}", err),
                    Style::default().fg(RED),
                )));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Press ENTER to return]",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().padding(Padding::new(2, 2, 2, 2)));
    frame.render_widget(paragraph, area);
}

