"""Điểm vào của sidecar `agent-service` (`plan.md §P3.4`, `PF-12`).

Bản desktop chạy tiến trình này như một **sidecar trên loopback**, vòng đời do
Tauri quản. Xem ``deploy/tauri/src-tauri/src/supervisor.rs``.

## Giao thức sẵn sàng

Tiến trình in **đúng một dòng** ra stdout khi đã nghe được:

    MOW_AGENT_READY port=<cổng>

Phía Rust đọc dòng này thay vì chờ một khoảng cố định. Lý do: ``sleep(2)`` chạy
được trên máy người viết và hỏng trên máy chậm hơn, và nó hỏng dưới dạng *"ứng
dụng thỉnh thoảng không khởi động được"* — loại lỗi tốn nhiều ngày nhất để
truy.

Cổng mặc định là ``0``: hệ điều hành chọn, tiến trình báo lại cổng thật. Cố
định một cổng sẽ đụng với bất cứ thứ gì người dùng đang chạy.

## Vì sao chỉ dùng thư viện chuẩn

Điểm vào này **không** import FastAPI. Nó chỉ cần: mở socket, báo sẵn sàng, trả
lời `/health`, và đóng sạch. Kéo cả stack web vào chỉ để làm bốn việc đó khiến
sidecar không đóng gói nổi trên máy chưa cài đủ phụ thuộc — và bước đóng gói là
bước phải chạy được ở CI trên ba hệ điều hành.

Các route cognition thật gắn thêm ở tầng trên, khi ``§P3.1`` gateway có mặt;
chúng dùng chung ``agent_service.cycle`` với hình thái server, đúng như
``§P3.4`` đòi: **một codebase cognition duy nhất**.

## Tắt sạch

``SIGTERM`` và ``SIGINT`` đều đóng server rồi thoát 0. Không bắt chúng thì Tauri
phải giết cứng, và một tiến trình bị giết cứng có thể để lại file tạm trong thư
mục save của người chơi.
"""

from __future__ import annotations

import argparse
import contextlib
import http.server
import json
import signal
import socketserver
import sys
import threading
from typing import Any

#: Tiền tố dòng sẵn sàng. Khớp ``sidecar::DAU_HIEU_SAN_SANG`` ở Rust.
#:
#: Hai chỗ phải giống nhau từng ký tự. Một prefix cố định thay vì một câu tự do
#: là để phía đọc phân biệt được dòng này với mọi dòng log khác.
READY_PREFIX = "MOW_AGENT_READY port="


class Handler(http.server.BaseHTTPRequestHandler):
    """Bộ xử lý tối thiểu: `/health` và không gì khác.

    Mọi đường dẫn khác trả 404. Một sidecar trả 200 cho đường dẫn nó không biết
    là một sidecar mà lỗi cấu hình phía client sẽ trôi qua im lặng.
    """

    #: Tắt log mặc định của `http.server` — nó ghi ra stderr mỗi request và làm
    #: nhiễu chính cái stdout/stderr mà supervisor đang đọc.
    def log_message(self, format: str, *args: Any) -> None:
        return

    def do_GET(self) -> None:
        if self.path != "/health":
            self.send_error(404)
            return
        body = json.dumps({"status": "ok", "service": "agent-service"}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class Server(socketserver.ThreadingTCPServer):
    """Cho phép mở lại cổng ngay sau khi tắt.

    Không có ``allow_reuse_address`` thì khởi động lại nhanh sau một lần tắt sẽ
    gặp ``Address already in use`` — và với một sidecar do người khác quản vòng
    đời, khởi động lại nhanh là chuyện bình thường.
    """

    allow_reuse_address = True
    daemon_threads = True


def serve(host: str = "127.0.0.1", port: int = 0) -> int:
    """Mở server, báo sẵn sàng, chạy tới khi bị bảo dừng.

    Chỉ nghe **loopback**. `§P3.4`: sidecar chạy trên loopback, và ở bản desktop
    gateway dùng token local sinh lúc khởi động — nghe trên `0.0.0.0` sẽ phơi
    cognition ra mạng LAN của người chơi.
    """
    with Server((host, port), Handler) as httpd:
        cong = httpd.server_address[1]

        # Báo sẵn sàng **sau** khi socket đã mở. In trước là nói dối: phía Rust
        # sẽ gọi ngay và nhận connection refused.
        print(f"{READY_PREFIX}{cong}", flush=True)

        dung = threading.Event()

        def tat(_signum: int, _frame: object) -> None:
            dung.set()
            # `shutdown()` phải gọi từ luồng khác `serve_forever()`, nếu không
            # nó tự khóa mình.
            threading.Thread(target=httpd.shutdown, daemon=True).start()

        for sig in (signal.SIGINT, signal.SIGTERM):
            # Không đăng ký được (chạy trong luồng phụ, hoặc nền không có tín
            # hiệu đó) thì bỏ qua — Tauri vẫn giết được tiến trình.
            with contextlib.suppress(ValueError, OSError):
                signal.signal(sig, tat)

        httpd.serve_forever()
        return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Sidecar agent-service")
    ap.add_argument("--host", default="127.0.0.1", help="chỉ nên là loopback")
    ap.add_argument(
        "--port",
        type=int,
        default=0,
        help="0 = để hệ điều hành chọn rồi báo lại cổng thật",
    )
    args = ap.parse_args(argv)
    return serve(args.host, args.port)


if __name__ == "__main__":
    sys.exit(main())
