//! Platform-tool catalog: productivity, infrastructure, runtimes, utilities.

use super::super::types::*;

pub(super) fn tools() -> Vec<Tool> {
    let mut tools = Vec::new();

    // Productivity — dev workflow tools
    tools.extend([
        mk(
            "FFmpeg",
            "ffmpeg",
            "ffmpeg",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk("JQ", "jq", "jq", Category::Prod, UpdateMethod::BrewPkg),
        mk("FZF", "fzf", "fzf", Category::Prod, UpdateMethod::BrewPkg),
        mk(
            "Ripgrep",
            "rg",
            "ripgrep",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "LazyGit",
            "lazygit",
            "lazygit",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Delta",
            "delta",
            "git-delta",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Starship",
            "starship",
            "starship",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Pre-commit",
            "pre-commit",
            "pre-commit",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Ruff",
            "ruff",
            "ruff",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Direnv",
            "direnv",
            "direnv",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Watchman",
            "watchman",
            "watchman",
            Category::Prod,
            UpdateMethod::BrewPkg,
        ),
    ]);

    // Infrastructure — cloud, containers, databases, SDKs
    tools.extend([
        mk(
            "Flutter SDK",
            "flutter",
            "flutter",
            Category::Infra,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Docker Desktop",
            "docker",
            "docker",
            Category::Infra,
            UpdateMethod::MacApp,
        ),
        mk(
            "AWS CLI",
            "aws",
            "awscli",
            Category::Infra,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Google Cloud",
            "gcloud",
            "google-cloud-sdk",
            Category::Infra,
            UpdateMethod::MacApp,
        ),
        mk(
            "Heroku CLI",
            "heroku",
            "heroku",
            Category::Infra,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Ngrok",
            "ngrok",
            "ngrok",
            Category::Infra,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Convex",
            "convex",
            "convex",
            Category::Infra,
            UpdateMethod::NpmPkg,
        ),
        mk(
            "PostgreSQL 16",
            "psql",
            "postgresql@16",
            Category::Infra,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "SQLite",
            "sqlite3",
            "sqlite",
            Category::Infra,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Redis",
            "redis-cli",
            "redis",
            Category::Infra,
            UpdateMethod::BrewPkg,
        ),
    ]);

    // Runtimes — grouped by ecosystem
    // JS/TS
    tools.extend([
        mk(
            "Node.js",
            "node",
            "node",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Bun",
            "bun",
            "bun",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Deno",
            "deno",
            "deno",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
    ]);
    // Python
    tools.extend([
        mk(
            "Python 3",
            "python3",
            "python@3.13",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "pyenv",
            "pyenv",
            "pyenv",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
        mk("uv", "uv", "uv", Category::Runtime, UpdateMethod::BrewPkg),
    ]);
    // Ruby
    tools.extend([
        mk(
            "Ruby",
            "ruby",
            "ruby",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "rbenv",
            "rbenv",
            "rbenv",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
    ]);
    // Go / Rust
    tools.extend([
        mk("Go", "go", "go", Category::Runtime, UpdateMethod::BrewPkg),
        mk(
            "Rust (rustup)",
            "rustup",
            "rustup",
            Category::Runtime,
            UpdateMethod::BrewPkg,
        ),
    ]);

    // Utilities — terminal multiplexers, system tools
    tools.extend([
        mk(
            "Zellij",
            "zellij",
            "zellij",
            Category::Utils,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Tmux",
            "tmux",
            "tmux",
            Category::Utils,
            UpdateMethod::BrewPkg,
        ),
        mk(
            "Mole",
            "mole",
            "mole",
            Category::Utils,
            UpdateMethod::BrewPkg,
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
