//! Background task spawners for the Updater: initial version checks,
//! remote version checks, cache warmup, and individual tool updates.

use crate::core::types::{Tool, ToolStatus};
use crate::tui::model::{AppMessage, UpdaterModel};
use crate::updater::detector::Detector;
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn spawn_version_checks(
    updater: &UpdaterModel,
    detector: Arc<Detector>,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    for (i, item) in updater.items.iter().enumerate() {
        let tool = item.tool.clone();
        let detector = detector.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let local = detector.get_local_version(&tool).await;

            let status = if local == "MISSING" {
                ToolStatus::Missing
            } else {
                ToolStatus::Installed
            };

            let message = if local == "MISSING" {
                "Not installed".into()
            } else {
                String::new()
            };

            let _ = tx.send(AppMessage::CheckResult {
                index: i,
                local_version: local,
                remote_version: "...".into(),
                status,
                message,
            });
        });
    }
}

pub fn spawn_remote_checks(
    updater: &UpdaterModel,
    detector: Arc<Detector>,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    for (i, item) in updater.items.iter().enumerate() {
        let tool = item.tool.clone();
        let local = item.local_version.clone();
        let detector = detector.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let remote = detector.get_remote_version(&tool, &local).await;

            let status = if local == "MISSING" {
                ToolStatus::Missing
            } else if remote != "Unknown" && remote != "Checking..." && remote != local {
                ToolStatus::Outdated
            } else {
                ToolStatus::Installed
            };

            let message = if status == ToolStatus::Outdated {
                "Update available".into()
            } else {
                String::new()
            };

            let _ = tx.send(AppMessage::CheckResult {
                index: i,
                local_version: local,
                remote_version: remote,
                status,
                message,
            });
        });
    }
}

pub fn spawn_warmup(detector: Arc<Detector>, tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        detector.warm_up_cache().await;
        let _ = tx.send(AppMessage::WarmUpFinished);
    });
}

pub fn spawn_update_task(
    index: usize,
    tool: Tool,
    detector: Arc<Detector>,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    tokio::spawn(async move {
        let result = crate::updater::executor::update_tool(&tool).await;
        let (success, message) = match result {
            Ok(_) => (true, format!("Updated {}", tool.name)),
            Err(e) => (false, e),
        };
        let new_version = if success {
            detector.get_local_version(&tool).await
        } else {
            String::new()
        };
        let _ = tx.send(AppMessage::UpdateResult {
            index,
            success,
            message,
            new_version,
        });
    });
}
