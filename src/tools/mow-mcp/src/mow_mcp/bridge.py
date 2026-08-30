"""Cầu nối tới phiên gỡ lỗi của ``mow-cli``.

Giao thức là NDJSON trên stdin/stdout của một tiến trình con. `plan.md §P7.2`
đòi hỏi ``mow-mcp`` chỉ nối được tới build có feature ``devtool``, qua loopback,
có token trong ``.env``. Tiến trình con thỏa yêu cầu đó **theo cấu trúc** chứ
không phải nhờ cấu hình:

- Không có cổng nào để nối tới, nên không có bề mặt tấn công, nên không cần token.
- Bản phát hành không đóng gói ``mow-cli``, nên đường này không tồn tại ở đó
  theo đúng nghĩa đen.
- Agent đóng tiến trình là thế giới biến mất. Không có world mồ côi.

Khi ``mow-server`` gRPC có thật ở Giai đoạn C, chỉ lớp này đổi; các tool MCP ở
``server.py`` giữ nguyên.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import threading
from pathlib import Path
from typing import Any

__all__ = ["DebugBridge", "BridgeError"]


class BridgeError(RuntimeError):
    """Lỗi khi nói chuyện với phiên gỡ lỗi."""


def _tim_mow_cli() -> list[str]:
    """Tìm cách chạy ``mow-cli``.

    Ưu tiên binary đã build, rồi mới tới ``cargo run``. Lý do thứ tự đó: ``cargo
    run`` có thể mất hàng chục giây để build lần đầu, và một tool MCP treo mà
    không nói gì thì agent sẽ tưởng là hỏng.
    """
    if env := os.environ.get("MOW_CLI"):
        return [env]

    goc = Path(__file__).resolve().parents[4]  # src/
    for profile in ("debug", "release"):
        for ten in ("mow-cli", "mow-cli.exe"):
            p = goc / "target" / profile / ten
            if p.exists():
                return [str(p)]

    if shutil.which("cargo"):
        return ["cargo", "run", "--quiet", "-p", "mow-cli", "--"]

    raise BridgeError(
        "không tìm thấy `mow-cli`. Build nó bằng `cargo build -p mow-cli`, "
        "hoặc đặt biến môi trường MOW_CLI trỏ tới binary."
    )


class DebugBridge:
    """Một phiên gỡ lỗi đang chạy."""

    def __init__(self, cwd: Path | None = None) -> None:
        self._cmd = _tim_mow_cli() + ["debug-session"]
        self._cwd = cwd or Path(__file__).resolve().parents[4]
        self._proc: subprocess.Popen[str] | None = None
        self._id = 0
        # Một phiên là một tiến trình có state; hai lời gọi đồng thời sẽ trộn
        # dòng của nhau. Khóa này rẻ và loại hẳn một lớp lỗi khó tái hiện.
        self._lock = threading.Lock()

    def start(self) -> None:
        """Khởi động tiến trình con."""
        if self._proc is not None:
            return
        self._proc = subprocess.Popen(  # noqa: S603
            self._cmd,
            cwd=self._cwd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            bufsize=1,
        )

    def stop(self) -> None:
        """Đóng phiên. Thế giới biến mất cùng nó."""
        if self._proc is None:
            return
        try:
            if self._proc.stdin:
                self._proc.stdin.close()
            self._proc.wait(timeout=5)
        except Exception:  # noqa: BLE001 — đóng được thì tốt, không thì giết
            self._proc.kill()
        finally:
            self._proc = None

    def call(self, tool: str, **args: Any) -> dict[str, Any]:
        """Gọi một tool và trả kết quả.

        Ném [`BridgeError`] khi phía kia báo lỗi. Không nuốt lỗi thành một dict
        rỗng: một agent nhận dict rỗng sẽ đi tiếp và kết luận sai.
        """
        with self._lock:
            self.start()
            assert self._proc is not None
            if self._proc.poll() is not None:
                err = self._proc.stderr.read() if self._proc.stderr else ""
                raise BridgeError(f"phiên gỡ lỗi đã chết: {err.strip()}")

            self._id += 1
            req = json.dumps({"id": self._id, "tool": tool, "args": args})

            assert self._proc.stdin and self._proc.stdout
            self._proc.stdin.write(req + "\n")
            self._proc.stdin.flush()

            dong = self._proc.stdout.readline()
            if not dong:
                err = self._proc.stderr.read() if self._proc.stderr else ""
                raise BridgeError(
                    f"phiên gỡ lỗi không trả lời `{tool}`. stderr: {err.strip()}"
                )

        try:
            res = json.loads(dong)
        except json.JSONDecodeError as e:
            raise BridgeError(f"trả lời không phải JSON: {dong!r}") from e

        if not res.get("ok"):
            raise BridgeError(res.get("error", "lỗi không rõ"))
        return res.get("result", {})

    def __enter__(self) -> DebugBridge:
        self.start()
        return self

    def __exit__(self, *exc: object) -> None:
        self.stop()
