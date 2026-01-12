# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The Heap is a keyboard-driven, AI-augmented native desktop email client built entirely in Rust. It uses gpui (from Zed) for GPU-accelerated UI, SQLite for local-first storage, and Candle for pure-Rust ML inference.

## Build Commands

```bash
# Development
cargo build                    # Debug build
cargo run                      # Run application
cargo check --all-features     # Type check without building

# Testing
cargo test --all-features --workspace    # Run all tests
cargo test <test_name>                   # Run specific test
cargo test -p heap                       # Test main crate only

# Quality (run before commits)
cargo fmt --check              # Check formatting
cargo clippy -- -D warnings    # Lint with warnings as errors

# Release
cargo build --release          # Optimized build
./script/bundle-mac            # Create macOS .app bundle
./script/bundle-mac --release  # Release bundle
./script/bundle-mac --sign     # Code-signed bundle

# Coverage
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
```

## Architecture

### Layered Structure

```
src/
├── ui/           # Presentation: gpui views and components
├── app/          # Application: state management, actions, events
├── domain/       # Domain: pure types (Email, Thread, Account, Label, Contact)
├── services/     # Business logic orchestration (14+ services)
├── providers/    # External integrations (Gmail, IMAP, OpenAI, Anthropic, Ollama)
├── storage/      # SQLite database and OS keychain
├── embedding/    # Semantic search via Candle (BERT/MiniLM)
└── config/       # Settings types
```

### Key Patterns

- **Domain types have no I/O**: `src/domain/` contains pure business logic
- **Trait-based providers**: `EmailProvider` and `LlmProvider` traits enable swappable backends
- **Services as orchestration**: Services bridge application and infrastructure layers
- **Storage queries separated**: `storage/queries/` modules isolate SQL from business logic
- **Strong type IDs**: `AccountId`, `ThreadId`, `EmailId`, etc. are newtype wrappers

### UI Framework (gpui)

- Components implement `Render` trait
- Actions defined via `actions!` macro for keyboard shortcuts
- Keybindings registered via `cx.bind_keys()`
- Tailwind-style API: `div().px_3().py_2().bg(color).child(...)`

## Code Style

- **MSRV**: 1.75
- **Max line width**: 100 characters
- **Cognitive complexity threshold**: 25
- **Edition**: 2021

## Git Hooks (lefthook)

Pre-commit runs format check and clippy. Pre-push runs tests.

## System Dependencies (Linux)

```bash
sudo apt-get install -y libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libwayland-dev libvulkan-dev libfreetype6-dev libfontconfig1-dev
```
