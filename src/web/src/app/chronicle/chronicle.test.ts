/**
 * Bài kiểm cho `chronicle.ts`.
 *
 * Mỗi `describe` khớp đúng một lời hứa trong tài liệu module: gộp đúng, không
 * sửa mảng gốc, thứ tự giảm dần, `limit` cắt đúng, hạng đúng theo loại, chia
 * ngày không vỡ khi `ticksPerDay = 0`, định danh chuỗi lớn không bị bào tròn
 * qua `Number`, và một sự kiện vô danh vẫn ra một chương đọc được thay vì
 * biến mất hoặc hiện `undefined`.
 */
import { describe, expect, it } from "vitest";
import type { WorldEvent } from "@/api/game";
import { pulse, weave, type Chapter } from "./chronicle";

/** Dựng nhanh một `WorldEvent` giả, chỉ ghi đè phần bài test quan tâm. */
function ev(overrides: Partial<WorldEvent> & Pick<WorldEvent, "seq" | "tick" | "kind">): WorldEvent {
  return { actor: null, payload: {}, ...overrides };
}

describe("weave — gộp `core.entity.moved` liên tiếp của cùng người", () => {
  it("năm bước liền nhau trong cửa sổ gộp thành một chương duy nhất", () => {
    const names = new Map([["1", "Linnea"]]);
    const events: WorldEvent[] = [];
    for (let i = 0; i < 5; i++) {
      events.push(
        ev({ seq: 10 + i, tick: 100 + i, kind: "core.entity.moved", actor: "1", payload: { x: i, y: 0 } }),
      );
    }

    const chapters = weave(events, names);

    expect(chapters).toHaveLength(1);
    const c = chapters[0] as Chapter;
    expect(c.count).toBe(5);
    expect(c.from).toBe(100);
    expect(c.to).toBe(104);
    expect(c.weight).toBe(0);
    expect(c.who).toBe("Linnea");
    // Sự kiện tiêu biểu là bước **cuối** của chuyến đi, để "vì sao" trỏ vào
    // đúng thời điểm gần nhất — không phải bước đầu tiên.
    expect(c.seq).toBe(14);
    expect(c.slots.count).toBe(5);
  });

  it("cách nhau quá cửa sổ gộp thì tách thành hai chương", () => {
    const names = new Map([["1", "Linnea"]]);
    const events: WorldEvent[] = [
      ev({ seq: 1, tick: 0, kind: "core.entity.moved", actor: "1", payload: { x: 0, y: 0 } }),
      ev({ seq: 2, tick: 1000, kind: "core.entity.moved", actor: "1", payload: { x: 1, y: 0 } }),
    ];

    const chapters = weave(events, names, { window: 20 });

    expect(chapters).toHaveLength(2);
  });

  it("hai người đi cùng lúc không bị trộn chung một chương", () => {
    const names = new Map([
      ["1", "Linnea"],
      ["2", "Bora"],
    ]);
    const events: WorldEvent[] = [
      ev({ seq: 1, tick: 10, kind: "core.entity.moved", actor: "1", payload: { x: 0, y: 0 } }),
      ev({ seq: 2, tick: 10, kind: "core.entity.moved", actor: "2", payload: { x: 5, y: 5 } }),
      ev({ seq: 3, tick: 11, kind: "core.entity.moved", actor: "1", payload: { x: 1, y: 0 } }),
    ];

    const chapters = weave(events, names);

    expect(chapters).toHaveLength(2);
    const byWho = new Map(chapters.map((c) => [c.who, c]));
    expect(byWho.get("Linnea")?.count).toBe(2);
    expect(byWho.get("Bora")?.count).toBe(1);
  });
});

describe("weave — không sửa mảng đầu vào", () => {
  it("mảng và các phần tử của nó giữ nguyên sau khi gọi", () => {
    const names = new Map([["1", "Linnea"]]);
    const events: WorldEvent[] = [
      ev({ seq: 5, tick: 50, kind: "core.entity.moved", actor: "1", payload: { x: 1, y: 1 } }),
      ev({ seq: 1, tick: 10, kind: "npc.intended", actor: "1", payload: { intent: "goto.field" } }),
    ];
    const snapshot = JSON.parse(JSON.stringify(events)) as unknown;

    weave(events, names);

    expect(JSON.parse(JSON.stringify(events))).toEqual(snapshot);
    // Cũng phải giữ nguyên **thứ tự** gốc — `weave` sắp một bản sao, không
    // được `sort()` tại chỗ trên mảng người gọi đưa vào.
    expect(events[0]?.seq).toBe(5);
    expect(events[1]?.seq).toBe(1);
  });
});

describe("weave — thứ tự trả về và `limit`", () => {
  const names = new Map([["1", "Linnea"]]);
  const events: WorldEvent[] = [
    ev({ seq: 1, tick: 10, kind: "npc.intended", actor: "1", payload: { intent: "a" } }),
    ev({ seq: 2, tick: 30, kind: "npc.intended", actor: "1", payload: { intent: "b" } }),
    ev({ seq: 3, tick: 20, kind: "npc.intended", actor: "1", payload: { intent: "c" } }),
  ];

  it("luôn trả về mới nhất trước, bất kể thứ tự đầu vào", () => {
    const chapters = weave(events, names);
    expect(chapters.map((c) => c.to)).toEqual([30, 20, 10]);
  });

  it("`limit` cắt đúng số chương, vẫn giữ những chương mới nhất", () => {
    const chapters = weave(events, names, { limit: 2 });
    expect(chapters).toHaveLength(2);
    expect(chapters.map((c) => c.to)).toEqual([30, 20]);
  });

  it("không truyền `limit` thì trả về hết", () => {
    expect(weave(events, names)).toHaveLength(3);
  });
});

describe("weave — hạng (`weight`) đúng theo loại sự kiện", () => {
  const names = new Map([["1", "Linnea"]]);

  it("`core.entity.moved` (đã gộp) là nền, hạng 0", () => {
    const [c] = weave(
      [ev({ seq: 1, tick: 1, kind: "core.entity.moved", actor: "1", payload: { x: 0, y: 0 } })],
      names,
    );
    expect(c?.weight).toBe(0);
  });

  it("`npc.intended` là hạng 1 — nó nói vì sao", () => {
    const [c] = weave(
      [ev({ seq: 1, tick: 1, kind: "npc.intended", actor: "1", payload: { intent: "goto.field" } })],
      names,
    );
    expect(c?.weight).toBe(1);
  });

  it("`core.entity.spawned` là biến cố, hạng 2", () => {
    const [c] = weave(
      [ev({ seq: 1, tick: 1, kind: "core.entity.spawned", payload: { kind: "villager" } })],
      names,
    );
    expect(c?.weight).toBe(2);
  });

  it("`truegod.intervened` luôn là biến cố, hạng 2 — bàn tay thần luôn đáng ghi", () => {
    const [c] = weave(
      [ev({ seq: 1, tick: 1, kind: "truegod.intervened", payload: { key: "need.hunger" } })],
      names,
    );
    expect(c?.weight).toBe(2);
  });
});

describe("pulse — chia ngày không vỡ khi `ticksPerDay = 0`", () => {
  it("mọi sự kiện rơi về ngày 0, không có `NaN`/`Infinity` nào", () => {
    const events: WorldEvent[] = [
      ev({ seq: 1, tick: 100, kind: "npc.intended" }),
      ev({ seq: 2, tick: 5_000, kind: "npc.intended" }),
    ];

    const buckets = pulse(events, 0);

    expect(buckets).toEqual([{ day: 0, count: 2 }]);
    for (const b of buckets) {
      expect(Number.isFinite(b.day)).toBe(true);
      expect(Number.isFinite(b.count)).toBe(true);
    }
  });

  it("`ticksPerDay` dương thì chia ngày đúng và sắp tăng dần", () => {
    const ticksPerDay = 100;
    const events: WorldEvent[] = [
      ev({ seq: 1, tick: 250, kind: "npc.intended" }), // ngày 2
      ev({ seq: 2, tick: 10, kind: "npc.intended" }), // ngày 0
      ev({ seq: 3, tick: 260, kind: "npc.intended" }), // ngày 2
    ];

    expect(pulse(events, ticksPerDay)).toEqual([
      { day: 0, count: 1 },
      { day: 2, count: 2 },
    ]);
  });
});

describe("định danh chuỗi lớn giữ nguyên, không bị `Number` bào tròn", () => {
  it("hai id 64-bit sát nhau (khác `Number` một đơn vị dưới `2^53`) vẫn tách biệt", () => {
    // `2^53 + 1` và `2^53 + 2` cùng làm tròn về một `Number` — đúng cái bẫy
    // `§22.10` cấm. Nếu `weave`/`nameOf` lỡ ép actor qua `Number` ở đâu đó,
    // bài test này sẽ trộn hai người thành một.
    const a = "9007199254740993";
    const b = "9007199254740994";
    const names = new Map([
      [a, "Người Lớn"],
      [b, "Người Nhỏ"],
    ]);
    const events: WorldEvent[] = [
      ev({ seq: 1, tick: 1, kind: "npc.intended", actor: a, payload: { intent: "eat" } }),
      ev({ seq: 2, tick: 2, kind: "npc.intended", actor: b, payload: { intent: "sleep" } }),
    ];

    const chapters = weave(events, names);

    expect(chapters).toHaveLength(2);
    const who = chapters.map((c) => c.who).sort();
    expect(who).toEqual(["Người Lớn", "Người Nhỏ"].sort());
  });

  it("cuộc đi của một id 64-bit lớn vẫn gộp đúng theo chính id đó, không theo id bị bào tròn", () => {
    const big = "9007199254740993";
    const names = new Map([[big, "Người Lớn"]]);
    const events: WorldEvent[] = [
      ev({ seq: 1, tick: 1, kind: "core.entity.moved", actor: big, payload: { x: 0, y: 0 } }),
      ev({ seq: 2, tick: 2, kind: "core.entity.moved", actor: big, payload: { x: 1, y: 0 } }),
    ];

    const chapters = weave(events, names);

    expect(chapters).toHaveLength(1);
    expect(chapters[0]?.count).toBe(2);
    expect(chapters[0]?.who).toBe("Người Lớn");
  });
});

describe("sự kiện không tra được tên vẫn ra một chương đọc được", () => {
  it("`truegod.intervened` — `actor` luôn null theo đúng hình dạng thật của sự kiện này", () => {
    const [c] = weave(
      [ev({ seq: 9, tick: 9, kind: "truegod.intervened", actor: null, payload: { key: "need.hunger" } })],
      new Map(),
    );

    expect(c).toBeDefined();
    expect(c?.who).toBeNull();
    expect(c?.weight).toBe(2);
    expect(c?.key).toBe("chronicle.intervened");
    expect(c?.slots.key).toBe("need.hunger");
    // Không được lộ chữ "undefined" ra bất kỳ đâu trong chương.
    expect(JSON.stringify(c)).not.toMatch(/undefined/);
  });

  it("có `actor` nhưng không tra được tên (id lạ với `names`) vẫn ra chương, `who: null`", () => {
    const [c] = weave(
      [ev({ seq: 1, tick: 1, kind: "npc.intended", actor: "999", payload: { intent: "goto.well" } })],
      new Map(), // bảng tên rỗng: id "999" không nằm trong đó
    );

    expect(c).toBeDefined();
    expect(c?.who).toBeNull();
    expect(c?.key).toBe("chronicle.intent.unknown");
    expect(c?.slots.intent).toBe("goto.well");
    expect(JSON.stringify(c)).not.toMatch(/undefined/);
  });

  it("một loại sự kiện hoàn toàn lạ vẫn ra chương, mang theo đúng `kind` thật", () => {
    const [c] = weave(
      [ev({ seq: 1, tick: 1, kind: "mypack.ritual.completed", actor: null, payload: {} })],
      new Map(),
    );

    expect(c).toBeDefined();
    expect(c?.key).toBe("chronicle.other.unknown");
    expect(c?.slots.kind).toBe("mypack.ritual.completed");
    expect(JSON.stringify(c)).not.toMatch(/undefined/);
  });

  it("mảng sự kiện rỗng cho ra mảng chương rỗng, không lỗi", () => {
    expect(weave([], new Map())).toEqual([]);
  });
});
