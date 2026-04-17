use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;
use crate::scanner::system_cleaner::CleanCategory;

pub fn render_system_cleaner(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(5),   // table
        Constraint::Length(1), // help
        Constraint::Length(1), // mole tip
    ])
    .split(area);

    render_header(frame, chunks[0], model);

    if model.scanning && model.items.is_empty() {
        render_scanning(frame, chunks[1]);
    } else if model.items.is_empty() && !model.scanning {
        render_empty(frame, chunks[1]);
    } else {
        render_table(frame, chunks[1], model);
    }

    render_help(frame, chunks[2], model);
    render_mole_tip(frame, chunks[3]);
}

fn render_header(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let total: u64 = model.items.iter().map(|i| i.size).sum();
    let docker_count = model.items.iter().filter(|i| i.category == CleanCategory::Docker).count();
    let vm_count = model.items.iter().filter(|i| i.category == CleanCategory::VMs).count();
    let cache_count = model.items.iter().filter(|i| i.category == CleanCategory::Cache).count();
    let log_count = model.items.iter().filter(|i| i.category == CleanCategory::Logs).count();
    let dl_count = model.items.iter().filter(|i| i.category == CleanCategory::Downloads).count();

    let status = if model.scanning {
        Span::styled("scanning...", Style::default().fg(YELLOW).italic())
    } else {
        Span::styled(format!("{} items", model.items.len()), Style::default().fg(GRAY))
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" SYSTEM CLEANUP ", Style::default().fg(WHITE).bg(Color::Rgb(180, 80, 40)).bold()),
            Span::raw("  "),
            status,
            Span::raw("  "),
            Span::styled(format!("{} reclaimable", format_size(total)), Style::default().fg(GREEN).bold()),
        ]),
        Line::from(vec![
            Span::styled("Docker, caches, VMs, logs  ", Style::default().fg(GRAY)),
            if docker_count > 0 {
                Span::styled(format!("Docker:{} ", docker_count), Style::default().fg(CYAN))
            } else { Span::raw("") },
            if vm_count > 0 {
                Span::styled(format!("VMs:{} ", vm_count), Style::default().fg(Color::Rgb(255, 100, 100)))
            } else { Span::raw("") },
            if cache_count > 0 {
                Span::styled(format!("Cache:{} ", cache_count), Style::default().fg(PURPLE))
            } else { Span::raw("") },
            if log_count > 0 {
                Span::styled(format!("Logs:{} ", log_count), Style::default().fg(YELLOW))
            } else { Span::raw("") },
            if dl_count > 0 {
                Span::styled(format!("Downloads:{}", dl_count), Style::default().fg(Color::Rgb(100, 200, 100)))
            } else { Span::raw("") },
        ]),
    ]);
    frame.render_widget(header, area);
}

fn render_scanning(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(180, 80, 40)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled("  Scanning Docker, caches, logs...", Style::default().fg(YELLOW))),
        Line::from(""),
        Line::from(Span::styled("  Checking brew, npm, pip, cargo, Docker", Style::default().fg(GRAY))),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn render_empty(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GRAY));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled("  System is clean!", Style::default().fg(GREEN).bold())),
        Line::from(""),
        Line::from(Span::styled("  No reclaimable caches, Docker resources, or large logs found.", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled("  Press [r] to rescan", Style::default().fg(TERM_GRAY))),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn render_table(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    // Group by category
    let categories = [CleanCategory::Docker, CleanCategory::VMs, CleanCategory::Cache, CleanCategory::Logs, CleanCategory::Downloads];

    let header = Row::new(vec![
        Cell::from("  Item").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Type").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Size").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Risk").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Detail").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
    ]);

    let mut rows: Vec<Row> = Vec::new();

    for cat in &categories {
        let cat_items: Vec<(usize, &crate::scanner::system_cleaner::CleanableItem)> = model.items.iter()
            .enumerate()
            .filter(|(_, item)| item.category == *cat)
            .collect();

        if cat_items.is_empty() {
            continue;
        }

        // Category header
        let cat_size: u64 = cat_items.iter().map(|(_, i)| i.size).sum();
        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(
                    format!("  {} ({}, {})", cat, cat_items.len(), format_size(cat_size)),
                    Style::default().fg(cat_color(cat)).bold(),
                )),
                Cell::from(""), Cell::from(""), Cell::from(""), Cell::from(""),
            ])
            .style(Style::default().bg(Color::Rgb(25, 25, 35)))
        );

        for (i, item) in cat_items {
            let is_selected = model.cursor == i;
            let is_checked = model.checked.contains(&i);
            let cursor = if is_selected { ">" } else { " " };
            let checkbox = if is_checked { "x" } else { " " };

            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };

            // Risk + status indicator
            use crate::scanner::system_cleaner::CleanRisk;
            let (risk_label, risk_style) = if item.app_running {
                ("RUNNING", Style::default().fg(RED).bold())
            } else {
                match item.risk {
                    CleanRisk::Safe => ("safe", Style::default().fg(GREEN)),
                    CleanRisk::Caution => ("caution", Style::default().fg(YELLOW)),
                    CleanRisk::Danger => ("danger", Style::default().fg(RED)),
                }
            };

            rows.push(Row::new(vec![
                Cell::from(format!("{} [{}] {}", cursor, checkbox, item.name)),
                Cell::from(format!("{}", item.category))
                    .style(Style::default().fg(cat_color(cat))),
                Cell::from(if item.size > 0 { format_size(item.size) } else { "?".into() })
                    .style(if item.size > 1_073_741_824 { Style::default().fg(RED).bold() }
                        else if item.size > 100_000_000 { Style::default().fg(YELLOW) }
                        else { Style::default() }),
                Cell::from(risk_label).style(risk_style),
                Cell::from(item.detail.clone())
                    .style(Style::default().fg(GRAY)),
            ]).style(row_style));
        }
    }

    // Scroll offset: use display_order for accurate visual row position
    let visible_height = area.height.saturating_sub(3) as usize;
    let cursor_row_pos = model.display_order.iter()
        .position(|d| *d == Some(model.cursor))
        .unwrap_or(0);
    let scroll_offset = if cursor_row_pos >= visible_height {
        cursor_row_pos - visible_height + 1
    } else { 0 };

    let scrolled_rows: Vec<Row> = rows.into_iter().skip(scroll_offset).collect();
    let scroll_info = format!(" {}/{} ", model.cursor + 1, model.items.len());

    let table = Table::new(
        scrolled_rows,
        [
            Constraint::Min(18),      // Item
            Constraint::Length(10),    // Type
            Constraint::Length(10),    // Size
            Constraint::Length(8),     // Risk
            Constraint::Percentage(40), // Detail
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(180, 80, 40)))
            .title(Span::styled(" Cleanable Resources ", Style::default().fg(Color::Rgb(180, 80, 40)).bold()))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, area);
}

fn render_help(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let selected = model.checked.len();
    let help_text = if selected > 0 {
        format!(" [ENTER] Detail/risk  [SPACE] Toggle  [x] Clean {} selected  [r] Rescan  [TAB] Switch  [q] Back", selected)
    } else {
        " [ENTER] Detail/risk  [SPACE] Select  [x] Clean selected  [r] Rescan  [TAB] Switch  [q] Back".into()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(GRAY))),
        area,
    );
}

pub fn render_risk_confirm(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    use crate::scanner::system_cleaner::CleanRisk;

    let item = match model.items.get(model.cursor) {
        Some(i) => i,
        None => return,
    };

    let modal_area = center_modal(frame, area, 65, 16);
    let border_color = match item.risk {
        CleanRisk::Safe => GREEN,
        CleanRisk::Caution => YELLOW,
        CleanRisk::Danger => RED,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(MODAL_BG));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let risk_label = match item.risk {
        CleanRisk::Safe => "SAFE TO CLEAN",
        CleanRisk::Caution => "CAUTION",
        CleanRisk::Danger => "DANGER",
    };

    let explanation = match (item.risk, item.category.clone()) {
        (CleanRisk::Danger, _) => vec![
            "This app is currently running.",
            "Cleaning while running may cause crashes or data loss.",
            "Close the app first, then retry.",
        ],
        (CleanRisk::Caution, CleanCategory::Docker) => vec![
            "Docker images/containers will need to be re-pulled.",
            "Build cache will be rebuilt on next docker build.",
            "This may take significant time on slow connections.",
        ],
        (CleanRisk::Caution, CleanCategory::VMs) => vec![
            "Virtual machine data may be permanently deleted.",
            "Emulator AVDs will need to be recreated.",
            "Docker VM disk will be rebuilt on restart.",
        ],
        (CleanRisk::Caution, CleanCategory::Cache) => vec![
            "Cache will be rebuilt automatically on next use.",
            "First build after cleaning may be slower.",
            "No permanent data will be lost.",
        ],
        (CleanRisk::Safe, CleanCategory::Cache) => vec![
            "This cache is rebuilt automatically on next use.",
            "No projects or data will be affected.",
            "Disk space will be freed immediately.",
        ],
        (CleanRisk::Safe, CleanCategory::Logs) => vec![
            "Old log files older than 7 days.",
            "These are regenerated by their apps automatically.",
            "No data or functionality will be lost.",
        ],
        (CleanRisk::Safe, CleanCategory::Downloads) => vec![
            "Large installer files (ISO, DMG, PKG).",
            "These are typically no longer needed after installation.",
            "Re-download from the original source if needed.",
        ],
        (CleanRisk::Safe, CleanCategory::VMs) => vec![
            "Legacy VM data no longer in active use.",
            "Safe to remove if you no longer use this tool.",
        ],
        _ => vec![
            "This item can be safely removed.",
            "It will be rebuilt automatically if needed.",
        ],
    };

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" {} ", risk_label),
            Style::default().fg(WHITE).bg(border_color).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Item: ", Style::default().fg(PURPLE)),
            Span::styled(&item.name, Style::default().fg(WHITE).bold()),
        ]),
        Line::from(vec![
            Span::styled("  Size: ", Style::default().fg(PURPLE)),
            Span::styled(format_size(item.size), Style::default().fg(YELLOW)),
        ]),
        Line::from(""),
    ];

    for exp in &explanation {
        lines.push(Line::from(Span::styled(
            format!("  {}", exp),
            Style::default().fg(GRAY),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Proceed? (y/N)",
        Style::default().fg(WHITE),
    )));

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
}

fn render_mole_tip(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Tip: ", Style::default().fg(TERM_GRAY)),
            Span::styled("tw93/mole", Style::default().fg(CYAN)),
            Span::styled(" — native macOS app for deep system cleanup  ", Style::default().fg(TERM_GRAY)),
            Span::styled("github.com/tw93/mole", Style::default().fg(GRAY)),
        ])),
        area,
    );
}

pub fn render_bulk_confirm(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let count = model.checked.len();
    let total: u64 = model.checked.iter()
        .filter_map(|i| model.items.get(*i))
        .map(|item| item.size)
        .sum();

    let modal_area = center_modal(frame, area, 55, 10);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(YELLOW))
        .style(Style::default().bg(MODAL_BG));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(
            " CLEAN SELECTED ",
            Style::default().fg(WHITE).bg(YELLOW).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Items: ", Style::default().fg(PURPLE)),
            Span::styled(format!("{} selected", count), Style::default().fg(WHITE).bold()),
        ]),
        Line::from(vec![
            Span::styled("  Total: ", Style::default().fg(PURPLE)),
            Span::styled(format_size(total), Style::default().fg(YELLOW)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Items marked as RUNNING will be skipped.",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Clean all selected? (y/N)",
            Style::default().fg(WHITE),
        )),
    ];

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
}

fn cat_color(cat: &CleanCategory) -> Color {
    match cat {
        CleanCategory::Docker => CYAN,
        CleanCategory::VMs => Color::Rgb(255, 100, 100),
        CleanCategory::Cache => PURPLE,
        CleanCategory::Logs => YELLOW,
        CleanCategory::Downloads => Color::Rgb(100, 200, 100),
    }
}
