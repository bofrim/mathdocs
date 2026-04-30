#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VSCODE_DIR="$ROOT_DIR/editors/vscode"
EXTENSION_ID="mathdocs.mathdocs-vscode"
CURSOR_CLI="${CURSOR_CLI:-}"

run_cursor() {
  if [ -n "$CURSOR_CLI" ]; then
    if command -v "$CURSOR_CLI" >/dev/null 2>&1; then
      "$CURSOR_CLI" "$@"
      return
    fi

    local cmd="$CURSOR_CLI"
    local arg
    for arg in "$@"; do
      cmd="$cmd $(printf '%q' "$arg")"
    done
    bash -lc "$cmd"
    return
  fi

  if command -v cursor >/dev/null 2>&1; then
    cursor "$@"
    return
  fi

  local cursor_bin="/Applications/Cursor.app/Contents/MacOS/Cursor"
  local cursor_cli_js="/Applications/Cursor.app/Contents/Resources/app/out/cli.js"
  if [ -x "$cursor_bin" ] && [ -f "$cursor_cli_js" ]; then
    ELECTRON_RUN_AS_NODE=1 "$cursor_bin" "$cursor_cli_js" "$@"
    return
  fi

  echo "error: Cursor CLI was not found." >&2
  echo "Install Cursor's command-line helper, or set CURSOR_CLI to an executable path or full CLI command." >&2
  exit 1
}

run_cursor --version >/dev/null 2>&1 || {
  echo "error: Cursor CLI was found but did not run successfully." >&2
  exit 1
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo was not found on PATH." >&2
  exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "error: npm was not found on PATH." >&2
  exit 1
fi

cd "$ROOT_DIR"
if [ ! -x "$ROOT_DIR/.venv/bin/python" ]; then
  echo "Creating Python virtual environment..."
  python3 -m venv "$ROOT_DIR/.venv"
fi

echo "Installing local Python mathdocs package..."
"$ROOT_DIR/.venv/bin/python" -m pip install -e "$ROOT_DIR/python" numpy

echo "Building mathdocs-lsp..."
cargo build -p mathdocs_lsp

cd "$VSCODE_DIR"
if [ ! -d node_modules ]; then
  echo "Installing VS Code extension dependencies..."
  npm ci
fi

echo "Compiling VS Code extension..."
npm run compile

echo "Packaging VSIX..."
npm run package

VSIX_PATH="$(find "$VSCODE_DIR" -maxdepth 1 -name 'mathdocs-vscode-*.vsix' -type f -print | sort | tail -n 1)"
if [ -z "$VSIX_PATH" ]; then
  echo "error: no VSIX package was produced." >&2
  exit 1
fi

echo "Installing $VSIX_PATH into Cursor..."
run_cursor --install-extension "$VSIX_PATH" --force

echo "Verifying install..."
if run_cursor --list-extensions | grep -qx "$EXTENSION_ID"; then
  echo "Installed $EXTENSION_ID successfully."
else
  echo "warning: install command completed, but $EXTENSION_ID was not listed by Cursor." >&2
  exit 1
fi
