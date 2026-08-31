/**
 * Test cho danh mục quyền năng (`powers.ts`).
 *
 * Bốn cụm, theo đúng yêu cầu của nhiệm vụ:
 * 1. Hình dạng danh mục — id duy nhất, `effect.kind` nằm trong danh sách trắng
 *    lệnh engine thật sự hiểu, mọi id có khóa chữ ở `strings.ts`.
 * 2. `readiness` — đúng cho cả bốn giá trị `PowerNeeds`.
 * 3. `fieldsFor` — trả `null` khi thiếu thứ bắt buộc (sinh mệnh/ô/tham số).
 * 4. `fieldsFor` — bọc định danh đúng khuôn dây `{entity: N}` (`§22.10`), chưa
 *    bao giờ để lọt một số trần.
 */

import { describe, expect, it } from "vitest";
import { fieldsFor, POWERS, POWER_GROUPS, readiness, type Power } from "./powers";
import { POWER_CATALOGS } from "./strings";

/**
 * Danh sách trắng — đúng các `kind` lệnh mà `mow-scenario/src/slice.rs` đăng
 * ký (`slice_handlers()` + `testing::handlers()`), theo yêu cầu của nhiệm vụ.
 * Không lấy từ `powers.ts`: nếu lấy từ chính module đang kiểm thì bài test
 * không còn bắt được lỗi "bịa thêm một lệnh không có thật".
 */
const ENGINE_COMMAND_KINDS = new Set([
  "core.walk",
  "core.take",
  "core.speak",
  "core.set_attr",
  "npc.intend",
  "truegod.set_attr",
]);

function byId(id: string): Power {
  const p = POWERS.find((x) => x.id === id);
  if (!p) throw new Error(`không thấy quyền năng: ${id}`);
  return p;
}

describe("POWERS — hình dạng danh mục", () => {
  it("có ít nhất 14 quyền năng", () => {
    expect(POWERS.length).toBeGreaterThanOrEqual(14);
  });

  it("mọi id là duy nhất", () => {
    const ids = POWERS.map((p) => p.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("mọi group nằm trong POWER_GROUPS", () => {
    for (const p of POWERS) {
      expect(POWER_GROUPS).toContain(p.group);
    }
  });

  it("mọi effect.kind (command/preview) nằm trong danh sách trắng lệnh engine hiểu", () => {
    for (const p of POWERS) {
      if (p.effect.via === "command" || p.effect.via === "preview") {
        expect(ENGINE_COMMAND_KINDS.has(p.effect.kind), `${p.id} dùng kind lạ: ${p.effect.kind}`).toBe(
          true,
        );
      }
    }
  });

  it("mọi id có khóa nhãn + gợi ý trong strings.ts, cả vi lẫn en", () => {
    const vi: Record<string, string> = POWER_CATALOGS.vi;
    const en: Record<string, string> = POWER_CATALOGS.en;
    for (const p of POWERS) {
      for (const suffix of ["label", "hint"]) {
        const key = `power.${p.id}.${suffix}`;
        expect(vi[key], `thiếu vi["${key}"]`).toBeTypeOf("string");
        expect(en[key], `thiếu en["${key}"]`).toBeTypeOf("string");
      }
    }
  });

  it("mọi tham số choice có ít nhất một lựa chọn, và mọi lựa chọn có nhãn giá trị trong strings.ts", () => {
    const vi: Record<string, string> = POWER_CATALOGS.vi;
    for (const p of POWERS) {
      for (const param of p.params ?? []) {
        if (param.kind !== "choice") continue;
        expect(param.options?.length ?? 0, `${p.id}.${param.key} không có lựa chọn`).toBeGreaterThan(0);
        for (const opt of param.options ?? []) {
          const key = `value.${param.key}.${opt}`;
          expect(vi[key], `thiếu nhãn giá trị vi["${key}"]`).toBeTypeOf("string");
        }
      }
    }
  });
});

describe("readiness", () => {
  const mk = (needs: Power["needs"]): Power => ({
    id: "test.power",
    group: "sight",
    needs,
    effect: { via: "view" },
    glyph: "x",
  });

  it("none: luôn sẵn sàng, kể cả khi chưa chọn gì", () => {
    expect(readiness(mk("none"), { being: false, tile: false })).toEqual({ ready: true });
  });

  it("being: cần một sinh mệnh, ô không liên quan", () => {
    expect(readiness(mk("being"), { being: false, tile: true })).toEqual({
      ready: false,
      reason: "need_being",
    });
    expect(readiness(mk("being"), { being: true, tile: false })).toEqual({ ready: true });
  });

  it("tile: cần một ô, sinh mệnh không liên quan", () => {
    expect(readiness(mk("tile"), { being: true, tile: false })).toEqual({
      ready: false,
      reason: "need_tile",
    });
    expect(readiness(mk("tile"), { being: false, tile: true })).toEqual({ ready: true });
  });

  it("being_and_tile: cần cả hai — thiếu being báo need_being trước, kể cả khi tile cũng thiếu", () => {
    expect(readiness(mk("being_and_tile"), { being: false, tile: false })).toEqual({
      ready: false,
      reason: "need_being",
    });
    expect(readiness(mk("being_and_tile"), { being: false, tile: true })).toEqual({
      ready: false,
      reason: "need_being",
    });
  });

  it("being_and_tile: có being rồi mà thiếu tile thì báo need_tile", () => {
    expect(readiness(mk("being_and_tile"), { being: true, tile: false })).toEqual({
      ready: false,
      reason: "need_tile",
    });
  });

  it("being_and_tile: có cả hai thì sẵn sàng", () => {
    expect(readiness(mk("being_and_tile"), { being: true, tile: true })).toEqual({ ready: true });
  });
});

describe("fieldsFor — null khi thiếu thứ bắt buộc", () => {
  it("needs=being mà không có beingId thì null", () => {
    expect(fieldsFor(byId("body.feed"), {})).toBeNull();
  });

  it("needs=tile mà không có tile thì null", () => {
    expect(fieldsFor(byId("land.till"), {})).toBeNull();
  });

  it("needs=being_and_tile mà chỉ có being thì null", () => {
    expect(fieldsFor(byId("body.guide"), { beingId: "1" })).toBeNull();
  });

  it("needs=being_and_tile mà chỉ có tile thì null", () => {
    expect(fieldsFor(byId("mind.uproot_x"), { tile: { x: 1, y: 2 } })).toBeNull();
  });

  it("needs=none không đòi gì cả — luôn trả về một object", () => {
    expect(fieldsFor(byId("time.still"), {})).toEqual({});
    expect(fieldsFor(byId("sight.pierce"), {})).not.toBeNull();
  });

  it("thiếu tham số text bắt buộc thì null dù đã có being", () => {
    expect(fieldsFor(byId("body.rename"), { beingId: "1" })).toBeNull();
    expect(fieldsFor(byId("body.rename"), { beingId: "1", params: { name: "   " } })).toBeNull();
  });

  it("thiếu tham số text bắt buộc thì null dù đã có tile", () => {
    expect(fieldsFor(byId("land.carve"), { tile: { x: 0, y: 0 } })).toBeNull();
  });

  it("tham số choice không khớp options thì null", () => {
    expect(
      fieldsFor(byId("body.recast"), { beingId: "1", params: { role: "wizard" } }),
    ).toBeNull();
    expect(
      fieldsFor(byId("mind.dream"), { beingId: "1", params: { intent: "levitate" } }),
    ).toBeNull();
  });

  it("thiếu tham số item của Khiến nhặt thì null dù đã có being", () => {
    expect(fieldsFor(byId("body.take"), { beingId: "1" })).toBeNull();
  });
});

describe("fieldsFor — bọc định danh đúng khuôn {entity: N}, không phải số trần (§22.10)", () => {
  it("body.feed: entity đi vào dạng {entity: N}", () => {
    const fields = fieldsFor(byId("body.feed"), { beingId: "42" });
    expect(fields).toEqual({ entity: { entity: 42 }, key: "need.hunger", value: 0 });
  });

  it("body.starve: cùng khuôn, value=9000", () => {
    const fields = fieldsFor(byId("body.starve"), { beingId: "7" });
    expect(fields).toEqual({ entity: { entity: 7 }, key: "need.hunger", value: 9000 });
  });

  it("body.rename: entity bọc {entity:N}, value là chuỗi tên", () => {
    const fields = fieldsFor(byId("body.rename"), { beingId: "3", params: { name: "Aren" } });
    expect(fields).toEqual({ entity: { entity: 3 }, key: "core.name", value: "Aren" });
  });

  it("body.recast: entity bọc {entity:N}, value là role hợp lệ", () => {
    const fields = fieldsFor(byId("body.recast"), { beingId: "3", params: { role: "smith" } });
    expect(fields).toEqual({ entity: { entity: 3 }, key: "npc.role", value: "smith" });
  });

  it("mind.dream (npc.intend): who bọc {entity:N}, không phải chuỗi/số trần", () => {
    const fields = fieldsFor(byId("mind.dream"), { beingId: "9", params: { intent: "eat" } });
    expect(fields).toEqual({ who: { entity: 9 }, intent: "eat" });
  });

  it("mind.proclaim (core.speak): who bọc {entity:N}", () => {
    const fields = fieldsFor(byId("mind.proclaim"), {
      beingId: "9",
      params: { text: "Hãy tới quảng trường" },
    });
    expect(fields).toEqual({ who: { entity: 9 }, text: "Hãy tới quảng trường" });
  });

  it("body.take (core.take): cả who lẫn what đều bọc {entity:N}", () => {
    const fields = fieldsFor(byId("body.take"), { beingId: "9", params: { item: "55" } });
    expect(fields).toEqual({ who: { entity: 9 }, what: { entity: 55 } });
  });

  it("mind.uproot_x/y: entity bọc {entity:N}, value là tọa độ trần (số, không phải định danh)", () => {
    const fx = fieldsFor(byId("mind.uproot_x"), { beingId: "5", tile: { x: 10, y: -3 } });
    expect(fx).toEqual({ entity: { entity: 5 }, key: "npc.home.x", value: 10 });
    const fy = fieldsFor(byId("mind.uproot_y"), { beingId: "5", tile: { x: 10, y: -3 } });
    expect(fy).toEqual({ entity: { entity: 5 }, key: "npc.home.y", value: -3 });
  });

  it("mind.reassign_x/y: cùng khuôn, khóa npc.work.x/y", () => {
    const fx = fieldsFor(byId("mind.reassign_x"), { beingId: "5", tile: { x: 8, y: 2 } });
    expect(fx).toEqual({ entity: { entity: 5 }, key: "npc.work.x", value: 8 });
    const fy = fieldsFor(byId("mind.reassign_y"), { beingId: "5", tile: { x: 8, y: 2 } });
    expect(fy).toEqual({ entity: { entity: 5 }, key: "npc.work.y", value: 2 });
  });

  it("body.guide đi qua api.guide riêng — who là chuỗi trần, KHÔNG bọc {entity}", () => {
    // `api.guide(who, x, y)` gửi `who` thẳng vào `/api/goto`, không đi qua
    // đường mã hóa `fields` của `/api/command`/`/api/preview` — endpoint đó
    // không hiểu và không cần khuôn `{entity: N}`.
    const fields = fieldsFor(byId("body.guide"), { beingId: "9", tile: { x: 1, y: 2 } });
    expect(fields).toEqual({ who: "9", x: 1, y: 2 });
  });

  it("land.*: không có định danh nào để bọc — x/y/material đều là giá trị trần", () => {
    expect(fieldsFor(byId("land.till"), { tile: { x: 4, y: 4 } })).toEqual({
      x: 4,
      y: 4,
      material: "farmland",
    });
    expect(fieldsFor(byId("land.pave"), { tile: { x: 4, y: 4 } })).toEqual({
      x: 4,
      y: 4,
      material: "path_gravel",
    });
    expect(
      fieldsFor(byId("land.carve"), { tile: { x: 4, y: 4 }, params: { material: "stone" } }),
    ).toEqual({ x: 4, y: 4, material: "stone" });
  });
});
