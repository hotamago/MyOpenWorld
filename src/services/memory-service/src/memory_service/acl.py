"""Lớp ACL — **cửa duy nhất** vào ký ức (`idea.md §11.5`, `§22.16`, `PC-05`).

## Vì sao phải là "duy nhất" chứ không phải "nên dùng"

`plan.md §P6.3` viết rõ: *"mọi truy vấn bắt buộc đi qua `acl.py` — không có đường
tắt gọi thẳng mem0 từ graph."*

Điều đó nghe như kỷ luật code, nhưng nó là kỹ thuật. Một truy vấn bỏ qua ACL
không báo lỗi và không trả về rác — nó trả về **kết quả tốt hơn**: nhiều ký ức
liên quan hơn, câu trả lời của NPC sâu sắc hơn. Cái sai chỉ lộ ra rất lâu sau
đó, khi một nhân vật nhắc tới điều nó không thể biết, và tới lúc ấy không ai
nhớ chỗ nào đã gọi tắt.

Nên hàm lọc ở đây nhận **toàn bộ** tham số cách ly dưới dạng bắt buộc. Không có
giá trị mặc định nào cả: quên truyền là lỗi ngay lúc gọi, không phải một bộ lọc
rộng hơn ý muốn.

## Bộ lọc dòng dõi

```text
created_branch_id ∈ ancestry(current_branch)
AND current_branch ∉ tombstoned_in_branches
AND created_tick <= fork_tick(nhánh tương ứng)
```

Vế thứ ba là vế hay bị bỏ quên, và là vế khiến hai vế kia có nghĩa. Không có nó,
nhánh con thấy được **cả ký ức nhánh cha tạo ra sau điểm fork** — tức là nó biết
những chuyện chỉ xảy ra ở một dòng thời gian khác.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, field

__all__ = ["AclScope", "MemoryRecord", "filter_visible"]


@dataclass(frozen=True, slots=True)
class MemoryRecord:
    """Bản ghi ký ức authoritative.

    Nguồn sự thật nằm ở SQL, không ở vector store. Bản sao ở đây chỉ để lọc.
    """

    id: str
    namespace: str
    content: str
    created_tick: int
    source_event_seq: int
    persona_version: str
    created_branch: str
    tombstoned_in_branches: frozenset[str] = field(default_factory=frozenset)


@dataclass(frozen=True, slots=True)
class AclScope:
    """Phạm vi mà một truy vấn được phép thấy.

    Mọi trường đều **bắt buộc**. Xem docstring của module để biết vì sao không có
    mặc định nào.
    """

    namespace: str
    branch: str
    # Dòng dõi từ gốc tới nhánh hiện tại, kể cả chính nó.
    ancestry: tuple[str, ...]
    persona_version: str
    now: int
    # `branch → tick` mà nhánh đó tách ra khỏi cha. Nhánh gốc không có mục nào.
    fork_ticks: Mapping[str, int]

    def __post_init__(self) -> None:
        if not self.ancestry:
            raise ValueError(
                "ancestry rỗng: một nhánh luôn có ít nhất chính nó trong dòng dõi. "
                "Rỗng ở đây sẽ lọc sạch mọi ký ức và nhân vật sẽ mất trí nhớ hoàn toàn."
            )
        if self.branch != self.ancestry[-1]:
            raise ValueError(
                f"ancestry phải kết thúc bằng nhánh hiện tại: {self.branch!r} "
                f"không phải {self.ancestry[-1]!r}"
            )


def _tran_tick(scope: AclScope, created_branch: str) -> int:
    """Ký ức của `created_branch` chỉ thấy được tới tick nào.

    Với nhánh hiện tại: tới bây giờ. Với nhánh tổ tiên: tới lúc **nhánh con của
    nó trong dòng dõi** tách ra. Đó chính là vế thứ ba của bộ lọc.
    """
    if created_branch == scope.branch:
        return scope.now

    try:
        i = scope.ancestry.index(created_branch)
    except ValueError:
        return -1

    # Nhánh tách ra ngay sau `created_branch` trong dòng dõi.
    con = scope.ancestry[i + 1]
    return scope.fork_ticks.get(con, scope.now)


def filter_visible(
    records: Iterable[MemoryRecord], scope: AclScope
) -> Sequence[MemoryRecord]:
    """Những ký ức mà `scope` được phép thấy, theo thứ tự ổn định."""
    to_to = set(scope.ancestry)
    ra: list[MemoryRecord] = []

    for r in records:
        if r.namespace != scope.namespace:
            continue
        if r.created_branch not in to_to:
            continue
        if scope.branch in r.tombstoned_in_branches:
            continue
        if r.created_tick > _tran_tick(scope, r.created_branch):
            continue
        ra.append(r)

    # Sắp theo `(tick, id)` chứ không theo thứ tự đầu vào: đầu vào tới từ một
    # vector store, và thứ tự của nó không có gì đảm bảo giữa hai lần chạy.
    ra.sort(key=lambda r: (r.created_tick, r.id))
    return ra
