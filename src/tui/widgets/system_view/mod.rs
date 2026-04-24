//! System cleaner view: Docker/caches/VMs/logs table with risk confirmation.
//!
//! Split:
//! - mod.rs   — dispatcher + header + scanning/empty placeholders + help
//!   + mole_tip + render_bulk_confirm
//! - table.rs — grouped table with category headers + risk indicators
//! - risk.rs  — single-item risk confirmation modal

mod risk;
mod table;

pub use risk::render_risk_confirm;

use crate::scanner::system_cleaner::CleanCategory;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::utils::fs::format_size;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_system_cleaner(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    render_header(frame, chunks[0], model);

    if model.scanning && model.items.is_empty() {
        render_scanning(frame, chunks[1]);
    } else if model.items.is_empty() && !model.scanning {
        render_empty(frame, chunks[1]);
    } else {
        table::render_table(frame, chunks[1], model);
    }

    render_help(frame, chunks[2], model);
    render_mole_tip(frame, chunks[3]);
}

fn render_header(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let total: u64 = model.items.iter().map(|i| i.size).sum();
    let docker_count = model
        .items
        .iter()
        .filter(|i| i.category == CleanCategory::Docker)
        .count();
    let vm_count = model
        .items
        .iter()
        .filter(|i| i.category == CleanCategory::VMs)
        .count();
    let cache_count = model
        .items
        .iter()
        .filter(|i| i.category == CleanCategory::Cache)
        .count();
    let log_count = model
        .items
        .iter()
        .filter(|i| i.category == CleanCategory::Logs)
        .count();
    let dl_count = model
        .items
        .iter()
        .filter(|i| i.category == CleanCategory::Downloads)
        .count();

    let status = if model.scanning {
        Span::styled("scanning...", Style::default().fg(YELLOW).italic())
    } else {
        Span::styled(
            format!("{} items", model.items.len()),
            Style::default().fg(GRAY),
        )
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " SYSTEM CLEANUP ",
                Style::default()
                    .fg(WHITE)
                    .bg(Color::Rgb(180, 80, 40))
                    .bold(),
            ),
            Span::raw("  "),
            status,
            Span::raw("  "),
            Span::styled(
                format!("{} reclaimable", format_size(total)),
                Style::default().fg(GREEN).bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Docker, caches, VMs, logs  ", Style::default().fg(GRAY)),
            cat_badge("Docker", docker_count, CYAN),
            cat_badge("VMs", vm_count, Color::Rgb(255, 100, 100)),
            cat_badge("Cache", cache_count, PURPLE),
            cat_badge("Logs", log_count, YELLOW),
            cat_badge("Downloads", dl_count, Color::Rgb(100, 200, 100)),
        ]),
    ]);
    frame.render_widget(header, area);
}

fn cat_badge(label: &str, count: usize, color: Color) -> Span<'static> {
    if count == 0 {
        Span::raw("")
    } else {
        Span::styled(format!("{}:{} ", label, count), Style::default().fg(color))
    }
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
        Line::from(Span::styled(
            "  Scanning Docker, caches, logs...",
            Style::default().fg(YELLOW),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Checking brew, npm, pip, cargo, Docker",
            Style::default().fg(GRAY),
        )),
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
        Line::from(Span::styled(
            "  System is clean!",
            Style::default().fg(GREEN).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  No reclaimable caches, Docker resources, or large logs found.",
            Style::default().fg(GRAY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press [r] to rescan",
            Style::default().fg(TERM_GRAY),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
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

fn render_mole_tip(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Tip: ", Style::default().fg(TERM_GRAY)),
            Span::styled("tw93/mole", Style::default().fg(CYAN)),
            Span::styled(
                " — native macOS app for deep system cleanup  ",
                Style::default().fg(TERM_GRAY),
            ),
            Span::styled("github.com/tw93/mole", Style::default().fg(GRAY)),
        ])),
        area,
    );
}

pub fn render_bulk_confirm(frame: &mut Frame, area: Rect, model: &SystemCleanerModel) {
    let count = model.checked.len();
    let total: u64 = model
        .checked
        .iter()
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
            Span::styled(
                format!("{} selected", count),
                Style::default().fg(WHITE).bold(),
            ),
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

pub(super) fn cat_color(cat: &CleanCategory) -> Color {
    match cat {
        CleanCategory::Docker => CYAN,
        CleanCategory::VMs => Color::Rgb(255, 100, 100),
        CleanCategory::Cache => PURPLE,
        CleanCategory::Logs => YELLOW,
        CleanCategory::Downloads => Color::Rgb(100, 200, 100),
    }
}
