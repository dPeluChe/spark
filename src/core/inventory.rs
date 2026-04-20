use super::types::*;

/// Returns the master list of all supported tools
pub fn get_inventory() -> Vec<Tool> {
    let mut tools = vec![
        // System — package managers, shell, version control
        Tool {
            id: String::new(),
            name: "Homebrew".into(),
            binary: "brew".into(),
            package: "homebrew".into(),
            category: Category::Sys,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "NPM".into(),
            binary: "npm".into(),
            package: "npm".into(),
            category: Category::Sys,
            method: UpdateMethod::NpmSys,
        },
        Tool {
            id: String::new(),
            name: "pnpm".into(),
            binary: "pnpm".into(),
            package: "pnpm".into(),
            category: Category::Sys,
            method: UpdateMethod::NpmPkg,
        },
        Tool {
            id: String::new(),
            name: "Yarn".into(),
            binary: "yarn".into(),
            package: "yarn".into(),
            category: Category::Sys,
            method: UpdateMethod::NpmPkg,
        },
        Tool {
            id: String::new(),
            name: "Git".into(),
            binary: "git".into(),
            package: "git".into(),
            category: Category::Sys,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "GitHub CLI".into(),
            binary: "gh".into(),
            package: "gh".into(),
            category: Category::Sys,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Oh My Zsh".into(),
            binary: "omz".into(),
            package: "oh-my-zsh".into(),
            category: Category::Sys,
            method: UpdateMethod::Omz,
        },
        Tool {
            id: String::new(),
            name: "Bash".into(),
            binary: "bash".into(),
            package: "bash".into(),
            category: Category::Sys,
            method: UpdateMethod::BrewPkg,
        },
        // AI Development
        Tool {
            id: String::new(),
            name: "Claude CLI".into(),
            binary: "claude".into(),
            package: "@anthropic-ai/claude-code".into(),
            category: Category::Code,
            method: UpdateMethod::Claude,
        },
        Tool {
            id: String::new(),
            name: "Droid CLI".into(),
            binary: "droid".into(),
            package: "factory-cli".into(),
            category: Category::Code,
            method: UpdateMethod::Droid,
        },
        Tool {
            id: String::new(),
            name: "Gemini CLI".into(),
            binary: "gemini".into(),
            package: "@google/gemini-cli".into(),
            category: Category::Code,
            method: UpdateMethod::NpmPkg,
        },
        Tool {
            id: String::new(),
            name: "OpenCode".into(),
            binary: "opencode".into(),
            package: "opencode-ai".into(),
            category: Category::Code,
            method: UpdateMethod::Opencode,
        },
        Tool {
            id: String::new(),
            name: "Codex CLI".into(),
            binary: "codex".into(),
            package: "@openai/codex".into(),
            category: Category::Code,
            method: UpdateMethod::NpmPkg,
        },
        Tool {
            id: String::new(),
            name: "Crush CLI".into(),
            binary: "crush".into(),
            package: "crush".into(),
            category: Category::Code,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Toad CLI".into(),
            binary: "toad".into(),
            package: "batrachian-toad".into(),
            category: Category::Code,
            method: UpdateMethod::Toad,
        },
        Tool {
            id: String::new(),
            name: "Ollama".into(),
            binary: "ollama".into(),
            package: "ollama".into(),
            category: Category::Code,
            method: UpdateMethod::Manual,
        },
        // IDEs & Editors
        Tool {
            id: String::new(),
            name: "VS Code".into(),
            binary: "code".into(),
            package: "visual-studio-code".into(),
            category: Category::Ide,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "Cursor IDE".into(),
            binary: "cursor".into(),
            package: "cursor".into(),
            category: Category::Ide,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "Zed Editor".into(),
            binary: "zed".into(),
            package: "zed".into(),
            category: Category::Ide,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "Windsurf".into(),
            binary: "windsurf".into(),
            package: "windsurf".into(),
            category: Category::Ide,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "Antigravity".into(),
            binary: "antigravity".into(),
            package: "antigravity".into(),
            category: Category::Ide,
            method: UpdateMethod::Manual,
        },
        // Terminals
        Tool {
            id: String::new(),
            name: "iTerm2".into(),
            binary: "iterm".into(),
            package: "iterm2".into(),
            category: Category::Term,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "Ghostty".into(),
            binary: "ghostty".into(),
            package: "ghostty".into(),
            category: Category::Term,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "Warp Terminal".into(),
            binary: "warp".into(),
            package: "warp".into(),
            category: Category::Term,
            method: UpdateMethod::MacApp,
        },
        // Productivity — dev workflow tools
        Tool {
            id: String::new(),
            name: "FFmpeg".into(),
            binary: "ffmpeg".into(),
            package: "ffmpeg".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "JQ".into(),
            binary: "jq".into(),
            package: "jq".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "FZF".into(),
            binary: "fzf".into(),
            package: "fzf".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Ripgrep".into(),
            binary: "rg".into(),
            package: "ripgrep".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "LazyGit".into(),
            binary: "lazygit".into(),
            package: "lazygit".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Delta".into(),
            binary: "delta".into(),
            package: "git-delta".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Starship".into(),
            binary: "starship".into(),
            package: "starship".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Pre-commit".into(),
            binary: "pre-commit".into(),
            package: "pre-commit".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Ruff".into(),
            binary: "ruff".into(),
            package: "ruff".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Direnv".into(),
            binary: "direnv".into(),
            package: "direnv".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Watchman".into(),
            binary: "watchman".into(),
            package: "watchman".into(),
            category: Category::Prod,
            method: UpdateMethod::BrewPkg,
        },
        // Infrastructure — cloud, containers, databases, SDKs
        Tool {
            id: String::new(),
            name: "Flutter SDK".into(),
            binary: "flutter".into(),
            package: "flutter".into(),
            category: Category::Infra,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Docker Desktop".into(),
            binary: "docker".into(),
            package: "docker".into(),
            category: Category::Infra,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "AWS CLI".into(),
            binary: "aws".into(),
            package: "awscli".into(),
            category: Category::Infra,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Google Cloud".into(),
            binary: "gcloud".into(),
            package: "google-cloud-sdk".into(),
            category: Category::Infra,
            method: UpdateMethod::MacApp,
        },
        Tool {
            id: String::new(),
            name: "Heroku CLI".into(),
            binary: "heroku".into(),
            package: "heroku".into(),
            category: Category::Infra,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Ngrok".into(),
            binary: "ngrok".into(),
            package: "ngrok".into(),
            category: Category::Infra,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Convex".into(),
            binary: "convex".into(),
            package: "convex".into(),
            category: Category::Infra,
            method: UpdateMethod::NpmPkg,
        },
        Tool {
            id: String::new(),
            name: "PostgreSQL 16".into(),
            binary: "psql".into(),
            package: "postgresql@16".into(),
            category: Category::Infra,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "SQLite".into(),
            binary: "sqlite3".into(),
            package: "sqlite".into(),
            category: Category::Infra,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Redis".into(),
            binary: "redis-cli".into(),
            package: "redis".into(),
            category: Category::Infra,
            method: UpdateMethod::BrewPkg,
        },
        // Runtimes — grouped by ecosystem
        // JavaScript/TypeScript
        Tool {
            id: String::new(),
            name: "Node.js".into(),
            binary: "node".into(),
            package: "node".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Bun".into(),
            binary: "bun".into(),
            package: "bun".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Deno".into(),
            binary: "deno".into(),
            package: "deno".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        // Python
        Tool {
            id: String::new(),
            name: "Python 3".into(),
            binary: "python3".into(),
            package: "python@3.13".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "pyenv".into(),
            binary: "pyenv".into(),
            package: "pyenv".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "uv".into(),
            binary: "uv".into(),
            package: "uv".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        // Ruby
        Tool {
            id: String::new(),
            name: "Ruby".into(),
            binary: "ruby".into(),
            package: "ruby".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "rbenv".into(),
            binary: "rbenv".into(),
            package: "rbenv".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        // Go / Rust
        Tool {
            id: String::new(),
            name: "Go".into(),
            binary: "go".into(),
            package: "go".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Rust (rustup)".into(),
            binary: "rustup".into(),
            package: "rustup".into(),
            category: Category::Runtime,
            method: UpdateMethod::BrewPkg,
        },
        // Utilities — terminal multiplexers, system tools
        Tool {
            id: String::new(),
            name: "Zellij".into(),
            binary: "zellij".into(),
            package: "zellij".into(),
            category: Category::Utils,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Tmux".into(),
            binary: "tmux".into(),
            package: "tmux".into(),
            category: Category::Utils,
            method: UpdateMethod::BrewPkg,
        },
        Tool {
            id: String::new(),
            name: "Mole".into(),
            binary: "mole".into(),
            package: "mole".into(),
            category: Category::Utils,
            method: UpdateMethod::BrewPkg,
        },
    ];

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
