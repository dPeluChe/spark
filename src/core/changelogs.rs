use super::types::{Tool, UpdateMethod};

/// Returns the changelog URL for a given tool
pub fn get_changelog_url(tool: &Tool) -> Option<String> {
    // Direct mapping for known tools
    let url = match tool.name.as_str() {
        // AI Development
        "Claude CLI" => "https://www.npmjs.com/package/@anthropic-ai/claude-code?activeTab=versions",
        "Droid CLI" => "https://github.com/factory-ai/cli/releases",
        "Gemini CLI" => "https://github.com/google-gemini/gemini-cli/releases",
        "OpenCode" => "https://github.com/opencode-ai/opencode/releases",
        "Codex CLI" => "https://www.npmjs.com/package/@openai/codex?activeTab=versions",
        "Crush CLI" => "https://github.com/crush-sh/crush/releases",
        "Toad CLI" => "https://pypi.org/project/batrachian-toad/#history",

        // Terminals
        "iTerm2" => "https://iterm2.com/downloads.html",
        "Ghostty" => "https://github.com/ghostty-org/ghostty/releases",
        "Warp Terminal" => "https://docs.warp.dev/help/changelog",

        // IDEs
        "VS Code" => "https://code.visualstudio.com/updates",
        "Cursor IDE" => "https://cursor.sh/changelog",
        "Zed Editor" => "https://github.com/zed-industries/zed/releases",
        "Windsurf" => "https://codeium.com/windsurf/changelog",
        "Antigravity" => "https://antigravity.ai/changelog",

        // Productivity
        "JQ" => "https://github.com/jqlang/jq/releases",
        "FZF" => "https://github.com/junegunn/fzf/releases",
        "Ripgrep" => "https://github.com/BurntSushi/ripgrep/releases",
        "Bat" => "https://github.com/sharkdp/bat/releases",
        "HTTPie" => "https://github.com/httpie/cli/releases",
        "LazyGit" => "https://github.com/jesseduffield/lazygit/releases",
        "TLDR" => "https://github.com/tldr-pages/tldr/releases",

        // Infrastructure
        "Docker Desktop" => "https://docs.docker.com/desktop/release-notes/",
        "Kubernetes CLI" => "https://github.com/kubernetes/kubectl/tags",
        "Helm" => "https://github.com/helm/helm/releases",
        "Terraform" => "https://github.com/hashicorp/terraform/releases",
        "AWS CLI" => "https://github.com/aws/aws-cli/tags",
        "Ngrok" => "https://ngrok.com/docs/agent/changelog/",

        // Utilities
        "Oh My Zsh" => "https://github.com/ohmyzsh/ohmyzsh/releases",
        "Zellij" => "https://github.com/zellij-org/zellij/releases",
        "Tmux" => "https://github.com/tmux/tmux/releases",
        "Git" => "https://github.com/git/git/releases",
        "Bash" => "https://git.savannah.gnu.org/cgit/bash.git/log/",
        "SQLite" => "https://sqlite.org/changes.html",
        "Watchman" => "https://github.com/facebook/watchman/releases",
        "Direnv" => "https://github.com/direnv/direnv/releases",
        "Heroku CLI" => "https://github.com/heroku/cli/releases",
        "Pre-commit" => "https://github.com/pre-commit/pre-commit/releases",

        // Runtimes
        "Node.js" => "https://github.com/nodejs/node/releases",
        "Go Lang" => "https://go.dev/doc/devel/release",
        "Python 3.13" => "https://docs.python.org/release/",
        "Ruby" => "https://www.ruby-lang.org/en/downloads/releases/",
        "PostgreSQL 16" => "https://www.postgresql.org/docs/release/",

        // System
        "Homebrew Core" => "https://github.com/Homebrew/brew/releases",
        "NPM Globals" => "https://github.com/npm/cli/releases",

        _ => {
            // Heuristic fallbacks
            if tool.package.contains("github.com") {
                return Some(format!("https://{}/releases", tool.package));
            }
            match tool.method {
                UpdateMethod::NpmPkg | UpdateMethod::NpmSys => {
                    return Some(format!(
                        "https://www.npmjs.com/package/{}?activeTab=versions",
                        tool.package
                    ));
                }
                UpdateMethod::BrewPkg => {
                    return Some(format!(
                        "https://formulae.brew.sh/formula/{}",
                        tool.package
                    ));
                }
                _ => return None,
            }
        }
    };

    Some(url.to_string())
}
