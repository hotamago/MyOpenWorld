"""Chu trình nhận thức, đủ chín bước (`idea.md §10.4`, `§20.10`, `PC-02`, `PC-09`).

## Chín bước, và không có đường tắt

```text
1  trigger        engine phát hiện       ← đã xảy ra trước khi vào đây
2  observe        engine dựng quan sát   ← engine, KHÔNG phải model
3  recall         memory-service
4  build_prompt   nén context            §20.9
5  invoke         model trả theo schema  §10.4 b5
6  validate       kiểm reference         §22.4
7  plan           registry tính precond  ← engine, KHÔNG phải model
8  execute        simulation chạy        ← Rust
9  remember       ghi ký ức              §11
```

Bước 2 và bước 7 nằm ở phía engine, và đó là toàn bộ kiến trúc gói trong một
sơ đồ. Nếu model tự dựng quan sát, nó sẽ dựng ra thứ nhân vật không thấy. Nếu
model tự khẳng định precondition, nó sẽ khẳng định cái nó cần để kế hoạch của nó
chạy. Cả hai đều là những lời nói dối rất tự nhiên và rất khó phát hiện.

Graph ở đây chạy các bước **3–6 và 9**. Bước 1, 2, 7, 8 thuộc Rust; chúng có mặt
trong sơ đồ để chỗ nối rõ ràng, không phải để service này làm.

## Fallback là một nhánh của graph, không phải một khối `except`

`§20.10` đòi mỗi lần hạ cấp model và mỗi lần rơi về policy đều là **một event có
lý do**. Viết nó thành `try/except` quanh lời gọi model là cách tự nhiên và làm
mất đúng thứ đó: một exception đã bắt là một thứ không có mặt ở đâu cả. Ở đây,
đường fallback là một node có tên, và nó ghi một [FallbackEvent] có
``reason`` — nên câu hỏi *"vì sao hôm đó cả vùng này hành xử ngờ nghệch"* có câu
trả lời.
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from dataclasses import dataclass, field
from typing import Any, TypedDict, cast

from langgraph.graph import END, StateGraph

from .context import Budget, compress
from .generated.mow.cognition.v1 import (
    CognitionContext,
    CognitionResponse,
    FallbackEvent,
    FallbackReason,
    IntentProposal,
    RetrievedMemory,
)
from .validator import ValidationResult, validate_response

__all__ = [
    "CycleDeps",
    "CycleState",
    "ModelTimeoutError",
    "build_graph",
    "run_cycle",
]


class ModelTimeoutError(Exception):
    """Model không trả lời kịp. Mô phỏng **không** được dừng vì việc này."""


@dataclass(frozen=True, slots=True)
class CycleDeps:
    """Những thứ chu trình cần từ bên ngoài.

    Truyền vào thay vì import trực tiếp, để test chạy được mà không cần mạng —
    và quan trọng hơn, để đường fallback kiểm được. Một đường fallback chưa bao
    giờ chạy trong test là một đường fallback chưa bao giờ chạy.
    """

    recall: Callable[[CognitionContext], Sequence[RetrievedMemory]]
    invoke: Callable[[CognitionContext], CognitionResponse]
    remember: Callable[[CognitionContext, Sequence[IntentProposal]], None]
    budget: Budget = field(default_factory=Budget)
    # Hành động dự phòng khi không có model: chờ, tự vệ, hoặc tiếp tục thói quen.
    #
    # `§20.10`: "Entity dùng fallback plan hợp lý ... **không nhận quyền năng mới
    # vì model lỗi.**" Nên đây là một tên action, và nó vẫn phải qua validator.
    fallback_action: str = "core.wait"


class CycleState(TypedDict, total=False):
    """State chảy qua graph."""

    ctx: CognitionContext
    compressed: CognitionContext
    response: CognitionResponse
    validation: ValidationResult
    intents: list[IntentProposal]
    fallback: FallbackEvent | None
    tokens: int
    dropped: tuple[int, int]


def _node_recall(deps: CycleDeps) -> Callable[[CycleState], dict[str, Any]]:
    def f(state: CycleState) -> dict[str, Any]:
        ctx = state["ctx"]
        ctx.memories = list(deps.recall(ctx))
        return {"ctx": ctx}

    return f


def _node_build_prompt(deps: CycleDeps) -> Callable[[CycleState], dict[str, Any]]:
    def f(state: CycleState) -> dict[str, Any]:
        nen = compress(state["ctx"], deps.budget)
        return {
            "compressed": nen.context,
            "tokens": nen.tokens,
            "dropped": (nen.dropped_observations, nen.dropped_memories),
        }

    return f


def _node_invoke(deps: CycleDeps) -> Callable[[CycleState], dict[str, Any]]:
    def f(state: CycleState) -> dict[str, Any]:
        ctx = state["compressed"]
        try:
            r = deps.invoke(ctx)
        except ModelTimeoutError:
            return {"fallback": _fallback(ctx, FallbackReason.TIMEOUT, deps)}
        except Exception:
            # Bất kỳ lỗi nào khác cũng là breaker mở. Bắt rộng ở đây là có chủ
            # đích: `§20.10` nói mô phỏng **không được dừng** vì model lỗi, và
            # một loại exception chưa lường trước không phải lý do để cả thế giới
            # đứng lại.
            return {"fallback": _fallback(ctx, FallbackReason.BREAKER_OPEN, deps)}
        return {"response": r}

    return f


def _node_validate(deps: CycleDeps) -> Callable[[CycleState], dict[str, Any]]:
    def f(state: CycleState) -> dict[str, Any]:
        ctx = state["compressed"]
        r = state["response"]
        kq = validate_response(list(r.intents), ctx)
        if not kq.accepted:
            # Mọi ý định đều trượt: không có gì để làm, và đó là một fallback
            # phải ghi lại — không phải một chu trình "thành công mà rỗng".
            return {
                "validation": kq,
                "fallback": _fallback(ctx, FallbackReason.VALIDATION_FAILED, deps),
            }
        return {"validation": kq, "intents": list(kq.accepted)}

    return f


def _node_fallback(deps: CycleDeps) -> Callable[[CycleState], dict[str, Any]]:
    def f(state: CycleState) -> dict[str, Any]:
        ctx = state["compressed"]
        # Hành động dự phòng vẫn phải nằm trong những gì thực thể biết làm. Nếu
        # không, một model lỗi lại **mở rộng** khả năng của nhân vật.
        act = (
            deps.fallback_action
            if deps.fallback_action in ctx.available_actions
            else (ctx.available_actions[0] if ctx.available_actions else "")
        )
        intents = [IntentProposal(action=act, rationale="fallback")] if act else []
        return {"intents": intents}

    return f


def _node_remember(deps: CycleDeps) -> Callable[[CycleState], dict[str, Any]]:
    def f(state: CycleState) -> dict[str, Any]:
        deps.remember(state["compressed"], state.get("intents", []))
        return {}

    return f


def _fallback(ctx: CognitionContext, reason: int, deps: CycleDeps) -> FallbackEvent:
    """Dựng một [FallbackEvent].

    ``reason`` khai báo là ``int`` chứ không phải [FallbackReason] vì lớp
    ``betterproto.Enum`` dùng metaclass riêng, nên bộ kiểm kiểu đọc
    ``FallbackReason.TIMEOUT`` thành ``int`` thay vì thành một thành viên enum.
    Ép kiểu **một lần ở đây** thay vì ở từng chỗ gọi: chỗ gọi vẫn viết tên hằng
    đọc được, và chỗ duy nhất biết về khiếm khuyết của bộ sinh là chỗ này.
    """
    return FallbackEvent(
        request_id=ctx.request_id,
        entity=ctx.entity,
        reason=cast(FallbackReason, reason),
        routed_model="",
        # Rỗng nghĩa là rơi hẳn về policy. `§20.10` đòi ghi model **thật sự đã
        # dùng**, và "không dùng model nào" cũng là một câu trả lời.
        actual_model="",
        fallback_action=deps.fallback_action,
        at_tick=ctx.now,
    )


def _sau_invoke(state: CycleState) -> str:
    return "fallback" if state.get("fallback") is not None else "validate"


def _sau_validate(state: CycleState) -> str:
    return "fallback" if state.get("fallback") is not None else "remember"


def build_graph(deps: CycleDeps) -> Any:
    """Dựng graph của chu trình nhận thức."""
    g: Any = StateGraph(CycleState)
    g.add_node("recall", _node_recall(deps))
    g.add_node("build_prompt", _node_build_prompt(deps))
    g.add_node("invoke", _node_invoke(deps))
    g.add_node("validate", _node_validate(deps))
    g.add_node("fallback", _node_fallback(deps))
    g.add_node("remember", _node_remember(deps))

    g.set_entry_point("recall")
    g.add_edge("recall", "build_prompt")
    g.add_edge("build_prompt", "invoke")
    g.add_conditional_edges("invoke", _sau_invoke, {"fallback": "fallback", "validate": "validate"})
    g.add_conditional_edges(
        "validate", _sau_validate, {"fallback": "fallback", "remember": "remember"}
    )
    # Fallback vẫn đi qua `remember`. Một chu trình rơi về policy vẫn là một
    # chuyện đã xảy ra với nhân vật, và nhân vật vẫn nên nhớ nó đã chờ.
    g.add_edge("fallback", "remember")
    g.add_edge("remember", END)
    return g.compile()


def run_cycle(ctx: CognitionContext, deps: CycleDeps) -> CycleState:
    """Chạy một chu trình. Không bao giờ ném — mô phỏng không được dừng."""
    graph = build_graph(deps)
    return cast(CycleState, graph.invoke({"ctx": ctx}))
