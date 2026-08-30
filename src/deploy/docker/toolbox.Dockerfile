# syntax=docker/dockerfile:1.7
# ─────────────────────────────────────────────────────────────────────────────
# Toolbox — môi trường build và test **cách ly khỏi máy thật**.
#
# Vì sao cần: `plan.md §P3.4` chốt rằng bản desktop phải chạy được KHÔNG cần
# Docker, và điều đó vẫn đúng. Toolbox không mâu thuẫn với nó — nó giải một vấn
# đề khác: khi một bài determinism fail trên CI mà không fail trên máy bạn, câu
# hỏi đầu tiên luôn là "toolchain có giống nhau không". Toolbox làm câu hỏi đó
# biến mất: cùng một image, cùng một phiên bản Rust/Python/Node, cùng một libc.
#
# Nó cũng là hàng rào an toàn: `cargo test` của một dự án lớn chạy build script
# và proc-macro tùy ý. Chạy chúng trong container nghĩa là chúng không với tới
# ổ đĩa và biến môi trường của máy thật.
# ─────────────────────────────────────────────────────────────────────────────

# Ghim theo digest-friendly tag; đổi phiên bản là một PR riêng (§P10.6).
ARG RUST_VERSION=1.90.0
ARG DEBIAN_SUITE=bookworm

FROM rust:${RUST_VERSION}-${DEBIAN_SUITE}

ARG NODE_MAJOR=24
ARG PNPM_VERSION=11.22.0
ARG UV_VERSION=0.6.14

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_TERM_COLOR=always \
    RUSTFLAGS="-D warnings" \
    # Cargo và target nằm trên volume, không nằm trong lớp image, để rebuild
    # nhanh mà image vẫn nhỏ.
    CARGO_HOME=/cache/cargo \
    CARGO_TARGET_DIR=/cache/target \
    UV_CACHE_DIR=/cache/uv \
    PNPM_HOME=/cache/pnpm \
    PATH=/cache/cargo/bin:/cache/pnpm:$PATH

# ── Gói hệ thống ─────────────────────────────────────────────────────────────
# `libssl-dev`/`pkg-config` cho các crate mạng; `protobuf-compiler` để pipeline
# RPC (§P4.1) không phải tải protoc lúc build; phần còn lại là phụ thuộc GTK/
# WebKit của `tauri-driver`, cần khi chạy e2e của bản desktop trong CI.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl git make jq unzip xz-utils \
        pkg-config libssl-dev protobuf-compiler \
        sqlite3 libsqlite3-dev \
        build-essential clang lld \
        python3 python3-venv \
        libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
        xvfb \
    && rm -rf /var/lib/apt/lists/*

# ── Node và pnpm ─────────────────────────────────────────────────────────────
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_MAJOR}.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && corepack enable \
    && corepack prepare pnpm@${PNPM_VERSION} --activate

# ── uv ───────────────────────────────────────────────────────────────────────
RUN curl -LsSf https://astral.sh/uv/${UV_VERSION}/install.sh | \
    env UV_INSTALL_DIR=/usr/local/bin sh

# ── Thành phần Rust ──────────────────────────────────────────────────────────
# `wasm32-unknown-unknown` cho module Tier 1 (§13.9.3).
RUN rustup component add rustfmt clippy \
    && rustup target add wasm32-unknown-unknown

# ── Người dùng không phải root ───────────────────────────────────────────────
# Không có bước này, mọi file do container tạo ra sẽ thuộc về root trên máy
# thật, và bạn sẽ phải `sudo rm` cả thư mục target.
ARG UID=1000
ARG GID=1000
RUN groupadd -g ${GID} mow 2>/dev/null || true \
    && useradd -m -u ${UID} -g ${GID} -s /bin/bash mow 2>/dev/null || true \
    && mkdir -p /cache/cargo /cache/target /cache/uv /cache/pnpm \
    && chown -R ${UID}:${GID} /cache

USER ${UID}:${GID}
WORKDIR /workspace

# Kiểm tra nhanh rằng toolbox thật sự có đủ đồ. Nếu một dòng ở đây fail thì
# image hỏng, và ta biết ngay lúc build chứ không phải lúc CI chạy nửa chừng.
RUN rustc --version && cargo --version && node --version \
    && pnpm --version && uv --version && protoc --version && sqlite3 --version

CMD ["bash"]
