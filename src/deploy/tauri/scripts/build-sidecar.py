#!/usr/bin/env python3
"""Đóng gói `agent-service` thành sidecar một file (`plan.md §P3.4`, `PF-12`).

Chạy:

    python deploy/tauri/scripts/build-sidecar.py

Kết quả: ``deploy/tauri/src-tauri/binaries/mow-agent-<target-triple>[.exe]``.

## Vì sao tên có target triple

Tauri đòi vậy, và lý do là đúng: một bundle build trên macOS arm64 không được
lỡ tay nhét vào binary của x86_64. Hậu tố triple làm chuyện lỡ tay đó thành một
lỗi lúc build thay vì một ứng dụng không mở được trên máy người dùng.

## Vì sao script này tách khỏi `tauri.conf.json`

`externalBin` khai trong config **nền** sẽ làm ``cargo build`` hỏng khi chưa có
sidecar — và ``cargo build`` là smoke build chạy mỗi PR chạm vào ``web/`` hoặc
``deploy/tauri/``. Nên nó nằm ở ``tauri.bundle.conf.json``, chỉ dùng khi thật
sự đóng gói:

    python deploy/tauri/scripts/build-sidecar.py
    cargo tauri build --config tauri.bundle.conf.json

Tách như vậy giữ được cả hai: smoke build nhanh và không cần Python, còn bản
phát hành thì có sidecar thật.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

# Thư mục `deploy/tauri/scripts/` → `src/`.
SRC = Path(__file__).resolve().parents[3]
SIDECAR_DIR = SRC / "deploy" / "tauri" / "src-tauri" / "binaries"
ENTRY = SRC / "services" / "agent-service" / "src" / "agent_service" / "main.py"

# Tên khớp `sidecar::TEN_SIDECAR` ở Rust. Hai chỗ phải giống nhau, và test
# `duong_dan_sidecar_nam_canh_tai_nguyen` ghim phía Rust.
NAME = "mow-agent"


def target_triple() -> str:
    """Triple của máy đang chạy, hỏi thẳng `rustc`.

    Không tự suy từ ``platform.machine()``: hai chuỗi đó không khớp nhau trên
    vài nền, và đoán sai ở đây cho ra một bundle chứa binary sai kiến trúc —
    thứ chỉ lộ ra khi người dùng bấm mở.
    """
    out = subprocess.run(
        ["rustc", "-vV"], capture_output=True, text=True, check=True
    ).stdout
    for line in out.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise SystemExit("khong doc duoc target triple tu `rustc -vV`")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--check",
        action="store_true",
        help="chỉ kiểm công cụ và đường dẫn, không build",
    )
    args = ap.parse_args()

    if not ENTRY.exists():
        print(f"khong tim thay entry point: {ENTRY}", file=sys.stderr)
        return 1

    triple = target_triple()
    suffix = ".exe" if triple.endswith("windows-msvc") or "windows" in triple else ""
    dest = SIDECAR_DIR / f"{NAME}-{triple}{suffix}"

    if args.check:
        print(f"entry   : {ENTRY}")
        print(f"triple  : {triple}")
        print(f"dest    : {dest}")
        print(f"pyinstaller: {shutil.which('pyinstaller') or 'KHONG CO'}")
        return 0

    if shutil.which("pyinstaller") is None:
        print(
            "thieu `pyinstaller`. Cai bang: uv tool install pyinstaller",
            file=sys.stderr,
        )
        return 1

    SIDECAR_DIR.mkdir(parents=True, exist_ok=True)
    build_dir = SRC / "target" / "sidecar-build"
    subprocess.run(
        [
            "pyinstaller",
            "--onefile",
            "--name",
            NAME,
            "--distpath",
            str(build_dir / "dist"),
            "--workpath",
            str(build_dir / "work"),
            "--specpath",
            str(build_dir),
            str(ENTRY),
        ],
        check=True,
    )

    built = build_dir / "dist" / f"{NAME}{suffix}"
    if not built.exists():
        print(f"pyinstaller khong tao ra {built}", file=sys.stderr)
        return 1
    shutil.copy2(built, dest)
    print(f"OK: {dest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
