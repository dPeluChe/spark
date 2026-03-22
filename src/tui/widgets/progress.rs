use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::styles::*;

/// Render an indeterminate progress bar animation
pub fn render_indeterminate_bar(frame: &mut Frame, area: Rect, tick: usize) {
    let width = area.width as usize - 2; // minus brackets
    let block_width = 10;
    let pos = tick % (width + block_width);

    let mut bar = String::with_capacity(width + 2);
    bar.push('[');
    for i in 0..width {
        let effective_pos = pos as i32 - block_width as i32;
        if i as i32 >= effective_pos && (i as i32) < pos as i32 {
            bar.push('▓');
        } else {
            bar.push('░');
        }
    }
    bar.push(']');

    let paragraph = Paragraph::new(Span::styled(bar, Style::default().fg(BLUE)));
    frame.render_widget(paragraph, area);
}

/// Render a determinate progress bar
pub fn render_progress_bar(frame: &mut Frame, area: Rect, percent: f64, label: &str) {
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(GREEN).bg(DARK))
        .percent((percent * 100.0) as u16)
        .label(Span::styled(label, Style::default().fg(WHITE)));

    frame.render_widget(gauge, area);
}

/// Render the updating modal overlay
pub fn render_updating_overlay(
    frame: &mut Frame,
    area: Rect,
    total: usize,
    remaining: usize,
    current_tool: Option<&str>,
    current_log: &str,
    tick: usize,
) {
    let completed = total - remaining;
    let percent = if total > 0 {
        completed as f64 / total as f64
    } else {
        0.0
    };

    let modal_width = 60u16.min(area.width - 4);
    let modal_height = 12u16.min(area.height - 4);

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

    // Clear the area behind the modal
    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(BLUE))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // title
        Constraint::Length(1), // spacing
        Constraint::Length(1), // label
        Constraint::Length(1), // progress bar
        Constraint::Length(1), // spacing
        Constraint::Length(1), // current tool
        Constraint::Length(1), // log
        Constraint::Min(0),
    ])
    .split(inner);

    // Title
    let title = Paragraph::new(Span::styled(
        "⟳ SYSTEM UPDATE IN PROGRESS",
        Style::default().fg(BLUE).bold(),
    ))
    .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Progress label
    let label = format!("Progress: {}/{} completed", completed, total);
    let tool_info = current_tool
        .map(|t| format!(" • Processing: {}", t))
        .unwrap_or_default();
    let progress_label = Paragraph::new(vec![Line::from(vec![
        Span::styled(label, Style::default().fg(BLUE)),
        Span::styled(tool_info, Style::default().fg(YELLOW).bold()),
    ])]);
    frame.render_widget(progress_label, chunks[2]);

    // Progress bar
    if total <= 1 || percent == 0.0 {
        render_indeterminate_bar(frame, chunks[3], tick);
    } else {
        render_progress_bar(
            frame,
            chunks[3],
            percent,
            &format!("{}%", (percent * 100.0) as u16),
        );
    }

    // Current log
    if !current_log.is_empty() {
        let log = Paragraph::new(Span::styled(
            current_log,
            Style::default().fg(TERM_GRAY).bg(DARK),
        ))
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(PURPLE)),
        );
        frame.render_widget(log, chunks[6]);
    }
}

/// Render the summary overlay
pub fn render_summary_overlay(
    frame: &mut Frame,
    area: Rect,
    items: &[crate::core::types::ToolState],
) {
    let mut success_count = 0;
    let mut fail_count = 0;
    let mut failure_details = Vec::new();

    for item in items {
        match item.status {
            crate::core::types::ToolStatus::Updated => success_count += 1,
            crate::core::types::ToolStatus::Failed => {
                fail_count += 1;
                failure_details.push(format!("• {}: {}", item.tool.name, item.message));
            }
            _ => {}
        }
    }

    let modal_width = 60u16.min(area.width - 4);
    let modal_height = (8 + failure_details.len() as u16).min(area.height - 4);

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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(PURPLE))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let mut lines = vec![
        Line::from(Span::styled(
            "UPDATE COMPLETE",
            Style::default().fg(WHITE).bg(PURPLE).bold(),
        )),
        Line::from(""),
        Line::from(format!(
            "Successful: {}  |  Failed: {}",
            success_count, fail_count
        )),
    ];

    if !failure_details.is_empty() {
        lines.push(Line::from(""));
        for detail in &failure_details {
            lines.push(Line::from(Span::styled(detail.clone(), Style::default().fg(RED))));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Press ENTER to close]",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}
