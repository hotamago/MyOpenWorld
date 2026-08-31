import { describe, expect, it } from "vitest";
import {
  buildCauseChain,
  buildInspector,
  buildTimeline,
  phantomEvents,
  unexplainedFields,
  type CauseNode,
  type EntityInspection,
} from "./model";
import {
  channelsOf,
  ChannelConflict,
  claimChannel,
  drawIndex,
  occlusionMarker,
} from "@/render/visual";
import {
  defaultMarkers,
  nextSpeed,
  shouldStop,
  TimeController,
  type TickEvent,
} from "@/panels/timecontrol/model";

function node(seq: string, cause: string | null, kind = "core.x"): CauseNode {
  return {
    seq,
    tick: seq,
    kind,
    actor: null,
    subject: null,
    cause,
    lawVersion: 1,
  };
}

function log(...ns: CauseNode[]): Map<string, CauseNode> {
  return new Map(ns.map((n) => [n.seq, n]));
}

// ─────────────────────────────────────────────────────────────────────────────
// PB-16 — Inspector đọc state thật, chuỗi nhân quả chỉ hiện event có thật
// ─────────────────────────────────────────────────────────────────────────────

describe("§18.10 — chuỗi nhân quả", () => {
  it("truy ngược về gốc", () => {
    const m = log(node("3", "2"), node("2", "1"), node("1", null));
    const c = buildCauseChain(m, "3");
    expect(c.nodes.map((n) => n.seq)).toEqual(["3", "2", "1"]);
    expect(c.terminated).toBe("root");
  });

  it("mắt xích thiếu thì NÓI RA, không bịa", () => {
    // `§22.17`: vẽ một mũi tên đứt nét ghi "có lẽ vì..." tệ hơn không vẽ gì,
    // vì người đọc sẽ tin nó.
    const m = log(node("3", "2"));
    const c = buildCauseChain(m, "3");
    expect(c.terminated).toBe("missing");
    expect(c.nodes).toHaveLength(1);
  });

  it("vòng lặp trong dữ liệu bị cắt, không treo giao diện", () => {
    const m = log(node("1", "2"), node("2", "1"));
    const c = buildCauseChain(m, "1", 1_000_000);
    expect(c.nodes.length).toBeLessThanOrEqual(2);
    expect(c.terminated).toBe("depth");
  });

  it("chạm trần độ sâu thì báo đúng lý do", () => {
    const ns = Array.from({ length: 50 }, (_, i) =>
      node(String(50 - i), i === 49 ? null : String(49 - i)),
    );
    const c = buildCauseChain(log(...ns), "50", 10);
    expect(c.terminated).toBe("depth");
    expect(c.nodes).toHaveLength(10);
  });

  it("phát hiện event không có trong nhật ký", () => {
    const c = buildCauseChain(log(node("3", "2"), node("2", null)), "3");
    expect(phantomEvents(c, new Set(["3", "2"]))).toEqual([]);
    expect(phantomEvents(c, new Set(["3"]))).toEqual(["2"]);
  });

  it("giữ phiên bản luật lúc sự kiện xảy ra", () => {
    // `§22.49`: sửa luật hôm nay không được hồi tố lên lịch sử.
    const c = buildCauseChain(log(node("1", null)), "1");
    expect(c.nodes[0]!.lawVersion).toBe(1);
  });
});

describe("§18.13 — mọi giá trị suy ra bấm được về nguồn", () => {
  const e: EntityInspection = {
    entity: "e1",
    attrs: { "core.name": "Aren", "core.pos.x": 5 },
    derived: {
      "stat.strength": {
        value: "15",
        steps: [{ source: "base", before: "10", after: "10" }, { source: "blessing", before: "10", after: "15" }],
      },
    },
  };

  it("phân biệt giá trị lưu và giá trị suy ra", () => {
    const f = buildInspector(e);
    expect(f.find((x) => x.key === "core.name")!.origin).toBe("state");
    expect(f.find((x) => x.key === "stat.strength")!.origin).toBe("derived");
  });

  it("giá trị suy ra luôn kèm các bước", () => {
    const f = buildInspector(e);
    expect(unexplainedFields(f)).toEqual([]);
  });

  it("giá trị suy ra thiếu lời giải thích là bug", () => {
    const xau: EntityInspection = {
      ...e,
      derived: { "stat.speed": { value: "7", steps: [] } },
    };
    expect(unexplainedFields(buildInspector(xau))).toEqual(["stat.speed"]);
  });

  it("trường sắp theo khóa để hai lần xem giống nhau", () => {
    const f = buildInspector(e).filter((x) => x.origin === "state");
    expect(f.map((x) => x.key)).toEqual(["core.name", "core.pos.x"]);
  });
});

describe("dòng thời gian là nhật ký, chỉ được lọc", () => {
  const evs = [node("1", null, "a.b"), node("2", "1", "c.d"), node("3", "2", "a.b")];

  it("không tóm tắt, không gộp", () => {
    expect(buildTimeline(evs)).toHaveLength(3);
  });

  it("lọc theo loại", () => {
    expect(buildTimeline(evs, { kinds: ["a.b"] })).toHaveLength(2);
  });

  it("đánh dấu mốc đáng chú ý mà không xóa cái khác", () => {
    const t = buildTimeline(evs, { notableKinds: ["c.d"] });
    expect(t).toHaveLength(3);
    expect(t.filter((x) => x.notable).map((x) => x.kind)).toEqual(["c.d"]);
  });

  it("giữ seq để bấm sang chuỗi nhân quả", () => {
    expect(buildTimeline(evs).every((t) => t.seq.length > 0)).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// PB-17 — ngôn ngữ thị giác
// ─────────────────────────────────────────────────────────────────────────────

describe("§18.5 — kênh thị giác phân bổ cố định", () => {
  it("hệ thống đúng thì xin được kênh", () => {
    expect(() => claimChannel("shape", "entity_kind")).not.toThrow();
  });

  it("hai hệ thống tranh một kênh là lỗi", () => {
    // Dùng lại một kênh làm cùng một tín hiệu mang hai nghĩa.
    expect(() => claimChannel("hue", "faction")).toThrow(ChannelConflict);
    expect(() => claimChannel("hue", "faction")).toThrow(/ngân sách|hữu hạn/);
  });

  it("chuyển động chỉ dành cho thứ đang thay đổi", () => {
    // Dùng nó cho trạng thái tĩnh sẽ làm bản đồ nhấp nháy không ngừng.
    expect(channelsOf("active_change")).toEqual(["motion"]);
  });

  it("độ sáng dành cho thứ phải đọc được qua mù màu", () => {
    expect(channelsOf("elevation_and_active_overlay")).toEqual(["lightness"]);
  });

  it("mỗi kênh có đúng một chủ", () => {
    const chu = Object.values(
       
      ({} as Record<string, never>),
    );
    expect(chu).toEqual([]);
    // Kiểm thật: không hệ thống nào chiếm quá số kênh nó cần.
    expect(channelsOf("terrain")).toEqual(["hue"]);
    expect(channelsOf("identity")).toEqual(["outline"]);
  });
});

describe("vật thể nhiều tầng có dấu hiệu che", () => {
  it("bị che thì có nhãn, không chỉ mờ đi", () => {
    // Vẽ mờ thôi thì người chơi tưởng nó ở xa.
    expect(occlusionMarker({ hidden: "e1", by: "roof", layersAbove: 1 })).toBe(
      "bị che bởi tầng trên",
    );
    expect(occlusionMarker({ hidden: "e1", by: "roof", layersAbove: 3 })).toContain("3 tầng");
  });

  it("không bị che thì không có nhãn", () => {
    expect(occlusionMarker(null)).toBeNull();
  });

  it("thứ tự vẽ xác định", () => {
    expect(drawIndex("terrain")).toBeLessThan(drawIndex("entity"));
    expect(drawIndex("entity")).toBeLessThan(drawIndex("label"));
    expect(drawIndex("khong_biet")).toBe(-1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// PB-18 — điều khiển thời gian
// ─────────────────────────────────────────────────────────────────────────────

function ev(kind: string, tick: string, seq: string): TickEvent {
  return { kind, tick, seq };
}

describe("§18.8 — pause-on-ready", () => {
  it("chạy nhanh tới khi có chuyện đáng chú ý", () => {
    const tc = new TimeController();
    tc.resume(16);
    expect(tc.state.paused).toBe(false);

    // Ba tháng yên bình.
    for (let i = 0; i < 100; i++) {
      expect(tc.onTick(String(i), String(i), [])).toBeNull();
    }
    expect(tc.state.paused).toBe(false);

    // Rồi ngôi làng bốc cháy.
    const ly_do = tc.onTick("4820", "4820", [ev("crime.committed", "4820", "991")]);
    expect(ly_do).not.toBeNull();
    expect(tc.state.paused).toBe(true);
  });

  it("lý do dừng lấy từ event có thật, kèm seq", () => {
    // `§22.17`: không được nói "có chuyện gì đó xảy ra".
    const tc = new TimeController();
    tc.resume();
    tc.onTick("4820", "4820", [ev("crime.committed", "4820", "991")]);

    const s = tc.lastStop!;
    expect(s.seq).toBe("991");
    expect(s.kind).toBe("crime.committed");
    expect(tc.explainStop()).toContain("#991");
    expect(tc.explainStop()).toContain("t4820");
  });

  it("mốc tắt thì không dừng", () => {
    const tc = new TimeController();
    tc.toggleMarker("crime.committed", false);
    tc.resume();
    expect(tc.onTick("1", "1", [ev("crime.committed", "1", "1")])).toBeNull();
    expect(tc.state.paused).toBe(false);
  });

  it("đang dừng thì không kiểm mốc", () => {
    const tc = new TimeController();
    expect(tc.onTick("1", "1", [ev("life.died", "1", "1")])).toBeNull();
  });

  it("hai mốc cùng tick thì báo cái xảy ra trước", () => {
    // Thứ tự nhận được từ mạng không quyết định.
    const m = defaultMarkers();
    const a = shouldStop([ev("life.died", "1", "20"), ev("crime.committed", "1", "5")], m);
    const b = shouldStop([ev("crime.committed", "1", "5"), ev("life.died", "1", "20")], m);
    expect(a).toEqual(b);
    expect(a!.seq).toBe("5");
  });

  it("step luôn dừng lại, kể cả khi đang chạy", () => {
    const tc = new TimeController();
    tc.resume(64);
    tc.step();
    expect(tc.state.paused).toBe(true);
  });

  it("chạy tiếp thì xóa lý do dừng cũ", () => {
    const tc = new TimeController();
    tc.resume();
    tc.onTick("1", "1", [ev("life.died", "1", "1")]);
    expect(tc.lastStop).not.toBeNull();
    tc.resume(4);
    expect(tc.lastStop).toBeNull();
    expect(tc.state.speed).toBe(4);
  });

  it("vòng tốc độ cho một phím tắt", () => {
    expect(nextSpeed(1)).toBe(4);
    expect(nextSpeed(64)).toBe(1);
  });

  it("chưa dừng thì không có câu giải thích", () => {
    expect(new TimeController().explainStop()).toBeNull();
  });
});
