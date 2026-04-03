use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::core::types::*;
use crate::core::changelogs::get_changelog_url;
use crate::tui::model::*;
use crate::tui::styles::*;

/// Render the main updater dashboard
pub fn render_dashboard(frame: &mut Frame, area: Rect, model: &UpdaterModel) {
    let chunks = Layout::vertical([
        Constraint::Length(1),  // header
        Constraint::Length(1),  // spacing
        Constraint::Min(10),   // grid
        Constraint::Length(2),  // help bar
    ])
    .split(area);

    render_header(frame, chunks[0], model);

    // Search bar (if active, takes a line from grid area)
    let grid_area = if model.state == UpdaterState::Search || !model.search_query.is_empty() {
        let search_chunks = Layout::vertical([
            Constraint::Length(2), // search bar
            Constraint::Min(8),   // grid
        ])
        .split(chunks[2]);
        render_search_bar(frame, search_chunks[0], model);
        search_chunks[1]
    } else {
        chunks[2]
    };

    render_grid(frame, grid_area, model);
    render_help_bar(frame, chunks[3], model);
}

/// Render the preview (dry-run) screen
pub fn render_preview(frame: &mut Frame, area: Rect, model: &UpdaterModel) {
    let mut lines = vec![
        Line::from(Span::styled(
            " 🔍 UPDATE PREVIEW (DRY-RUN) ",
            Style::default().fg(DARK).bg(YELLOW).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Review the tools that will be updated. No changes will be made yet.",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
    ];

    // Count by category
    let mut has_dangerous = false;
    let mut total = 0;
    let mut by_category: std::collections::HashMap<Category, Vec<&ToolState>> =
        std::collections::HashMap::new();

    for (i, item) in model.items.iter().enumerate() {
        if model.checked.contains(&i) {
            total += 1;
            by_category
                .entry(item.tool.category)
                .or_default()
                .push(item);
            if item.tool.category == Category::Runtime {
                has_dangerous = true;
            }
        }
    }

    lines.push(Line::from(Span::styled(
        format!("Total Tools Selected: {}", total),
        Style::default().fg(PURPLE).bold(),
    )));
    lines.push(Line::from(""));

    for cat in Category::all() {
        if let Some(tools) = by_category.get(cat) {
            lines.push(Line::from(Span::styled(
                cat.label(),
                Style::default().fg(GREEN).bold(),
            )));
            for tool in tools {
                let version_info = if tool.local_version != "..."
                    && tool.local_version != "MISSING"
                    && !tool.local_version.is_empty()
                {
                    format!(" (current: {})", tool.local_version)
                } else if tool.status == ToolStatus::Missing {
                    " (will install)".into()
                } else {
                    String::new()
                };

                lines.push(Line::from(vec![
                    Span::raw("  → "),
                    Span::styled(&tool.tool.name, Style::default().fg(WHITE)),
                    Span::styled(version_info, Style::default().fg(GRAY)),
                ]));

                if let Some(url) = get_changelog_url(&tool.tool) {
                    lines.push(Line::from(Span::styled(
                        format!("    ↳ Changelog: {}", url),
                        Style::default().fg(DIM_BLUE),
                    )));
                }
            }
            lines.push(Line::from(""));
        }
    }

    if has_dangerous {
        lines.push(Line::from(Span::styled(
            " ⚠ WARNING: Runtime updates detected - confirmation will be required ",
            Style::default().fg(WHITE).bg(RED).bold(),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "[ENTER] Proceed with Updates • [ESC] Cancel",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines)
        .block(Block::default().padding(Padding::new(2, 2, 1, 1)));
    frame.render_widget(paragraph, area);
}

fn render_header(frame: &mut Frame, area: Rect, model: &UpdaterModel) {
    let checked = model.checked.len();
    let installed = model.items.iter().filter(|i| i.status == ToolStatus::Installed).count();
    let outdated = model.items.iter().filter(|i| i.status == ToolStatus::Outdated).count();
    let missing = model.items.iter().filter(|i| i.status == ToolStatus::Missing).count();

    let text = match model.state {
        UpdaterState::Updating => format!(" UPDATING ({} remaining)... ", model.updating_remaining),
        UpdaterState::Summary => " UPDATE SUMMARY ".into(),
        _ => {
            if model.loading_count > 0 {
                format!(" TOOL UPDATER  Scanning {}...", model.loading_count)
            } else {
                let mut status = format!(" TOOL UPDATER  {} tools", model.items.len());
                if outdated > 0 { status.push_str(&format!("  {} outdated", outdated)); }
                if missing > 0 { status.push_str(&format!("  {} missing", missing)); }
                if checked > 0 { status.push_str(&format!("  {} selected", checked)); }
                if outdated == 0 && missing == 0 && installed > 0 {
                    status.push_str("  all up to date");
                }
                status
            }
        }
    };

    let header = Paragraph::new(Span::styled(
        text,
        Style::default().fg(WHITE).bg(BLUE).bold(),
    ));
    frame.render_widget(header, area);
}

fn render_search_bar(frame: &mut Frame, area: Rect, model: &UpdaterModel) {
    let cursor = if model.state == UpdaterState::Search {
        "█"
    } else {
        ""
    };

    let result_count = model
        .filtered_indices
        .as_ref()
        .map(|f| format!(" ({} results)", f.len()))
        .unwrap_or_default();

    let line = Line::from(vec![
        Span::styled("Search: ", Style::default().fg(YELLOW)),
        Span::styled(
            format!("{}{}", model.search_query, cursor),
            Style::default().fg(WHITE).bg(DARK),
        ),
        Span::styled(result_count, Style::default().fg(GRAY)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

fn render_grid(frame: &mut Frame, area: Rect, model: &UpdaterModel) {
    let categories = [
        Category::Sys, Category::Code, Category::Ide, Category::Term,
        Category::Prod, Category::Infra, Category::Runtime, Category::Utils,
    ];

    let header = Row::new(vec![
        Cell::from("  Tool").style(Style::default().fg(BLUE).bold()),
        Cell::from("Installed").style(Style::default().fg(BLUE).bold()),
        Cell::from("Latest").style(Style::default().fg(BLUE).bold()),
        Cell::from("Status").style(Style::default().fg(BLUE).bold()),
    ]);

    let mut rows: Vec<Row> = Vec::new();

    for cat in &categories {
        let cat_items: Vec<(usize, &ToolState)> = model.items.iter().enumerate()
            .filter(|(i, item)| item.tool.category == *cat && model.is_item_visible(*i))
            .collect();
        if cat_items.is_empty() { continue; }

        // Category header row
        rows.push(
            Row::new(vec![
                Cell::from(format!("  {} ({})", cat.label(), cat_items.len()))
                    .style(Style::default().fg(PURPLE).bold()),
                Cell::from(""), Cell::from(""), Cell::from(""),
            ]).style(Style::default().bg(Color::Rgb(25, 25, 35)))
        );

        for (i, item) in cat_items {
            let is_selected = model.cursor == i && model.state == UpdaterState::Main;
            let is_checked = model.checked.contains(&i);
            let cursor = if is_selected { ">" } else { " " };
            let checkbox = if is_checked { "x" } else { " " };

            let row_style = if is_selected { Style::default().bg(DARK_BG) } else { Style::default() };

            let name_style = if is_selected {
                Style::default().fg(WHITE).bold()
            } else {
                Style::default().fg(GRAY)
            };

            let (status_text, status_style): (String, Style) = match &item.status {
                ToolStatus::Checking => ("checking...".into(), Style::default().fg(YELLOW)),
                ToolStatus::Missing => ("not installed".into(), Style::default().fg(RED)),
                ToolStatus::Outdated => ("outdated".into(), Style::default().fg(YELLOW).bold()),
                ToolStatus::Installed => ("up to date".into(), Style::default().fg(GREEN)),
                ToolStatus::Updated => ("updated".into(), Style::default().fg(GREEN).bold()),
                ToolStatus::Failed => ("failed".into(), Style::default().fg(RED)),
                ToolStatus::Updating => ("updating...".into(), Style::default().fg(BLUE)),
            };

            let local_ver = if item.local_version == "MISSING" || item.local_version == "..." {
                "-".into()
            } else {
                item.local_version.clone()
            };

            let remote_ver = if item.remote_version == "..." || item.remote_version == "Checking..." || item.remote_version == "Unknown" {
                "-".into()
            } else {
                item.remote_version.clone()
            };

            rows.push(Row::new(vec![
                Cell::from(format!("{} [{}] {}", cursor, checkbox, item.tool.name)).style(name_style),
                Cell::from(local_ver).style(Style::default().fg(GRAY)),
                Cell::from(remote_ver).style(
                    if item.status == ToolStatus::Outdated { Style::default().fg(YELLOW) }
                    else { Style::default().fg(GRAY) }
                ),
                Cell::from(status_text).style(status_style),
            ]).style(row_style));
        }
    }

    // Scroll
    let visible_height = area.height.saturating_sub(3) as usize;
    let mut cursor_pos = 0usize;
    let mut found = false;
    for (ri, _) in rows.iter().enumerate() {
        if found { break; }
        // Count actual tool rows to match cursor
        cursor_pos = ri;
    }
    // Simple scroll: find cursor position in rows
    let mut row_idx = 0usize;
    for cat in &categories {
        let cat_items: Vec<(usize, &ToolState)> = model.items.iter().enumerate()
            .filter(|(i, item)| item.tool.category == *cat && model.is_item_visible(*i))
            .collect();
        if cat_items.is_empty() { continue; }
        row_idx += 1; // header
        for (i, _) in &cat_items {
            if *i == model.cursor { found = true; cursor_pos = row_idx; break; }
            row_idx += 1;
        }
        if found { break; }
    }

    let scroll_offset = if cursor_pos >= visible_height {
        cursor_pos - visible_height + 1
    } else { 0 };

    let scrolled: Vec<Row> = rows.into_iter().skip(scroll_offset).collect();
    let scroll_info = format!(" {}/{} ", model.cursor + 1, model.items.len());

    let table = Table::new(
        scrolled,
        [
            Constraint::Min(15),       // Tool name
            Constraint::Length(21),    // Installed version
            Constraint::Length(21),    // Latest version
            Constraint::Length(14),    // Status
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BLUE))
            .title(Span::styled(" Tools ", Style::default().fg(BLUE).bold()))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, area);
}

fn render_help_bar(frame: &mut Frame, area: Rect, model: &UpdaterModel) {
    let text = match model.state {
        UpdaterState::Search => "[Type to search] • [ESC] Cancel • [ENTER] Confirm",
        UpdaterState::Updating => "[UPDATING IN PROGRESS... PLEASE WAIT]",
        UpdaterState::Summary => "[UPDATE COMPLETE] Press any key to return to dashboard",
        _ => {
            if !model.search_query.is_empty() {
                "[Filter active] [SPACE] Select • [G/A] Group • [/] Search • [D] Dry-Run • [ENTER] Update • [Q] Quit • [ESC] Clear"
            } else {
                "[SPACE] Select • [G/A] Group • [/] Search • [D] Dry-Run • [ENTER] Update • [TAB] Mode • [Q] Quit"
            }
        }
    };

    let help = Paragraph::new(Span::styled(text, Style::default().fg(GRAY)));
    frame.render_widget(help, area);
}
