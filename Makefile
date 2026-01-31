# Vanbeach CLI Makefile
# Build, test, and release automation

BINARY_NAME := vanbeach
VERSION := $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
GITHUB_REPO := Zxela/beach-cli

# Build targets
TARGETS := x86_64-unknown-linux-gnu \
           x86_64-unknown-linux-musl \
           aarch64-unknown-linux-gnu \
           x86_64-apple-darwin \
           aarch64-apple-darwin

.PHONY: all build build-release test clean install uninstall \
        release release-build release-package release-upload \
        lint fmt check help

all: build

## Development

build: ## Build debug binary
	cargo build

build-release: ## Build optimized release binary
	cargo build --release

test: ## Run all tests
	cargo test

lint: ## Run clippy linter
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

check: ## Run all checks (fmt, lint, test)
	cargo fmt --check
	cargo clippy -- -D warnings
	cargo test

## Installation

install: build-release ## Install binary to ~/.local/bin
	@mkdir -p ~/.local/bin
	cp target/release/$(BINARY_NAME) ~/.local/bin/
	@echo "Installed $(BINARY_NAME) to ~/.local/bin/"
	@echo "Make sure ~/.local/bin is in your PATH"

install-system: build-release ## Install binary to /usr/local/bin (requires sudo)
	sudo cp target/release/$(BINARY_NAME) /usr/local/bin/
	@echo "Installed $(BINARY_NAME) to /usr/local/bin/"

uninstall: ## Remove binary from ~/.local/bin
	rm -f ~/.local/bin/$(BINARY_NAME)
	@echo "Uninstalled $(BINARY_NAME) from ~/.local/bin/"

uninstall-system: ## Remove binary from /usr/local/bin (requires sudo)
	sudo rm -f /usr/local/bin/$(BINARY_NAME)
	@echo "Uninstalled $(BINARY_NAME) from /usr/local/bin/"

## Release

release: check release-build release-package ## Full release: check, build all targets, package
	@echo "Release v$(VERSION) ready in dist/"

release-build: ## Build release binaries for all targets
	@mkdir -p dist
	@for target in $(TARGETS); do \
		echo "Building for $$target..."; \
		if command -v cross >/dev/null 2>&1; then \
			cross build --release --target $$target || echo "Skipped $$target (build failed)"; \
		else \
			cargo build --release --target $$target || echo "Skipped $$target (not installed)"; \
		fi; \
	done

release-build-local: ## Build release for current platform only
	cargo build --release
	@mkdir -p dist
	@cp target/release/$(BINARY_NAME) dist/$(BINARY_NAME)-$(VERSION)-$$(uname -s)-$$(uname -m)
	@echo "Built dist/$(BINARY_NAME)-$(VERSION)-$$(uname -s)-$$(uname -m)"

release-package: ## Package built binaries into tarballs
	@mkdir -p dist
	@for target in $(TARGETS); do \
		if [ -f "target/$$target/release/$(BINARY_NAME)" ]; then \
			echo "Packaging $$target..."; \
			tar -czf dist/$(BINARY_NAME)-$(VERSION)-$$target.tar.gz \
				-C target/$$target/release $(BINARY_NAME); \
		fi; \
	done
	@echo "Packages created in dist/"
	@ls -la dist/

release-upload: ## Upload release to GitHub (requires gh CLI)
	@if [ -z "$$(gh auth status 2>&1 | grep 'Logged in')" ]; then \
		echo "Error: Not logged in to GitHub CLI. Run 'gh auth login' first."; \
		exit 1; \
	fi
	gh release create v$(VERSION) dist/*.tar.gz \
		--title "v$(VERSION)" \
		--notes "Release v$(VERSION)" \
		--repo $(GITHUB_REPO)
	@echo "Release v$(VERSION) uploaded to GitHub"

release-draft: ## Create draft release on GitHub
	gh release create v$(VERSION) dist/*.tar.gz \
		--title "v$(VERSION)" \
		--notes "Release v$(VERSION)" \
		--draft \
		--repo $(GITHUB_REPO)
	@echo "Draft release v$(VERSION) created on GitHub"

## Tagging & Versioning

release-patch: ## Bump patch version, commit, push, and create release tag
	@make bump-patch
	@git add Cargo.toml
	@git commit -m "chore: bump version to $$(grep '^version' Cargo.toml | head -1 | cut -d'\"' -f2)"
	@git push
	@make tag

release-minor: ## Bump minor version, commit, push, and create release tag
	@make bump-minor
	@git add Cargo.toml
	@git commit -m "chore: bump version to $$(grep '^version' Cargo.toml | head -1 | cut -d'\"' -f2)"
	@git push
	@make tag

release-major: ## Bump major version, commit, push, and create release tag
	@make bump-major
	@git add Cargo.toml
	@git commit -m "chore: bump version to $$(grep '^version' Cargo.toml | head -1 | cut -d'\"' -f2)"
	@git push
	@make tag

tag: ## Create and push a git tag for current version (triggers GitHub release)
	@if git rev-parse "v$(VERSION)" >/dev/null 2>&1; then \
		echo "Error: Tag v$(VERSION) already exists"; \
		exit 1; \
	fi
	@echo "Creating tag v$(VERSION)..."
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	@echo "Pushing tag to origin..."
	git push origin "v$(VERSION)"
	@echo ""
	@echo "Tag v$(VERSION) pushed! GitHub Actions will build and create the release."
	@echo "Watch progress at: https://github.com/$(GITHUB_REPO)/actions"

tag-dry-run: ## Show what tag would be created (without creating it)
	@echo "Would create tag: v$(VERSION)"
	@echo "Current git status:"
	@git status --short

bump-patch: ## Bump patch version (0.1.0 -> 0.1.1)
	@NEW_VERSION=$$(echo $(VERSION) | awk -F. '{print $$1"."$$2"."$$3+1}'); \
	sed -i 's/^version = "$(VERSION)"/version = "'$$NEW_VERSION'"/' Cargo.toml; \
	echo "Bumped version: $(VERSION) -> $$NEW_VERSION"

bump-minor: ## Bump minor version (0.1.0 -> 0.2.0)
	@NEW_VERSION=$$(echo $(VERSION) | awk -F. '{print $$1"."$$2+1".0"}'); \
	sed -i 's/^version = "$(VERSION)"/version = "'$$NEW_VERSION'"/' Cargo.toml; \
	echo "Bumped version: $(VERSION) -> $$NEW_VERSION"

bump-major: ## Bump major version (0.1.0 -> 1.0.0)
	@NEW_VERSION=$$(echo $(VERSION) | awk -F. '{print $$1+1".0.0"}'); \
	sed -i 's/^version = "$(VERSION)"/version = "'$$NEW_VERSION'"/' Cargo.toml; \
	echo "Bumped version: $(VERSION) -> $$NEW_VERSION"

## Utilities

clean: ## Remove build artifacts
	cargo clean
	rm -rf dist/

version: ## Show current version
	@echo $(VERSION)

help: ## Show this help
	@echo "Vanbeach CLI v$(VERSION)"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'
