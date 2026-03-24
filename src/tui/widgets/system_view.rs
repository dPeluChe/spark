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
        Cell::from("Detail").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
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

            // Safety indicator
            let safety = if item.app_running {
                "running"
            } else {
                ""
            };

            rows.push(Row::new(vec![
                Cell::from(format!("{} [{}] {}", cursor, checkbox, item.name)),
                Cell::from(format!("{}", item.category))
                    .style(Style::default().fg(cat_color(cat))),
                Cell::from(if item.size > 0 { format_size(item.size) } else { "?".into() })
                    .style(if item.size > 1_073_741_824 { Style::default().fg(RED).bold() }
                        else if item.size > 100_000_000 { Style::default().fg(YELLOW) }
                        else { Style::default() }),
                Cell::from(item.detail.clone())
                    .style(Style::default().fg(GRAY)),
                Cell::from(safety)
                    .style(Style::default().fg(RED)),
            ]).style(row_style));
        }
    }

    // Scroll offset
    let visible_height = area.height.saturating_sub(3) as usize;
    let mut cursor_row_pos = 0usize;
    let mut found = false;
    for cat in &categories {
        let has_items = model.items.iter().any(|i| i.category == *cat);
        if !has_items { continue; }
        cursor_row_pos += 1; // header
        for (i, item) in model.items.iter().enumerate() {
            if item.category != *cat { continue; }
            if i == model.cursor { found = true; break; }
            cursor_row_pos += 1;
        }
        if found { break; }
    }
    let scroll_offset = if cursor_row_pos >= visible_height {
        cursor_row_pos - visible_height + 1
    } else { 0 };

    let scrolled_rows: Vec<Row> = rows.into_iter().skip(scroll_offset).collect();
    let scroll_info = format!(" {}/{} ", model.cursor + 1, model.items.len());

    let table = Table::new(
        scrolled_rows,
        [
            Constraint::Percentage(22),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(20),
            Constraint::Length(8),
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
        format!(" [x] Clean {} selected  [SPACE] Toggle  [r] Rescan  [TAB] Switch  [q] Back", selected)
    } else {
        " [ENTER] Clean item  [SPACE] Select  [x] Clean selected  [r] Rescan  [TAB] Switch  [q] Back".into()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(help_text, Style::default().fg(GRAY))),
        area,
    );
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
