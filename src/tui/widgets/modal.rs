use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::styles::*;

/// Render the danger zone confirmation modal
pub fn render_danger_modal(frame: &mut Frame, area: Rect) {
    let modal_area = center_modal(frame, area, 50, 10);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(RED))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(
            "⚠️  DANGER ZONE ⚠️",
            Style::default().fg(RED).bold(),
        )),
        Line::from(""),
        Line::from("You have selected Critical Runtimes."),
        Line::from("Updating Node/Python may break your projects."),
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure? (y/N)",
            Style::default().fg(WHITE),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

/// Render a clean confirmation modal for scanner
pub fn render_clean_confirm_modal(
    frame: &mut Frame,
    area: Rect,
    item_count: usize,
    total_size: &str,
) {
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
            "🧹 CONFIRM CLEANUP",
            Style::default().fg(YELLOW).bold(),
        )),
        Line::from(""),
        Line::from(format!(
            "This will clean {} item(s), recovering ~{}",
            item_count, total_size
        )),
        Line::from("Files will be moved to trash (recoverable)."),
        Line::from(""),
        Line::from(Span::styled(
            "Proceed? (y/N)",
            Style::default().fg(WHITE),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}
