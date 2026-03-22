use super::types::*;

/// Returns the master list of all supported tools
pub fn get_inventory() -> Vec<Tool> {
    let mut tools = vec![
        // AI Development
        Tool { id: String::new(), name: "Claude CLI".into(), binary: "claude".into(), package: "@anthropic-ai/claude-code".into(), category: Category::Code, method: UpdateMethod::Claude, description: String::new() },
        Tool { id: String::new(), name: "Droid CLI".into(), binary: "droid".into(), package: "factory-cli".into(), category: Category::Code, method: UpdateMethod::Droid, description: String::new() },
        Tool { id: String::new(), name: "Gemini CLI".into(), binary: "gemini".into(), package: "@google/gemini-cli".into(), category: Category::Code, method: UpdateMethod::NpmPkg, description: String::new() },
        Tool { id: String::new(), name: "OpenCode".into(), binary: "opencode".into(), package: "opencode-ai".into(), category: Category::Code, method: UpdateMethod::Opencode, description: String::new() },
        Tool { id: String::new(), name: "Codex CLI".into(), binary: "codex".into(), package: "@openai/codex".into(), category: Category::Code, method: UpdateMethod::NpmPkg, description: String::new() },
        Tool { id: String::new(), name: "Crush CLI".into(), binary: "crush".into(), package: "crush".into(), category: Category::Code, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Toad CLI".into(), binary: "toad".into(), package: "batrachian-toad".into(), category: Category::Code, method: UpdateMethod::Toad, description: String::new() },
        Tool { id: String::new(), name: "Ollama".into(), binary: "ollama".into(), package: "ollama".into(), category: Category::Code, method: UpdateMethod::Manual, description: String::new() },

        // Terminal Emulators
        Tool { id: String::new(), name: "iTerm2".into(), binary: "iterm".into(), package: "iterm2".into(), category: Category::Term, method: UpdateMethod::MacApp, description: String::new() },
        Tool { id: String::new(), name: "Ghostty".into(), binary: "ghostty".into(), package: "ghostty".into(), category: Category::Term, method: UpdateMethod::MacApp, description: String::new() },
        Tool { id: String::new(), name: "Warp Terminal".into(), binary: "warp".into(), package: "warp".into(), category: Category::Term, method: UpdateMethod::MacApp, description: String::new() },

        // IDEs
        Tool { id: String::new(), name: "VS Code".into(), binary: "code".into(), package: "visual-studio-code".into(), category: Category::Ide, method: UpdateMethod::MacApp, description: String::new() },
        Tool { id: String::new(), name: "Cursor IDE".into(), binary: "cursor".into(), package: "cursor".into(), category: Category::Ide, method: UpdateMethod::MacApp, description: String::new() },
        Tool { id: String::new(), name: "Zed Editor".into(), binary: "zed".into(), package: "zed".into(), category: Category::Ide, method: UpdateMethod::MacApp, description: String::new() },
        Tool { id: String::new(), name: "Windsurf".into(), binary: "windsurf".into(), package: "windsurf".into(), category: Category::Ide, method: UpdateMethod::MacApp, description: String::new() },
        Tool { id: String::new(), name: "Antigravity".into(), binary: "antigravity".into(), package: "antigravity".into(), category: Category::Ide, method: UpdateMethod::Manual, description: String::new() },

        // Productivity
        Tool { id: String::new(), name: "JQ".into(), binary: "jq".into(), package: "jq".into(), category: Category::Prod, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "FZF".into(), binary: "fzf".into(), package: "fzf".into(), category: Category::Prod, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Ripgrep".into(), binary: "rg".into(), package: "ripgrep".into(), category: Category::Prod, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Bat".into(), binary: "bat".into(), package: "bat".into(), category: Category::Prod, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "HTTPie".into(), binary: "http".into(), package: "httpie".into(), category: Category::Prod, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "LazyGit".into(), binary: "lazygit".into(), package: "lazygit".into(), category: Category::Prod, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "TLDR".into(), binary: "tldr".into(), package: "tldr".into(), category: Category::Prod, method: UpdateMethod::BrewPkg, description: String::new() },

        // Infrastructure
        Tool { id: String::new(), name: "Docker Desktop".into(), binary: "docker".into(), package: "docker".into(), category: Category::Infra, method: UpdateMethod::MacApp, description: String::new() },
        Tool { id: String::new(), name: "Kubernetes CLI".into(), binary: "kubectl".into(), package: "kubernetes-cli".into(), category: Category::Infra, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Helm".into(), binary: "helm".into(), package: "helm".into(), category: Category::Infra, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Terraform".into(), binary: "terraform".into(), package: "terraform".into(), category: Category::Infra, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "AWS CLI".into(), binary: "aws".into(), package: "awscli".into(), category: Category::Infra, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Ngrok".into(), binary: "ngrok".into(), package: "ngrok".into(), category: Category::Infra, method: UpdateMethod::BrewPkg, description: String::new() },

        // Utilities
        Tool { id: String::new(), name: "Oh My Zsh".into(), binary: "omz".into(), package: "oh-my-zsh".into(), category: Category::Utils, method: UpdateMethod::Omz, description: String::new() },
        Tool { id: String::new(), name: "Zellij".into(), binary: "zellij".into(), package: "zellij".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Tmux".into(), binary: "tmux".into(), package: "tmux".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Git".into(), binary: "git".into(), package: "git".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Bash".into(), binary: "bash".into(), package: "bash".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "SQLite".into(), binary: "sqlite3".into(), package: "sqlite".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Watchman".into(), binary: "watchman".into(), package: "watchman".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Direnv".into(), binary: "direnv".into(), package: "direnv".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Heroku CLI".into(), binary: "heroku".into(), package: "heroku".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Pre-commit".into(), binary: "pre-commit".into(), package: "pre-commit".into(), category: Category::Utils, method: UpdateMethod::BrewPkg, description: String::new() },

        // Runtimes (High Risk)
        Tool { id: String::new(), name: "Node.js".into(), binary: "node".into(), package: "node".into(), category: Category::Runtime, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Python 3.13".into(), binary: "python3".into(), package: "python@3.13".into(), category: Category::Runtime, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Go Lang".into(), binary: "go".into(), package: "go".into(), category: Category::Runtime, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "Ruby".into(), binary: "ruby".into(), package: "ruby".into(), category: Category::Runtime, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "PostgreSQL 16".into(), binary: "psql".into(), package: "postgresql@16".into(), category: Category::Runtime, method: UpdateMethod::BrewPkg, description: String::new() },

        // System
        Tool { id: String::new(), name: "Homebrew Core".into(), binary: "brew".into(), package: "homebrew".into(), category: Category::Sys, method: UpdateMethod::BrewPkg, description: String::new() },
        Tool { id: String::new(), name: "NPM Globals".into(), binary: "npm".into(), package: "npm".into(), category: Category::Sys, method: UpdateMethod::NpmSys, description: String::new() },
    ];

    // Auto-assign IDs: S-01, S-02, etc.
    for (i, tool) in tools.iter_mut().enumerate() {
        tool.id = format!("S-{:02}", i + 1);
    }

    tools
}
