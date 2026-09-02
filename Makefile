SHELL := /bin/bash
.DEFAULT_GOAL := help

.PHONY: help setup dev lint format type-check test guard-no-shell rust-fmt rust-lint rust-test build icons ci-local mock-server

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-16s\033[0m %s\n",$$1,$$2}'

setup: ## Install toolchains + dependencies (Linux/macOS; use scripts/setup-windows.ps1 on Windows)
	@case "$$(uname -s)" in \
	  Darwin) bash scripts/setup-macos.sh ;; \
	  *) bash scripts/setup-linux.sh ;; \
	esac

dev: ## Run the app in development mode (plaintext key + local API URL on WSL/Linux)
	SYNAPLAN_DESKTOP_ALLOW_PLAINTEXT_KEY=$${SYNAPLAN_DESKTOP_ALLOW_PLAINTEXT_KEY:-1} \
	VITE_SYNAPLAN_DEV_URL=$${VITE_SYNAPLAN_DEV_URL:-http://localhost:8000} \
	npm run tauri dev

lint: ## ESLint + Prettier (JS/TS/Vue)
	npm run lint

format: ## Auto-fix formatting (JS/TS/Vue + Rust)
	npm run format
	cd src-tauri && cargo fmt --all

type-check: ## Type-check the frontend (vue-tsc)
	npm run type-check

test: ## Frontend unit tests (Vitest)
	npm run test

guard-no-shell: ## C12: fail if any client source constructs a shell
	bash scripts/no-shell-guard.sh

rust-fmt: ## Check Rust formatting
	cd src-tauri && cargo fmt --all --check

rust-lint: ## Clippy on the platform-independent core
	cd src-tauri && cargo clippy -p synaplan-core --all-targets -- -D warnings

rust-test: ## Rust unit tests (synaplan-core)
	cd src-tauri && cargo test -p synaplan-core

build: ## Build the frontend + a debug app binary (whole-workspace clippy included)
	npm run build
	cd src-tauri && cargo clippy --workspace --all-targets -- -D warnings
	cd src-tauri && cargo build

icons: ## Regenerate app icons from src-tauri/icons/source.png
	npx tauri icon src-tauri/icons/source.png

mock-server: ## Run the offline Synaplan mock server (http://localhost:8788)
	npm run mock-server

ci-local: lint type-check test guard-no-shell rust-fmt rust-lint rust-test build ## Full local gate (mirrors CI; green here => green CI)
	@echo "ci-local: all checks passed"
