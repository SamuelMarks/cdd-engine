.PHONY: all help install_base install_deps docs build_docs build lint test coverage build_docker run_docker

DOCS_DIR ?= docs

all: help

help:
	@echo "Available commands:"
	@echo "  install_base   - Install language toolchains (Rust)"
	@echo "  install_deps   - Install project dependencies & WASM targets"
	@echo "  build_docs     - Build the API documentation"
	@echo "  docs           - Build and prepare documentation in $(DOCS_DIR)"
	@echo "  build          - Build the cdd-engine library"
	@echo "  lint           - Run cargo fmt and clippy with -D warnings"
	@echo "  test           - Run the test suite"
	@echo "  coverage       - Run test and documentation coverage verification"
	@echo "  build_docker   - Build Alpine and Debian Docker images"
	@echo "  run_docker     - Run verification inside Docker containers"

install_base:
	@echo "Checking for cargo..."
	@command -v cargo >/dev/null 2>&1 || { echo "Installing Rust..."; curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; }

install_deps: install_base
	@echo "Fetching cargo dependencies..."
	cargo fetch
	@echo "Acquiring WASM binaries for cdd-* ecosystems..."
	@chmod +x scripts/fetch_wasm.sh
	@./scripts/fetch_wasm.sh

build_docs:
	@echo "Generating Rust documentation into $(DOCS_DIR)..."
	cargo doc --no-deps --target-dir $(DOCS_DIR)_tmp
	@mkdir -p $(DOCS_DIR)
	@cp -r $(DOCS_DIR)_tmp/doc/* $(DOCS_DIR)/
	@rm -rf $(DOCS_DIR)_tmp

docs: build_docs
	@echo "Documentation generated in $(DOCS_DIR)/cdd_engine/index.html"
	@echo '<meta http-equiv="refresh" content="0; url=cdd_engine/index.html">' > $(DOCS_DIR)/index.html

build:
	@echo "Building cdd-engine..."
	cargo build --release

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	@echo "Running unit tests..."
	cargo test --all-targets

coverage:
	@chmod +x scripts/update_shields.sh
	@./scripts/update_shields.sh

build_docker:
	@echo "Building Docker images..."
	docker build -f alpine.Dockerfile -t cdd-engine:alpine .
	docker build -f debian.Dockerfile -t cdd-engine:debian .

run_docker: build_docker
	@echo "Running cdd-engine test suite in Alpine container..."
	docker run --rm cdd-engine:alpine
	docker rmi cdd-engine:alpine cdd-engine:debian
