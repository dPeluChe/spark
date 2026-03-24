use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::scanner::repo_manager::RepoStatus;
use crate::utils::fs::format_size;

/// Render the repo manager view
pub fn render_repo_manager(frame: &mut Frame, area: Rect, model: &RepoManagerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(5),   // repo table
        Constraint::Length(2), // help
    ])
    .split(area);

    render_header(frame, chunks[0], model);
    render_repo_table(frame, chunks[1], model);
    render_help(frame, chunks[2], model);
}

fn render_header(frame: &mut Frame, area: Rect, model: &RepoManagerModel) {
    let behind_count = model
        .repos
        .iter()
        .filter(|r| matches!(r.status, RepoStatus::Behind(_) | RepoStatus::Diverged { .. }))
        .count();

    let dirty_count = model.repos.iter().filter(|r| r.status == RepoStatus::Dirty).count();

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " 📦 REPO MANAGER ",
                Style::default().fg(WHITE).bg(GREEN).bold(),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} repos", model.repos.len()),
                Style::default().fg(GRAY),
            ),
            if behind_count > 0 {
                Span::styled(
                    format!("  {} need pull", behind_count),
                    Style::default().fg(YELLOW).bold(),
                )
            } else {
                Span::raw("")
            },
            if dirty_count > 0 {
                Span::styled(
                    format!("  {} dirty", dirty_count),
                    Style::default().fg(RED),
                )
            } else {
                Span::raw("")
            },
        ]),
        Line::from(Span::styled(
            format!("Root: {}", model.root.display()),
            Style::default().fg(GRAY),
        )),
    ]);
    frame.render_widget(header, area);
}

fn render_repo_table(frame: &mut Frame, area: Rect, model: &RepoManagerModel) {
    if model.repos.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No managed repos found.",
                Style::default().fg(GRAY),
            )),
            Line::from(Span::styled(
                "  Press [c] to clone a repo (ghq-style: host/owner/name layout)",
                Style::default().fg(GRAY),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(GREEN)),
        );
        frame.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("  Repository").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Branch").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Status").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Size").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Commit").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Host/Owner").style(Style::default().fg(PURPLE).bold()),
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

            let status_style = match &repo.status {
                RepoStatus::UpToDate => Style::default().fg(GREEN),
                RepoStatus::Behind(_) => Style::default().fg(YELLOW).bold(),
                RepoStatus::Ahead(_) => Style::default().fg(BLUE),
                RepoStatus::Diverged { .. } => Style::default().fg(RED).bold(),
                RepoStatus::Dirty => Style::default().fg(YELLOW),
                RepoStatus::Error(_) => Style::default().fg(RED),
                RepoStatus::Checking => Style::default().fg(GRAY),
            };

            let status_icon = match &repo.status {
                RepoStatus::UpToDate => "✓",
                RepoStatus::Behind(_) => "↓",
                RepoStatus::Ahead(_) => "↑",
                RepoStatus::Diverged { .. } => "↕",
                RepoStatus::Dirty => "●",
                RepoStatus::Error(_) => "✘",
                RepoStatus::Checking => "⟳",
            };

            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };

            let last_commit = repo.last_commit.as_deref().unwrap_or("-");

            let size_str = if repo.size > 0 { format_size(repo.size) } else { "-".into() };

            Row::new(vec![
                Cell::from(format!("{} [{}] {}", cursor, checkbox, repo.name))
                    .style(if is_selected {
                        Style::default().fg(WHITE).bold()
                    } else {
                        Style::default().fg(GRAY)
                    }),
                Cell::from(repo.branch.clone())
                    .style(Style::default().fg(PURPLE)),
                Cell::from(format!("{} {}", status_icon, repo.status))
                    .style(status_style),
                Cell::from(size_str)
                    .style(Style::default().fg(GRAY)),
                Cell::from(last_commit)
                    .style(Style::default().fg(TERM_GRAY)),
                Cell::from(format!("{}/{}", repo.host, repo.owner))
                    .style(Style::default().fg(GRAY)),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(22),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Min(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GREEN))
            .title(Span::styled(
                " Managed Repositories ",
                Style::default().fg(GREEN).bold(),
            )),
    );
    frame.render_widget(table, area);
}

fn render_help(frame: &mut Frame, area: Rect, _model: &RepoManagerModel) {
    let help = Paragraph::new(Span::styled(
        "[ENTER] Actions • [c] Clone • [SPACE] Select • [u] Pull • [U] Pull All Behind • [r] Refresh • [TAB] Switch",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, area);
}

/// Render action modal for the currently selected repo
pub fn render_action_modal(frame: &mut Frame, area: Rect, model: &RepoManagerModel) {
    let repo = match model.repos.get(model.cursor) {
        Some(r) => r,
        None => return,
    };

    let modal_area = center_modal(frame, area, 62, 18);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(GREEN))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let home = std::env::var("HOME").unwrap_or_default();
    let path_display = repo.path.display().to_string();
    let short_path = if path_display.starts_with(&home) {
        format!("~{}", &path_display[home.len()..])
    } else {
        path_display
    };

    let status_icon = match &repo.status {
        RepoStatus::UpToDate => "✓ Up to date",
        RepoStatus::Behind(n) => &format!("↓ {} behind", n),
        RepoStatus::Ahead(n) => &format!("↑ {} ahead", n),
        RepoStatus::Diverged { ahead, behind } => &format!("↕ {} ahead, {} behind", ahead, behind),
        RepoStatus::Dirty => "● Dirty (uncommitted changes)",
        RepoStatus::Error(e) => e,
        RepoStatus::Checking => "⟳ Checking...",
    };

    let last_commit = repo.last_commit.as_deref().unwrap_or("unknown");

    let lines = vec![
        Line::from(Span::styled(
            format!(" {} ", repo.name),
            Style::default().fg(WHITE).bg(GREEN).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Branch:  ", Style::default().fg(PURPLE)),
            Span::styled(&repo.branch, Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Status:  ", Style::default().fg(PURPLE)),
            Span::styled(status_icon, Style::default().fg(match &repo.status {
                RepoStatus::UpToDate => GREEN,
                RepoStatus::Behind(_) => YELLOW,
                RepoStatus::Ahead(_) => BLUE,
                RepoStatus::Diverged { .. } => RED,
                RepoStatus::Dirty => YELLOW,
                _ => GRAY,
            })),
        ]),
        Line::from(vec![
            Span::styled("  Updated: ", Style::default().fg(PURPLE)),
            Span::styled(last_commit, Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Host:    ", Style::default().fg(PURPLE)),
            Span::styled(format!("{}/{}", repo.host, repo.owner), Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Path:    ", Style::default().fg(PURPLE)),
            Span::styled(&short_path, Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Actions:",
            Style::default().fg(YELLOW).bold(),
        )),
        Line::from(vec![
            Span::styled("  [u] ", Style::default().fg(BLUE).bold()),
            Span::styled("Pull (fast-forward)", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  [o] ", Style::default().fg(CYAN).bold()),
            Span::styled("Open in terminal", Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  [d] ", Style::default().fg(RED).bold()),
            Span::styled("Remove repository", Style::default().fg(WHITE)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [q] Close    [ESC] Close",
            Style::default().fg(GRAY),
        )),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render post-clone summary with path, alias suggestion, and agent tips
pub fn render_clone_summary(frame: &mut Frame, area: Rect, model: &RepoManagerModel) {
    let summary = match &model.last_clone {
        Some(s) => s,
        None => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Min(10),   // content
        Constraint::Length(2),  // help
    ])
    .split(area);

    // Header
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            " ✓ CLONE SUCCESSFUL ",
            Style::default().fg(WHITE).bg(GREEN).bold(),
        )),
        Line::from(""),
    ]);
    frame.render_widget(header, chunks[0]);

    // Content
    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Repository:  ", Style::default().fg(PURPLE).bold()),
            Span::styled(&summary.repo_name, Style::default().fg(WHITE).bold()),
        ]),
        Line::from(vec![
            Span::styled("  Remote:      ", Style::default().fg(PURPLE)),
            Span::styled(&summary.remote_url, Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Path:        ", Style::default().fg(PURPLE)),
            Span::styled(&summary.short_path, Style::default().fg(CYAN).bold()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Quick Access",
            Style::default().fg(YELLOW).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Shell alias: ", Style::default().fg(PURPLE)),
            Span::styled(&summary.alias_cmd, Style::default().fg(WHITE)),
        ]),
        Line::from(vec![
            Span::styled("  Add to:      ", Style::default().fg(PURPLE)),
            Span::styled("~/.bashrc  or  ~/.zshrc", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  For AI Agents (Claude Code, Cursor, etc.)",
            Style::default().fg(YELLOW).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  CLAUDE.md:   ", Style::default().fg(PURPLE)),
            Span::styled(
                format!("Add to project: \"Repo path: {}\"", summary.short_path),
                Style::default().fg(GRAY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  .cursorrules:", Style::default().fg(PURPLE)),
            Span::styled(
                format!("\"Project root: {}\"", summary.short_path),
                Style::default().fg(GRAY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  cd command:  ", Style::default().fg(PURPLE)),
            Span::styled(
                format!("cd {}", summary.short_path),
                Style::default().fg(WHITE),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Spark Repo Manager",
            Style::default().fg(YELLOW).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Pull update: ", Style::default().fg(PURPLE)),
            Span::styled(
                "spark --scan-only  →  [G] Repos  →  [u] Pull",
                Style::default().fg(GRAY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Config root: ", Style::default().fg(PURPLE)),
            Span::styled(
                "repos_root in ~/.config/spark/config.toml",
                Style::default().fg(GRAY),
            ),
        ]),
    ];

    // Add full path if different from short
    if summary.repo_path != summary.short_path {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Full path:   ", Style::default().fg(PURPLE)),
            Span::styled(&summary.repo_path, Style::default().fg(GRAY)),
        ]));
    }

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GREEN)),
    );
    frame.render_widget(content, chunks[1]);

    // Help
    let help = Paragraph::new(Span::styled(
        "[ENTER] Continue to Repo Manager",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[2]);
}

/// Render clone URL input overlay
pub fn render_clone_input(frame: &mut Frame, area: Rect, model: &RepoManagerModel) {
    let modal_area = center_modal(frame, area, 70, 10);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(GREEN))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let mut lines = vec![
        Line::from(Span::styled(
            "CLONE REPOSITORY",
            Style::default().fg(GREEN).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter git URL (SSH or HTTPS):",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
    ];

    // Input field
    let input_display = if model.cloning {
        format!("⟳ Cloning {}...", model.clone_input)
    } else {
        format!("{}█", model.clone_input)
    };

    lines.push(Line::from(Span::styled(
        input_display,
        Style::default().fg(WHITE).bg(DARK),
    )));

    // Error message
    if let Some(err) = &model.clone_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(RED),
        )));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "e.g. git@github.com:user/repo.git",
            Style::default().fg(GRAY),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[ENTER] Clone • [ESC] Cancel",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}
