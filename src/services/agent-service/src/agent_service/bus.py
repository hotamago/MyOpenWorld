"""Consumer của `MessageBus` (`plan.md §P3.4`, `§P5`, `PC-01`).

## Vì sao Python đọc thẳng SQLite thay vì gọi một API

`§P3.4` chọn bus bền trên SQLite cho bản desktop, và cùng một file được cả Rust
lẫn Python mở. Dựng thêm một tầng HTTP ở giữa chỉ để Python "không chạm vào cơ
sở dữ liệu" sẽ thêm một tiến trình phải chạy, một cổng phải mở, và một chế độ
hỏng mới — trong khi hợp đồng thật (`bus_message`: ba trạng thái, một bảng) đủ
nhỏ để hai bên cùng giữ đúng.

Ranh giới vẫn nguyên vẹn ở chỗ nó thật sự quan trọng: **Python không ghi state
authoritative.** Nó chỉ nhận đề nghị và trả đề nghị. Bảng `bus_message` là một
hàng đợi, không phải state của thế giới.

## Ba trạng thái, và vì sao `nack` không phải là "thất bại"

```text
READY(0) ──fetch──► LEASED(1) ──ack──► DONE(2)
   ▲                    │
   └────── nack ────────┘
```

`nack` trả một thông điệp về hàng đợi để thử lại. Cách hỏng mà nó tồn tại để
tránh: một consumer chết giữa chừng sẽ để lại thông điệp ở `LEASED` mãi mãi, và
proposal đó biến mất mà không ai biết. `recover()` quét sạch chúng về `READY`
lúc khởi động — nên **phải gọi `recover()` khi mở bus**, không phải chỉ khi nghi
có sự cố.

`delivery_count` là thứ phân biệt "mạng chập một lần" với "thông điệp này làm
consumer chết mọi lần". Không đếm thì một proposal độc hại sẽ quay vòng vô tận.
"""

from __future__ import annotations

import sqlite3
from collections.abc import Iterator, Sequence
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path

__all__ = [
    "DONE",
    "LEASED",
    "READY",
    "BusError",
    "BusMessage",
    "MessageBusConsumer",
    "NotLeasedError",
]

READY = 0
LEASED = 1
DONE = 2


class BusError(Exception):
    """Lỗi bus."""


class NotLeasedError(BusError):
    """Ack hoặc nack một thông điệp không đang được giữ.

    Đây là lỗi lập trình, không phải chuyện bình thường — nuốt nó đi sẽ giấu mất
    một consumer đang ack hai lần, và consumer đó đang xử lý mọi thứ hai lần.
    """


@dataclass(frozen=True, slots=True)
class BusMessage:
    """Một thông điệp đã được giữ."""

    seq: int
    subject: str
    payload: bytes
    delivery_count: int

    @property
    def poisoned(self) -> bool:
        """Đã chết đủ nhiều lần để coi là thông điệp độc.

        Ngưỡng 3 là một lựa chọn, không phải một hằng số vũ trụ: đủ để chịu được
        một sự cố thoáng qua, đủ ít để một proposal làm consumer chết không quay
        vòng cả buổi.
        """
        return self.delivery_count >= 3


class MessageBusConsumer:
    """Đọc từ bus bền trên SQLite mà `mow-server` ghi vào."""

    def __init__(self, path: str | Path) -> None:
        self._conn = sqlite3.connect(str(path))
        self._conn.execute("PRAGMA journal_mode=WAL")
        # `FULL` để khớp với phía Rust. Hai bên đặt khác nhau thì bên lỏng hơn
        # quyết định độ bền thật, và lời hứa "publish xong là đã trên đĩa" hỏng.
        self._conn.execute("PRAGMA synchronous=FULL")

    def close(self) -> None:
        """Đóng kết nối."""
        self._conn.close()

    def recover(self) -> int:
        """Trả mọi thông điệp đang bị giữ về hàng đợi.

        **Gọi lúc khởi động, luôn luôn.** Một consumer chết giữa chừng để lại
        thông điệp ở `LEASED` vĩnh viễn; không có bước này thì mỗi lần crash lại
        nuốt mất một ít proposal, và không có gì báo.
        """
        cur = self._conn.execute(
            "UPDATE bus_message SET state = ? WHERE state = ?", (READY, LEASED)
        )
        self._conn.commit()
        return cur.rowcount

    def fetch(self, subject: str, max_messages: int = 16) -> Sequence[BusMessage]:
        """Giữ tối đa `max_messages` thông điệp sẵn sàng của một chủ đề."""
        cur = self._conn.execute(
            "SELECT seq FROM bus_message WHERE subject = ? AND state = ? ORDER BY seq LIMIT ?",
            (subject, READY, max_messages),
        )
        seqs = [int(r[0]) for r in cur.fetchall()]

        ra: list[BusMessage] = []
        for seq in seqs:
            self._conn.execute(
                "UPDATE bus_message SET state = ?, delivery_count = delivery_count + 1 "
                "WHERE seq = ?",
                (LEASED, seq),
            )
            row = self._conn.execute(
                "SELECT payload, delivery_count FROM bus_message WHERE seq = ?", (seq,)
            ).fetchone()
            ra.append(
                BusMessage(
                    seq=seq,
                    subject=subject,
                    payload=bytes(row[0]),
                    delivery_count=int(row[1]),
                )
            )
        self._conn.commit()
        return ra

    def ack(self, seq: int) -> None:
        """Xong."""
        self._chuyen(seq, tu=LEASED, sang=DONE)

    def nack(self, seq: int) -> None:
        """Trả lại hàng đợi để thử lần sau."""
        self._chuyen(seq, tu=LEASED, sang=READY)

    def pending(self, subject: str) -> int:
        """Còn bao nhiêu chưa xong."""
        row = self._conn.execute(
            "SELECT COUNT(*) FROM bus_message WHERE subject = ? AND state != ?",
            (subject, DONE),
        ).fetchone()
        return int(row[0])

    def publish(self, subject: str, payload: bytes) -> int:
        """Đăng một **đề nghị** lên bus.

        Đây là cách duy nhất Python đưa thứ gì đó về phía Rust. Không có hàm nào
        ở đây ghi vào state của thế giới, và sẽ không có: `§22.1` nói một thay
        đổi authoritative chỉ được commit qua transaction handler.
        """
        cur = self._conn.execute(
            "INSERT INTO bus_message (subject, payload, state) VALUES (?, ?, ?)",
            (subject, payload, READY),
        )
        self._conn.commit()
        return int(cur.lastrowid or 0)

    def _chuyen(self, seq: int, *, tu: int, sang: int) -> None:
        cur = self._conn.execute(
            "UPDATE bus_message SET state = ? WHERE seq = ? AND state = ?", (sang, seq, tu)
        )
        self._conn.commit()
        if cur.rowcount == 0:
            raise NotLeasedError(f"thông điệp {seq} không đang được giữ")

    @contextmanager
    def leased(self, subject: str, max_messages: int = 16) -> Iterator[Sequence[BusMessage]]:
        """Giữ, xử lý, rồi ack — hoặc nack nếu có ngoại lệ.

        Viết tay ba bước đó ở mỗi chỗ gọi là cách một thông điệp bị kẹt ở
        `LEASED`: chỉ cần một nhánh `return` sớm.
        """
        msgs = self.fetch(subject, max_messages)
        try:
            yield msgs
        except Exception:
            for m in msgs:
                self.nack(m.seq)
            raise
        else:
            for m in msgs:
                self.ack(m.seq)
