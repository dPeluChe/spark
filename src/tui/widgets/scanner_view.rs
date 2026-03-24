use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;

/// Render the scanner mode
pub fn render_scanner(frame: &mut Frame, area: Rect, app: &App, tick: usize) {
    let model = &app.scanner;
    match model.state {
        ScannerState::ScanConfig => render_scan_config(frame, area, model),
        ScannerState::ScanAddPath => {
            render_scan_config(frame, area, model);
            render_add_path_modal(frame, area, model);
        }
        ScannerState::ContainerLoading => {
            let repo_name = model.repos.get(model.cursor)
                .map(|r| r.name.as_str()).unwrap_or("...");
            let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
            let loading = Paragraph::new(vec![
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(
                    format!("{} Loading repos in {}...", spinner, repo_name),
                    Style::default().fg(CYAN).bold(),
                )),
                Line::from(""),
                Line::from(Span::styled("  [ESC] Cancel", Style::default().fg(GRAY))),
            ]).alignment(Alignment::Center);
            frame.render_widget(loading, area);
        }
        ScannerState::Scanning => render_scanning(frame, area, model, tick),
        ScannerState::ScanResults => render_scan_results(frame, area, model),
        ScannerState::RepoDetail => {
            super::detail_panel::render_detail(frame, area, model);
        }
        ScannerState::HealthHelp => {
            render_scan_results(frame, area, model);
            render_health_help(frame, area);
        }
        ScannerState::DeleteRepoConfirm => {
            render_scan_results(frame, area, model);
            render_delete_repo_confirm(frame, area, model);
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
        // CleanSummary removed — cleanup goes back to ScanResults with toast
        ScannerState::RepoManager => {
            super::repo_manager_view::render_repo_manager(frame, area, &app.repo_manager);
        }
        ScannerState::RepoAction => {
            super::repo_manager_view::render_repo_manager(frame, area, &app.repo_manager);
            super::repo_manager_view::render_action_modal(frame, area, &app.repo_manager);
        }
        ScannerState::RepoCloneInput => {
            super::repo_manager_view::render_repo_manager(frame, area, &app.repo_manager);
            super::repo_manager_view::render_clone_input(frame, area, &app.repo_manager);
        }
        ScannerState::RepoCloneSummary => {
            super::repo_manager_view::render_clone_summary(frame, area, &app.repo_manager);
        }
        ScannerState::PortScan => {
            super::port_view::render_ports(frame, area, &app.port_scanner);
        }
        ScannerState::PortAction => {
            super::port_view::render_ports(frame, area, &app.port_scanner);
            super::port_view::render_action_modal(frame, area, &app.port_scanner);
        }
        ScannerState::PortKillConfirm => {
            super::port_view::render_ports(frame, area, &app.port_scanner);
            let ports_str: String = app
                .port_scanner
                .checked
                .iter()
                .filter_map(|&i| app.port_scanner.ports.get(i).map(|p| format!(":{}", p.port)))
                .collect::<Vec<_>>()
                .join(", ");
            super::port_view::render_kill_confirm(
                frame,
                area,
                app.port_scanner.checked.len(),
                &ports_str,
            );
        }
    }
}

fn render_scan_config(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(4), // title
        Constraint::Min(5),   // directory list
        Constraint::Length(2), // help
    ])
    .split(area);

    let selected_count = model.selected_scan_dirs.len();
    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" REPOSITORY SCANNER ", Style::default().fg(WHITE).bg(PURPLE).bold()),
            Span::raw("  "),
            if selected_count > 0 {
                Span::styled(
                    format!("{} selected", selected_count),
                    Style::default().fg(GREEN).bold(),
                )
            } else {
                Span::styled("none selected", Style::default().fg(YELLOW))
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Folders in ~/ with git repos detected. Select which to scan.",
            Style::default().fg(GRAY),
        )),
        Line::from(Span::styled(
            "[a] to add a custom path manually.",
            Style::default().fg(TERM_GRAY),
        )),
    ]);
    frame.render_widget(title, chunks[0]);

    // Directory list with repo counts
    let home = std::env::var("HOME").unwrap_or_default();
    let mut lines = Vec::new();
    for (i, discovered) in model.discovered_dirs.iter().enumerate() {
        let is_selected = model.cursor == i;
        let is_checked = model.selected_scan_dirs.contains(&i);

        let cursor = if is_selected {
            Span::styled("> ", Style::default().fg(GREEN).bold())
        } else {
            Span::raw("  ")
        };

        let checkbox = render_checkbox(is_checked);

        let dir_str = discovered.path.display().to_string();
        let short = if dir_str.starts_with(&home) {
            format!("~{}", &dir_str[home.len()..])
        } else {
            dir_str
        };

        let dir_style = if is_selected {
            Style::default().fg(WHITE).bold()
        } else {
            Style::default().fg(GRAY)
        };

        let count_label = if discovered.repo_count == 1 {
            "1 repo".to_string()
        } else {
            format!("{} repos", discovered.repo_count)
        };

        lines.push(Line::from(vec![
            cursor,
            checkbox,
            Span::styled(format!("{:<40}", short), dir_style),
            Span::styled(
                count_label,
                Style::default().fg(if discovered.repo_count > 10 { GREEN } else { TERM_GRAY }),
            ),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No directories with git repos found in ~/",
            Style::default().fg(YELLOW),
        )));
        lines.push(Line::from(Span::styled(
            "  Press [a] to add a specific path",
            Style::default().fg(TERM_GRAY),
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
        "[ENTER] Scan • [SPACE] Toggle • [a] Add path • [d] Remove • [r] Refresh • [TAB] Next • [q] Quit",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[2]);
}

fn render_scanning(frame: &mut Frame, area: Rect, model: &ScannerModel, tick: usize) {
    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let home = std::env::var("HOME").unwrap_or_default();

    // Show selected dirs being scanned
    let scanning_dirs: Vec<String> = model.selected_scan_dirs.iter()
        .filter_map(|&i| model.discovered_dirs.get(i))
        .map(|d| {
            let s = d.path.display().to_string();
            if s.starts_with(&home) { format!("~{}", &s[home.len()..]) } else { s }
        })
        .collect();

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Scanning directories...", spinner),
            Style::default().fg(BLUE).bold(),
        )),
        Line::from(""),
    ];

    // Show which dirs
    for dir_name in &scanning_dirs {
        lines.push(Line::from(vec![
            Span::styled("  > ", Style::default().fg(GREEN)),
            Span::styled(dir_name.clone(), Style::default().fg(WHITE)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Entries scanned: ", Style::default().fg(GRAY)),
        Span::styled(format!("{}", model.scan_progress_dirs), Style::default().fg(WHITE)),
        Span::styled("    Repos found: ", Style::default().fg(GRAY)),
        Span::styled(format!("{}", model.scan_progress_repos), Style::default().fg(GREEN).bold()),
    ]));

    if !model.scan_progress_current.is_empty() {
        let current = if model.scan_progress_current.starts_with(&home) {
            format!("~{}", &model.scan_progress_current[home.len()..])
        } else {
            model.scan_progress_current.clone()
        };
        // Truncate long paths
        let display = if current.len() > 60 {
            format!("...{}", &current[current.len()-57..])
        } else {
            current
        };
        lines.push(Line::from(Span::styled(
            format!("  {}", display),
            Style::default().fg(TERM_GRAY),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [ESC] Cancel scan",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines).block(
        Block::default().padding(Padding::new(2, 2, 1, 1)),
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

    // Build grouped display: group headers + repo rows
    let header = Row::new(vec![
        Cell::from("  Name").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Grade").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Branch").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Commit").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Size").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Cleanup").style(Style::default().fg(PURPLE).bold()),
        Cell::from("").style(Style::default().fg(PURPLE).bold()),
    ]);

    // Collect groups in order of appearance
    let mut group_order: Vec<String> = Vec::new();
    for repo in &model.repos {
        if !group_order.contains(&repo.group) {
            group_order.push(repo.group.clone());
        }
    }

    let mut rows: Vec<Row> = Vec::new();
    for group in &group_order {
        let group_repos: Vec<(usize, &crate::scanner::repo_scanner::RepoInfo)> = model.repos.iter()
            .enumerate()
            .filter(|(_, r)| r.group == *group)
            .collect();

        // Group header - compact
        let group_label = format!("  {} ({})", group, group_repos.len());
        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(group_label, Style::default().fg(CYAN).bold())),
                Cell::from(""), Cell::from(""), Cell::from(""),
                Cell::from(""), Cell::from(""), Cell::from(""),
            ])
            .style(Style::default().bg(Color::Rgb(25, 25, 35)))
        );

        for (i, repo) in group_repos {
            let is_selected = model.cursor == i;
            let is_checked = model.checked.contains(&i);
            let cursor = if is_selected { ">" } else { " " };
            let checkbox = if is_checked { "x" } else { " " };

            let grade_style = health_grade_style(&repo.health_grade);

            let last_commit = repo.last_commit_date
                .map(|d| {
                    let days = (chrono::Utc::now() - d).num_days();
                    if days < 1 { "today".into() }
                    else if days < 30 { format!("{}d", days) }
                    else if days < 365 { format!("{}mo", days / 30) }
                    else { format!("{}y", days / 365) }
                })
                .unwrap_or_else(|| "-".into());

            let dirty = if repo.is_dirty { "●" } else { "" };

            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };

            // Build name with workspace/container tags
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

            let size_display = if repo.is_container {
                "-".into()
            } else if repo.total_size > 0 {
                format_size(repo.total_size)
            } else {
                "-".into()
            };

            let cleanup_display = if repo.artifact_size > 0 {
                format_size(repo.artifact_size)
            } else {
                "-".into()
            };

            rows.push(Row::new(vec![
                Cell::from(name_display).style(name_style),
                Cell::from(if repo.is_container { "-".into() } else { format!("{}{}", repo.health_grade, repo.health_score) })
                    .style(if repo.is_container { Style::default().fg(GRAY) } else { grade_style }),
                Cell::from(if repo.is_container { "container".into() } else { repo.branch.clone() })
                    .style(if repo.is_container { Style::default().fg(CYAN) } else { Style::default().fg(PURPLE) }),
                Cell::from(if repo.is_container { "-".into() } else { last_commit }),
                Cell::from(size_display)
                    .style(Style::default().fg(GRAY)),
                Cell::from(cleanup_display)
                    .style(if repo.artifact_size > 100_000_000 { Style::default().fg(RED) }
                        else if repo.artifact_size > 10_000_000 { Style::default().fg(YELLOW) }
                        else { Style::default() }),
                Cell::from(dirty)
                    .style(Style::default().fg(YELLOW)),
            ]).style(row_style));
        }
    }

    // Calculate scroll offset for manual scrolling
    let visible_height = chunks[1].height.saturating_sub(3) as usize;
    let scroll_offset = if model.cursor >= visible_height {
        // Find cursor position in the flat rows list
        let mut cursor_row = 0;
        let mut found = false;
        for row_group in &group_order {
            cursor_row += 1; // group header
            let repos_in_group: Vec<usize> = model.repos.iter().enumerate()
                .filter(|(_, r)| r.group == *row_group)
                .map(|(i, _)| i)
                .collect();
            for &repo_idx in &repos_in_group {
                if repo_idx == model.cursor {
                    found = true;
                    break;
                }
                cursor_row += 1;
            }
            if found { break; }
        }
        if cursor_row >= visible_height {
            cursor_row - visible_height + 1
        } else {
            0
        }
    } else {
        0
    };

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
            Constraint::Length(2),
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

    // Help bar
    let help = Paragraph::new(Span::styled(
        "[ENTER] Detail • [c] Clean • [x] Delete • [s] Sort • [?] Health info • [TAB] Next",
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

fn render_health_help(frame: &mut Frame, area: Rect) {
    let modal_area = center_modal(frame, area, 58, 18);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(PURPLE))
        .style(Style::default().bg(MODAL_BG));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(" HEALTH SCORE ", Style::default().fg(WHITE).bg(PURPLE).bold())),
        Line::from(""),
        Line::from(Span::styled("  Score 0-100 based on:", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Recent commits  ", Style::default().fg(WHITE)),
            Span::styled("up to -30 if >12 months old", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Has remote      ", Style::default().fg(WHITE)),
            Span::styled("-15 if no remote configured", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Clean status    ", Style::default().fg(WHITE)),
            Span::styled("-10 if dirty + stale", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Artifact size   ", Style::default().fg(WHITE)),
            Span::styled("-20 if >100MB of artifacts", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Grades:", Style::default().fg(YELLOW).bold())),
        Line::from(vec![
            Span::styled("  A", Style::default().fg(GREEN).bold()),
            Span::styled(" 80-100  ", Style::default().fg(GRAY)),
            Span::styled("B", Style::default().fg(BLUE).bold()),
            Span::styled(" 60-79   ", Style::default().fg(GRAY)),
            Span::styled("C", Style::default().fg(YELLOW).bold()),
            Span::styled(" 40-59", Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  D", Style::default().fg(Color::Rgb(255, 165, 0)).bold()),
            Span::styled(" 20-39   ", Style::default().fg(GRAY)),
            Span::styled("F", Style::default().fg(RED).bold()),
            Span::styled("  0-19", Style::default().fg(GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled("  [q] Close", Style::default().fg(GRAY))),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_delete_repo_confirm(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let repo = match model.repos.get(model.cursor) {
        Some(r) => r,
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
    let path_str = repo.path.display().to_string();
    let short_path = if path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else {
        path_str
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "DELETE REPOSITORY",
            Style::default().fg(RED).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Repo: ", Style::default().fg(PURPLE)),
            Span::styled(&repo.name, Style::default().fg(WHITE).bold()),
        ]),
        Line::from(vec![
            Span::styled("  Path: ", Style::default().fg(PURPLE)),
            Span::styled(&short_path, Style::default().fg(GRAY)),
        ]),
    ];

    // Warn if dirty
    if repo.is_dirty {
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
    lines.push(Line::from(Span::styled(
        "  Delete? (y/N)",
        Style::default().fg(WHITE),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

fn render_add_path_modal(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let modal_area = center_modal(frame, area, 65, 9);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(PURPLE))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(
            "ADD SCAN DIRECTORY",
            Style::default().fg(PURPLE).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enter full path (~ for home):",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("{}█", model.path_input),
            Style::default().fg(WHITE).bg(DARK),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "e.g.  ~/Projects  or  /opt/code",
            Style::default().fg(TERM_GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[ENTER] Add • [ESC] Cancel",
            Style::default().fg(GRAY),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

