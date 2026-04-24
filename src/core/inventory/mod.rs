//! Master catalog of supported tools.
//!
//! Split into:
//! - dev.rs      — system + AI code tools + IDEs + terminals
//! - platform.rs — productivity + infrastructure + runtimes + utilities
//!
//! `get_inventory()` concatenates both lists and auto-assigns sequential
//! `S-##` IDs.

mod dev;
mod platform;

use super::types::*;

pub fn get_inventory() -> Vec<Tool> {
    let mut tools = dev::tools();
    tools.extend(platform::tools());

    // Auto-assign IDs: S-01, S-02, etc.
    for (i, tool) in tools.iter_mut().enumerate() {
        tool.id = format!("S-{:02}", i + 1);
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_not_empty() {
        let inv = get_inventory();
        assert!(!inv.is_empty());
    }

    #[test]
    fn test_inventory_ids_auto_assigned() {
        let inv = get_inventory();
        assert_eq!(inv[0].id, "S-01");
        assert_eq!(inv[1].id, "S-02");
    }

    #[test]
    fn test_inventory_ids_unique() {
        let inv = get_inventory();
        let ids: std::collections::HashSet<&str> = inv.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids.len(), inv.len());
    }

    #[test]
    fn test_inventory_all_categories_present() {
        let inv = get_inventory();
        for cat in Category::all() {
            assert!(
                inv.iter().any(|t| t.category == *cat),
                "Category {:?} has no tools",
                cat
            );
        }
    }

    #[test]
    fn test_inventory_has_claude_cli() {
        let inv = get_inventory();
        assert!(inv.iter().any(|t| t.name == "Claude CLI"));
    }

    #[test]
    fn test_inventory_first_tool_is_sys_category() {
        let inv = get_inventory();
        assert_eq!(inv[0].category, Category::Sys);
    }
}
