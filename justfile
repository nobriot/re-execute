# List available recipes
help:
    @just --list

# Clean all build artifacts
clean:
    cargo clean

# Set up the development environment
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    ok="\033[0;32m✓\033[0m"
    err="\033[0;31m✗\033[0m"
    info="\033[0;34m→\033[0m"

    # --- rustup & cargo ---
    if ! command -v rustup &>/dev/null; then
        echo -e "$err rustup not found — installing Rust"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    else
        echo -e "$ok rustup $(rustup --version 2>/dev/null | head -1 | awk '{print $2}')"
    fi

    if ! command -v cargo &>/dev/null; then
        echo -e "$err cargo not found"
        exit 1
    fi
    echo -e "$ok cargo $(cargo --version | awk '{print $2}')"

    # --- stable toolchain ---
    for toolchain in stable nightly; do
        if ! rustup toolchain list | grep -q "^${toolchain}"; then
            echo -e "$info Installing $toolchain toolchain..."
            rustup toolchain install $toolchain
        fi
    done
    echo -e "$ok $toolchain toolchain"

    # --- components: clippy, rustfmt, rust-analyzer, rust-src ---
    for comp in clippy rustfmt rust-analyzer rust-src; do
        if ! rustup component list --installed | grep -q "^${comp}"; then
            echo -e "$info Installing component $comp..."
            rustup component add "$comp"
        fi
        echo -e "$ok $comp"
    done

    # --- cargo-deny ---
    if ! command -v cargo-deny &>/dev/null; then
        echo -e "$info Installing cargo-deny..."
        cargo install cargo-deny --locked
    fi
    echo -e "$ok cargo-deny $(cargo deny --version 2>/dev/null | awk '{print $2}')"

    # --- prek ---
    if ! command -v prek &>/dev/null; then
        echo -e "$info Installing prek..."
        cargo install prek
    fi
    echo -e "$ok prek $(prek --version 2>/dev/null | awk '{print $2}')"

    # --- install pre-commit hooks ---
    echo -e "$info Installing pre-commit hooks..."
    prek install

    # --- verify build ---
    echo ""
    echo -e "$info Building..."
    cargo build
    echo -e "$ok project builds"
    echo ""
    echo "Dev environment ready. Run 'just' to see available recipes."

# Run tests
test:
    cargo test

# Lint: clippy + format
lint:
    cargo clippy --all-targets --all-features -- -D warnings
    cargo +nightly fmt --all

# Check lint without modifying files
lint-check:
    cargo clippy --all-targets --all-features -- -D warnings
    cargo +nightly fmt --all -- --check --color always

# Check dependencies with cargo-deny
deny:
    cargo deny check

# Build release binary
release:
    cargo build --release

# Run rex (pass arguments after --)
run *args:
    cargo run -- {{args}}
