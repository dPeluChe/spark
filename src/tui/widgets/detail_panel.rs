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
        render_container_detail(frame, area, repo);
    } else {
        render_repo_detail(frame, area, repo);
    }
}

fn render_container_detail(frame: &mut Frame, area: Rect, repo: &crate::scanner::repo_scanner::RepoInfo) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Min(5),    // child repo list
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

    // Header
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", repo.name),
                Style::default().fg(WHITE).bg(CYAN).bold(),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Container — {} repos inside", repo.child_repo_count),
                Style::default().fg(CYAN).bold(),
            ),
        ]),
        Line::from(Span::styled(short, Style::default().fg(GRAY))),
    ]);
    frame.render_widget(header, chunks[0]);

    // List child repos by scanning child dirs that have .git
    let mut lines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&repo.path) {
        let mut children: Vec<(String, bool)> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let has_git = e.path().join(".git").exists();
                (name, has_git)
            })
            .filter(|(name, _)| !name.starts_with('.'))
            .collect();
        children.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        for (name, has_git) in &children {
            if *has_git {
                lines.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(format!("{:<30}", name), Style::default().fg(WHITE)),
                    Span::styled("repo", Style::default().fg(GREEN)),
                ]));
            }
        }

        // Show non-repo dirs too but dimmed
        let non_repo_count = children.iter().filter(|(_, g)| !g).count();
        if non_repo_count > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("    + {} other folders", non_repo_count),
                Style::default().fg(TERM_GRAY),
            )));
        }
    }

    let list = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CYAN))
            .title(Span::styled(" Contents ", Style::default().fg(CYAN).bold())),
    );
    frame.render_widget(list, chunks[1]);

    // Help
    let help = Paragraph::new(Line::from(vec![
        Span::styled("[a] ", Style::default().fg(GREEN).bold()),
        Span::styled(format!("Add to scan paths ({} repos)  ", repo.child_repo_count), Style::default().fg(GRAY)),
        Span::styled("[q] ", Style::default().fg(GRAY).bold()),
        Span::styled("Back", Style::default().fg(GRAY)),
    ]));
    frame.render_widget(help, chunks[2]);
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

    let info = Paragraph::new(vec![
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
    ]).block(
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
