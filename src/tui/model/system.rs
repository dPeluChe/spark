//! System cleaner tab state: cleanable items grouped by category.

use crate::scanner::system_cleaner::{CleanCategory, CleanableItem};

pub struct SystemCleanerModel {
    pub items: Vec<CleanableItem>,
    /// Current item index (indexes into `items`).
    pub cursor: usize,
    pub checked: std::collections::HashSet<usize>,
    pub scanning: bool,
    /// Display rows: None = category header (non-selectable), Some(i) = items[i].
    pub display_order: Vec<Option<usize>>,
}

impl SystemCleanerModel {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            cursor: 0,
            checked: std::collections::HashSet::new(),
            scanning: false,
            display_order: Vec::new(),
        }
    }

    /// Rebuild display_order after items change. Inserts category header sentinels.
    pub fn rebuild_display_order(&mut self) {
        let categories = [
            CleanCategory::Docker,
            CleanCategory::VMs,
            CleanCategory::Cache,
            CleanCategory::Logs,
            CleanCategory::Downloads,
        ];
        let mut order = Vec::new();
        for cat in &categories {
            let cat_indices: Vec<usize> = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, i)| i.category == *cat)
                .map(|(idx, _)| idx)
                .collect();
            if cat_indices.is_empty() {
                continue;
            }
            order.push(None); // category header row
            for idx in cat_indices {
                order.push(Some(idx));
            }
        }
        self.display_order = order;
        if self.cursor >= self.items.len() && !self.items.is_empty() {
            self.cursor = self.items.len() - 1;
        }
    }

    pub fn move_up(&mut self) {
        let dc = self
            .display_order
            .iter()
            .position(|d| *d == Some(self.cursor))
            .unwrap_or(0);
        let mut i = dc;
        loop {
            if i == 0 {
                break;
            }
            i -= 1;
            if let Some(Some(idx)) = self.display_order.get(i) {
                self.cursor = *idx;
                break;
            }
        }
    }

    pub fn move_down(&mut self) {
        let dc = self
            .display_order
            .iter()
            .position(|d| *d == Some(self.cursor))
            .unwrap_or(0);
        let mut i = dc + 1;
        while i < self.display_order.len() {
            if let Some(Some(idx)) = self.display_order.get(i) {
                self.cursor = *idx;
                break;
            }
            i += 1;
        }
    }
}
