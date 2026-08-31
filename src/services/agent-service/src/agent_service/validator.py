"""Validator của chu trình nhận thức (`idea.md §10.4` bước 6, `§22.4`, `PC-04`).

## Validator này **không** cố làm gì

Nó không cố chứng minh rằng văn bản tự do của model "không dùng kiến thức ngoài
context". `§10.4` bước 6 nói thẳng điều đó, và việc nói thẳng ra quan trọng hơn
nó có vẻ: một validator hứa nhiều hơn khả năng của nó là một validator mà người
ta tin, và niềm tin sai chỗ tệ hơn không có validator.

Ranh giới thật:

- **Kiểm được**: action có tồn tại không, thực thể có biết action đó không, mọi
  ``evidence_refs`` có trỏ vào đúng ``CognitionContext`` này không.
- **Không kiểm được**: một câu văn có ngầm dùng thông tin mà nhân vật không thể
  biết hay không.

Nên phần văn bản không có reference **không được trao hiệu lực authoritative**.
Nó vào UI, nó vào log, nó không vào luật. Đó không phải là một hạn chế tạm thời
chờ ai đó viết bộ kiểm tốt hơn — đó là ranh giới của bài toán.

## Vì sao reference chéo là lỗi nghiêm trọng nhất ở đây

`§20.6` cho phép gộp nhiều thực thể vào một request để tiết kiệm. Nếu validator
không từ chối reference trỏ sang context của thực thể khác, thì batch trở thành
một đường rò: nhân vật A biện minh cho hành động của mình bằng một quan sát mà
chỉ B mới thấy. Không có gì trông sai trong log — chỉ có một NPC biết điều nó
không thể biết.
"""

from __future__ import annotations

from dataclasses import dataclass

from .generated.mow.cognition.v1 import CognitionContext, IntentProposal

__all__ = [
    "Rejection",
    "ValidationResult",
    "validate_intent",
    "validate_response",
]


@dataclass(frozen=True, slots=True)
class Rejection:
    """Vì sao một ý định bị từ chối.

    ``code`` là mã ổn định để engine xử lý, ``detail`` là để người đọc. Không
    gộp làm một: một chuỗi tiếng Việt trong câu lệnh ``if`` là một bug đang chờ
    ai đó sửa lỗi chính tả.
    """

    code: str
    detail: str

    def __str__(self) -> str:
        return f"{self.code}: {self.detail}"


@dataclass(frozen=True, slots=True)
class ValidationResult:
    """Kết quả kiểm một response."""

    accepted: tuple[IntentProposal, ...]
    rejected: tuple[tuple[IntentProposal, Rejection], ...]

    @property
    def ok(self) -> bool:
        """Mọi ý định đều qua."""
        return not self.rejected


def validate_intent(intent: IntentProposal, ctx: CognitionContext) -> Rejection | None:
    """Kiểm một ý định. ``None`` nghĩa là qua."""
    if not intent.action:
        return Rejection("action_missing", "ý định không nêu action")

    # Bước 6a: action phải tồn tại **và** thực thể phải biết nó.
    #
    # Hai điều kiện này gộp làm một ở đây vì `available_actions` đã là giao của
    # chúng: nó được engine dựng từ những gì *thực thể này* biết làm, không phải
    # từ danh mục action toàn cục. Một danh mục toàn cục sẽ cho phép một đứa trẻ
    # đề nghị `forge_sword` và chỉ hỏng ở bước sau, khi thông tin về lý do đã mất.
    if intent.action not in ctx.available_actions:
        return Rejection(
            "action_unknown",
            f"`{intent.action}` không nằm trong danh sách thực thể biết làm",
        )

    # Bước 6b: mọi evidence phải thuộc **đúng** context này.
    hop_le = {o.id for o in ctx.observations} | {m.id for m in ctx.memories}
    for ref in intent.evidence_refs:
        if ref not in hop_le:
            return Rejection(
                "evidence_foreign",
                f"`{ref}` không có trong cognition context của thực thể này",
            )

    # Target phải là thứ thực thể đã quan sát thấy.
    #
    # Không có bước này, model có thể nhắm vào một thực thể mà nhân vật chưa hề
    # trông thấy — và vì `EntityId` là số nguyên liên tiếp, đoán bừa một id hợp
    # lệ là chuyện dễ.
    if intent.target is not None:
        thay = {o.subject.value for o in ctx.observations if o.subject is not None}
        if intent.target.value not in thay and intent.target.value != ctx.entity.value:
            return Rejection(
                "target_unobserved",
                f"thực thể chưa quan sát thấy `{intent.target.value}`",
            )

    return None


def validate_response(
    intents: list[IntentProposal], ctx: CognitionContext
) -> ValidationResult:
    """Kiểm toàn bộ ý định của một response.

    Trả về cả phần qua lẫn phần trượt thay vì ném ngoại lệ ở cái trượt đầu tiên.
    Lý do: một response có bốn ý định, ba tốt và một sai, vẫn dùng được ba cái
    tốt — và cái sai vẫn phải được ghi lại để `§20.10` biết model đang hỏng chỗ
    nào. Ném ngoại lệ sẽ vứt cả bốn và không ghi gì.
    """
    qua: list[IntentProposal] = []
    truot: list[tuple[IntentProposal, Rejection]] = []
    for i in intents:
        loi = validate_intent(i, ctx)
        if loi is None:
            qua.append(i)
        else:
            truot.append((i, loi))
    return ValidationResult(accepted=tuple(qua), rejected=tuple(truot))
