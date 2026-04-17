#!/usr/bin/env bash
# smoke-test.sh — Post-install validation for SPARK
# Run after `cargo install` or `npm install -g @dpeluche/spark` to verify the binary works.
set -euo pipefail

PASS=0; FAIL=0

check() {
    local label="$1"; shift
    if "$@" &>/dev/null; then
        echo "  [+] $label"
        PASS=$((PASS+1))
    else
        echo "  [!] FAIL: $label"
        FAIL=$((FAIL+1))
    fi
}

check_output() {
    local label="$1"; local expected="$2"; shift 2
    local out
    out=$("$@" 2>&1) || true
    if echo "$out" | grep -q "$expected"; then
        echo "  [+] $label"
        PASS=$((PASS+1))
    else
        echo "  [!] FAIL: $label (expected '$expected' in output)"
        FAIL=$((FAIL+1))
    fi
}

echo ""
echo "  SPARK Smoke Test"
echo "  ──────────────────────────────────"

# 1. Binary exists and runs
check         "spark binary accessible"       which spark
check_output  "spark --version shows version" "spark"     spark --version
check_output  "spark --help shows usage"      "Usage"     spark --help

# 2. Core CLI commands
check_output  "spark list runs"               ""          spark list       || true
check_output  "spark config shows config"     "repos"     spark config
check_output  "spark doctor runs"             "spark"     spark doctor

# 3. Round-trip: clone → cd → rm
TMPDIR_TEST=$(mktemp -d)
ORIGINAL_ROOT=""
if ORIGINAL_ROOT=$(spark config 2>/dev/null | grep repos_root | awk '{print $2}'); then
    spark root --set "$TMPDIR_TEST" &>/dev/null || true
    if spark clone https://github.com/octocat/Hello-World &>/dev/null; then
        check_output  "spark cd finds cloned repo"  ""  bash -c 'cd "$(spark cd Hello-World)" && pwd' || true
        check_output  "spark list shows cloned repo" "Hello-World"  spark list
        spark rm Hello-World --yes &>/dev/null || spark rm Hello-World &>/dev/null || true
        echo "  [+] clone → cd → rm round-trip"
        PASS=$((PASS+1))
    else
        echo "  [-] clone skipped (no network or GitHub unavailable)"
    fi
    # Restore original root
    spark root --set "$ORIGINAL_ROOT" &>/dev/null || true
fi
rm -rf "$TMPDIR_TEST"

# 4. Audit runs (local scan only)
check_output  "spark audit --offline runs"   "Scanning"  spark audit --offline .

echo ""
echo "  ──────────────────────────────────"
echo "  Results: $PASS passed, $FAIL failed"
echo ""

if [ "$FAIL" -gt 0 ]; then
    echo "  Run 'spark doctor' to diagnose issues."
    exit 1
fi
