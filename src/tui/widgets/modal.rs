use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::styles::*;

/// Render the danger zone confirmation modal
pub fn render_danger_modal(frame: &mut Frame, area: Rect) {
    let modal_width = 50u16.min(area.width - 4);
    let modal_height = 10u16.min(area.height - 4);

    let h_center = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(modal_width),
        Constraint::Fill(1),
    ])
    .split(area);

    let v_center = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(modal_height),
        Constraint::Fill(1),
    ])
    .split(h_center[1]);

    let modal_area = v_center[1];

    frame.render_widget(Clear, modal_area);

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
    let modal_width = 55u16.min(area.width - 4);
    let modal_height = 10u16.min(area.height - 4);

    let h_center = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(modal_width),
        Constraint::Fill(1),
    ])
    .split(area);

    let v_center = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(modal_height),
        Constraint::Fill(1),
    ])
    .split(h_center[1]);

    let modal_area = v_center[1];

    frame.render_widget(Clear, modal_area);

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
