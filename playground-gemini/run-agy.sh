#!/usr/bin/env bash
# Chay Gemini 3.7 flash high tren nhiem vu playground.
# Tu thu muc goc repo, go:   ! bash playground-gemini/run-agy.sh
cd "$(dirname "$0")" || exit 1
agy --print-timeout 180m --dangerously-skip-permissions --effort high \
  -p "Doc file BRIEF.md trong thu muc hien tai va thuc hien day du nhiem vu duoc mo ta trong do. Lam viec doc lap, khong hoi lai, tu quyet dinh moi thu. Chi thao tac file trong thu muc hien tai (playground-gemini). Khong duoc dung git. Khi hoan thanh, viet README.md tong ket." \
  2>&1 | tee agy_run.log
