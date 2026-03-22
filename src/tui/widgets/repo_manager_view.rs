use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::scanner::repo_manager::RepoStatus;

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
            format!(
                "Root: {}",
                model.root.as_deref().unwrap_or("~/repos")
            ),
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
                Cell::from(format!("{}/{}", repo.host, repo.owner))
                    .style(Style::default().fg(GRAY)),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(25),
            Constraint::Length(15),
            Constraint::Length(20),
            Constraint::Min(20),
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
        "[c] Clone URL • [SPACE] Select • [u] Pull Selected • [U] Pull All Behind • [r] Refresh • [ESC] Back",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, area);
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
