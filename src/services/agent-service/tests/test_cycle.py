"""Test chu trình nhận thức (`PC-01`, `PC-02`, `PC-03`, `PC-04`, `PC-09`)."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any, cast

import pytest
from agent_service.context import Budget, compress, uoc_luong_token
from agent_service.cycle import CycleDeps, ModelTimeoutError, run_cycle
from agent_service.generated.mow.cognition.v1 import (
    CognitionContext,
    CognitionResponse,
    FallbackReason,
    IntentProposal,
    Observation,
    RetrievedMemory,
    Trigger,
)
from agent_service.generated.mow.common.v1 import BranchId, EntityId, Tick, WorldId
from agent_service.validator import validate_intent, validate_response


def quan_sat(i: int, subject: int | None = None, summary: str = "thấy gì đó") -> Observation:
    return Observation(
        id=f"obs-{i}",
        at_tick=Tick(value=i),
        subject=None if subject is None else EntityId(value=subject),
        summary=summary,
        channel="sight",
    )


def ky_uc(i: int, relevance: int, content: str = "nhớ gì đó") -> RetrievedMemory:
    return RetrievedMemory(
        id=f"mem-{i}", content=content, at_tick=Tick(value=i), relevance=relevance
    )


def ctx(
    *,
    observations: Sequence[Observation] = (),
    memories: Sequence[RetrievedMemory] = (),
    actions: Sequence[str] = ("core.wait", "core.walk", "core.eat"),
) -> CognitionContext:
    return CognitionContext(
        request_id="req-1",
        entity=EntityId(value=1),
        world=WorldId(value="w"),
        branch=BranchId(value="b"),
        now=Tick(value=100),
        trigger=cast(Trigger, Trigger.SCHEDULED),
        observations=list(observations),
        memories=list(memories),
        available_actions=list(actions),
        persona_summary="một người thợ rèn",
        persona_version="v1",
        active_goals=["sửa cái lò"],
        needs={"hunger": 400},
    )


# ───────────────────────── PC-04 · validator ─────────────────────────


def test_action_khong_biet_lam_thi_bi_tu_choi() -> None:
    """Danh mục action **toàn cục** sẽ cho một đứa trẻ đề nghị `forge_sword`."""
    c = ctx()
    loi = validate_intent(IntentProposal(action="core.forge_sword"), c)
    assert loi is not None
    assert loi.code == "action_unknown"


def test_evidence_tro_ra_ngoai_context_thi_bi_tu_choi() -> None:
    """Đây là thứ giữ cho `§20.6` batch được mà không rò ngữ cảnh.

    Không có nó, nhân vật A biện minh cho hành động của mình bằng một quan sát mà
    chỉ B mới thấy — và không có gì trông sai trong log.
    """
    c = ctx(observations=[quan_sat(1)])
    ok = IntentProposal(action="core.wait", evidence_refs=["obs-1"])
    assert validate_intent(ok, c) is None

    cheo = IntentProposal(action="core.wait", evidence_refs=["obs-999"])
    loi = validate_intent(cheo, c)
    assert loi is not None
    assert loi.code == "evidence_foreign"


def test_target_chua_quan_sat_thay_thi_bi_tu_choi() -> None:
    """`EntityId` là số nguyên liên tiếp, nên đoán bừa một id hợp lệ rất dễ."""
    c = ctx(observations=[quan_sat(1, subject=7)])
    assert validate_intent(IntentProposal(action="core.wait", target=EntityId(value=7)), c) is None

    loi = validate_intent(IntentProposal(action="core.wait", target=EntityId(value=8)), c)
    assert loi is not None
    assert loi.code == "target_unobserved"


def test_mot_y_dinh_sai_khong_vut_ca_response() -> None:
    """Ba ý định tốt vẫn dùng được, và cái sai vẫn phải được ghi lại."""
    c = ctx(observations=[quan_sat(1)])
    kq = validate_response(
        [
            IntentProposal(action="core.wait"),
            IntentProposal(action="core.bay_len_troi"),
            IntentProposal(action="core.walk"),
        ],
        c,
    )
    assert len(kq.accepted) == 2
    assert len(kq.rejected) == 1
    assert not kq.ok


# ───────────────────────── PC-03 · nén context ─────────────────────────


def test_quan_sat_moi_nhat_khong_bao_gio_bi_cat() -> None:
    """Cắt nó là nguồn gốc kinh điển của 'NPC đứng nhìn nhà cháy'."""
    qs = [quan_sat(i, summary="x" * 500) for i in range(20)]
    nen = compress(ctx(observations=qs), Budget(max_tokens=50, min_observations=3))

    giu = {o.id for o in nen.context.observations}
    assert giu >= {"obs-19", "obs-18", "obs-17"}, "ba quan sát mới nhất phải còn"
    assert nen.dropped_observations > 0, "test này chỉ có nghĩa khi thật sự có cắt"


def test_ky_uc_cat_theo_lien_quan_khong_theo_thoi_gian() -> None:
    """Ký ức quan trọng nhất của một người thường là ký ức cũ nhất."""
    ku = [
        ky_uc(1, relevance=900, content="y" * 100),  # cũ nhất, liên quan nhất
        ky_uc(50, relevance=100, content="y" * 100),
        ky_uc(99, relevance=50, content="y" * 100),
    ]
    nen = compress(ctx(memories=ku), Budget(max_tokens=100))
    giu = {m.id for m in nen.context.memories}
    assert "mem-1" in giu, "ký ức liên quan nhất bị cắt vì nó cũ"


def test_ky_uc_cung_diem_thi_thu_tu_van_xac_dinh() -> None:
    """Prompt đổi thì câu trả lời đổi, nên thứ tự phải xác định."""
    ku = [ky_uc(i, relevance=500) for i in range(10)]
    a = compress(ctx(memories=ku), Budget(max_tokens=100)).context.memories
    b = compress(ctx(memories=list(reversed(ku))), Budget(max_tokens=100)).context.memories
    assert [m.id for m in a] == [m.id for m in b]


def test_nen_khong_sua_context_goc() -> None:
    """Sửa tại chỗ khiến mỗi lần thử lại nhân vật lại biết ít hơn."""
    goc = ctx(observations=[quan_sat(i, summary="z" * 400) for i in range(10)])
    truoc = len(goc.observations)
    compress(goc, Budget(max_tokens=30, min_observations=1))
    assert len(goc.observations) == truoc


def test_uoc_luong_token_nghieng_ve_phia_thua() -> None:
    """Ước lượng thiếu làm cả chu trình rơi vào fallback — ồn ào hơn nhiều."""
    assert uoc_luong_token("abc") >= 1
    assert uoc_luong_token("a" * 300) >= 100
    # Tiếng Việt có dấu không được ước lượng thấp hơn tiếng Anh cùng độ dài.
    assert uoc_luong_token("nhân vật đang đói") >= uoc_luong_token("hungry character x")


def test_min_observations_khong_duoc_bang_khong() -> None:
    with pytest.raises(ValueError, match="min_observations"):
        Budget(min_observations=0)


# ───────────────────────── PC-02 / PC-09 · chu trình ─────────────────────────


def deps_ok(**kw: Any) -> CycleDeps:
    def recall(_c: CognitionContext) -> list[RetrievedMemory]:
        return [ky_uc(1, relevance=800)]

    def invoke(c: CognitionContext) -> CognitionResponse:
        return CognitionResponse(
            request_id=c.request_id,
            intents=[IntentProposal(action="core.walk", evidence_refs=["mem-1"])],
            model_used="test-model",
            prompt_id="p",
            prompt_version="1",
        )

    def remember(_c: CognitionContext, _i: Sequence[IntentProposal]) -> None:
        return None

    d: dict[str, Any] = {"recall": recall, "invoke": invoke, "remember": remember}
    d.update(kw)
    return CycleDeps(**d)


def test_chu_trinh_di_du_cac_buoc_va_tra_ve_y_dinh() -> None:
    ra = run_cycle(ctx(observations=[quan_sat(1)]), deps_ok())
    assert ra.get("fallback") is None
    assert [i.action for i in ra["intents"]] == ["core.walk"]
    # Ký ức được truy xuất phải thật sự tới được model, nếu không thì bước 3 của
    # `§10.4` chỉ là trang trí.
    assert [m.id for m in ra["compressed"].memories] == ["mem-1"]


def test_timeout_khong_dung_mo_phong_va_ghi_event() -> None:
    """`§20.10`: timeout không dừng simulation, và nó **phải** để lại dấu vết."""

    def timeout(_c: CognitionContext) -> CognitionResponse:
        raise ModelTimeoutError

    ra = run_cycle(ctx(observations=[quan_sat(1)]), deps_ok(invoke=timeout))
    fb = ra["fallback"]
    assert fb is not None
    assert fb.reason == FallbackReason.TIMEOUT
    assert fb.actual_model == "", "rơi hẳn về policy thì không có model nào được dùng"
    # Và nhân vật vẫn làm một việc gì đó hợp lý.
    assert [i.action for i in ra["intents"]] == ["core.wait"]


def test_loi_la_cua_provider_cung_thanh_event_chu_khong_lam_treo() -> None:
    def no(_c: CognitionContext) -> CognitionResponse:
        raise RuntimeError("provider 503")

    fb = run_cycle(ctx(observations=[quan_sat(1)]), deps_ok(invoke=no))["fallback"]
    assert fb is not None
    assert fb.reason == FallbackReason.BREAKER_OPEN


def test_moi_y_dinh_deu_truot_thi_van_la_mot_fallback_co_ly_do() -> None:
    """Không phải một chu trình 'thành công mà rỗng' — đó là cách bug biến mất."""

    def bay(c: CognitionContext) -> CognitionResponse:
        return CognitionResponse(
            request_id=c.request_id,
            intents=[IntentProposal(action="core.bay_len_troi")],
            model_used="m",
        )

    fb = run_cycle(ctx(observations=[quan_sat(1)]), deps_ok(invoke=bay))["fallback"]
    assert fb is not None
    assert fb.reason == FallbackReason.VALIDATION_FAILED


def test_fallback_khong_cho_nhan_vat_quyen_nang_moi() -> None:
    """`§20.10`: entity **không nhận quyền năng mới vì model lỗi**."""

    def timeout(_c: CognitionContext) -> CognitionResponse:
        raise ModelTimeoutError

    # Nhân vật này không biết `core.wait`.
    c = ctx(actions=["core.walk"])
    ra = run_cycle(c, deps_ok(invoke=timeout))
    assert all(i.action in c.available_actions for i in ra["intents"])


def test_fallback_van_di_qua_remember() -> None:
    """Một chu trình rơi về policy vẫn là chuyện đã xảy ra với nhân vật."""
    da_nho: list[str] = []

    def timeout(_c: CognitionContext) -> CognitionResponse:
        raise ModelTimeoutError

    def remember(_c: CognitionContext, i: Sequence[IntentProposal]) -> None:
        da_nho.append(",".join(x.action for x in i))

    run_cycle(ctx(observations=[quan_sat(1)]), deps_ok(invoke=timeout, remember=remember))
    assert da_nho == ["core.wait"]


def test_context_dua_vao_model_da_duoc_nen() -> None:
    """Model phải thấy context đã nén, không phải context thô."""
    thay: list[int] = []

    def invoke(c: CognitionContext) -> CognitionResponse:
        thay.append(len(c.observations))
        return CognitionResponse(
            request_id=c.request_id, intents=[IntentProposal(action="core.wait")]
        )

    qs = [quan_sat(i, summary="q" * 400) for i in range(30)]
    run_cycle(
        ctx(observations=qs),
        deps_ok(invoke=invoke, budget=Budget(max_tokens=60, min_observations=2)),
    )
    assert thay[0] < 30
