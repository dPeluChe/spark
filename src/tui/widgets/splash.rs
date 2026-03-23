use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::styles::*;

/// Render the animated splash screen
pub fn render_splash(frame: &mut Frame, area: Rect, splash_frame: usize) {
    let color_index = splash_frame % SPLASH_COLORS.len();
    let color = SPLASH_COLORS[color_index];

    // Loading dots animation
    let dot_count = (splash_frame / 3) % 4;
    let dots = ".".repeat(dot_count);

    let logo = SPARK_ART.trim_start_matches('\n');

    let subtitle = format!(
        "\n   Surgical Precision Update Utility {}\n   Initializing System Core{}",
        VERSION, dots
    );

    let text = vec![
        Line::from(Span::styled(logo, Style::default().fg(color).bold())),
        Line::from(""),
        Line::from(Span::styled(
            subtitle,
            Style::default().fg(GRAY).italic(),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default());

    // Center vertically
    let v_layout = Layout::vertical([
        Constraint::Percentage(35),
        Constraint::Min(10),
        Constraint::Percentage(35),
    ])
    .split(area);

    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, v_layout[1]);
}
