use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;

/// Render detailed view — container or regular repo
pub fn render_detail(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    if model.cursor >= model.repos.len() {
        return;
    }
    let repo = &model.repos[model.cursor];

    if repo.is_container {
        render_container_detail(frame, area, model);
    } else {
        render_repo_detail(frame, area, repo);
    }
}

fn render_container_detail(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let repo = &model.repos[model.cursor];

    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Min(5),    // child repo table
        Constraint::Length(1), // help
    ])
    .split(area);

    let home = std::env::var("HOME").unwrap_or_default();
    let path_str = repo.path.display().to_string();
    let short = if path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else {
        path_str
    };

    let total_artifacts: u64 = model.container_children.iter().map(|r| r.artifact_size).sum();

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", repo.name),
                Style::default().fg(WHITE).bg(CYAN).bold(),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} repos", model.container_children.len()),
                Style::default().fg(WHITE),
            ),
            if total_artifacts > 0 {
                Span::styled(
                    format!("  {} cleanable", format_size(total_artifacts)),
                    Style::default().fg(YELLOW).bold(),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::from(Span::styled(short, Style::default().fg(GRAY))),
    ]);
    frame.render_widget(header, chunks[0]);

    // Child repo table with scroll
    let visible_height = chunks[1].height.saturating_sub(3) as usize; // borders + header
    let scroll_offset = if model.container_cursor >= visible_height {
        model.container_cursor - visible_height + 1
    } else {
        0
    };

    let table_header = Row::new(vec![
        Cell::from("  Name").style(Style::default().fg(CYAN).bold()),
        Cell::from("Grade").style(Style::default().fg(CYAN).bold()),
        Cell::from("Branch").style(Style::default().fg(CYAN).bold()),
        Cell::from("Commit").style(Style::default().fg(CYAN).bold()),
        Cell::from("Artifacts").style(Style::default().fg(CYAN).bold()),
        Cell::from("").style(Style::default().fg(CYAN).bold()),
    ]);

    let rows: Vec<Row> = model.container_children.iter().enumerate()
        .skip(scroll_offset)
        .map(|(i, child)| {
            let is_selected = model.container_cursor == i;
            let grade_style = health_grade_style(&child.health_grade);

            let last_commit = child.last_commit_date
                .map(|d| {
                    let days = (chrono::Utc::now() - d).num_days();
                    if days < 1 { "today".into() }
                    else if days < 30 { format!("{}d", days) }
                    else if days < 365 { format!("{}mo", days / 30) }
                    else { format!("{}y", days / 365) }
                })
                .unwrap_or_else(|| "-".into());

            let cursor = if is_selected { ">" } else { " " };
            let dirty = if child.is_dirty { "changes" } else { "" };
            let ws = if child.workspace != crate::scanner::repo_scanner::WorkspaceType::None {
                format!(" [{}]", child.workspace)
            } else {
                String::new()
            };

            Row::new(vec![
                Cell::from(format!("{} {}{}", cursor, child.name, ws)),
                Cell::from(format!("{}{}", child.health_grade, child.health_score))
                    .style(grade_style),
                Cell::from(child.branch.clone())
                    .style(Style::default().fg(PURPLE)),
                Cell::from(last_commit),
                Cell::from(if child.artifact_size > 0 { format_size(child.artifact_size) } else { "-".into() })
                    .style(if child.artifact_size > 100_000_000 { Style::default().fg(RED) }
                        else if child.artifact_size > 10_000_000 { Style::default().fg(YELLOW) }
                        else { Style::default() }),
                Cell::from(dirty).style(Style::default().fg(YELLOW)),
            ]).style(if is_selected { Style::default().bg(DARK_BG) } else { Style::default() })
        }).collect();

    let scroll_info = format!(
        " {}/{} ",
        model.container_cursor + 1,
        model.container_children.len()
    );

    let table = Table::new(
        rows,
        [
            Constraint::Min(18),
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .header(table_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CYAN))
            .title(Span::styled(" Repos ", Style::default().fg(CYAN).bold()))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Enter] ", Style::default().fg(CYAN).bold()),
        Span::styled("Detail  ", Style::default().fg(GRAY)),
        Span::styled("[s] ", Style::default().fg(PURPLE).bold()),
        Span::styled("Sort  ", Style::default().fg(GRAY)),
        Span::styled("[a] ", Style::default().fg(GREEN).bold()),
        Span::styled("Add to scan  ", Style::default().fg(GRAY)),
        Span::styled("[q] ", Style::default().fg(GRAY).bold()),
        Span::styled("Back", Style::default().fg(GRAY)),
    ]));
    frame.render_widget(help, chunks[2]);
}

/// Render the selected container child's full detail (branch, commit, artifacts, actions)
pub fn render_child_detail(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    if let Some(child) = model.container_children.get(model.container_cursor) {
        render_repo_detail(frame, area, child);
    }
}

/// Confirm modal for deleting a container child repo
pub fn render_child_delete_confirm(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let child = match model.container_children.get(model.container_cursor) {
        Some(c) => c,
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
    let path_str = child.path.display().to_string();
    let short_path = if path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else {
        path_str
    };

    let mut lines = vec![
        Line::from(Span::styled("DELETE REPOSITORY", Style::default().fg(RED).bold())),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Repo: ", Style::default().fg(PURPLE)),
            Span::styled(&child.name, Style::default().fg(WHITE).bold()),
        ]),
        Line::from(vec![
            Span::styled("  Path: ", Style::default().fg(PURPLE)),
            Span::styled(&short_path, Style::default().fg(GRAY)),
        ]),
    ];

    if child.is_dirty {
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
    lines.push(Line::from(Span::styled("  Delete? (y/N)", Style::default().fg(WHITE))));

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn render_repo_detail(frame: &mut Frame, area: Rect, repo: &crate::scanner::repo_scanner::RepoInfo) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Length(7),  // info block
        Constraint::Min(5),    // artifacts
        Constraint::Length(1), // help
    ])
    .split(area);

    // Header
    let grade_style = health_grade_style(&repo.health_grade);
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", repo.name),
                Style::default().fg(WHITE).bg(PURPLE).bold(),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Health: {} ({})", repo.health_grade, repo.health_score),
                grade_style.bold(),
            ),
        ]),
        Line::from(Span::styled(
            repo.path.display().to_string(),
            Style::default().fg(GRAY),
        )),
    ]);
    frame.render_widget(header, chunks[0]);

    // Info block
    let last_commit = repo
        .last_commit_date
        .map(|d| {
            let days = (chrono::Utc::now() - d).num_days();
            if days < 1 {
                format!("{} (today)", d.format("%Y-%m-%d"))
            } else {
                format!("{} ({} days ago)", d.format("%Y-%m-%d"), days)
            }
        })
        .unwrap_or_else(|| "No commits".into());

    let remote_info = repo.remote_url.as_deref().unwrap_or("No remote");
    let git_status_text = if repo.is_dirty { "Dirty" } else { "Clean" };
    let ws_type = &repo.workspace;

    let mut info_lines = vec![
        Line::from(vec![
            Span::styled("  Branch:  ", Style::default().fg(PURPLE)),
            Span::styled(&repo.branch, Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Remote:  ", Style::default().fg(PURPLE)),
            Span::styled(remote_info, Style::default().fg(if repo.has_remote { GREEN } else { RED })),
        ]),
        Line::from(vec![
            Span::styled("  Commit:  ", Style::default().fg(PURPLE)),
            Span::styled(last_commit, Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Status:  ", Style::default().fg(PURPLE)),
            Span::styled(git_status_text, Style::default().fg(if repo.is_dirty { YELLOW } else { GREEN })),
        ]),
    ];

    if *ws_type != crate::scanner::repo_scanner::WorkspaceType::None {
        info_lines.push(Line::from(vec![
            Span::styled("  Type:    ", Style::default().fg(PURPLE)),
            Span::styled(format!("{}", ws_type), Style::default().fg(CYAN).bold()),
        ]));
    }

    let info = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(PURPLE))
            .title(Span::styled(" Info ", Style::default().fg(PURPLE).bold())),
    );
    frame.render_widget(info, chunks[1]);

    // Artifacts
    let mut artifact_lines = Vec::new();
    if repo.artifacts.is_empty() {
        artifact_lines.push(Line::from(Span::styled(
            "  No build artifacts found",
            Style::default().fg(GRAY),
        )));
    } else {
        for artifact in &repo.artifacts {
            artifact_lines.push(Line::from(vec![
                Span::styled(
                    format!("    {:<15}", artifact.kind.to_string()),
                    Style::default().fg(YELLOW),
                ),
                Span::styled(
                    format_size(artifact.size),
                    Style::default().fg(if artifact.size > 100_000_000 { RED } else { WHITE }),
                ),
            ]));
        }
    }

    let artifact_title = if repo.artifacts.is_empty() {
        " Artifacts (none) ".to_string()
    } else {
        format!(" Artifacts: {} recoverable ", format_size(repo.artifact_size))
    };

    let artifacts = Paragraph::new(artifact_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(YELLOW))
            .title(Span::styled(artifact_title, Style::default().fg(YELLOW).bold())),
    );
    frame.render_widget(artifacts, chunks[2]);

    // Help
    let artifact_names: Vec<String> = repo.artifacts.iter().map(|a| a.kind.to_string()).collect();
    let mut help_spans = Vec::new();
    if !artifact_names.is_empty() {
        help_spans.push(Span::styled("[c] ", Style::default().fg(YELLOW).bold()));
        help_spans.push(Span::styled(
            format!("Clean ({})", artifact_names.join(", ")),
            Style::default().fg(GRAY),
        ));
        help_spans.push(Span::raw("  "));
    }
    help_spans.push(Span::styled("[x] ", Style::default().fg(RED).bold()));
    help_spans.push(Span::styled("Delete repo", Style::default().fg(GRAY)));
    help_spans.push(Span::styled("  [q] ", Style::default().fg(GRAY).bold()));
    help_spans.push(Span::styled("Back", Style::default().fg(GRAY)));

    let help = Paragraph::new(Line::from(help_spans));
    frame.render_widget(help, chunks[3]);
}
