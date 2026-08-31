"""Test gộp request (`PC-19`, `§20.6`)."""

from __future__ import annotations

from typing import cast

import pytest
from agent_service.batching import (
    Batch,
    BatchPolicy,
    NoBatchReason,
    delimit,
    plan_batches,
)
from agent_service.generated.mow.cognition.v1 import CognitionContext, Trigger
from agent_service.generated.mow.common.v1 import BranchId, EntityId, Tick, WorldId


def ctx(
    e: int,
    *,
    trigger: int = Trigger.SCHEDULED,
    persona: str = "nông dân",
) -> CognitionContext:
    return CognitionContext(
        request_id=f"r{e}",
        entity=EntityId(value=e),
        world=WorldId(value="w"),
        branch=BranchId(value="b"),
        now=Tick(value=1),
        trigger=cast(Trigger, trigger),
        available_actions=["core.wait"],
        persona_summary=persona,
        persona_version="v1",
    )


def test_nhieu_request_ngan_duoc_gop_lam_mot() -> None:
    bs = plan_batches([ctx(i) for i in range(5)])
    assert len(bs) == 1
    assert len(bs[0].contexts) == 5
    assert bs[0].solo_reason is None


def test_doi_thoai_khong_bao_gio_bi_gop() -> None:
    """Gộp làm ai cũng 'trả lời' cùng một lúc, và thứ tự phát ngôn mất."""
    bs = plan_batches([ctx(1), ctx(2, trigger=Trigger.DIALOGUE), ctx(3)])
    solo = [b for b in bs if b.is_solo and b.solo_reason is not None]
    assert len(solo) == 1
    assert solo[0].solo_reason == NoBatchReason.DIALOGUE_ORDER


def test_bi_mat_can_cach_ly_thi_di_mot_minh() -> None:
    """Cách ly tri thức **là** luật chơi, không phải một tối ưu bỏ được."""
    bs = plan_batches([ctx(1), ctx(2), ctx(3)], secrets=frozenset({2}))
    for b in bs:
        ids = {c.entity.value for c in b.contexts}
        if 2 in ids:
            assert ids == {2}, "thực thể giữ bí mật không được ở chung batch"
            assert b.solo_reason == NoBatchReason.SECRET_ISOLATION


def test_stakes_cao_di_mot_minh() -> None:
    for t in (Trigger.GOAL_CONFLICT, Trigger.MAJOR_EVENT):
        bs = plan_batches([ctx(1, trigger=t)])
        assert bs[0].solo_reason == NoBatchReason.HIGH_STAKES


def test_context_qua_dai_di_mot_minh() -> None:
    """Lỗi ở một phần lan ra cả batch nếu để nó lấn át những cái ngắn."""
    dai = ctx(9, persona="x" * 9_000)
    bs = plan_batches([ctx(1), dai, ctx(2)], BatchPolicy(solo_above_tokens=100))
    lon = [b for b in bs if b.solo_reason == NoBatchReason.CONTEXT_TOO_LONG]
    assert len(lon) == 1
    assert lon[0].contexts[0].entity.value == 9


def test_ngan_sach_theo_tong_do_dai_khong_theo_so_thuc_the() -> None:
    """Tám nông dân đang ngủ khác tám tướng lĩnh giữa trận."""
    ngan = [ctx(i) for i in range(8)]
    assert len(plan_batches(ngan, BatchPolicy(max_tokens=10_000))) == 1

    vua = [ctx(i, persona="y" * 300) for i in range(8)]
    bs = plan_batches(vua, BatchPolicy(max_tokens=400, solo_above_tokens=400))
    assert len(bs) > 1, "cùng số thực thể nhưng dài hơn thì phải chia nhỏ"


def test_tran_so_thuc_the_van_duoc_ton_trong() -> None:
    bs = plan_batches([ctx(i) for i in range(20)], BatchPolicy(max_entities=3))
    assert all(len(b.contexts) <= 3 for b in bs)


def test_thu_tu_dau_vao_duoc_giu_nguyen() -> None:
    """Một thuật toán gộp 'tối ưu' có sắp lại sẽ phá `§22.9`."""
    vao = [ctx(i) for i in range(12)]
    bs = plan_batches(vao, BatchPolicy(max_entities=3))
    ra = [c.entity.value for b in bs for c in b.contexts]
    assert ra == list(range(12))


def test_chia_batch_xac_dinh_giua_hai_lan_chay() -> None:
    vao = [ctx(i) for i in range(9)]
    a = plan_batches(vao, BatchPolicy(max_entities=4))
    b = plan_batches(vao, BatchPolicy(max_entities=4))
    assert [[c.entity.value for c in x.contexts] for x in a] == [
        [c.entity.value for c in x.contexts] for x in b
    ]


def test_cau_hinh_gop_ma_thuc_ra_khong_gop_bi_tu_choi() -> None:
    """`solo_above > max` cho ra những batch một phần tử **không có lý do**.

    Chúng trông như batch bình thường trong log, nên `§20.10` không trả lời được
    câu 'vì sao hôm đó tốn nhiều lời gọi thế'.
    """
    with pytest.raises(ValueError, match="solo_above_tokens"):
        BatchPolicy(max_tokens=100, solo_above_tokens=200)


def test_delimiter_mang_id_de_validator_doi_chieu_duoc() -> None:
    s = delimit(ctx(42))
    assert "42" in s
    assert s.count("42") == 2, "ID phải có ở cả delimiter mở lẫn đóng"


def test_batch_rong_khong_sinh_ra_batch_nao() -> None:
    assert plan_batches([]) == []


def test_moi_batch_bao_dung_so_token() -> None:
    bs: list[Batch] = plan_batches([ctx(i) for i in range(4)])
    assert bs[0].tokens > 0
