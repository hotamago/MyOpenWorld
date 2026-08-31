"""Chỉ mục vector **dựng lại được** (`plan.md §P6.3`, `PC-06`, `PC-07`).

## Chỉ mục phải là thứ vứt đi được

```text
mow-server (nguồn sự thật)        memory-service (chỉ mục)
  event log ────► MemoryRecord ────► index.add(...) ────► vector store
  (SQL)           version, ACL,
                  branch, tombstone
```

Nếu mất chỉ mục mà mất dữ liệu, thì chỉ mục đã trở thành nguồn sự thật thứ hai —
và hai nguồn sự thật luôn lệch nhau, chỉ là chưa lệch thôi. Vì vậy `rebuild()`
xóa sạch rồi dựng lại từ bản ghi authoritative, và có test chứng minh không mất gì.

## Thứ tự trong `rebuild` không phải chuyện tùy tiện

`§11.5` viết: *"tạo tombstone/version mới và vô hiệu embedding cũ **trước khi**
index lại. Vector stale không được trả về trong khoảng rebuild."*

Đó là một ràng buộc về **thứ tự**, và cách hỏng của nó rất đặc trưng: nếu index
trước rồi mới áp tombstone, thì trong khoảng giữa hai bước, một truy vấn sẽ trả
về đúng cái ký ức mà người chơi vừa yêu cầu xóa. Cửa sổ đó ngắn — và đó chính là
lý do bug này sống sót qua mọi lần thử tay.

Nên `rebuild()` áp tombstone **trước**, và có một test giữ lấy thứ tự đó.
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from dataclasses import dataclass

from .acl import AclScope, MemoryRecord, filter_visible

__all__ = ["Hit", "MemoryIndex", "RebuildReport"]


@dataclass(frozen=True, slots=True)
class Hit:
    """Một kết quả truy vấn."""

    record: MemoryRecord
    relevance: int


@dataclass(frozen=True, slots=True)
class RebuildReport:
    """Kết quả một lần dựng lại."""

    records_indexed: int
    tombstones_applied: int
    # Số bản ghi có trước nhưng không dựng lại được. **Phải bằng 0.**
    lost: int


def _diem_lien_quan(record: MemoryRecord, query: str) -> int:
    """Điểm liên quan, thang `0`–`1000`.

    Đây là chỗ mà bản thật sẽ gọi embedding. Bản này dùng trùng lặp từ, và điều
    đó là đủ cho mục đích của module: mọi thứ quanh nó — ACL, tombstone, thứ tự
    rebuild — không phụ thuộc vào cách tính điểm, và phải đúng với bất kỳ cách
    tính nào. Đổi sang embedding thật không được làm test nào ở đây đỏ.
    """
    if not query:
        return 0
    q = {t for t in query.lower().split() if t}
    c = {t for t in record.content.lower().split() if t}
    if not q or not c:
        return 0
    chung = len(q & c)
    return min(1000, chung * 1000 // len(q))


class MemoryIndex:
    """Chỉ mục có thể dựng lại từ bản ghi authoritative."""

    def __init__(self) -> None:
        self._points: dict[str, MemoryRecord] = {}

    def add(self, record: MemoryRecord) -> None:
        """Thêm hoặc thay một điểm."""
        self._points[record.id] = record

    def invalidate(self, record_id: str, branch: str) -> bool:
        """Vô hiệu một điểm trong một nhánh — tức là đặt tombstone.

        **Không xóa vật lý.** `§11.5` đòi việc quên phải truy ngược được, và xóa
        vật lý còn làm replay của một save cũ hỏng: bản ghi biến mất khỏi lịch sử
        mà event log vẫn nhắc tới nó.
        """
        r = self._points.get(record_id)
        if r is None:
            return False
        self._points[record_id] = MemoryRecord(
            id=r.id,
            namespace=r.namespace,
            content=r.content,
            created_tick=r.created_tick,
            source_event_seq=r.source_event_seq,
            persona_version=r.persona_version,
            created_branch=r.created_branch,
            tombstoned_in_branches=r.tombstoned_in_branches | {branch},
        )
        return True

    def query(self, text: str, scope: AclScope, limit: int = 10) -> Sequence[Hit]:
        """Truy vấn. **Luôn** đi qua ACL trước, semantic sau.

        Thứ tự này quan trọng: lọc sau khi xếp hạng sẽ cho ra ít kết quả hơn
        `limit` một cách khó đoán, và tệ hơn, nó đã tính điểm trên những bản ghi
        mà thực thể không được biết — nghĩa là điểm của kết quả hợp lệ phụ thuộc
        vào nội dung mà nó không được thấy.
        """
        thay = filter_visible(self._points.values(), scope)
        cham = [Hit(record=r, relevance=_diem_lien_quan(r, text)) for r in thay]
        cham = [h for h in cham if h.relevance > 0]
        # Phá hòa bằng `id` để hai lần chạy cho cùng thứ tự.
        cham.sort(key=lambda h: (-h.relevance, h.record.id))
        return cham[:limit]

    def rebuild(self, authoritative: Iterable[MemoryRecord]) -> RebuildReport:
        """Xóa sạch rồi dựng lại từ bản ghi authoritative.

        Tombstone được áp **trước** khi index — xem docstring của module.
        """
        truoc = set(self._points)
        ban_ghi = list(authoritative)

        # Bước 1: tombstone. Trước, luôn luôn.
        tombstone = {
            r.id: r.tombstoned_in_branches for r in ban_ghi if r.tombstoned_in_branches
        }

        # Bước 2: xóa sạch chỉ mục cũ.
        self._points.clear()

        # Bước 3: index lại, mang theo tombstone đã biết từ bước 1.
        for r in ban_ghi:
            self._points[r.id] = MemoryRecord(
                id=r.id,
                namespace=r.namespace,
                content=r.content,
                created_tick=r.created_tick,
                source_event_seq=r.source_event_seq,
                persona_version=r.persona_version,
                created_branch=r.created_branch,
                tombstoned_in_branches=tombstone.get(r.id, frozenset()),
            )

        return RebuildReport(
            records_indexed=len(ban_ghi),
            tombstones_applied=len(tombstone),
            lost=len(truoc - set(self._points)),
        )

    def __len__(self) -> int:
        return len(self._points)
