#!/bin/bash
set -e

echo ""
echo "  _____ ____  ___  ____  __ __"
echo " / ___// __ \\/   |/ __ \\/ //_/"
echo " \\__ \\/ /_/ / /| / /_/ / ,<"
echo "___/ / ____/ ___ / _, _/ /| |"
echo "/____/_/   /_/  |/_/ |_/_/ |_|"
echo ""
echo "  Developer Operations Platform"
echo ""

# Check for Rust toolchain
if ! command -v cargo &> /dev/null; then
    echo "  Rust toolchain not found. Install from https://rustup.rs"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build
echo "  Building spark..."
cargo build --release 2>&1 | tail -1

# Install binary
INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"
cp target/release/spark "$INSTALL_DIR/spark"
chmod +x "$INSTALL_DIR/spark"
echo "  Installed to $INSTALL_DIR/spark"

# Check PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "  Add to PATH (add to your shell rc file):"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# Generate shell completions
SHELL_NAME=$(basename "$SHELL")
case "$SHELL_NAME" in
    zsh)
        COMP_DIR="${HOME}/.zsh/completions"
        mkdir -p "$COMP_DIR"
        "$INSTALL_DIR/spark" completions zsh > "$COMP_DIR/_spark"
        echo "  Installed zsh completions to $COMP_DIR/_spark"
        ;;
    bash)
        COMP_DIR="${HOME}/.local/share/bash-completion/completions"
        mkdir -p "$COMP_DIR"
        "$INSTALL_DIR/spark" completions bash > "$COMP_DIR/spark"
        echo "  Installed bash completions"
        ;;
    fish)
        COMP_DIR="${HOME}/.config/fish/completions"
        mkdir -p "$COMP_DIR"
        "$INSTALL_DIR/spark" completions fish > "$COMP_DIR/spark.fish"
        echo "  Installed fish completions"
        ;;
esac

echo ""
echo "  Run 'spark init' to complete setup."
echo "  Run 'spark' to start the TUI."
echo ""
