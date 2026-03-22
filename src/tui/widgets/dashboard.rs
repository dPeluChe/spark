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
    let text = match model.state {
        UpdaterState::Updating => format!(" UPDATING ({} remaining)... ", model.updating_remaining),
        UpdaterState::Summary => " UPDATE SUMMARY ".into(),
        _ => {
            if model.loading_count > 0 {
                format!(" SPARK DASHBOARD (Scanning {}...)", model.loading_count)
            } else {
                " SPARK DASHBOARD ".into()
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
    let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_cats = [Category::Code, Category::Term, Category::Ide, Category::Prod];
    let right_cats = [
        Category::Infra,
        Category::Utils,
        Category::Runtime,
        Category::Sys,
    ];

    render_column(frame, columns[0], model, &left_cats);
    render_column(frame, columns[1], model, &right_cats);
}

fn render_column(frame: &mut Frame, area: Rect, model: &UpdaterModel, categories: &[Category]) {
    // Count visible items per category to allocate space
    let mut cat_heights: Vec<u16> = Vec::new();
    for cat in categories {
        let count = model
            .items
            .iter()
            .enumerate()
            .filter(|(i, item)| item.tool.category == *cat && model.is_item_visible(*i))
            .count();
        if count > 0 {
            cat_heights.push(count as u16 + 3); // +3 for border and title
        } else {
            cat_heights.push(0);
        }
    }

    let constraints: Vec<Constraint> = cat_heights
        .iter()
        .map(|&h| {
            if h > 0 {
                Constraint::Length(h)
            } else {
                Constraint::Length(0)
            }
        })
        .collect();

    let chunks = Layout::vertical(constraints).split(area);

    let mut chunk_idx = 0;
    for cat in categories {
        if cat_heights[categories.iter().position(|c| c == cat).unwrap()] > 0 {
            render_category_card(frame, chunks[chunk_idx], model, *cat);
        }
        chunk_idx += 1;
    }
}

fn render_category_card(frame: &mut Frame, area: Rect, model: &UpdaterModel, cat: Category) {
    let title = format!("[{}] {}", cat.short_key(), cat.label());

    let mut lines = Vec::new();
    for (i, item) in model.items.iter().enumerate() {
        if item.tool.category != cat || !model.is_item_visible(i) {
            continue;
        }
        lines.push(render_tool_line(i, item, model));
    }

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(PURPLE).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(PURPLE));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_tool_line<'a>(index: usize, item: &'a ToolState, model: &UpdaterModel) -> Line<'a> {
    let is_selected = model.cursor == index && model.state == UpdaterState::Main;

    // Cursor indicator
    let cursor = if is_selected {
        Span::styled("❯ ", Style::default().fg(GREEN).bold())
    } else {
        Span::raw("  ")
    };

    // Checkbox
    let checkbox = render_checkbox(model.checked.contains(&index));

    // Tool name (truncated to 18 chars)
    let name = if item.tool.name.len() > 18 {
        format!("{:<18}", format!("{}…", &item.tool.name[..17]))
    } else {
        format!("{:<18}", item.tool.name)
    };

    let name_span = if is_selected {
        Span::styled(name, Style::default().fg(WHITE).bold())
    } else {
        Span::styled(name, Style::default().fg(GRAY))
    };

    // Status
    let status = render_status(index, item, model);

    Line::from(vec![cursor, checkbox, name_span, status])
}

fn render_status<'a>(index: usize, item: &'a ToolState, model: &UpdaterModel) -> Span<'a> {
    // During update/summary phase
    if model.state == UpdaterState::Updating || model.state == UpdaterState::Summary {
        return match item.status {
            ToolStatus::Updating => {
                let frame = SPINNER_FRAMES[model.splash_frame % SPINNER_FRAMES.len()];
                Span::styled(
                    format!("{} Updating...", frame),
                    Style::default().fg(BLUE),
                )
            }
            ToolStatus::Updated => Span::styled(
                format!("✔ {}", item.local_version),
                Style::default().fg(GREEN),
            ),
            ToolStatus::Failed => Span::styled("✘ Failed", Style::default().fg(RED)),
            _ if model.checked.contains(&index) => {
                Span::styled("⏳ Pending...", Style::default().fg(GRAY))
            }
            _ => Span::styled(
                item.local_version.clone(),
                Style::default().fg(DARK),
            ),
        };
    }

    // Normal state
    match item.status {
        ToolStatus::Checking => Span::styled("⟳ Checking...", Style::default().fg(YELLOW)),
        ToolStatus::Missing => Span::styled("○ Not Installed", Style::default().fg(RED)),
        ToolStatus::Outdated => {
            // Show version path: local → remote
            if item.remote_version != "..."
                && item.remote_version != "Checking..."
                && item.remote_version != "Unknown"
            {
                Span::styled(
                    format!("{} → {}", item.local_version, item.remote_version),
                    Style::default().fg(YELLOW),
                )
            } else {
                Span::styled(
                    item.local_version.clone(),
                    Style::default().fg(YELLOW),
                )
            }
        }
        ToolStatus::Installed => {
            if item.local_version == "MISSING" {
                Span::styled("MISSING", Style::default().fg(YELLOW))
            } else if item.remote_version == item.local_version && item.local_version != "..." {
                Span::styled(
                    format!("{} ✓", item.local_version),
                    Style::default().fg(GRAY),
                )
            } else {
                Span::styled(item.local_version.clone(), Style::default().fg(GRAY))
            }
        }
        _ => Span::styled(item.local_version.clone(), Style::default().fg(GRAY)),
    }
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
