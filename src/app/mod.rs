//! Application event loop: draws the TUI, polls events, dispatches actions
//! via `actions::dispatch_action`, and drains background task messages.

mod actions;
mod spawn;

use crate::config::SparkConfig;
use crate::tui::model::*;
use crate::tui::update;
use crate::tui::view;
use crate::updater::detector::Detector;
use crossterm::event::{self, Event};
use ratatui::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    config: SparkConfig,
    scan_only: bool,
    _update_only: bool,
    dry_run: bool,
) -> color_eyre::Result<()> {
    let mut app = App::new(config);
    app.dry_run = dry_run;

    if scan_only {
        app.show_welcome = false;
        app.mode = AppMode::Scanner;
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<AppMessage>();
    let detector = Arc::new(Detector::new());

    spawn::spawn_version_checks(&app.updater, detector.clone(), tx.clone());
    spawn::spawn_warmup(detector.clone(), tx.clone());

    // Pre-discover directories so the Scanner tab is ready when the user opens it
    {
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let dirs = crate::scanner::repo_scanner::discover_project_dirs();
            let _ = tx2.send(AppMessage::DiscoveredDirs { dirs });
        });
    }

    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|frame| view::draw(frame, &app))?;

        let has_event =
            tokio::task::spawn_blocking(move || event::poll(tick_rate).unwrap_or(false))
                .await
                .unwrap_or(false);

        if has_event {
            let ev = event::read();

            if let Ok(Event::FocusGained) = &ev {
                terminal.clear()?;
            }
            if let Ok(Event::Resize(w, h)) = &ev {
                app.width = *w;
                app.height = *h;
                terminal.clear()?;
            }

            if let Ok(Event::Key(key)) = ev {
                if let Some(action) = update::handle_key(&mut app, key) {
                    if actions::dispatch_action(action, &mut app, &tx, &detector) {
                        break;
                    }
                }
            }
        }

        while let Ok(msg) = rx.try_recv() {
            if let Some(action) = update::handle_message(&mut app, msg) {
                if actions::dispatch_message_action(action, &mut app, &tx, &detector) {
                    break;
                }
            }
        }

        app.tick_count = app.tick_count.wrapping_add(1);
        if app.updater.state == UpdaterState::Updating {
            app.updater.splash_frame = app.tick_count;
        }

        // Auto-dismiss toast after 30 ticks (~3 seconds at 100ms tick rate)
        if let Some(toast) = &app.toast {
            if app.tick_count.saturating_sub(toast.created_at) >= 30 {
                app.toast = None;
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
