//! Scan config screen: directory picker before a scan + in-progress view.

use crate::tui::model::*;
use crate::tui::styles::*;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub(super) fn render_scan_config(frame: &mut Frame, area: Rect, model: &ScannerModel) {
    let chunks = Layout::vertical([
        Constraint::Length(4), // title
        Constraint::Min(5),    // directory list
        Constraint::Length(2), // help
    ])
    .split(area);

    let selected_count = model.selected_scan_dirs.len();
    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " REPOSITORY SCANNER ",
                Style::default().fg(WHITE).bg(PURPLE).bold(),
            ),
            Span::raw("  "),
            if selected_count > 0 {
                Span::styled(
                    format!("{} selected", selected_count),
                    Style::default().fg(GREEN).bold(),
                )
            } else {
                Span::styled("none selected", Style::default().fg(YELLOW))
            },
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Folders in ~/ with git repos detected. Select which to scan.",
            Style::default().fg(GRAY),
        )),
        Line::from(Span::styled(
            "[a] to add a custom path manually.",
            Style::default().fg(TERM_GRAY),
        )),
    ]);
    frame.render_widget(title, chunks[0]);

    let home = std::env::var("HOME").unwrap_or_default();
    let mut lines = Vec::new();
    for (i, discovered) in model.discovered_dirs.iter().enumerate() {
        let is_selected = model.cursor == i;
        let is_checked = model.selected_scan_dirs.contains(&i);

        let cursor = if is_selected {
            Span::styled("> ", Style::default().fg(GREEN).bold())
        } else {
            Span::raw("  ")
        };

        let checkbox = render_checkbox(is_checked);

        let dir_str = discovered.path.display().to_string();
        let short = if dir_str.starts_with(&home) {
            format!("~{}", &dir_str[home.len()..])
        } else {
            dir_str
        };

        let dir_style = if is_selected {
            Style::default().fg(WHITE).bold()
        } else {
            Style::default().fg(GRAY)
        };

        let count_label = if discovered.repo_count == 1 {
            "1 repo".to_string()
        } else {
            format!("{} repos", discovered.repo_count)
        };

        lines.push(Line::from(vec![
            cursor,
            checkbox,
            Span::styled(format!("{:<40}", short), dir_style),
            Span::styled(
                count_label,
                Style::default().fg(if discovered.repo_count > 10 {
                    GREEN
                } else {
                    TERM_GRAY
                }),
            ),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No directories with git repos found in ~/",
            Style::default().fg(YELLOW),
        )));
        lines.push(Line::from(Span::styled(
            "  Press [a] to add a specific path",
            Style::default().fg(TERM_GRAY),
        )));
    }

    let list = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(PURPLE))
            .title(Span::styled(
                " Directories ",
                Style::default().fg(PURPLE).bold(),
            )),
    );
    frame.render_widget(list, chunks[1]);

    let help = Paragraph::new(Span::styled(
        "[ENTER] Scan • [SPACE] Toggle • [a] Add path • [d] Remove • [r] Refresh • [TAB] Next • [q] Quit",
        Style::default().fg(GRAY),
    ));
    frame.render_widget(help, chunks[2]);
}

pub(super) fn render_scanning(frame: &mut Frame, area: Rect, model: &ScannerModel, tick: usize) {
    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let home = std::env::var("HOME").unwrap_or_default();

    let scanning_dirs: Vec<String> = model
        .selected_scan_dirs
        .iter()
        .filter_map(|&i| model.discovered_dirs.get(i))
        .map(|d| {
            let s = d.path.display().to_string();
            if s.starts_with(&home) {
                format!("~{}", &s[home.len()..])
            } else {
                s
            }
        })
        .collect();

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Scanning directories...", spinner),
            Style::default().fg(BLUE).bold(),
        )),
        Line::from(""),
    ];

    for dir_name in &scanning_dirs {
        lines.push(Line::from(vec![
            Span::styled("  > ", Style::default().fg(GREEN)),
            Span::styled(dir_name.clone(), Style::default().fg(WHITE)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Entries scanned: ", Style::default().fg(GRAY)),
        Span::styled(
            format!("{}", model.scan_progress_dirs),
            Style::default().fg(WHITE),
        ),
        Span::styled("    Repos found: ", Style::default().fg(GRAY)),
        Span::styled(
            format!("{}", model.scan_progress_repos),
            Style::default().fg(GREEN).bold(),
        ),
    ]));

    if !model.scan_progress_current.is_empty() {
        let current = if model.scan_progress_current.starts_with(&home) {
            format!("~{}", &model.scan_progress_current[home.len()..])
        } else {
            model.scan_progress_current.clone()
        };
        let display = if current.len() > 60 {
            format!("...{}", &current[current.len() - 57..])
        } else {
            current
        };
        lines.push(Line::from(Span::styled(
            format!("  {}", display),
            Style::default().fg(TERM_GRAY),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  [ESC] Cancel scan",
        Style::default().fg(GRAY),
    )));

    let paragraph = Paragraph::new(lines).block(Block::default().padding(Padding::new(2, 2, 1, 1)));
    frame.render_widget(paragraph, area);
}
