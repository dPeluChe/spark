//! Dev-tool catalog: system, code (AI dev), IDEs, terminals.

use super::super::types::*;

pub(super) fn tools() -> Vec<Tool> {
    let mut tools = Vec::new();

    // System — package managers, shell, version control
    tools.extend([
        mk(
            "Homebrew",
            "brew",
            "homebrew",
            Category::Sys,
            UpdateMethod::BrewPkg,
        ),
        mk("NPM", "npm", "npm", Category::Sys, UpdateMethod::NpmSys),
        mk("pnpm", "pnpm", "pnpm", Category::Sys, UpdateMethod::NpmPkg),
        mk("Yarn", "yarn", "yarn", Category::Sys, UpdateMethod::NpmPkg),
        mk("Git", "git", "git", Category::Sys, UpdateMethod::BrewPkg),
        mk(
            "GitHub CLI",
            "gh",
            "gh",
            Category::Sys,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Oh My Zsh",
            "omz",
            "oh-my-zsh",
            Category::Sys,
            UpdateMethod::Omz,
        ),
        mk("Bash", "bash", "bash", Category::Sys, UpdateMethod::BrewPkg),
    ]);

    // AI Development
    tools.extend([
        mk(
            "Claude CLI",
            "claude",
            "@anthropic-ai/claude-code",
            Category::Code,
            UpdateMethod::Claude,
        ),
        mk(
            "Droid CLI",
            "droid",
            "factory-cli",
            Category::Code,
            UpdateMethod::Droid,
        ),
        mk(
            "Gemini CLI",
            "gemini",
            "@google/gemini-cli",
            Category::Code,
            UpdateMethod::NpmPkg,
        ),
        mk(
            "OpenCode",
            "opencode",
            "opencode-ai",
            Category::Code,
            UpdateMethod::Opencode,
        ),
        mk(
            "Codex CLI",
            "codex",
            "@openai/codex",
            Category::Code,
            UpdateMethod::NpmPkg,
        ),
        mk(
            "Crush CLI",
            "crush",
            "crush",
            Category::Code,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Toad CLI",
            "toad",
            "batrachian-toad",
            Category::Code,
            UpdateMethod::Toad,
        ),
        mk(
            "Ollama",
            "ollama",
            "ollama",
            Category::Code,
            UpdateMethod::Manual,
        ),
    ]);

    // IDEs & Editors
    tools.extend([
        mk(
            "VS Code",
            "code",
            "visual-studio-code",
            Category::Ide,
            UpdateMethod::MacApp,
        ),
        mk(
            "Cursor IDE",
            "cursor",
            "cursor",
            Category::Ide,
            UpdateMethod::MacApp,
        ),
        mk(
            "Zed Editor",
            "zed",
            "zed",
            Category::Ide,
            UpdateMethod::MacApp,
        ),
        mk(
            "Windsurf",
            "windsurf",
            "windsurf",
            Category::Ide,
            UpdateMethod::MacApp,
        ),
        mk(
            "Antigravity",
            "antigravity",
            "antigravity",
            Category::Ide,
            UpdateMethod::Manual,
        ),
    ]);

    // Terminals
    tools.extend([
        mk(
            "iTerm2",
            "iterm",
            "iterm2",
            Category::Term,
            UpdateMethod::MacApp,
        ),
        mk(
            "Ghostty",
            "ghostty",
            "ghostty",
            Category::Term,
            UpdateMethod::MacApp,
        ),
        mk(
            "Warp Terminal",
            "warp",
            "warp",
            Category::Term,
            UpdateMethod::MacApp,
        ),
    ]);

    tools
}

fn mk(name: &str, binary: &str, package: &str, category: Category, method: UpdateMethod) -> Tool {
    Tool {
        id: String::new(),
        name: name.into(),
        binary: binary.into(),
        package: package.into(),
        category,
        method,
    }
}
