#!/usr/bin/env bash
# Version management for spark — keeps Cargo.toml and the four npm manifests in sync.
#
# Usage:
#   scripts/version.sh check          validate the 5 version fields match; exit 1 on mismatch
#   scripts/version.sh current        print the current version (from Cargo.toml)
#   scripts/version.sh bump <semver>  update all 5 manifests + regenerate Cargo.lock

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

CARGO_TOML="Cargo.toml"
NPM_BASE="npm/package.json"
PLATFORM_DIRS=("darwin-arm64" "darwin-x64" "linux-x64")

# ── helpers ────────────────────────────────────────────────────────────────────

read_cargo_version() {
    # First `^version = "X.Y.Z"` in Cargo.toml (under [package])
    grep -m1 '^version = ' "$CARGO_TOML" | cut -d'"' -f2
}

read_npm_version() {
    node -p "require('./$1').version"
}

# ── subcommands ────────────────────────────────────────────────────────────────

cmd_current() {
    read_cargo_version
}

cmd_check() {
    local cargo_ver
    cargo_ver="$(read_cargo_version)"
    local fail=0

    printf '  %-45s %s\n' "$CARGO_TOML" "$cargo_ver"

    local npm_ver
    npm_ver="$(read_npm_version "$NPM_BASE")"
    printf '  %-45s %s' "$NPM_BASE" "$npm_ver"
    if [ "$npm_ver" != "$cargo_ver" ]; then
        printf '  \x1b[31m✗ mismatch\x1b[0m\n'
        fail=1
    else
        printf '\n'
    fi

    # Also check optionalDependencies pin inside npm/package.json
    local opt_deps
    opt_deps="$(node -e "
        const p = require('./$NPM_BASE');
        if (!p.optionalDependencies) process.exit(0);
        const versions = Object.values(p.optionalDependencies);
        const uniq = [...new Set(versions)];
        if (uniq.length === 1) console.log(uniq[0]);
        else console.log('MIXED:' + uniq.join(','));
    ")"
    if [ -n "$opt_deps" ]; then
        printf '  %-45s %s' "$NPM_BASE optionalDependencies" "$opt_deps"
        if [ "$opt_deps" != "$cargo_ver" ]; then
            printf '  \x1b[31m✗ mismatch\x1b[0m\n'
            fail=1
        else
            printf '\n'
        fi
    fi

    for dir in "${PLATFORM_DIRS[@]}"; do
        local path="npm/platforms/$dir/package.json"
        local v
        v="$(read_npm_version "$path")"
        printf '  %-45s %s' "$path" "$v"
        if [ "$v" != "$cargo_ver" ]; then
            printf '  \x1b[31m✗ mismatch\x1b[0m\n'
            fail=1
        else
            printf '\n'
        fi
    done

    if [ $fail -ne 0 ]; then
        printf '\n  \x1b[31mVersion mismatch detected.\x1b[0m Run: scripts/version.sh bump <version>\n'
        exit 1
    fi
}

cmd_bump() {
    local new_version="$1"

    if ! [[ "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
        echo "  Invalid semver: $new_version" >&2
        echo "  Expected: X.Y.Z or X.Y.Z-prerelease" >&2
        exit 1
    fi

    local current
    current="$(read_cargo_version)"
    if [ "$new_version" = "$current" ]; then
        echo "  Already at $new_version — nothing to do."
        return 0
    fi

    echo "  Bumping $current → $new_version"

    # 1. Cargo.toml — the ONLY `^version = "..."` line at column 0 is the package
    # version. Dependencies with `version` are always inside tables (indented or
    # inline `= { version = "..." }`), so a plain unanchored replace is safe.
    # Portable sed (works on BSD/macOS and GNU/Linux): pass the backup suffix
    # with a separator, then delete the backup file.
    sed -i.bak -E "s/^version = \"[^\"]*\"/version = \"$new_version\"/" "$CARGO_TOML"
    rm "$CARGO_TOML.bak"

    # 2. npm manifests — bump version + optionalDependencies pin
    node -e "
        const fs = require('fs');
        const v = '$new_version';
        const bump = (path) => {
            const pkg = JSON.parse(fs.readFileSync(path, 'utf8'));
            pkg.version = v;
            if (pkg.optionalDependencies) {
                for (const k of Object.keys(pkg.optionalDependencies)) {
                    pkg.optionalDependencies[k] = v;
                }
            }
            fs.writeFileSync(path, JSON.stringify(pkg, null, 2) + '\n');
        };
        bump('$NPM_BASE');
        for (const d of ['darwin-arm64', 'darwin-x64', 'linux-x64']) {
            bump(\`npm/platforms/\${d}/package.json\`);
        }
    "

    # 3. Regenerate Cargo.lock
    cargo check --quiet 2>/dev/null || cargo build --quiet

    echo
    echo "  Files updated:"
    git diff --stat Cargo.toml Cargo.lock npm/package.json npm/platforms/ 2>/dev/null | sed 's/^/    /'
    echo
    echo "  Next steps:"
    echo "    git add Cargo.toml Cargo.lock npm/"
    echo "    git commit -m \"Release: v$new_version\""
    echo "    git tag v$new_version"
    echo "    git push origin main --follow-tags"
}

# ── main ───────────────────────────────────────────────────────────────────────

case "${1:-}" in
    check)   cmd_check ;;
    current) cmd_current ;;
    bump)
        if [ -z "${2:-}" ]; then
            echo "Usage: scripts/version.sh bump <X.Y.Z>" >&2
            exit 1
        fi
        cmd_bump "$2"
        ;;
    *)
        cat <<USAGE
Usage: scripts/version.sh <command> [args]

Commands:
  check          validate the 5 version fields match; exit 1 on mismatch
  current        print the current version (from Cargo.toml)
  bump <semver>  update all 5 manifests + regenerate Cargo.lock

Examples:
  scripts/version.sh check
  scripts/version.sh bump 0.6.0
USAGE
        exit 1
        ;;
esac
