//! Clone flow: URL input modal and post-clone summary screen.

use crate::tui::model::*;
use crate::tui::styles::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_clone_summary(frame: &mut Frame, area: Rect, model: &RepoManagerModel) {
    let summary = match &model.last_clone {
        Some(s) => s,
        None => return,
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(2),
    ])
    .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            " ✓ CLONE SUCCESSFUL ",
            Style::default().fg(WHITE).bg(GREEN).bold(),
        )),
        Line::from(""),
    ]);
    frame.render_widget(header, chunks[0]);

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

    let help = Paragraph::new(Span::styled(
        "[ENTER] Continue to Repo Manager",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[2]);
}

/// Clone URL input overlay.
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

    let input_display = if model.cloning {
        format!("⟳ Cloning {}...", model.clone_input)
    } else {
        format!("{}█", model.clone_input)
    };

    lines.push(Line::from(Span::styled(
        input_display,
        Style::default().fg(WHITE).bg(DARK),
    )));

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
