//! System cleaner table: grouped by category with risk indicators.

use super::cat_color;
use crate::scanner::system_cleaner::{CleanCategory, CleanRisk};
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub(super) fn render_table(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let categories = [
        CleanCategory::Docker,
        CleanCategory::VMs,
        CleanCategory::Cache,
        CleanCategory::Logs,
        CleanCategory::Downloads,
    ];

    let header = Row::new(vec![
        Cell::from("  Item").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Type").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Size").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Risk").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
        Cell::from("Detail").style(Style::default().fg(Color::Rgb(180, 80, 40)).bold()),
    ]);

    let mut rows: Vec<Row> = Vec::new();

    for cat in &categories {
        let cat_items: Vec<(usize, &crate::scanner::system_cleaner::CleanableItem)> = model
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.category == *cat)
            .collect();

        if cat_items.is_empty() {
            continue;
        }

        let cat_size: u64 = cat_items.iter().map(|(_, i)| i.size).sum();
        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(
                    format!("  {} ({}, {})", cat, cat_items.len(), format_size(cat_size)),
                    Style::default().fg(cat_color(cat)).bold(),
                )),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
                Cell::from(""),
            ])
            .style(Style::default().bg(Color::Rgb(25, 25, 35))),
        );

        for (i, item) in cat_items {
            rows.push(build_item_row(model, i, item, cat));
        }
    }

    let visible_height = area.height.saturating_sub(3) as usize;
    let cursor_row_pos = model
        .display_order
        .iter()
        .position(|d| *d == Some(model.cursor))
        .unwrap_or(0);
    let scroll_offset = if cursor_row_pos >= visible_height {
        cursor_row_pos - visible_height + 1
    } else {
        0
    };

    let scrolled_rows: Vec<Row> = rows.into_iter().skip(scroll_offset).collect();
    let scroll_info = format!(" {}/{} ", model.cursor + 1, model.items.len());

    let table = Table::new(
        scrolled_rows,
        [
            Constraint::Min(18),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Percentage(40),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(180, 80, 40)))
            .title(Span::styled(
                " Cleanable Resources ",
                Style::default().fg(Color::Rgb(180, 80, 40)).bold(),
            ))
            .title_bottom(Span::styled(scroll_info, Style::default().fg(GRAY))),
    );
    frame.render_widget(table, area);
}

fn build_item_row<'a>(
    model: &'a SystemCleanerModel,
    i: usize,
    item: &'a crate::scanner::system_cleaner::CleanableItem,
    cat: &'a CleanCategory,
) -> Row<'a> {
    let is_selected = model.cursor == i;
    let is_checked = model.checked.contains(&i);
    let cursor = if is_selected { ">" } else { " " };
    let checkbox = if is_checked { "x" } else { " " };

    let row_style = if is_selected {
        Style::default().bg(DARK_BG)
    } else {
        Style::default()
    };

    let (risk_label, risk_style) = if item.app_running {
        ("RUNNING", Style::default().fg(RED).bold())
    } else {
        match item.risk {
            CleanRisk::Safe => ("safe", Style::default().fg(GREEN)),
            CleanRisk::Caution => ("caution", Style::default().fg(YELLOW)),
            CleanRisk::Danger => ("danger", Style::default().fg(RED)),
        }
    };

    let size_style = if item.size > 1_073_741_824 {
        Style::default().fg(RED).bold()
    } else if item.size > 100_000_000 {
        Style::default().fg(YELLOW)
    } else {
        Style::default()
    };
    let size_display = if item.size > 0 {
        format_size(item.size)
    } else {
        "?".into()
    };

    Row::new(vec![
        Cell::from(format!("{} [{}] {}", cursor, checkbox, item.name)),
        Cell::from(format!("{}", item.category)).style(Style::default().fg(cat_color(cat))),
        Cell::from(size_display).style(size_style),
        Cell::from(risk_label).style(risk_style),
        Cell::from(item.detail.clone()).style(Style::default().fg(GRAY)),
    ])
    .style(row_style)
}
