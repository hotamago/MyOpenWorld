"""Nén context cho prompt (`idea.md §20.9`, `plan.md §P6.2`, `PC-03`).

## Bài toán không phải là "cho vừa token"

Cho vừa thì cắt bớt là xong. Bài toán là **cắt cái gì**, và một lựa chọn sai làm
hỏng thứ khác hẳn: nhân vật vẫn trả lời trôi chảy, chỉ là nó quên mất điều quan
trọng nhất đang xảy ra với nó. Không có gì báo lỗi.

Ba quy tắc, và cả ba đều là phản ứng với một cách hỏng cụ thể:

1. **Quan sát mới nhất không bao giờ bị cắt.** Cắt nó là bỏ đói nhân vật đúng
   thứ nó vừa thấy — nguồn gốc kinh điển của "NPC đứng nhìn nhà cháy".

2. **Ký ức cắt theo `relevance`, không theo thời gian.** Cắt theo thời gian nghe
   hợp lý và sai: ký ức quan trọng nhất của một người thường là ký ức cũ nhất.

3. **Mỗi phần tóm tắt giữ link về nguồn.** `§20.9` đòi điều này để audit, và nó
   cũng là thứ khiến ``evidence_refs`` ở `§10.4` bước 5 có nghĩa: một ``id``
   trong prompt phải trỏ ngược được về một bản ghi có thật.

## Vì sao đếm token bằng một hàm xấp xỉ

Đếm chính xác cần tokenizer của đúng model đang dùng, và model thì đổi. Một hàm
xấp xỉ **luôn ước lượng thừa** an toàn hơn: cắt hơi nhiều thì nhân vật hơi thiếu
thông tin, còn ước lượng thiếu thì request bị nhà cung cấp từ chối và cả chu
trình rơi vào fallback — một cách hỏng ồn ào hơn nhiều.
"""

from __future__ import annotations

from dataclasses import dataclass

from .generated.mow.cognition.v1 import (
    CognitionContext,
    Observation,
    RetrievedMemory,
)

__all__ = ["Budget", "CompressedContext", "compress", "uoc_luong_token"]

# Số ký tự trung bình cho một token, làm tròn **xuống** để ước lượng token thừa.
#
# Với tiếng Anh con số thật khoảng 4; với tiếng Việt có dấu thì thấp hơn vì mỗi
# ký tự chiếm nhiều byte hơn. Lấy 3 để cùng một ngân sách vẫn đúng cho cả hai.
KY_TU_MOI_TOKEN = 3


def uoc_luong_token(text: str) -> int:
    """Ước lượng số token, luôn nghiêng về phía thừa."""
    return (len(text) + KY_TU_MOI_TOKEN - 1) // KY_TU_MOI_TOKEN


@dataclass(frozen=True, slots=True)
class Budget:
    """Ngân sách của một prompt."""

    max_tokens: int = 2_000
    # Quan sát gần nhất luôn được giữ, kể cả khi vượt ngân sách. Xem quy tắc 1.
    min_observations: int = 3

    def __post_init__(self) -> None:
        if self.min_observations < 1:
            raise ValueError(
                "min_observations phải ≥ 1: một nhân vật không thấy gì cả thì "
                "không có gì để nghĩ, và nó sẽ đứng im một cách khó hiểu"
            )


@dataclass(frozen=True, slots=True)
class CompressedContext:
    """Context đã nén, kèm dấu vết đã bỏ gì."""

    context: CognitionContext
    tokens: int
    dropped_observations: int
    dropped_memories: int

    @property
    def lossless(self) -> bool:
        """Không bỏ gì cả."""
        return self.dropped_observations == 0 and self.dropped_memories == 0


def _do_dai(o: Observation | RetrievedMemory) -> int:
    if isinstance(o, Observation):
        return uoc_luong_token(o.summary) + uoc_luong_token(o.id)
    return uoc_luong_token(o.content) + uoc_luong_token(o.id)


def _co_dinh(ctx: CognitionContext) -> int:
    """Phần không nén được: persona, mục tiêu, danh sách action."""
    n = uoc_luong_token(ctx.persona_summary)
    n += sum(uoc_luong_token(g) for g in ctx.active_goals)
    n += sum(uoc_luong_token(a) for a in ctx.available_actions)
    return n


def compress(ctx: CognitionContext, budget: Budget | None = None) -> CompressedContext:
    """Nén một context cho vừa ngân sách.

    Trả về context **mới**; không sửa cái được truyền vào. Sửa tại chỗ ở đây sẽ
    rất tiện và sẽ khiến lần thử lại sau một timeout dùng phải một context đã bị
    cắt ở lần trước — nghĩa là mỗi lần thử lại nhân vật lại biết ít hơn.
    """
    b = budget or Budget()

    con_lai = b.max_tokens - _co_dinh(ctx)

    # Quan sát: mới nhất trước, và luôn giữ ít nhất `min_observations`.
    quan_sat = sorted(ctx.observations, key=lambda o: o.at_tick.value, reverse=True)
    giu_qs: list[Observation] = []
    for i, o in enumerate(quan_sat):
        chi_phi = _do_dai(o)
        if i < b.min_observations or chi_phi <= con_lai:
            giu_qs.append(o)
            con_lai -= chi_phi

    # Ký ức: theo `relevance` giảm dần. Phá hòa bằng `id` để kết quả xác định —
    # hai ký ức cùng điểm mà thứ tự đổi theo lần chạy sẽ làm prompt đổi, và một
    # prompt đổi là một câu trả lời đổi.
    ky_uc = sorted(ctx.memories, key=lambda m: (-m.relevance, m.id))
    giu_ku: list[RetrievedMemory] = []
    for m in ky_uc:
        chi_phi = _do_dai(m)
        if chi_phi <= con_lai:
            giu_ku.append(m)
            con_lai -= chi_phi

    # Giữ đúng thứ tự thời gian khi đưa vào prompt: model đọc một dòng thời gian
    # dễ hơn đọc một danh sách đã sắp theo điểm liên quan.
    giu_qs.sort(key=lambda o: o.at_tick.value)
    giu_ku.sort(key=lambda m: m.at_tick.value)

    moi = CognitionContext(
        request_id=ctx.request_id,
        entity=ctx.entity,
        world=ctx.world,
        branch=ctx.branch,
        now=ctx.now,
        trigger=ctx.trigger,
        observations=giu_qs,
        memories=giu_ku,
        available_actions=list(ctx.available_actions),
        persona_summary=ctx.persona_summary,
        persona_version=ctx.persona_version,
        active_goals=list(ctx.active_goals),
        needs=dict(ctx.needs),
    )

    da_dung = (
        _co_dinh(moi)
        + sum(_do_dai(o) for o in giu_qs)
        + sum(_do_dai(m) for m in giu_ku)
    )
    return CompressedContext(
        context=moi,
        tokens=da_dung,
        dropped_observations=len(ctx.observations) - len(giu_qs),
        dropped_memories=len(ctx.memories) - len(giu_ku),
    )
