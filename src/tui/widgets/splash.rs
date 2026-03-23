use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::styles::*;

/// Render the welcome screen with capabilities overview
pub fn render_splash(frame: &mut Frame, area: Rect, tick: usize) {
    let color_index = tick % SPLASH_COLORS.len();
    let color = SPLASH_COLORS[color_index];

    frame.render_widget(Clear, area);

    // Build logo as separate lines
    let logo_lines: Vec<Line> = SPARK_ART
        .trim_matches('\n')
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(color).bold())))
        .collect();

    let logo_height = logo_lines.len() as u16;

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(logo_height),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(11),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(area);

    let logo_widget = Paragraph::new(logo_lines).alignment(Alignment::Center);
    frame.render_widget(logo_widget, chunks[1]);

    let subtitle = Paragraph::new(Line::from(Span::styled(
        format!("Developer Operations Platform {}", VERSION),
        Style::default().fg(GRAY).italic(),
    )))
    .alignment(Alignment::Center);
    frame.render_widget(subtitle, chunks[3]);

    let sep_width = area.width.min(50) as usize;
    let sep = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("─".repeat(sep_width), Style::default().fg(DARK_BG))),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(sep, chunks[4]);

    // Capabilities - Scanner first (primary), then Repos, Ports, Updater
    let cap_width = 56u16;
    let cap_area = if area.width > cap_width + 4 {
        Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(cap_width),
            Constraint::Fill(1),
        ]).split(chunks[5])[1]
    } else {
        chunks[5]
    };

    let capabilities = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  S  ", Style::default().fg(PURPLE).bold()),
            Span::styled("Scanner      ", Style::default().fg(WHITE).bold()),
            Span::styled("Score git repo health", Style::default().fg(GRAY)),
        ]),
        Line::from(Span::styled(
            "                   Stale repos, clean artifacts",
            Style::default().fg(TERM_GRAY),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  G  ", Style::default().fg(GREEN).bold()),
            Span::styled("Repo Manager ", Style::default().fg(WHITE).bold()),
            Span::styled("Clone & sync (ghq-style)", Style::default().fg(GRAY)),
        ]),
        Line::from(Span::styled(
            "                   host/owner/name, pull all",
            Style::default().fg(TERM_GRAY),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  P  ", Style::default().fg(YELLOW).bold()),
            Span::styled("Port Scanner ", Style::default().fg(WHITE).bold()),
            Span::styled("Find & kill dev servers", Style::default().fg(GRAY)),
        ]),
        Line::from(Span::styled(
            "                   Node, Python, Go, Rust, ...",
            Style::default().fg(TERM_GRAY),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  U  ", Style::default().fg(BLUE).bold()),
            Span::styled("Updater      ", Style::default().fg(WHITE).bold()),
            Span::styled("Update 44+ dev tools", Style::default().fg(GRAY)),
        ]),
        Line::from(Span::styled(
            "                   AI, IDEs, CLIs, runtimes, infra",
            Style::default().fg(TERM_GRAY),
        )),
    ]);
    frame.render_widget(capabilities, cap_area);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("ENTER", Style::default().fg(GREEN).bold()),
        Span::styled(" start  ", Style::default().fg(GRAY)),
        Span::styled("Q", Style::default().fg(RED).bold()),
        Span::styled(" quit", Style::default().fg(GRAY)),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(prompt, chunks[7]);
}
