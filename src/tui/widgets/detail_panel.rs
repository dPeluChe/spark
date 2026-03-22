use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;

/// Render detailed view of a single repository
pub fn render_detail(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    if model.cursor >= model.repos.len() {
        return;
    }
    let repo = &model.repos[model.cursor];

    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Length(8),  // info block
        Constraint::Min(5),    // artifacts
        Constraint::Length(2),  // help
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

    let remote_info = repo
        .remote_url
        .as_deref()
        .unwrap_or("No remote configured");

    let git_status_text = if repo.is_dirty {
        "Dirty (uncommitted changes)"
    } else {
        "Clean"
    };

    let info_lines = vec![
        Line::from(vec![
            Span::styled("  Branch:     ", Style::default().fg(PURPLE)),
            Span::styled(&repo.branch, Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Remote:     ", Style::default().fg(PURPLE)),
            Span::styled(remote_info, Style::default().fg(if repo.has_remote { GREEN } else { RED })),
        ]),
        Line::from(vec![
            Span::styled("  Last Commit:", Style::default().fg(PURPLE)),
            Span::styled(format!(" {}", last_commit), Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Commits:    ", Style::default().fg(PURPLE)),
            Span::styled(format!(" {}", repo.commit_count), Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Git Status: ", Style::default().fg(PURPLE)),
            Span::styled(
                git_status_text,
                Style::default().fg(if repo.is_dirty { YELLOW } else { GREEN }),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Total Size: ", Style::default().fg(PURPLE)),
            Span::styled(
                format!(" {}", format_size(repo.total_size)),
                Style::default().fg(WHITE),
            ),
        ]),
    ];

    let info = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(PURPLE))
            .title(Span::styled(" Repository Info ", Style::default().fg(PURPLE).bold())),
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
        for (i, artifact) in repo.artifacts.iter().enumerate() {
            let is_selected = i == 0; // TODO: track artifact cursor
            let cursor = if is_selected { "❯" } else { " " };

            artifact_lines.push(Line::from(vec![
                Span::raw(format!("  {} ", cursor)),
                Span::styled(
                    format!("{:<15}", artifact.kind.to_string()),
                    Style::default().fg(YELLOW),
                ),
                Span::styled(
                    format_size(artifact.size),
                    Style::default().fg(if artifact.size > 100_000_000 { RED } else { WHITE }),
                ),
                Span::styled(
                    format!("  {}", artifact.path.display()),
                    Style::default().fg(GRAY),
                ),
            ]));
        }
    }

    let artifacts = Paragraph::new(artifact_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(YELLOW))
            .title(Span::styled(
                format!(" Artifacts ({}) ", format_size(repo.artifact_size)),
                Style::default().fg(YELLOW).bold(),
            )),
    );
    frame.render_widget(artifacts, chunks[2]);

    // Help
    let help = Paragraph::new(Span::styled(
        "[d] Clean Artifacts • [D] Trash Repo • [ESC] Back to List",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[3]);
}
