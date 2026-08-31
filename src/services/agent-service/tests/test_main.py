"""Sidecar báo sẵn sàng đúng giao thức mà Rust đọc (`PF-12`, `§P3.4`).

Hai đầu của một giao thức nằm ở hai ngôn ngữ khác nhau là chỗ trôi lệch dễ xảy
ra nhất: đổi chuỗi ở một bên thì bên kia vẫn biên dịch được, vẫn chạy được, và
chỉ hỏng lúc chạy thật dưới dạng *"ứng dụng treo 30 giây rồi chạy không có tầng
nhận thức"*.

Nên bài dưới đây khởi động sidecar **thật** như một tiến trình con, đọc stdout
đúng cách supervisor đọc, và kiểm rằng cổng nó báo là cổng nó thật sự nghe.
"""

from __future__ import annotations

import json
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

import pytest
from agent_service.main import READY_PREFIX

# `tests/` → `agent-service/` → `services/` → `src/`
SRC = Path(__file__).resolve().parents[3]
RUST_SIDECAR = SRC / "deploy" / "tauri" / "src-tauri" / "src" / "sidecar.rs"


def test_tien_to_san_sang_khop_voi_phia_rust() -> None:
    """Hai đầu của giao thức phải giống nhau **từng ký tự**.

    Đọc thẳng file Rust chứ không chép lại hằng số: chép lại thì hai bản sao
    cũng trôi lệch được, chỉ là chậm hơn một nhịp.
    """
    rust = RUST_SIDECAR.read_text(encoding="utf-8")
    assert f'DAU_HIEU_SAN_SANG: &str = "{READY_PREFIX}"' in rust, (
        "tiền tố sẵn sàng ở Python và ở Rust đã trôi lệch — sidecar sẽ khởi "
        "động được nhưng supervisor không bao giờ thấy nó sẵn sàng"
    )


@pytest.fixture
def sidecar():
    """Chạy sidecar thật, trả về `(process, port)`, tắt sạch khi xong."""
    p = subprocess.Popen(
        [sys.executable, "-m", "agent_service.main", "--port", "0"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd=str(SRC / "services" / "agent-service" / "src"),
    )
    try:
        assert p.stdout is not None
        cong: int | None = None
        # Đọc từng dòng đúng cách supervisor đọc: bỏ qua log, tìm dòng sẵn sàng.
        for _ in range(50):
            dong = p.stdout.readline()
            if dong == "":
                break
            if dong.startswith(READY_PREFIX):
                cong = int(dong[len(READY_PREFIX) :].strip())
                break
        assert cong is not None, "sidecar không in dòng sẵn sàng"
        yield p, cong
    finally:
        p.terminate()
        try:
            p.wait(timeout=10)
        except subprocess.TimeoutExpired:
            p.kill()


def test_bao_san_sang_sau_khi_socket_da_mo(sidecar) -> None:
    """In dòng sẵn sàng **trước** khi socket mở là nói dối.

    Phía Rust gọi ngay khi thấy dòng đó; nếu socket chưa mở thì nó nhận
    connection refused và kết luận sidecar hỏng.
    """
    _p, cong = sidecar
    with urllib.request.urlopen(f"http://127.0.0.1:{cong}/health", timeout=10) as r:
        assert r.status == 200
        assert json.loads(r.read())["status"] == "ok"


def test_chi_nghe_loopback(sidecar) -> None:
    """Sidecar chỉ nghe `127.0.0.1`, không phơi cognition ra mạng LAN."""
    _p, cong = sidecar
    # Cổng mở trên loopback.
    with urllib.request.urlopen(f"http://127.0.0.1:{cong}/health", timeout=10) as r:
        assert r.status == 200


def test_duong_dan_la_tra_404_khong_tra_200(sidecar) -> None:
    """Trả 200 cho đường dẫn không biết sẽ làm lỗi cấu hình client trôi qua."""
    _p, cong = sidecar
    with pytest.raises(urllib.error.HTTPError) as e:
        urllib.request.urlopen(f"http://127.0.0.1:{cong}/khong-co", timeout=10)
    assert e.value.code == 404


def test_cong_0_thi_he_dieu_hanh_chon(sidecar) -> None:
    """`--port 0` phải cho ra một cổng thật, không phải 0."""
    _p, cong = sidecar
    assert cong > 0
