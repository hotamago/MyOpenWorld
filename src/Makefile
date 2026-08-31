# ─────────────────────────────────────────────────────────────────────────────
# My Open World — entrypoint chuẩn.
#
# Mọi lệnh ở đây chạy **trên máy thật**. Bản chạy trong môi trường cách ly là
# `./mow <lệnh>` ở thư mục gốc repo — xem `deploy/README.md`.
#
# Quy ước: mỗi target làm đúng một việc và in ra thứ nó đang làm. Không có
# target nào âm thầm sửa file nguồn trừ `fmt` và `codegen`, và cả hai đều nói rõ.
# ─────────────────────────────────────────────────────────────────────────────

.DEFAULT_GOAL := help
.PHONY: help setup codegen check test test-rust test-py test-web lint fmt \
        scenario determinism soak clean schema doctor

# ── Trợ giúp ─────────────────────────────────────────────────────────────────

help: ## Danh sách target
	@echo "My Open World — make <target>"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
	  | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

# ── Cài đặt ──────────────────────────────────────────────────────────────────

setup: ## Cài toàn bộ phụ thuộc. Chạy sạch trên máy mới là điều kiện của P0-01.
	@echo "▸ Rust"
	cargo fetch
	@echo "▸ Python"
	uv sync --all-packages
	@echo "▸ Web"
	pnpm install --frozen-lockfile || pnpm install
	@echo "▸ pre-commit"
	uv run pre-commit install || true
	@echo "✓ sẵn sàng — thử \`make test\`"

# ── Sinh mã ──────────────────────────────────────────────────────────────────

codegen: ## Sinh mã từ proto/ và schemas/. SỬA FILE NGUỒN.
	@echo "▸ pipeline 1 — RPC (proto → Rust/Python/TS)"
	cargo run -p mow-codegen --quiet -- rpc
	@echo "▸ pipeline 2 — CONTENT (JSON Schema → Rust/Python/TS)"
	cargo run -p mow-codegen --quiet -- content
	@echo "▸ pipeline 3 — CONFIG (struct Rust → JSON Schema)"
	cargo run -p mow-codegen --quiet -- config
	@echo "✓ xong — nhớ commit mã đã sinh, CI so nó với mã trong repo"

schema: ## Sinh lại schemas/config/app_config.v1.json
	cargo run -p mow-codegen --quiet -- config

# ── Kiểm tra ─────────────────────────────────────────────────────────────────

check: lint test ## lint + test, đúng bộ mà CI chạy

config-check: ## Nạp .env + config, kiểm, in tóm tắt (không chạm mạng)
	cargo run -q -p mow-cli -- config check

test: test-rust ## Toàn bộ test có thể chạy được hiện tại

test-rust: ## Test Rust
	cargo test --workspace --all-features

test-py: ## Test Python
	uv run pytest

test-e2e: ## E2E Playwright cho panel chính (§P7.8)
	pnpm --filter web build
	pnpm --filter web test:e2e

test-web: ## Test web
	pnpm -r test

lint: ## fmt --check + clippy + ruff + eslint
	@echo "▸ rustfmt"
	cargo fmt --all -- --check
	@echo "▸ clippy"
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "▸ ruff"
	-uv run ruff check .
	@echo "▸ eslint"
	pnpm -r lint

fmt: ## Định dạng lại. SỬA FILE NGUỒN.
	cargo fmt --all
	-uv run ruff format .
	-pnpm -r format

# ── Harness cho agent ────────────────────────────────────────────────────────

scenario: ## Chạy kịch bản given/when/then (§P7.3)
	cargo run -p mow-cli --quiet -- scenario run tests/scenarios

determinism: ## Chạy lại với số luồng khác nhau rồi so state hash (§P7.5)
	cargo run -p mow-cli --quiet -- determinism --runs 3

soak: ## Mô phỏng dài, theo dõi bất biến và bộ nhớ (§P7.7)
	cargo run -p mow-cli --release --quiet -- soak --years 200 --worlds 3

bench: ## Đo hiệu năng (§P8.1)
	cargo bench -p mow-worldgen

budget: ## Áp bảng ngân sách hiệu năng lên phép đo ở stdin (§P8.1)
	cargo run -p mow-cli --quiet -- budget --phase F

# ── Khác ─────────────────────────────────────────────────────────────────────

doctor: ## Kiểm tra máy có đủ công cụ không
	@bash ../mow doctor

clean: ## Xóa sản phẩm build
	cargo clean
	rm -rf web/dist web/node_modules/.vite
