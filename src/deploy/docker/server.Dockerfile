# syntax=docker/dockerfile:1.7
# ─────────────────────────────────────────────────────────────────────────────
# Image chạy engine trong container, không cần cài Rust trên máy đích.
#
# ## Đọc cái này trước: `mow-server` CHƯA TỒN TẠI
#
# Bản trước của file này build `--bin mow-server --bin mow-worker`. Hai binary
# đó có trong `plan.md §P3.1` như hai tiến trình trung tâm của kiến trúc, nhưng
# **chưa có trong repo**: workspace hôm nay có đúng hai binary, `mow-cli` và
# `mow-codegen`. Nghĩa là file này chưa bao giờ build được — nó chỉ chưa ai
# chạy thử.
#
# `progress.md` 147/147 xây engine dưới dạng **thư viện** cộng với `mow-cli`;
# không có task nào dựng một tiến trình phục vụ. Đó là hình trạng desktop-first
# của `§P3.4` và nó nhất quán — chỉ là nó không phải hình trạng mà file này mô
# tả trước đây.
#
# Nên image này đóng gói thứ thật sự chạy được: `mow-cli`. Nó đủ để chạy kịch
# bản, soak, và kiểm determinism trong một môi trường cách ly. Khi `mow-server`
# ra đời, thêm nó vào dòng `cargo build` và đổi `ENTRYPOINT`.
#
# ## Khác toolbox ở một điểm quyết định
#
# **Không có `mow-devtool`.** `§P10.5` yêu cầu devtool không tồn tại trong build
# phát hành, và `P0-11` yêu cầu có bằng chứng. Ở đây điều đó được bảo đảm bằng
# cách không bật feature `devtool` — và được **chứng minh** bằng một bước quét
# symbol ở cuối tầng build.
# ─────────────────────────────────────────────────────────────────────────────

ARG RUST_VERSION=1.90.0

# ── Tầng build ───────────────────────────────────────────────────────────────
FROM rust:${RUST_VERSION}-bookworm AS builder

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY src/Cargo.toml src/Cargo.lock ./
COPY src/rust-toolchain.toml ./
COPY src/crates ./crates
COPY src/proto ./proto

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked --bin mow-cli \
    && mkdir -p /out \
    && cp target/release/mow-cli /out/

# Bằng chứng, không phải lời hứa: nếu chuỗi `mow_devtool` còn trong binary thì
# build fail ngay tại đây chứ không lọt ra tới bản phát hành.
RUN if strings /out/mow-cli | grep -q 'mow_devtool'; then \
        echo 'LỖI: devtool bị link vào bản release' >&2; exit 1; \
    fi

# ── Tầng chạy ────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 -s /usr/sbin/nologin mow

COPY --from=builder /out/mow-cli /usr/local/bin/mow-cli
# Content pack và config đi kèm image: một image chạy được mà không cần mount
# gì thêm, và content hash của nó cố định theo tag (§22.30).
COPY src/content /opt/mow/content
COPY src/config  /opt/mow/config
COPY src/prompts /opt/mow/prompts

# `MOW_CONTENT__ROOT` → `content.root`, một field có thật.
#
# Bản trước còn đặt `MOW_CONFIG__ROOT` và `MOW_PROMPTS__ROOT`. Không có field
# `config` hay `prompts` nào trong `AppConfig`, và `deny_unknown_fields` biến
# chúng thành lỗi khởi động — nghĩa là image này sẽ chết ngay ở lệnh đầu tiên,
# với một thông báo nói về "unknown field" chứ không nói về Dockerfile.
#
# Thư mục config đi qua `--root`, là tham số dòng lệnh chứ không phải config.
ENV MOW_ENV=prod \
    MOW_CONTENT__ROOT=/opt/mow/content \
    RUST_LOG=info

USER 10001:10001
WORKDIR /var/lib/mow

# Kiểm cấu hình là healthcheck đúng cho một CLI: nó trả 0 khi config nạp và
# hợp lệ, 78 khi không. Không cần một cổng nào mở.
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/mow-cli", "config", "check", "--root", "/opt/mow/config"]

ENTRYPOINT ["/usr/local/bin/mow-cli"]
CMD ["help"]
