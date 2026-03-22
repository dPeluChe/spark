package core

import "strings"

// GetChangelogURL returns the specific changelog URL for a tool
func GetChangelogURL(t Tool) string {
	// 1. Direct Mapping for known tools (High Accuracy)
	// Keys must match the "DisplayName" in tools.conf exactly.
	repoMap := map[string]string{
		// AI Development
		"Claude CLI": "https://www.npmjs.com/package/@anthropic-ai/claude-code?activeTab=versions",
		"Droid CLI":  "https://github.com/factory-ai/cli/releases",           // Best guess for "factory-cli"
		"Gemini CLI": "https://github.com/google-gemini/gemini-cli/releases", // Confirmed Open Source
		"OpenCode":   "https://github.com/opencode-ai/opencode/releases",     // Assumed GitHub pattern
		"Codex CLI":  "https://www.npmjs.com/package/@openai/codex?activeTab=versions",
		"Crush CLI":  "https://github.com/crush-sh/crush/releases",        // Assumed GitHub pattern
		"Toad CLI":   "https://pypi.org/project/batrachian-toad/#history", // Python package convention

		// Terminals
		"iTerm2":        "https://iterm2.com/downloads.html",
		"Ghostty":       "https://github.com/ghostty-org/ghostty/releases",
		"Warp Terminal": "https://docs.warp.dev/help/changelog",

		// IDEs
		"VS Code":     "https://code.visualstudio.com/updates",
		"Cursor IDE":  "https://cursor.sh/changelog",
		"Zed Editor":  "https://github.com/zed-industries/zed/releases",
		"Windsurf":    "https://codeium.com/windsurf/changelog",
		"Antigravity": "https://antigravity.ai/changelog",

		// Productivity (Power Tools)
		"JQ (JSON Processor)":   "https://github.com/jqlang/jq/releases",
		"FZF (Fuzzy Finder)":    "https://github.com/junegunn/fzf/releases",
		"Ripgrep (Search)":      "https://github.com/BurntSushi/ripgrep/releases",
		"Bat (Better Cat)":      "https://github.com/sharkdp/bat/releases",
		"HTTPie (API Client)":   "https://github.com/httpie/cli/releases",
		"LazyGit":               "https://github.com/jesseduffield/lazygit/releases",
		"TLDR Pages":            "https://github.com/tldr-pages/tldr/releases",

		// Infrastructure
		"Docker Desktop": "https://docs.docker.com/desktop/release-notes/",
		"Kubernetes CLI": "https://github.com/kubernetes/kubectl/tags",
		"Helm Charts":    "https://github.com/helm/helm/releases",
		"Terraform":      "https://github.com/hashicorp/terraform/releases",
		"AWS CLI":        "https://github.com/aws/aws-cli/tags",
		"Ngrok Tunnel":   "https://ngrok.com/docs/agent/changelog/",

		// Utilities
		"Oh My Zsh":   "https://github.com/ohmyzsh/ohmyzsh/releases",
		"Zellij":      "https://github.com/zellij-org/zellij/releases",
		"Tmux":        "https://github.com/tmux/tmux/releases",
		"Git":         "https://github.com/git/git/releases",
		"Bash":        "https://git.savannah.gnu.org/cgit/bash.git/log/", // Official source
		"SQLite":      "https://sqlite.org/changes.html",
		"Watchman":    "https://github.com/facebook/watchman/releases",
		"Direnv":      "https://github.com/direnv/direnv/releases",
		"Heroku CLI":  "https://github.com/heroku/cli/releases",
		"Pre-commit":  "https://github.com/pre-commit/pre-commit/releases",

		// Runtimes
		"Node.js":       "https://github.com/nodejs/node/releases",
		"Go Lang":       "https://go.dev/doc/devel/release",
		"Python 3.13":   "https://docs.python.org/release/",
		"Ruby":          "https://www.ruby-lang.org/en/downloads/releases/",
		"PostgreSQL 16": "https://www.postgresql.org/docs/release/",

		// System
		"Homebrew Core": "https://github.com/Homebrew/brew/releases",
		"NPM Globals":   "https://github.com/npm/cli/releases",
	}

	// Return exact match if found
	if url, ok := repoMap[t.Name]; ok {
		return url
	}

	// 2. Heuristics for unknown tools
	if strings.Contains(t.Package, "github.com") {
		return "https://" + t.Package + "/releases"
	}

	if t.Method == MethodNpmPkg || t.Method == MethodNpmSys {
		return "https://www.npmjs.com/package/" + t.Package + "?activeTab=versions"
	}

	if t.Method == MethodBrewPkg {
		return "https://formulae.brew.sh/formula/" + t.Package
	}

	return "" // No link available
}