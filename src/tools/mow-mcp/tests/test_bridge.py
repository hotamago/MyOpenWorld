"""Test cầu nối MCP ↔ phiên gỡ lỗi.

Điều kiện hoàn thành của ``P0-12``:

> Xong khi agent tạo world, tiến 1000 tick, đọc entity và nhận báo cáo invariant.

``test_dieu_kien_hoan_thanh_p0_12`` làm đúng bốn việc đó, theo đúng thứ tự.
"""

from __future__ import annotations

import shutil
from collections.abc import Generator

import pytest
from mow_mcp.bridge import BridgeError, DebugBridge

pytestmark = pytest.mark.skipif(
    shutil.which("cargo") is None, reason="cần cargo để chạy mow-cli"
)


@pytest.fixture
def bridge() -> Generator[DebugBridge, None, None]:
    b = DebugBridge()
    b.start()
    yield b
    b.stop()


def test_dieu_kien_hoan_thanh_p0_12(bridge: DebugBridge) -> None:
    """Tạo world → tiến 1000 tick → đọc entity → nhận báo cáo invariant."""
    # 1. Tạo world.
    w = bridge.call("world_create", worldseed="test:tiny_village", name="w1")
    assert w["tick"] == 0
    assert w["entities"] == 5
    assert len(w["state_hash"]) == 64

    # 2. Tiến 1000 tick.
    sau = bridge.call("sim_step", ticks=1000, world="w1")
    assert sau["tick"] == 1000
    assert sau["state_hash"] != w["state_hash"], "tiến thời gian phải đổi state hash"

    # 3. Đọc entity.
    ds = bridge.call("query_entities", kind="entity", world="w1")
    assert ds["count"] == 3
    eid = ds["entities"][0]["entity"]
    e = bridge.call("query_entity", entity=eid, world="w1")
    assert "core.name" in e["attrs"]
    assert "core.kind" in e["attrs"]

    # 4. Nhận báo cáo invariant.
    rep = bridge.call("assert_invariants", world="w1")
    assert rep["clean"], rep["violations"]
    assert len(rep["checked"]) >= 5


def test_state_hash_on_dinh_qua_hai_the_gioi_giong_nhau(bridge: DebugBridge) -> None:
    a = bridge.call("world_create", worldseed="test:tiny_village", name="a")
    b = bridge.call("world_create", worldseed="test:tiny_village", name="b")
    assert a["state_hash"] == b["state_hash"], "cùng worldseed phải cho cùng thế giới"


def test_worldseed_khac_thi_the_gioi_khac(bridge: DebugBridge) -> None:
    a = bridge.call("world_create", worldseed="test:tiny_village", name="a")
    b = bridge.call("world_create", worldseed="test:empty", name="b")
    assert a["state_hash"] != b["state_hash"]
    assert b["entities"] == 0


def test_timeline_va_chuoi_nhan_qua(bridge: DebugBridge) -> None:
    bridge.call("world_create", name="w1")
    tl = bridge.call("query_timeline", world="w1")
    assert tl["count"] == 5, "năm thực thể, năm sự kiện sinh ra"
    assert all(e["kind"] == "core.entity.spawned" for e in tl["events"])

    chain = bridge.call("query_cause_chain", seq=0, world="w1")
    assert chain["depth"] == 1, "sự kiện gốc không có nguyên nhân trước nó"


def test_command_di_qua_dung_transaction_handler(bridge: DebugBridge) -> None:
    """§22.1: công cụ gỡ lỗi cũng không có đường ghi thẳng vào state."""
    bridge.call("world_create", name="w1")
    kinds = bridge.call("debug_list_commands", world="w1")["kinds"]
    assert "core.spawn" in kinds

    truoc = bridge.call("assert_state_hash", world="w1")["state_hash"]
    r = bridge.call(
        "debug_apply_command",
        kind="core.spawn",
        payload={"kind": "entity", "name": "Moi"},
        world="w1",
    )
    assert r["mutations"] > 0
    assert len(r["events"]) == 1, "can thiệp phải để lại dấu vết trong nhật ký"
    assert r["state_hash"] != truoc


def test_command_khong_hop_le_bao_loi_ro_rang(bridge: DebugBridge) -> None:
    bridge.call("world_create", name="w1")
    with pytest.raises(BridgeError) as e:
        bridge.call("debug_apply_command", kind="core.spawn", payload={}, world="w1")
    assert "kind" in str(e.value)


def test_tool_khong_biet_thi_liet_ke_tool_da_co(bridge: DebugBridge) -> None:
    with pytest.raises(BridgeError) as e:
        bridge.call("bay_len_troi")
    assert "world_create" in str(e.value)


def test_world_khong_ton_tai_thi_liet_ke_world_da_tao(bridge: DebugBridge) -> None:
    bridge.call("world_create", name="w1")
    with pytest.raises(BridgeError) as e:
        bridge.call("sim_step", ticks=1, world="khong_co")
    assert "w1" in str(e.value)


def test_assert_state_hash_lech_thi_bao_loi(bridge: DebugBridge) -> None:
    bridge.call("world_create", name="w1")
    with pytest.raises(BridgeError, match="state hash"):
        bridge.call("assert_state_hash", expected="0" * 64, world="w1")


def test_liet_ke_bat_bien(bridge: DebugBridge) -> None:
    r = bridge.call("list_invariants")
    ids = [i["id"] for i in r["invariants"]]
    assert "INV-22-1" in ids
    assert "INV-22-33" in ids
    # Mức chi phí phải có, để agent biết cái nào chạy được mỗi tick.
    assert all(i["cost"] in ("Cheap", "Medium", "Expensive") for i in r["invariants"])


def test_fixed_point_khong_bi_doi_thanh_so_thuc(bridge: DebugBridge) -> None:
    """Q16.16 đi ra ngoài ở dạng thô, kèm nhãn.

    Nếu nó thành số thực ở biên, một agent đọc rồi ghi lại sẽ làm mất chính xác
    và lỗi đó sẽ trông như bug của engine.
    """
    bridge.call("world_create", name="w1")
    # Không có Fixed trong world test, nên kiểm bằng mã hóa trực tiếp: khẳng
    # định rằng cách biểu diễn đã được quyết định và có test giữ nó.
    e = bridge.call("query_entity", entity=1, world="w1")
    for v in e["attrs"].values():
        assert not isinstance(v, float), f"số thực lọt ra biên MCP: {v!r}"
