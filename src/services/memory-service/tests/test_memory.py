"""Test memory-service (`PC-05`, `PC-06`, `PC-07`)."""

from __future__ import annotations

import pytest
from memory_service.acl import AclScope, MemoryRecord, filter_visible
from memory_service.index import MemoryIndex


def rec(
    i: str,
    *,
    tick: int,
    branch: str = "main",
    ns: str = "entity:1",
    content: str = "con dao trong bep",
    tomb: frozenset[str] = frozenset(),
) -> MemoryRecord:
    return MemoryRecord(
        id=i,
        namespace=ns,
        content=content,
        created_tick=tick,
        source_event_seq=tick,
        persona_version="v1",
        created_branch=branch,
        tombstoned_in_branches=tomb,
    )


def scope(
    *,
    branch: str = "main",
    ancestry: tuple[str, ...] = ("main",),
    now: int = 1_000,
    fork_ticks: dict[str, int] | None = None,
    ns: str = "entity:1",
) -> AclScope:
    return AclScope(
        namespace=ns,
        branch=branch,
        ancestry=ancestry,
        persona_version="v1",
        now=now,
        fork_ticks=fork_ticks or {},
    )


# ───────────────────────────── PC-05 · ACL ─────────────────────────────


def test_namespace_cach_ly_tuyet_doi() -> None:
    """`§11.5`: đừng vô tình thừa kế ký ức của identity khác."""
    r = [rec("a", tick=1), rec("b", tick=2, ns="entity:2")]
    thay = filter_visible(r, scope())
    assert [x.id for x in thay] == ["a"]


def test_ancestry_rong_bi_tu_choi_ngay() -> None:
    """Rỗng sẽ lọc sạch mọi ký ức và nhân vật mất trí nhớ hoàn toàn — im lặng."""
    with pytest.raises(ValueError, match="ancestry rỗng"):
        AclScope(
            namespace="n",
            branch="main",
            ancestry=(),
            persona_version="v1",
            now=0,
            fork_ticks={},
        )


def test_ancestry_phai_ket_thuc_bang_nhanh_hien_tai() -> None:
    with pytest.raises(ValueError, match="kết thúc bằng nhánh hiện tại"):
        AclScope(
            namespace="n",
            branch="child",
            ancestry=("main",),
            persona_version="v1",
            now=0,
            fork_ticks={},
        )


def test_nhanh_con_thay_ky_uc_cha_truoc_diem_fork() -> None:
    r = [rec("cu", tick=10), rec("moi", tick=900)]
    s = scope(branch="child", ancestry=("main", "child"), fork_ticks={"child": 100})
    assert [x.id for x in filter_visible(r, s)] == ["cu"]


def test_nhanh_con_khong_thay_ky_uc_cha_tao_sau_diem_fork() -> None:
    """Vế hay bị bỏ quên, và là vế khiến hai vế kia có nghĩa.

    Không có nó, nhánh con biết những chuyện chỉ xảy ra ở một dòng thời gian khác.
    """
    r = [rec("sau_fork", tick=500)]
    s = scope(branch="child", ancestry=("main", "child"), fork_ticks={"child": 100})
    assert filter_visible(r, s) == []


def test_ky_uc_moi_cua_nhanh_con_khong_ro_nguoc_sang_cha() -> None:
    r = [rec("cua_con", tick=200, branch="child")]
    assert filter_visible(r, scope(branch="main", ancestry=("main",))) == []


def test_ky_uc_anh_em_khong_thay_nhau() -> None:
    """Fork ra hai nhánh: cái này không được biết cái kia."""
    r = [rec("cua_a", tick=200, branch="a")]
    s = scope(branch="b", ancestry=("main", "b"), fork_ticks={"b": 100, "a": 100})
    assert filter_visible(r, s) == []


def test_tombstone_o_nhanh_nay_khong_anh_huong_nhanh_khac() -> None:
    """Quên ở một dòng thời gian không phải quên ở mọi dòng thời gian."""
    r = [rec("x", tick=10, tomb=frozenset({"child"}))]
    assert [y.id for y in filter_visible(r, scope())] == ["x"]
    s = scope(branch="child", ancestry=("main", "child"), fork_ticks={"child": 100})
    assert filter_visible(r, s) == []


def test_thu_tu_tra_ve_on_dinh_du_dau_vao_xao_tron() -> None:
    """Đầu vào tới từ vector store; thứ tự của nó không có gì đảm bảo."""
    r = [rec("c", tick=3), rec("a", tick=1), rec("b", tick=2)]
    a = [x.id for x in filter_visible(r, scope())]
    b = [x.id for x in filter_visible(list(reversed(r)), scope())]
    assert a == b == ["a", "b", "c"]


# ──────────────────────── PC-06 · dựng lại chỉ mục ────────────────────────


def test_xoa_sach_roi_rebuild_khong_mat_du_lieu() -> None:
    """Nếu mất chỉ mục mà mất dữ liệu thì nó đã là nguồn sự thật thứ hai."""
    goc = [rec(f"m{i}", tick=i, content=f"chuyen so {i}") for i in range(1, 21)]
    idx = MemoryIndex()
    for r in goc:
        idx.add(r)
    assert len(idx) == 20

    bc = idx.rebuild(goc)
    assert bc.records_indexed == 20
    assert bc.lost == 0
    assert len(idx) == 20


def test_rebuild_tu_bang_ghi_authoritative_chu_khong_tu_chinh_no() -> None:
    """Chỉ mục mất sạch vẫn dựng lại được từ SQL."""
    goc = [rec("a", tick=1), rec("b", tick=2)]
    idx = MemoryIndex()
    assert len(idx) == 0
    idx.rebuild(goc)
    assert len(idx) == 2


# ─────────────────────────── PC-07 · tombstone ───────────────────────────


def test_quen_roi_thi_khong_truy_van_ra_nua() -> None:
    idx = MemoryIndex()
    idx.add(rec("bi_mat", tick=10, content="con dao trong bep"))
    assert idx.query("con dao", scope())

    assert idx.invalidate("bi_mat", "main")
    assert idx.query("con dao", scope()) == []


def test_tombstone_khong_xoa_vat_ly() -> None:
    """`§11.5` đòi việc quên phải truy ngược được; xóa vật lý còn phá replay."""
    idx = MemoryIndex()
    idx.add(rec("x", tick=1))
    idx.invalidate("x", "main")
    # Điểm vẫn còn trong chỉ mục, chỉ là không nhìn thấy từ nhánh đó.
    assert len(idx) == 1


def test_vector_stale_khong_tra_ve_trong_khoang_rebuild() -> None:
    """Bug này sống sót qua mọi lần thử tay vì cửa sổ của nó rất ngắn.

    Nếu `rebuild` index trước rồi mới áp tombstone, thì trong khoảng giữa hai
    bước, truy vấn trả về đúng cái ký ức người chơi vừa yêu cầu xóa.
    """
    idx = MemoryIndex()
    idx.add(rec("bi_mat", tick=10, content="con dao trong bep"))
    idx.invalidate("bi_mat", "main")

    # Bản ghi authoritative mang theo tombstone.
    goc = [rec("bi_mat", tick=10, content="con dao trong bep", tomb=frozenset({"main"}))]
    bc = idx.rebuild(goc)

    assert bc.tombstones_applied == 1
    assert idx.query("con dao", scope()) == [], "ký ức đã quên quay lại sau rebuild"


def test_rebuild_khong_lam_song_lai_ky_uc_da_quen_o_moi_nhanh() -> None:
    goc = [
        rec("a", tick=1, content="chuyen a", tomb=frozenset({"main"})),
        rec("b", tick=2, content="chuyen b"),
    ]
    idx = MemoryIndex()
    idx.rebuild(goc)
    ra = idx.query("chuyen a chuyen b", scope())
    assert [h.record.id for h in ra] == ["b"]


# ───────────────────────── truy vấn đi qua ACL ─────────────────────────


def test_truy_van_loc_acl_truoc_xep_hang_sau() -> None:
    """Lọc sau khi xếp hạng làm điểm của kết quả hợp lệ phụ thuộc vào nội dung
    mà thực thể không được thấy."""
    idx = MemoryIndex()
    idx.add(rec("cua_toi", tick=1, content="con dao trong bep"))
    idx.add(rec("cua_nguoi_khac", tick=1, ns="entity:2", content="con dao trong bep"))

    ra = idx.query("con dao", scope())
    assert [h.record.id for h in ra] == ["cua_toi"]


def test_ket_qua_truy_van_xac_dinh_giua_hai_lan_chay() -> None:
    idx = MemoryIndex()
    for i in range(10):
        idx.add(rec(f"m{i}", tick=i, content="con dao trong bep"))
    a = [h.record.id for h in idx.query("con dao", scope())]
    b = [h.record.id for h in idx.query("con dao", scope())]
    assert a == b
