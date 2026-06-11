.PHONY: check check-fast check-full test lint fmt build release clean setup-hooks help

# ── Default target ──────────────────────────────────────────────────────────

help:
	@echo "mcp-filesystem-rs"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Quality gates:"
	@echo "  check       Run fmt + clippy + test + build (same as pre-commit)"
	@echo "  check-fast  Run fmt + clippy only (no test, no build)"
	@echo "  check-full  Run everything including pedantic/nursery/cargo lints"
	@echo ""
	@echo "Individual:"
	@echo "  fmt         Run cargo fmt"
	@echo "  lint        Run cargo clippy (standard)"
	@echo "  lint-full   Run cargo clippy (all + pedantic + nursery + cargo)"
	@echo "  test        Run cargo test"
	@echo "  build       Run cargo build (debug)"
	@echo "  release     Run cargo build --release"
	@echo ""
	@echo "Setup:"
	@echo "  setup-hooks  Install git pre-commit hook from .githooks/"

# ── Quality gates ──────────────────────────────────────────────────────────

check: fmt lint test build
	@echo "All checks passed."

check-fast: fmt lint
	@echo "Fast checks passed."

check-full: fmt lint-full test build
	@echo "Full checks passed."

# ── Individual checks ──────────────────────────────────────────────────────

fmt:
	@echo "══ cargo fmt ══"
	cargo fmt --check

lint:
	@echo "══ cargo clippy ══"
	cargo clippy --all-targets --all-features -- -D warnings

lint-full:
	@echo "══ cargo clippy (full) ══"
	cargo clippy --all-targets --all-features -- \
		-D warnings \
		-W clippy::all \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo

test:
	@echo "══ cargo test ══"
	cargo test

build:
	@echo "══ cargo build ══"
	cargo build

release:
	@echo "══ cargo build --release ══"
	cargo build --release

# ── Setup ──────────────────────────────────────────────────────────────────

setup-hooks:
	@mkdir -p .githooks
	@ln -sf ../../.githooks/pre-commit .git/hooks/pre-commit
	@chmod +x .githooks/pre-commit
	@echo "pre-commit hook installed → .git/hooks/pre-commit → .githooks/pre-commit"

# ── Helpers ────────────────────────────────────────────────────────────────

clean:
	cargo clean
