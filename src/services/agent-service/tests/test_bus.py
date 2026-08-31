"""Test consumer MessageBus (`PC-01`).

Test ở đây dựng bảng bằng **đúng câu SQL của phía Rust** (`crates/mow-bus/src/
sqlite.rs`). Chép schema là chỗ dễ trôi nhất giữa hai ngôn ngữ, nên nếu Rust đổi
cột mà quên báo, ít nhất một test ở đây sẽ đỏ chứ không phải một NPC im lặng.
"""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest
from agent_service.bus import DONE, LEASED, READY, MessageBusConsumer, NotLeasedError

SCHEMA = """
CREATE TABLE IF NOT EXISTS bus_message (
    seq            INTEGER PRIMARY KEY AUTOINCREMENT,
    subject        TEXT    NOT NULL,
    payload        BLOB    NOT NULL,
    state          INTEGER NOT NULL DEFAULT 0,
    delivery_count INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX IF NOT EXISTS bus_ready ON bus_message (subject, state, seq);
"""


@pytest.fixture
def bus(tmp_path: Path) -> MessageBusConsumer:
    p = tmp_path / "bus.sqlite"
    conn = sqlite3.connect(str(p))
    conn.executescript(SCHEMA)
    conn.commit()
    conn.close()
    return MessageBusConsumer(p)


def test_fetch_giu_thong_diep_va_ack_ket_thuc(bus: MessageBusConsumer) -> None:
    bus.publish("cognition.request", b"xin chao")
    msgs = bus.fetch("cognition.request")
    assert [m.payload for m in msgs] == [b"xin chao"]
    assert bus.pending("cognition.request") == 1

    bus.ack(msgs[0].seq)
    assert bus.pending("cognition.request") == 0


def test_thong_diep_dang_giu_khong_duoc_giao_lai(bus: MessageBusConsumer) -> None:
    """Hai consumer cùng lấy một proposal là xử lý mọi thứ hai lần."""
    bus.publish("s", b"x")
    assert len(bus.fetch("s")) == 1
    assert list(bus.fetch("s")) == [], "lần hai không được thấy gì"


def test_nack_tra_ve_hang_doi_va_dem_lan_giao(bus: MessageBusConsumer) -> None:
    bus.publish("s", b"x")
    m = bus.fetch("s")[0]
    assert m.delivery_count == 1
    bus.nack(m.seq)

    lai = bus.fetch("s")[0]
    assert lai.delivery_count == 2, "không đếm thì proposal độc quay vòng vô tận"


def test_thong_diep_doc_nhan_ra_duoc(bus: MessageBusConsumer) -> None:
    bus.publish("s", b"x")
    for _ in range(3):
        m = bus.fetch("s")[0]
        bus.nack(m.seq)
    assert bus.fetch("s")[0].poisoned


def test_recover_cuu_thong_diep_bi_ket_sau_khi_crash(bus: MessageBusConsumer) -> None:
    """Không có bước này thì mỗi lần crash lại nuốt mất một ít proposal."""
    bus.publish("s", b"a")
    bus.publish("s", b"b")
    bus.fetch("s")  # giữ cả hai rồi "chết"

    assert list(bus.fetch("s")) == [], "đang bị giữ thì không ai lấy được"
    assert bus.recover() == 2
    assert len(bus.fetch("s")) == 2


def test_ack_hai_lan_la_loi_chu_khong_im_lang(bus: MessageBusConsumer) -> None:
    """Nuốt nó đi sẽ giấu mất một consumer đang xử lý mọi thứ hai lần."""
    bus.publish("s", b"x")
    m = bus.fetch("s")[0]
    bus.ack(m.seq)
    with pytest.raises(NotLeasedError):
        bus.ack(m.seq)


def test_nack_cai_chua_giu_cung_la_loi(bus: MessageBusConsumer) -> None:
    bus.publish("s", b"x")
    with pytest.raises(NotLeasedError):
        bus.nack(1)


def test_chu_de_khac_nhau_khong_lan_sang_nhau(bus: MessageBusConsumer) -> None:
    bus.publish("a", b"1")
    bus.publish("b", b"2")
    assert [m.payload for m in bus.fetch("a")] == [b"1"]
    assert [m.payload for m in bus.fetch("b")] == [b"2"]


def test_thu_tu_theo_seq_khong_theo_thu_tu_chen(bus: MessageBusConsumer) -> None:
    for i in range(5):
        bus.publish("s", str(i).encode())
    assert [m.payload for m in bus.fetch("s")] == [b"0", b"1", b"2", b"3", b"4"]


def test_leased_tu_dong_ack_khi_thanh_cong(bus: MessageBusConsumer) -> None:
    bus.publish("s", b"x")
    with bus.leased("s") as msgs:
        assert len(msgs) == 1
    assert bus.pending("s") == 0


def test_leased_tu_dong_nack_khi_co_ngoai_le(bus: MessageBusConsumer) -> None:
    """Chỉ cần một nhánh `return` sớm là một thông điệp kẹt ở LEASED mãi mãi."""
    bus.publish("s", b"x")
    with pytest.raises(RuntimeError), bus.leased("s") as msgs:
        assert len(msgs) == 1
        raise RuntimeError("xử lý hỏng")

    # Vẫn còn trong hàng đợi, sẵn sàng thử lại.
    assert bus.pending("s") == 1
    assert len(bus.fetch("s")) == 1


def test_trang_thai_khop_voi_phia_rust() -> None:
    """Ba hằng số này là hợp đồng với `crates/mow-bus/src/sqlite.rs`."""
    assert (READY, LEASED, DONE) == (0, 1, 2)
