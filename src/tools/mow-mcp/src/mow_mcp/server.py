"""``mow-mcp`` — MCP server để agent code vào thế giới (`plan.md §P7.2`).

Đây là cách một agent như Claude Code "vào thế giới": tạo world từ worldseed,
đẩy thời gian tới đúng thời điểm cần xem, nhìn state thật, hỏi *vì sao chuyện
này xảy ra*, và kiểm chứng bằng bất biến.

Nhóm công cụ ở Giai đoạn 0 — World, Time, Query, Verify — là điều kiện hoàn
thành của ``P0-12``. Các nhóm còn lại ở `§P7.2` (Perception, Mutate, Snapshot,
Repro, Scenario, UI, Metrics) mở dần theo giai đoạn, và mỗi nhóm mở ra khi phần
engine tương ứng có thật — không phải trước.

Chạy từ Claude Code bằng cách thêm vào cấu hình MCP::

    {
      "mcpServers": {
        "mow": { "command": "uv", "args": ["run", "mow-mcp"] }
      }
    }
"""

from __future__ import annotations

import json
from typing import Any

from mcp.server.fastmcp import FastMCP  # type: ignore[attr-defined]

from .bridge import BridgeError, DebugBridge

mcp = FastMCP("mow")
_bridge = DebugBridge()


def _goi(tool: str, **args: Any) -> str:
    """Gọi phiên gỡ lỗi và trả JSON đã định dạng.

    Lỗi được trả về **dưới dạng văn bản có cấu trúc**, không phải exception:
    agent cần đọc được thông báo để tự sửa hướng đi, và một traceback Python
    không nói cho nó biết thế giới đang ở trạng thái nào.
    """
    try:
        return json.dumps(_bridge.call(tool, **args), ensure_ascii=False, indent=2)
    except BridgeError as e:
        return json.dumps({"error": str(e)}, ensure_ascii=False, indent=2)


# ── Nhóm World ───────────────────────────────────────────────────────────────


@mcp.tool()
def world_create(worldseed: str = "test:tiny_village", name: str = "w1") -> str:
    """Dựng một thế giới thử từ worldseed.

    Trả về tick, số thực thể, số sự kiện và state hash. State hash là thứ đáng
    ghi lại: nó là danh tính của thế giới ở thời điểm này, và so nó là cách rẻ
    nhất để biết hai lần chạy có giống nhau không.
    """
    return _goi("world_create", worldseed=worldseed, name=name)


@mcp.tool()
def world_list() -> str:
    """Liệt kê các thế giới đang mở trong phiên này."""
    return _goi("world_list")


@mcp.tool()
def world_drop(world: str = "w1") -> str:
    """Đóng một thế giới và giải phóng bộ nhớ."""
    return _goi("world_drop", world=world)


# ── Nhóm Time ────────────────────────────────────────────────────────────────


@mcp.tool()
def sim_step(ticks: int = 1, world: str = "w1") -> str:
    """Đẩy thời gian đi ``ticks`` tick thần.

    Đồng hồ thần là đồng hồ chủ; đồng hồ địa phương của mỗi thế giới chạy theo
    tỉ lệ riêng của nó (`§4.5`).
    """
    return _goi("sim_step", ticks=ticks, world=world)


# ── Nhóm Query ───────────────────────────────────────────────────────────────


@mcp.tool()
def query_entity(entity: int, world: str = "w1") -> str:
    """Đọc toàn bộ thuộc tính của một thực thể.

    Đây là **state thật**, không lọc theo tri giác của ai. Muốn xem thế giới
    qua mắt một nhân vật thì dùng ``debug_observe_as`` — sẽ có từ Giai đoạn C,
    khi tầng tri giác tồn tại.
    """
    return _goi("query_entity", entity=entity, world=world)


@mcp.tool()
def query_entities(kind: str | None = None, tag: str | None = None, world: str = "w1") -> str:
    """Tìm thực thể theo loại và tag."""
    args: dict[str, Any] = {"world": world}
    if kind:
        args["kind"] = kind
    if tag:
        args["tag"] = tag
    return _goi("query_entities", **args)


@mcp.tool()
def query_timeline(from_tick: int = 0, to_tick: int | None = None, world: str = "w1") -> str:
    """Đọc nhật ký sự kiện trong một khoảng tick."""
    args: dict[str, Any] = {"world": world, "from": from_tick}
    if to_tick is not None:
        args["to"] = to_tick
    return _goi("query_timeline", **args)


@mcp.tool()
def query_cause_chain(seq: int, world: str = "w1") -> str:
    """Truy ngược chuỗi nhân quả từ một sự kiện về nguyên nhân gốc.

    Trả lời câu hỏi *"vì sao chuyện này xảy ra"* bằng **event có thật**, không
    phải bằng suy đoán. Nếu một mắt xích thiếu thì nó thật sự thiếu — đừng đoán
    bù, vì một chuỗi nhân quả được đoán ra thì tệ hơn không có, người đọc sẽ
    tin nó.
    """
    return _goi("query_cause_chain", seq=seq, world=world)


# ── Nhóm Mutate ──────────────────────────────────────────────────────────────


@mcp.tool()
def debug_list_commands(world: str = "w1") -> str:
    """Liệt kê mọi loại command mà thế giới này nhận.

    Gọi cái này trước khi dùng ``debug_apply_command``: danh sách phụ thuộc vào
    content pack đang nạp, nên nó khác nhau giữa các thế giới.
    """
    return _goi("debug_list_commands", world=world)


@mcp.tool()
def debug_apply_command(kind: str, payload: dict[str, Any] | None = None, world: str = "w1") -> str:
    """Áp một command để dựng điều kiện tái hiện.

    Mọi can thiệp đi qua **đúng transaction handler** như hành động của NPC
    (`§22.1`). Không có đường ghi thẳng vào state, kể cả cho công cụ gỡ lỗi —
    nếu có, thế giới dựng bằng công cụ sẽ khác thế giới dựng bằng cách chơi, và
    bug tái hiện được ở một bên nhưng không ở bên kia.
    """
    return _goi("debug_apply_command", kind=kind, payload=payload or {}, world=world)


# ── Nhóm Verify ──────────────────────────────────────────────────────────────


@mcp.tool()
def assert_invariants(world: str = "w1") -> str:
    """Chạy toàn bộ bất biến ``INV-22-*`` lên state hiện tại.

    Một vi phạm **luôn là bug của engine**, không bao giờ là lỗi người chơi.
    Kết quả gồm danh sách bất biến đã kiểm và chi tiết từng vi phạm.
    """
    return _goi("assert_invariants", world=world)


@mcp.tool()
def assert_state_hash(expected: str | None = None, world: str = "w1") -> str:
    """Đọc state hash, hoặc khẳng định nó bằng một giá trị.

    Không truyền ``expected`` thì chỉ đọc. Truyền vào thì lệch sẽ báo lỗi — đây
    là cách rẻ nhất để khẳng định một chuỗi thao tác cho ra đúng thế giới đã
    thấy lần trước.
    """
    args: dict[str, Any] = {"world": world}
    if expected:
        args["expected"] = expected
    return _goi("assert_state_hash", **args)


@mcp.tool()
def list_invariants() -> str:
    """Liệt kê mọi bất biến, kèm mức chi phí và mô tả."""
    return _goi("list_invariants")


def main() -> None:
    """Điểm vào của lệnh ``mow-mcp``."""
    try:
        mcp.run()
    finally:
        _bridge.stop()


if __name__ == "__main__":
    main()
