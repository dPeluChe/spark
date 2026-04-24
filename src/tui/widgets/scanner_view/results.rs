//! Scan results table with grouped repo rows and scroll tracking.

use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub(super) fn render_scan_results(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // summary bar
        Constraint::Min(5),    // table
        Constraint::Length(2), // help
    ])
    .split(area);

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

    let header = Row::new(vec![
        Cell::from("  Name").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Grade").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Branch").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Commit").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Size").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Cleanup").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Status").style(Style::default().fg(PURPLE).bold()),
    ]);

    let group_order = &model.group_order;

    let mut rows: Vec<Row> = Vec::new();
    for group in group_order {
        let group_repos: Vec<(usize, &crate::scanner::repo_scanner::RepoInfo)> = model
            .repos
            .iter()
            .enumerate()
            .filter(|(_, r)| r.group == *group)
            .collect();

        let group_label = format!("  {} ({})", group, group_repos.len());
        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(group_label, Style::default().fg(CYAN).bold())),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(Style::default().bg(Color::Rgb(25, 25, 35))),
        );

        for (i, repo) in group_repos {
            rows.push(build_repo_row(model, i, repo));
        }
    }

    let visible_height = chunks[1].height.saturating_sub(3) as usize;
    let scroll_offset = compute_scroll_offset(model, group_order, visible_height);
    let scrolled_rows: Vec<Row> = rows.into_iter().skip(scroll_offset).collect();
    let scroll_info = format!(" {}/{} ", model.cursor + 1, model.repos.len());

    let table = Table::new(
        scrolled_rows,
        [
            Constraint::Percentage(30),
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(PURPLE))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, chunks[1]);

    let help = Paragraph::new(Span::styled(
        "[ENTER] Detail • [a] Add path • [c] Clean • [x] Delete • [s] Sort • [?] Health info • [TAB] Next",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[2]);
}

fn build_repo_row<'a>(
    model: &'a ScannerModel,
    i: usize,
    repo: &'a crate::scanner::repo_scanner::RepoInfo,
) -> Row<'a> {
    let is_selected = model.cursor == i;
    let is_checked = model.checked.contains(&i);
    let cursor = if is_selected { ">" } else { " " };
    let checkbox = if is_checked { "x" } else { " " };

    let grade_style = health_grade_style(&repo.health_grade);

    let last_commit = repo
        .last_commit_date
        .map(|d| {
            let days = (chrono::Utc::now() - d).num_days();
            if days < 1 {
                "today".into()
            } else if days < 30 {
                format!("{}d", days)
            } else if days < 365 {
                format!("{}mo", days / 30)
            } else {
                format!("{}y", days / 365)
            }
        })
        .unwrap_or_else(|| "-".into());

    let dirty = if repo.is_dirty { "changes" } else { "" };

    let row_style = if is_selected {
        Style::default().bg(DARK_BG)
    } else {
        Style::default()
    };

    let mut name_display = format!("{} [{}] {}", cursor, checkbox, repo.name);
    if repo.is_container {
        name_display.push_str(&format!(" ({} repos)", repo.child_repo_count));
    }
    if repo.workspace != crate::scanner::repo_scanner::WorkspaceType::None {
        name_display.push_str(&format!(" [{}]", repo.workspace));
    }

    let name_style = if repo.is_container {
        Style::default().fg(CYAN).italic()
    } else if is_selected {
        Style::default().fg(WHITE)
    } else {
        Style::default()
    };

    let size_display = if repo.is_container || repo.total_size == 0 {
        "-".into()
    } else {
        format_size(repo.total_size)
    };

    let cleanup_display = if repo.is_container {
        "-".into()
    } else if repo.artifact_size > 0 {
        format_size(repo.artifact_size)
    } else {
        "clean".into()
    };

    Row::new(vec![
        Cell::from(name_display).style(name_style),
        Cell::from(if repo.is_container {
            "-".into()
        } else {
            format!("{}{}", repo.health_grade, repo.health_score)
        })
        .style(if repo.is_container {
            Style::default().fg(GRAY)
        } else {
            grade_style
        }),
        Cell::from(if repo.is_container {
            "container".into()
        } else {
            repo.branch.clone()
        })
        .style(if repo.is_container {
            Style::default().fg(CYAN)
        } else {
            Style::default().fg(PURPLE)
        }),
        Cell::from(if repo.is_container {
            "-".into()
        } else {
            last_commit
        }),
        Cell::from(size_display).style(Style::default().fg(GRAY)),
        Cell::from(cleanup_display).style(if repo.artifact_size > 100_000_000 {
            Style::default().fg(RED)
        } else if repo.artifact_size > 10_000_000 {
            Style::default().fg(YELLOW)
        } else {
            Style::default()
        }),
        Cell::from(dirty).style(Style::default().fg(YELLOW)),
    ])
    .style(row_style)
}

fn compute_scroll_offset(
    model: &ScannerModel,
    group_order: &[String],
    visible_height: usize,
) -> usize {
    let mut cursor_row_pos = 0usize;
    let mut found = false;
    for row_group in group_order {
        cursor_row_pos += 1; // group header row
        for (i, repo) in model.repos.iter().enumerate() {
            if repo.group != *row_group {
                continue;
            }
            if i == model.cursor {
                found = true;
                break;
            }
            cursor_row_pos += 1;
        }
        if found {
            break;
        }
    }

    if cursor_row_pos >= visible_height {
        cursor_row_pos - visible_height + 1
    } else {
        0
    }
}
