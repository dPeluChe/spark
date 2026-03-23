use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::model::*;
use crate::tui::styles::*;
use crate::scanner::port_scanner::{self, Runtime};

/// Color for a runtime badge
fn runtime_color(runtime: &Runtime) -> Color {
    match runtime {
        Runtime::Node => Color::Rgb(67, 160, 71),
        Runtime::Python => Color::Rgb(55, 118, 171),
        Runtime::Go => Color::Rgb(0, 173, 216),
        Runtime::Ruby => Color::Rgb(204, 52, 45),
        Runtime::Java => Color::Rgb(236, 112, 37),
        Runtime::Rust => Color::Rgb(222, 165, 97),
        Runtime::Php => Color::Rgb(119, 123, 179),
        Runtime::Dotnet => Color::Rgb(81, 43, 212),
        Runtime::Elixir => Color::Rgb(110, 74, 126),
        Runtime::Deno => Color::Rgb(18, 18, 18),
        Runtime::Bun => Color::Rgb(251, 240, 223),
        Runtime::Nginx => Color::Rgb(0, 150, 57),
        Runtime::Docker => Color::Rgb(36, 150, 237),
        Runtime::Other(_) => GRAY,
    }
}

/// Extract project name from project_dir string
fn extract_project_name(project_dir: &Option<String>) -> String {
    match project_dir {
        Some(p) if p != "/" && !p.is_empty() => {
            // Format: "project-name (~/path/to/project)" or just a path
            if let Some(paren) = p.find(" (") {
                p[..paren].to_string()
            } else {
                // Just a path - take last component
                p.rsplit('/').next().unwrap_or(p).to_string()
            }
        }
        _ => String::new(),
    }
}

/// Extract short path from project_dir
fn extract_short_path(project_dir: &Option<String>) -> String {
    match project_dir {
        Some(p) if p != "/" && !p.is_empty() => {
            if let Some(start) = p.find('(') {
                if let Some(end) = p.find(')') {
                    return p[start + 1..end].to_string();
                }
            }
            // Shorten home prefix
            let home = std::env::var("HOME").unwrap_or_default();
            if p.starts_with(&home) {
                format!("~{}", &p[home.len()..])
            } else {
                p.clone()
            }
        }
        _ => String::new(),
    }
}

/// Render the port scanner view
pub fn render_ports(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(5),   // port table
        Constraint::Length(1), // help
    ])
    .split(area);

    render_header(frame, chunks[0], model);

    if model.scanning && model.ports.is_empty() {
        render_scanning(frame, chunks[1]);
    } else if model.ports.is_empty() {
        render_empty(frame, chunks[1]);
    } else {
        render_port_table(frame, chunks[1], model);
    }

    render_help(frame, chunks[2], model);
}

fn render_header(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    let dev_count = model.ports.iter().filter(|p| port_scanner::is_dev_server(p)).count();
    let sys_count = model.ports.len().saturating_sub(dev_count);

    let mut runtime_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for p in &model.ports {
        if port_scanner::is_dev_server(p) {
            *runtime_counts.entry(p.runtime.short_label().to_string()).or_default() += 1;
        }
    }
    let mut rt_parts: Vec<(String, usize)> = runtime_counts.into_iter().collect();
    rt_parts.sort_by(|a, b| b.1.cmp(&a.1));
    let runtime_summary: String = rt_parts.iter().map(|(k, v)| format!("{}:{}", k, v)).collect::<Vec<_>>().join(" ");

    let status = if model.scanning {
        Span::styled("scanning...", Style::default().fg(YELLOW).italic())
    } else {
        Span::styled(format!("{} listening", model.ports.len()), Style::default().fg(GRAY))
    };

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" PORT SCANNER ", Style::default().fg(WHITE).bg(CYAN).bold()),
            Span::raw("  "),
            status,
            Span::raw("  "),
            Span::styled(format!("{} dev", dev_count), Style::default().fg(GREEN).bold()),
            Span::raw("  "),
            Span::styled(format!("{} system", sys_count), Style::default().fg(GRAY)),
            Span::raw("  "),
            Span::styled(runtime_summary, Style::default().fg(PURPLE)),
        ]),
        Line::from(Span::styled(
            "Find & manage dev servers and listening ports",
            Style::default().fg(TERM_GRAY),
        )),
    ]);
    frame.render_widget(header, area);
}

fn render_scanning(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(CYAN))
        .title(Span::styled(" Scanning ", Style::default().fg(CYAN).bold()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled("  Analyzing listening ports...", Style::default().fg(YELLOW))),
        Line::from(""),
        Line::from(Span::styled("  Resolving processes and project paths", Style::default().fg(GRAY))),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn render_empty(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(PURPLE));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled("  No listening ports detected", Style::default().fg(GRAY))),
        Line::from(""),
        Line::from(Span::styled("  Press [r] to rescan", Style::default().fg(TERM_GRAY))),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn render_port_table(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    // Use pre-built display_order from model
    // Split into dev and system sections
    let mut dev_indices: Vec<usize> = Vec::new();
    let mut sys_indices: Vec<usize> = Vec::new();
    for &idx in &model.display_order {
        if port_scanner::is_dev_server(&model.ports[idx]) {
            dev_indices.push(idx);
        } else {
            sys_indices.push(idx);
        }
    }

    // Build display list with section headers
    let mut display_items: Vec<DisplayItem> = Vec::new();
    if !dev_indices.is_empty() {
        display_items.push(DisplayItem::Section(format!(" Dev Servers ({})", dev_indices.len())));
        for &i in &dev_indices {
            display_items.push(DisplayItem::Port(i));
        }
    }
    if !sys_indices.is_empty() {
        display_items.push(DisplayItem::Section(format!(" System & Apps ({})", sys_indices.len())));
        for &i in &sys_indices {
            display_items.push(DisplayItem::Port(i));
        }
    }

    // cursor is an index into display_order; find the real port index under cursor
    let cursor_real_idx = model.cursor_port_index();
    let header = Row::new(vec![
        Cell::from("  Port").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Lang").style(Style::default().fg(PURPLE).bold()),
        Cell::from("PID").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Process").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Project").style(Style::default().fg(PURPLE).bold()),
        Cell::from("Path").style(Style::default().fg(PURPLE).bold()),
    ]);

    let rows: Vec<Row> = display_items
        .iter()
        .map(|item| match item {
            DisplayItem::Section(title) => {
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(Span::styled(
                        title.clone(),
                        Style::default().fg(CYAN).bold().italic(),
                    )),
                    Cell::from(""),
                ])
                .style(Style::default().bg(Color::Rgb(25, 25, 35)))
            }
            DisplayItem::Port(idx) => {
                let i = *idx;
                let port_info = &model.ports[i];
                let is_selected = cursor_real_idx == Some(i);
                let is_checked = model.checked.contains(&i);
                let is_devport = port_scanner::is_dev_server(port_info);

                let cursor = if is_selected { ">" } else { " " };
                let checkbox = if is_checked { "x" } else { " " };

                let port_style = if is_devport {
                    Style::default().fg(YELLOW).bold()
                } else {
                    Style::default().fg(TERM_GRAY)
                };

                let rt_color = runtime_color(&port_info.runtime);
                let rt_label = port_info.runtime.short_label();

                let project = extract_project_name(&port_info.project_dir);
                let path = extract_short_path(&port_info.project_dir);

                let process_style = if is_devport {
                    Style::default().fg(WHITE)
                } else {
                    Style::default().fg(TERM_GRAY)
                };

                let row_style = if is_selected {
                    Style::default().bg(DARK_BG)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(format!("{} [{}] :{}", cursor, checkbox, port_info.port))
                        .style(port_style),
                    Cell::from(format!(" {} ", rt_label))
                        .style(Style::default().fg(WHITE).bg(rt_color).bold()),
                    Cell::from(format!("{}", port_info.pid))
                        .style(Style::default().fg(TERM_GRAY)),
                    Cell::from(port_info.process_name.clone())
                        .style(process_style),
                    Cell::from(project)
                        .style(if is_devport {
                            Style::default().fg(GREEN)
                        } else {
                            Style::default().fg(TERM_GRAY)
                        }),
                    Cell::from(path)
                        .style(Style::default().fg(TERM_GRAY)),
                ])
                .style(row_style)
            }
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),  // Port
            Constraint::Length(6),   // Lang
            Constraint::Length(8),   // PID
            Constraint::Length(14),  // Process
            Constraint::Length(22),  // Project
            Constraint::Min(20),    // Path
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

fn render_help(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    let selected_count = model.checked.len();
    let help_text = if selected_count > 0 {
        format!(
            " [x] Kill {} selected  [SPACE] Toggle  [r] Rescan  [TAB] Switch  [q] Quit",
            selected_count
        )
    } else {
        " [ENTER] Details  [SPACE] Select  [x] Kill  [X] Kill all dev  [r] Rescan  [TAB] Switch".to_string()
    };
    let help = Paragraph::new(Span::styled(help_text, Style::default().fg(GRAY)));
    frame.render_widget(help, area);
}

enum DisplayItem {
    Section(String),
    Port(usize),
}

/// Render action modal for the currently selected port
pub fn render_action_modal(frame: &mut Frame, area: Rect, model: &PortScannerModel) {
    let real_idx = match model.cursor_port_index() {
        Some(i) => i,
        None => return,
    };
    let port_info = match model.ports.get(real_idx) {
        Some(p) => p,
        None => return,
    };

    let modal_area = center_modal(frame, area, 60, 16);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(CYAN))
        .style(Style::default().bg(MODAL_BG));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let project = extract_project_name(&port_info.project_dir);
    let path = extract_short_path(&port_info.project_dir);

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" :{} ", port_info.port),
            Style::default().fg(WHITE).bg(CYAN).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Process:  ", Style::default().fg(PURPLE)),
            Span::styled(&port_info.process_name, Style::default().fg(WHITE)),
            Span::styled(format!("  (PID {})", port_info.pid), Style::default().fg(GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Runtime:  ", Style::default().fg(PURPLE)),
            Span::styled(format!("{}", port_info.runtime), Style::default().fg(WHITE)),
        ]),
    ];

    if !project.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Project:  ", Style::default().fg(PURPLE)),
            Span::styled(&project, Style::default().fg(GREEN).bold()),
        ]));
    }
    if !path.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Path:     ", Style::default().fg(PURPLE)),
            Span::styled(&path, Style::default().fg(GRAY)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Actions:",
        Style::default().fg(YELLOW).bold(),
    )));
    lines.push(Line::from(vec![
        Span::styled("  [k] ", Style::default().fg(RED).bold()),
        Span::styled("Kill this process", Style::default().fg(WHITE)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  [o] ", Style::default().fg(BLUE).bold()),
        Span::styled("Open project folder in terminal", Style::default().fg(WHITE)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [q] Close    [ESC] Close",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
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
