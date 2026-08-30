# syntax=docker/dockerfile:1.7
# ─────────────────────────────────────────────────────────────────────────────
# `mow-server` — image chạy thật, tối giản.
#
# Khác toolbox ở một điểm quyết định: **không có `mow-devtool`**. `plan.md
# §P10.5` yêu cầu devtool không tồn tại trong build phát hành, và `P0-11` yêu
# cầu có test chứng minh không còn symbol nào của nó. Ở đây điều đó được bảo
# đảm bằng cách không bật feature `devtool` — không phải bằng cách tin rằng
# không ai gọi nó.
# ─────────────────────────────────────────────────────────────────────────────

ARG RUST_VERSION=1.90.0

# ── Tầng build ───────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-bookworm AS builder

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Nạp manifest trước để lớp phụ thuộc được cache riêng: sửa một dòng code
# không kéo theo build lại toàn bộ cây phụ thuộc.
COPY src/Cargo.toml src/Cargo.lock ./
COPY src/rust-toolchain.toml ./
COPY src/crates ./crates
COPY src/proto ./proto

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked --bin mow-server --bin mow-worker \
    && mkdir -p /out \
    && cp target/release/mow-server target/release/mow-worker /out/

# Bằng chứng, không phải lời hứa: nếu chuỗi `mow_devtool` còn trong binary thì
# build fail ngay tại đây chứ không lọt ra tới bản phát hành.
RUN if strings /out/mow-server | grep -q 'mow_devtool'; then \
        echo 'LỖI: devtool bị link vào bản release' >&2; exit 1; \
    fi

# ── Tầng chạy ────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 -s /usr/sbin/nologin mow

COPY --from=builder /out/mow-server /usr/local/bin/mow-server
COPY --from=builder /out/mow-worker /usr/local/bin/mow-worker
# Content pack và config đi kèm image: một image chạy được mà không cần mount
# gì thêm, và content hash của nó cố định theo tag (§22.30).
COPY src/content /opt/mow/content
COPY src/config  /opt/mow/config
COPY src/prompts /opt/mow/prompts

ENV MOW_ENV=prod \
    MOW_CONTENT__ROOT=/opt/mow/content \
    MOW_CONFIG__ROOT=/opt/mow/config \
    MOW_PROMPTS__ROOT=/opt/mow/prompts \
    RUST_LOG=info

USER 10001:10001
WORKDIR /var/lib/mow
EXPOSE 50051 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=20s --retries=5 \
    CMD ["/usr/local/bin/mow-server", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/mow-server"]
CMD ["serve"]
