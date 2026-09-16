# Adding New Tools to SPARK

## Quick Start

Adding a new tool requires modifying **only 1 file** in most cases:

1. Open `src/core/inventory/dev.rs` (system/AI/IDE/terminal tools) or `src/core/inventory/platform.rs` (productivity/infra/runtime/utility tools)
2. Add a `mk(...)` entry to the `tools()` vector
3. Rebuild: `cargo build --release`

SPARK will automatically handle version detection for most standard CLI tools.

---

## Step-by-Step Guide

### 1. Identify Tool Metadata

| Field | Description | Example |
|-------|-------------|---------|
| **name** | Display name | `"Prettier"` |
| **binary** | Command name | `"prettier"` |
| **package** | Install package name | `"prettier"` |
| **category** | Logical grouping | `Category::Prod` |
| **method** | How to update it | `UpdateMethod::NpmPkg` |

### 2. Choose the Right Category

```rust
Category::Code     // AI Development tools (Claude, Droid, etc.)
Category::Term     // Terminal emulators (iTerm, Ghostty, etc.)
Category::Ide      // Code editors (VS Code, Cursor, etc.)
Category::Prod     // Productivity CLI tools (jq, fzf, ripgrep, etc.)
Category::Infra    // Infrastructure tools (Docker, Kubernetes, etc.)
Category::Utils    // System utilities (Git, Tmux, Oh My Zsh, etc.)
Category::Runtime  // Programming runtimes (Node, Python, Go, etc.) -- triggers safety modal
Category::Sys      // Package managers (Homebrew, NPM, etc.)
```

### 3. Choose the Right Update Method

```rust
UpdateMethod::BrewPkg   // Homebrew formula: brew upgrade <package>
UpdateMethod::NpmPkg    // npm package: npm install -g <package>@latest
UpdateMethod::NpmSys    // npm system update
UpdateMethod::MacApp    // macOS app via brew cask
UpdateMethod::Claude    // Claude CLI specific
UpdateMethod::Droid     // Droid CLI specific
UpdateMethod::Toad      // Toad CLI specific
UpdateMethod::Opencode  // OpenCode specific
UpdateMethod::Omz       // Oh My Zsh (git-based)
UpdateMethod::Manual    // Requires manual intervention
```

### 4. Add to Inventory

Edit `src/core/inventory/dev.rs` or `platform.rs` (by category):

```rust
mk("Prettier", "prettier", "prettier", Category::Prod, UpdateMethod::NpmPkg),
```

`mk(name, binary, package, category, method)` builds the `Tool` — `id` is auto-assigned.

That's it! SPARK will auto-assign an ID (`S-XX`) and handle version detection.

---

## Advanced Customization

### Custom Version Detection

If the tool has non-standard version output, add custom parsing to `src/updater/version.rs`:

```rust
// In parse_tool_version()
"yourtool" => {
    // Custom parsing logic
    if let Some(ver) = output.split(" - ").nth(1) {
        return clean_version(ver);
    }
}
```

### macOS App Detection

For macOS `.app` bundles, add the path to `src/updater/detector.rs`:

```rust
// In get_mac_app_version()
"yourtool" => "/Applications/YourTool.app",
```

### Changelog URL

Add to `src/core/changelogs.rs`:

```rust
"Your Tool" => "https://github.com/org/tool/releases",
```

If not added, SPARK uses heuristic fallbacks based on the update method.

---

## Testing Your Addition

```bash
# Run tests to ensure inventory is valid
cargo test

# Build and run
cargo run
```

Navigate to your tool's category and verify:
- Tool appears in the list
- Version is detected correctly
- Category is correct

---

## Next Steps

- See `docs/dev/ARCHITECTURE.md` for code structure
- See `docs/dev/WORKFLOWS.md` for user interaction flows
