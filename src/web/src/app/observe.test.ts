/**
 * Test cho chế độ quan sát (`observe.ts`).
 *
 * Hai cụm: `lifeOf` (lọc + xếp hạng vai trò từ dữ liệu sự kiện thật, không
 * bịa trường) và `followStep` (vùng chết / nhảy camera). Cả hai đều thuần nên
 * test dựng dữ liệu tay, không cần server hay Vue.
 */

import { describe, expect, it } from "vitest";
import type { WorldEvent } from "@/api/game";
import { followStep, lifeOf } from "./observe";

/** Id chuỗi lớn thật sự — vượt `2^53`, nên nếu ai đó lỡ `Number(id)` ở đâu đó
 *  trong đường đi, hai id khác nhau sẽ trộn làm một mà không có gì báo. */
const HUGE_A = "9007199254740993";
const HUGE_B = "9007199254740994"; // = HUGE_A + 1, nhưng `Number()` cả hai đều thành 9007199254740992.

function ev(partial: Partial<WorldEvent> & { seq: number }): WorldEvent {
  return {
    tick: partial.seq * 10,
    kind: "core.entity.moved",
    actor: null,
    payload: {},
    ...partial,
  };
}

describe("lifeOf", () => {
  it("chỉ giữ sự kiện liên quan tới id, bỏ phần còn lại", () => {
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: "a" }),
      ev({ seq: 2, actor: "b" }),
      ev({ seq: 3, actor: "a" }),
    ];
    const life = lifeOf("a", events);
    expect(life.map((l) => l.seq)).toEqual([3, 1]);
  });

  it("actor === id thì vai là actor", () => {
    const life = lifeOf("a", [ev({ seq: 1, actor: "a", kind: "core.speech.uttered" })]);
    expect(life).toEqual([{ seq: 1, tick: 10, kind: "core.speech.uttered", text: "speech.uttered", role: "actor" }]);
  });

  it("bỏ tiền tố namespace khỏi text nhưng giữ nguyên kind", () => {
    const life = lifeOf("a", [ev({ seq: 1, actor: "a", kind: "npc.intended" })]);
    expect(life[0]?.kind).toBe("npc.intended");
    expect(life[0]?.text).toBe("intended");
  });

  it("tham chiếu {entity: id} trong payload (không phải actor) là vai subject", () => {
    // Khuôn dây thật cho một `Value::Uint`: đúng một khóa `entity`. Đây là tín
    // hiệu có kiểu, không phải chuỗi trùng ngẫu nhiên.
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: "b", kind: "core.item.taken", payload: { what: { entity: "a" } } }),
    ];
    expect(lifeOf("a", events)[0]?.role).toBe("subject");
  });

  it("id trần trụi trong payload, không theo khuôn {entity}, là vai bystander", () => {
    // So khớp là **đẳng thức chuỗi tuyệt đối**, không phải chứa chuỗi con —
    // nếu không thì id "1" sẽ khớp bậy vào bất cứ văn bản nào có chữ số 1.
    // Đây là trường hợp giá trị chuỗi trùng id nhưng không đi theo khuôn dây
    // `{entity: ...}`, nên tín hiệu yếu hơn "ref".
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: "b", kind: "core.speech.uttered", payload: { witness: "a" } }),
    ];
    expect(lifeOf("a", events)[0]?.role).toBe("bystander");
  });

  it("chuỗi con trùng id không được tính là khớp — chỉ đẳng thức tuyệt đối", () => {
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: "b", kind: "core.speech.uttered", payload: { text: "chào a nhé" } }),
    ];
    expect(lifeOf("a", events)).toEqual([]);
  });

  it("actor thắng ref/mention khi cả hai đều khớp cùng một sự kiện", () => {
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: "a", kind: "core.take", payload: { what: { entity: "a" } } }),
    ];
    expect(lifeOf("a", events)[0]?.role).toBe("actor");
  });

  it("sự kiện không nhắc tới id ở đâu cả thì bị loại hẳn", () => {
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: "b", kind: "core.item.taken", payload: { what: { entity: "c" } } }),
    ];
    expect(lifeOf("a", events)).toEqual([]);
  });

  it("giữ thứ tự thời gian giảm dần bất kể thứ tự mảng đầu vào", () => {
    const events: WorldEvent[] = [
      ev({ seq: 5, actor: "a" }),
      ev({ seq: 1, actor: "a" }),
      ev({ seq: 9, actor: "a" }),
      ev({ seq: 3, actor: "a" }),
    ];
    expect(lifeOf("a", events).map((l) => l.seq)).toEqual([9, 5, 3, 1]);
  });

  it("không sửa mảng đầu vào", () => {
    const events: WorldEvent[] = [
      ev({ seq: 5, actor: "a" }),
      ev({ seq: 1, actor: "a" }),
      ev({ seq: 9, actor: "a" }),
    ];
    const snapshot = events.map((e) => e.seq);
    lifeOf("a", events);
    expect(events.map((e) => e.seq)).toEqual(snapshot);
  });

  it("limit cắt đúng số mắt sau khi đã sắp", () => {
    const events: WorldEvent[] = [1, 2, 3, 4, 5].map((seq) => ev({ seq, actor: "a" }));
    expect(lifeOf("a", events, 2).map((l) => l.seq)).toEqual([5, 4]);
    expect(lifeOf("a", events, 0)).toEqual([]);
    expect(lifeOf("a", events).length).toBe(5); // không truyền limit thì không cắt
  });

  it("id chuỗi lớn (vượt 2^53) không bị trộn với id lân cận", () => {
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: HUGE_A }),
      ev({ seq: 2, actor: HUGE_B }),
    ];
    expect(lifeOf(HUGE_A, events).map((l) => l.seq)).toEqual([1]);
    expect(lifeOf(HUGE_B, events).map((l) => l.seq)).toEqual([2]);
  });

  it("id chuỗi lớn khớp đúng qua khuôn {entity} trong payload", () => {
    const events: WorldEvent[] = [
      ev({ seq: 1, actor: "khác", payload: { who: { entity: HUGE_A } } }),
    ];
    expect(lifeOf(HUGE_A, events)[0]?.role).toBe("subject");
    expect(lifeOf(HUGE_B, events)).toEqual([]);
  });
});

describe("followStep", () => {
  it("mục tiêu nhích một ô (kể cả chéo) vẫn nằm trong vùng chết: camera đứng yên", () => {
    const cam = { x: 100, y: 50 };
    for (const target of [
      { x: 101, y: 50 },
      { x: 100, y: 51 },
      { x: 99, y: 49 },
      { x: 101, y: 51 },
    ]) {
      const r = followStep(cam, target);
      expect(r).toEqual({ x: 100, y: 50, snapped: false });
    }
  });

  it("lệch vượt snapDistance thì nhảy thẳng tới mục tiêu", () => {
    const r = followStep({ x: 0, y: 0 }, { x: 50, y: 0 }, { snapDistance: 10 });
    expect(r).toEqual({ x: 50, y: 0, snapped: true });
  });

  it("lệch nằm giữa deadZone và snapDistance thì trôi, không nhảy và không đứng yên", () => {
    const r = followStep({ x: 0, y: 0 }, { x: 6, y: 0 }, { deadZone: 1, snapDistance: 10 });
    expect(r.snapped).toBe(false);
    expect(r.x).toBeGreaterThan(0);
    expect(r.x).toBeLessThan(6);
    expect(r.y).toBe(0);
    expect(Number.isInteger(r.x)).toBe(true);
  });

  it("trôi nhiều bước liên tiếp thì hội tụ về mục tiêu mà không vượt qua", () => {
    let cam = { x: 0, y: 0 };
    const target = { x: 6, y: -3 };
    let steps = 0;
    while ((cam.x !== target.x || cam.y !== target.y) && steps < 100) {
      const r = followStep(cam, target, { deadZone: 0.4, snapDistance: 20 });
      expect(r.snapped).toBe(false);
      cam = { x: r.x, y: r.y };
      steps++;
    }
    expect(cam).toEqual(target);
    expect(steps).toBeGreaterThan(0);
    expect(steps).toBeLessThan(100);
  });

  it("tọa độ trả về luôn là số nguyên, kể cả khi đầu vào là số thực", () => {
    const r = followStep({ x: 0.3, y: 0.7 }, { x: 6.6, y: 0.2 }, { deadZone: 1, snapDistance: 10 });
    expect(Number.isInteger(r.x)).toBe(true);
    expect(Number.isInteger(r.y)).toBe(true);
  });

  it("camera đã đúng vị trí mục tiêu thì đứng yên, không rung", () => {
    const r = followStep({ x: 12, y: -4 }, { x: 12, y: -4 });
    expect(r).toEqual({ x: 12, y: -4, snapped: false });
  });
});
