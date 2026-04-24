use std::fmt;

/// How a tool gets updated
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateMethod {
    /// Homebrew formula: `brew upgrade <package>`
    BrewPkg,
    /// npm global: `npm install -g <package>@latest`
    NpmSys,
    /// npm package variant
    NpmPkg,
    /// macOS app via cask: `brew upgrade --cask <package>`
    MacApp,
    /// Claude CLI via npm
    Claude,
    /// Android emulator
    Droid,
    /// Batrachian installer: `curl -fsSL batrachian.ai/install | sh`
    Toad,
    /// Opencode tool
    Opencode,
    /// Oh My Zsh: `$ZSH/tools/upgrade.sh`
    Omz,
    /// Manual update (no automation)
    Manual,
}

/// Tool category grouping for the dashboard grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Code,
    Term,
    Ide,
    Prod,
    Infra,
    Utils,
    Runtime,
    Sys,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Code => "AI Development",
            Category::Term => "Terminals",
            Category::Ide => "IDEs & Editors",
            Category::Prod => "Productivity",
            Category::Infra => "Infrastructure",
            Category::Utils => "Utilities",
            Category::Runtime => "Runtimes",
            Category::Sys => "System",
        }
    }

    #[allow(dead_code)]
    pub fn short_key(&self) -> &'static str {
        match self {
            Category::Code => "C",
            Category::Term => "T",
            Category::Ide => "I",
            Category::Prod => "P",
            Category::Infra => "F",
            Category::Utils => "U",
            Category::Runtime => "R",
            Category::Sys => "S",
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Category::Sys,
            Category::Code,
            Category::Ide,
            Category::Term,
            Category::Prod,
            Category::Infra,
            Category::Runtime,
            Category::Utils,
        ]
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Current status of a tool in the update lifecycle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    /// Version check in progress
    Checking,
    /// Tool installed and up to date
    Installed,
    /// Newer version available
    Outdated,
    /// Tool not found on the system
    Missing,
    /// Update in progress
    Updating,
    /// Successfully updated
    Updated,
    /// Update failed
    Failed,
}

/// A software tool managed by Spark
#[derive(Debug, Clone)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub binary: String,
    pub package: String,
    pub category: Category,
    pub method: UpdateMethod,
}

/// Runtime state for a tool (version info + status)
#[derive(Debug, Clone)]
pub struct ToolState {
    pub tool: Tool,
    pub status: ToolStatus,
    pub local_version: String,
    pub remote_version: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_labels() {
        assert_eq!(Category::Code.label(), "AI Development");
        assert_eq!(Category::Runtime.label(), "Runtimes");
        assert_eq!(Category::Sys.label(), "System");
    }

    #[test]
    fn test_category_short_keys() {
        assert_eq!(Category::Code.short_key(), "C");
        assert_eq!(Category::Infra.short_key(), "F");
    }

    #[test]
    fn test_category_all_count() {
        assert_eq!(Category::all().len(), 8);
    }

    #[test]
    fn test_category_display() {
        assert_eq!(format!("{}", Category::Term), "Terminals");
    }

    #[test]
    fn test_tool_status_variants() {
        let statuses = [
            ToolStatus::Checking,
            ToolStatus::Installed,
            ToolStatus::Outdated,
            ToolStatus::Missing,
            ToolStatus::Updating,
            ToolStatus::Updated,
            ToolStatus::Failed,
        ];
        assert_eq!(statuses.len(), 7);
    }
}
