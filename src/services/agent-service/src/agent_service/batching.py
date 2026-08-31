"""Gộp request nhận thức (`idea.md §20.6`, `PC-19`).

## Sự thật khó chịu nằm ở giữa mục §20.6

> Một model xử lý batch về mặt kỹ thuật **vẫn nhìn thấy toàn bộ context**.

Mọi thứ trong file này đi ra từ câu đó. Delimiter và ID cho từng context là cần
thiết, nhưng chúng không phải là một **tường** — chúng là một quy ước mà model
được yêu cầu tôn trọng, và model đôi khi không tôn trọng. Nên có hai lớp:

1. **Trước khi gửi**: từ chối gộp những request mà việc rò rỉ là hỏng gameplay.
   Đây là lớp thật sự bảo vệ, vì thứ không được gửi đi thì không rò được.
2. **Sau khi nhận**: validator loại reference chéo (`validator.py`). Đây là lưới
   bắt lỗi của lớp 1, không phải thay thế nó.

## Bốn trường hợp không được gộp

Chúng không phải là gợi ý hiệu năng. Mỗi cái là một cách hỏng gameplay:

| Không gộp khi | Vì sao |
|---|---|
| Đang đối thoại trực tiếp | Thứ tự phát ngôn quan trọng; gộp làm ai cũng "trả lời" cùng một lúc |
| Stakes cao | Chất lượng giảm theo độ dài context, và đây là chỗ chất lượng đáng tiền |
| Có bí mật cần cách ly | Cách ly tri thức **là** luật chơi (`§18.9`), không phải một tối ưu |
| Context quá dài | Lỗi ở một phần lan ra cả batch |

## Ngân sách theo **tổng độ dài**, không theo số thực thể

"Tối đa 8 thực thể một batch" nghe gọn và sai: tám nông dân đang ngủ là một
prompt ngắn, còn tám tướng lĩnh giữa trận là một prompt dài gấp mười. Giới hạn
theo số lượng sẽ vừa lãng phí ở ca thứ nhất vừa vỡ ở ca thứ hai.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from enum import Enum

from .context import uoc_luong_token
from .generated.mow.cognition.v1 import CognitionContext, Trigger

__all__ = [
    "Batch",
    "BatchPolicy",
    "NoBatchReason",
    "delimit",
    "plan_batches",
]


class NoBatchReason(Enum):
    """Vì sao một request phải đi một mình."""

    DIALOGUE_ORDER = "dialogue_order"
    HIGH_STAKES = "high_stakes"
    SECRET_ISOLATION = "secret_isolation"
    CONTEXT_TOO_LONG = "context_too_long"

    def __str__(self) -> str:
        return self.value


@dataclass(frozen=True, slots=True)
class BatchPolicy:
    """Khi nào được gộp."""

    max_tokens: int = 6_000
    # Một context dài hơn ngưỡng này thì đi một mình, kể cả khi còn chỗ trong
    # ngân sách: nó sẽ lấn át những context ngắn cùng batch.
    solo_above_tokens: int = 1_500
    max_entities: int = 8

    def __post_init__(self) -> None:
        if self.solo_above_tokens > self.max_tokens:
            raise ValueError(
                "solo_above_tokens > max_tokens: một context dài hơn max_tokens "
                "nhưng chưa tới solo_above_tokens vẫn sẽ đi một mình — chỉ là "
                "không có `solo_reason` nào được ghi. Lúc đó `§20.10` không trả "
                "lời được câu 'vì sao hôm đó tốn nhiều lời gọi thế', và cấu hình "
                "này trông như đang gộp trong khi thực tế thì không."
            )


@dataclass(frozen=True, slots=True)
class Batch:
    """Một nhóm sẽ gửi cùng nhau."""

    contexts: tuple[CognitionContext, ...]
    tokens: int
    # Rỗng nghĩa là gộp bình thường. Có giá trị nghĩa là batch một phần tử, và
    # đây là lý do — ghi lại để `§20.10` biết vì sao hôm đó tốn nhiều lời gọi.
    solo_reason: NoBatchReason | None = None

    @property
    def is_solo(self) -> bool:
        """Chỉ có một context."""
        return len(self.contexts) == 1


def _ly_do_khong_gop(
    ctx: CognitionContext, policy: BatchPolicy, secrets: frozenset[int]
) -> NoBatchReason | None:
    """Request này có buộc phải đi một mình không."""
    if ctx.trigger == Trigger.DIALOGUE:
        return NoBatchReason.DIALOGUE_ORDER
    if ctx.entity.value in secrets:
        return NoBatchReason.SECRET_ISOLATION
    if _do_dai(ctx) > policy.solo_above_tokens:
        return NoBatchReason.CONTEXT_TOO_LONG
    if ctx.trigger in (Trigger.GOAL_CONFLICT, Trigger.MAJOR_EVENT):
        # Stakes cao: chất lượng giảm theo độ dài context, và đây đúng là chỗ
        # đáng trả tiền cho một lời gọi riêng.
        return NoBatchReason.HIGH_STAKES
    return None


def _do_dai(ctx: CognitionContext) -> int:
    n = uoc_luong_token(ctx.persona_summary)
    n += sum(uoc_luong_token(o.summary) for o in ctx.observations)
    n += sum(uoc_luong_token(m.content) for m in ctx.memories)
    n += sum(uoc_luong_token(a) for a in ctx.available_actions)
    n += sum(uoc_luong_token(g) for g in ctx.active_goals)
    return n


def plan_batches(
    contexts: Sequence[CognitionContext],
    policy: BatchPolicy | None = None,
    secrets: frozenset[int] = frozenset(),
) -> list[Batch]:
    """Chia thành các batch, giữ nguyên thứ tự đầu vào.

    Giữ thứ tự là một yêu cầu, không phải một chi tiết: `§22.9` cần cùng đầu vào
    cho cùng lịch sử lời gọi, và một thuật toán gộp "tối ưu" có sắp xếp lại sẽ
    làm hai lần chạy giống hệt nhau gọi model theo hai thứ tự khác nhau.
    """
    p = policy or BatchPolicy()
    ra: list[Batch] = []

    hien_tai: list[CognitionContext] = []
    tong = 0

    def chot() -> None:
        nonlocal hien_tai, tong
        if hien_tai:
            ra.append(Batch(contexts=tuple(hien_tai), tokens=tong))
            hien_tai = []
            tong = 0

    for c in contexts:
        ly_do = _ly_do_khong_gop(c, p, secrets)
        if ly_do is not None:
            chot()
            ra.append(Batch(contexts=(c,), tokens=_do_dai(c), solo_reason=ly_do))
            continue

        d = _do_dai(c)
        if tong + d > p.max_tokens or len(hien_tai) >= p.max_entities:
            chot()
        hien_tai.append(c)
        tong += d

    chot()
    return ra


# Delimiter dùng lại đúng cặp của prompt registry (`§22.18`): một quy ước là
# thứ chỉ có giá trị khi nó chỉ có một.
OPEN = "<<<MOW:CTX"
CLOSE = "MOW:CTX>>>"


def delimit(ctx: CognitionContext) -> str:
    """Bọc một context trong delimiter mang ID của nó.

    ID có mặt trong delimiter chứ không chỉ trong nội dung, để output của model
    có thứ mà nó phải trích lại — và để validator có thứ mà nó đối chiếu được.
    """
    return f"{OPEN}:{ctx.entity.value}\n{ctx.persona_summary}\n{CLOSE}:{ctx.entity.value}"
