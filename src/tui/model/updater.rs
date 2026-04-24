//! Updater tab state: tool table, selection, search filter, update queue.

use super::UpdaterState;
use crate::core::inventory::get_inventory;
use crate::core::types::*;
use std::collections::{HashSet, VecDeque};

pub struct UpdaterModel {
    pub state: UpdaterState,
    pub items: Vec<ToolState>,
    pub cursor: usize,
    pub checked: HashSet<usize>,
    pub loading_count: usize,
    pub update_queue: VecDeque<usize>,
    pub current_update: Option<usize>,
    pub current_log: String,
    pub total_update: usize,
    pub updating_remaining: usize,
    pub search_query: String,
    pub filtered_indices: Option<Vec<usize>>,
    pub splash_frame: usize,
}

impl UpdaterModel {
    pub fn new() -> Self {
        let inv = get_inventory();
        let items: Vec<ToolState> = inv
            .into_iter()
            .map(|t| ToolState {
                tool: t,
                status: ToolStatus::Checking,
                local_version: "...".into(),
                remote_version: "...".into(),
                message: String::new(),
            })
            .collect();
        let loading_count = items.len();

        Self {
            state: UpdaterState::Main,
            items,
            cursor: 0,
            checked: HashSet::new(),
            loading_count,
            update_queue: VecDeque::new(),
            current_update: None,
            current_log: String::new(),
            total_update: 0,
            updating_remaining: 0,
            search_query: String::new(),
            filtered_indices: None,
            splash_frame: 0,
        }
    }

    pub fn is_item_visible(&self, index: usize) -> bool {
        match &self.filtered_indices {
            None => true,
            Some(indices) => indices.contains(&index),
        }
    }

    pub fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = None;
            return;
        }

        let query = self.search_query.to_lowercase();
        let indices: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.tool.name.to_lowercase().contains(&query)
                    || item.tool.binary.to_lowercase().contains(&query)
                    || item.tool.package.to_lowercase().contains(&query)
                    || item.tool.category.label().to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        if let Some(first) = indices.first() {
            self.cursor = *first;
        }
        self.filtered_indices = Some(indices);
    }

    pub fn jump_to_category(&mut self, cat: Category) {
        for (i, item) in self.items.iter().enumerate() {
            if item.tool.category == cat {
                self.cursor = i;
                return;
            }
        }
    }

    pub fn has_critical_selected(&self) -> bool {
        self.checked
            .iter()
            .any(|&i| self.items[i].tool.category == Category::Runtime)
    }

    pub fn build_update_queue(&mut self) {
        self.update_queue.clear();
        self.total_update = 0;
        self.updating_remaining = 0;
        self.current_update = None;

        for i in 0..self.items.len() {
            if self.checked.contains(&i) {
                self.items[i].status = ToolStatus::Updating;
                self.update_queue.push_back(i);
                self.total_update += 1;
                self.updating_remaining += 1;
            }
        }
    }

    pub fn get_update_log_text(tool: &Tool) -> String {
        match tool.method {
            UpdateMethod::BrewPkg => {
                format!("> brew upgrade {}", tool.package)
            }
            UpdateMethod::NpmPkg | UpdateMethod::NpmSys | UpdateMethod::Claude => {
                format!("> npm install -g {}@latest", tool.package)
            }
            UpdateMethod::Omz => "> $ZSH/tools/upgrade.sh".into(),
            UpdateMethod::Toad => "> curl -fsSL batrachian.ai/install | sh".into(),
            UpdateMethod::MacApp => format!("> brew upgrade --cask {}", tool.package),
            _ => format!("> Updating {}...", tool.name),
        }
    }
}
