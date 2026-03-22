use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::scanner::port_scanner;

/// Render the port scanner view
pub fn render_ports(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(5),   // port table
        Constraint::Length(2), // help
    ])
    .split(area);

    render_header(frame, chunks[0], model);
    render_port_table(frame, chunks[1], model);
    render_help(frame, chunks[2]);
}

fn render_header(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    let dev_count = model
        .ports
        .iter()
        .filter(|p| port_scanner::is_dev_port(p.port))
        .count();

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " 🔌 PORT SCANNER ",
                Style::default().fg(WHITE).bg(CYAN).bold(),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} listening ports", model.ports.len()),
                Style::default().fg(GRAY),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} dev servers", dev_count),
                Style::default().fg(YELLOW).bold(),
            ),
        ]),
        Line::from(Span::styled(
            "Kill forgotten dev servers and free up ports",
            Style::default().fg(GRAY),
        )),
    ]);
    frame.render_widget(header, area);
}

fn render_port_table(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    if model.ports.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No listening ports found (or insufficient permissions)",
                Style::default().fg(GRAY),
            )),
            Line::from(Span::styled(
                "  Try running spark with sudo for full visibility",
                Style::default().fg(GRAY),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(PURPLE)),
        );
        frame.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("  Port").style(Style::default().fg(PURPLE).bold()),
        Cell::from("PID").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Process").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Command / Working Dir").style(Style::default().fg(PURPLE).bold()),
    ]);

    let rows: Vec<Row> = model
        .ports
        .iter()
        .enumerate()
        .map(|(i, port_info)| {
            let is_selected = model.cursor == i;
            let is_checked = model.checked.contains(&i);
            let is_dev = port_scanner::is_dev_port(port_info.port);

            let cursor = if is_selected { "❯" } else { " " };
            let checkbox = if is_checked { "✔" } else { " " };

            let port_style = if is_dev {
                Style::default().fg(YELLOW).bold()
            } else {
                Style::default().fg(GRAY)
            };

            let cwd_str = port_info
                .cwd
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            let cmd_display = if !port_info.cmdline.is_empty() {
                if port_info.cmdline.len() > 50 {
                    format!("{}...", &port_info.cmdline[..47])
                } else {
                    port_info.cmdline.clone()
                }
            } else if !cwd_str.is_empty() {
                format!("cwd: {}", cwd_str)
            } else {
                String::new()
            };

            let row_style = if is_selected {
                Style::default().bg(DARK_BG)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("{} [{}] :{}", cursor, checkbox, port_info.port))
                    .style(port_style),
                Cell::from(format!("{}", port_info.pid)),
                Cell::from(port_info.process_name.clone())
                    .style(if is_dev {
                        Style::default().fg(WHITE)
                    } else {
                        Style::default().fg(GRAY)
                    }),
                Cell::from(cmd_display).style(Style::default().fg(GRAY)),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Length(18),
            Constraint::Min(30),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(CYAN))
            .title(Span::styled(
                " Listening Ports ",
                Style::default().fg(CYAN).bold(),
            )),
    );
    frame.render_widget(table, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(Span::styled(
        "[SPACE] Select • [k] Kill Selected • [K] Kill All Dev Ports • [r] Refresh • [ESC] Back • [Q] Quit",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, area);
}

/// Render kill confirmation modal
pub fn render_kill_confirm(
    frame: &mut Frame,
    area: Rect,
    count: usize,
    ports: &str,
) {
    let modal_area = center_modal(frame, area, 55, 10);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(RED))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = vec![
        Line::from(Span::styled(
            "KILL PROCESSES",
            Style::default().fg(RED).bold(),
        )),
        Line::from(""),
        Line::from(format!(
            "This will terminate {} process(es) on ports:",
            count
        )),
        Line::from(Span::styled(ports, Style::default().fg(YELLOW).bold())),
        Line::from(""),
        Line::from(Span::styled(
            "Proceed? (y/N)",
            Style::default().fg(WHITE),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}
