/**
 * Test cho trường dữ liệu phủ bản đồ (`PG-07`).
 *
 * Trọng tâm là ba lỗi im lặng mà đặc tả đã lường trước: `max === min` biến cả
 * trường thành `NaN`, "không có dữ liệu" bị vẽ như `0`, và chuẩn hóa theo một
 * hằng số thay vì theo lô thật. Cả ba đều là loại lỗi không ném exception nào
 * — bản đồ vẫn vẽ ra, chỉ là vẽ sai, và không có test thì không ai nhận ra.
 */

import { describe, expect, it } from "vitest";
import type { Entity, TileBatch } from "@/api/game";
import { computeField, LAYERS, paintField, type Field } from "./field";

function fill<T>(n: number, v: T): T[] {
  return Array.from({ length: n }, () => v);
}

interface BatchOptions {
  x?: number;
  y?: number;
  drop?: number[];
  height?: number[];
  river?: number[];
}

/** Dựng một lô ô tối thiểu: mặc định đất bằng, không sông, cao độ 0. */
function batchOf(w: number, h: number, opt: BatchOptions = {}): TileBatch {
  const n = w * h;
  return {
    x: opt.x ?? 0,
    y: opt.y ?? 0,
    w,
    h,
    z: 0,
    material: fill(n, "topsoil"),
    surface: fill(n, "topsoil"),
    drop: opt.drop ?? fill(n, 0),
    built: fill(n, 0),
    biome: fill(n, "plains"),
    height: opt.height ?? fill(n, 0),
    river: opt.river ?? fill(n, 0),
  };
}

function beingAt(x: number, y: number, id = "1"): Entity {
  return {
    id,
    name: id,
    x,
    y,
    kind: "being",
    hunger: null,
    role: null,
    intent: null,
  };
}

describe("LAYERS", () => {
  it("bốn lớp, đúng thứ tự đặc tả", () => {
    expect(LAYERS).toEqual(["elevation", "water", "walkable", "crowd"]);
  });
});

describe("elevation", () => {
  it("chuẩn hóa theo min/max thực tế của lô, không theo hằng số", () => {
    // Hai lô ở hai độ cao tuyệt đối rất khác nhau (đồng bằng thấp và cao
    // nguyên) nhưng cùng chênh lệch nội bộ 10 m — cả hai phải đọc ra CÙNG một
    // trường chuẩn hóa. Một thang cố định (theo độ cao tuyệt đối của thế giới)
    // sẽ dồn lô cao nguyên về gần 1 và lô đồng bằng về gần 0, xóa mất chênh
    // lệch nội bộ mà overlay có nhiệm vụ phải cho thấy.
    const dongBang = computeField("elevation", batchOf(2, 1, { height: [0, 10] }), []);
    const caoNguyen = computeField("elevation", batchOf(2, 1, { height: [4000, 4010] }), []);
    expect(Array.from(dongBang.v)).toEqual([0, 1]);
    expect(Array.from(caoNguyen.v)).toEqual([0, 1]);
    expect(dongBang.min).toBe(0);
    expect(dongBang.max).toBe(10);
    expect(caoNguyen.min).toBe(4000);
    expect(caoNguyen.max).toBe(4010);
  });

  it("lô toàn đồng bằng (max === min) cho một trường hằng, không phải NaN", () => {
    // Đây là chỗ chia cho 0 kinh điển: `(x - min) / (max - min)` với
    // `max === min` là `0 / 0`. Một lô bằng phẳng tuyệt đối vẫn là dữ liệu
    // thật — nó phải đọc ra một màu hằng ở giữa thang, không phải biến mất.
    const f = computeField("elevation", batchOf(3, 3, { height: fill(9, 42) }), []);
    expect(Array.from(f.v).every((x) => x === 0.5)).toBe(true);
    expect(f.v.some((x) => Number.isNaN(x))).toBe(false);
    expect(f.min).toBe(42);
    expect(f.max).toBe(42);
  });

  it("đơn vị là mét", () => {
    expect(computeField("elevation", batchOf(1, 1), []).unit).toBe("m");
  });
});

describe("water", () => {
  it("ô khô là NaN, không phải 0", () => {
    const f = computeField(
      "water",
      batchOf(2, 1, { river: [1, 0], drop: [0, 0] }),
      [],
    );
    expect(f.v[0]).toBe(0.5); // sông, drop bằng nhau ở mọi ô sông -> hằng
    expect(Number.isNaN(f.v[1]!)).toBe(true);
  });

  it("không ô sông nào trong lô: cả trường là NaN, không rơi vào Infinity", () => {
    const f = computeField("water", batchOf(2, 2), []);
    expect(Array.from(f.v).every((x) => Number.isNaN(x))).toBe(true);
    expect(f.min).toBe(0);
    expect(f.max).toBe(0);
  });

  it("sông ngay lát đang đứng (drop nhỏ) đọc khác sông nhìn từ trên cao (drop lớn)", () => {
    const f = computeField(
      "water",
      batchOf(2, 1, { river: [1, 1], drop: [0, 20] }),
      [],
    );
    expect(f.v[0]).toBe(0);
    expect(f.v[1]).toBe(1);
    expect(f.min).toBe(0);
    expect(f.max).toBe(20);
  });

  it("mọi ô sông cùng drop (max === min) vẫn là một trường hằng, không NaN", () => {
    const f = computeField(
      "water",
      batchOf(2, 2, { river: [1, 1, 1, 1], drop: [3, 3, 3, 3] }),
      [],
    );
    expect(Array.from(f.v).every((x) => x === 0.5)).toBe(true);
  });
});

describe("walkable", () => {
  it("drop === 0 là đi được, drop > 0 là không", () => {
    const f = computeField("walkable", batchOf(3, 1, { drop: [0, 1, 5] }), []);
    expect(Array.from(f.v)).toEqual([1, 0, 0]);
  });

  it("nhị phân KHÔNG bị `normalize` dồn về hằng 0.5", () => {
    // Nếu `walkable` vô tình đi qua cùng đường chuẩn hóa với các lớp liên
    // tục, một lô toàn đi được (mọi ô cùng giá trị 1) sẽ đọc thành `max ===
    // min` và bị dồn về `0.5` — phá mất nghĩa "đi được" thành một màu xám lửng
    // lơ. Test này khóa lại: toàn đi được phải là `1` ở mọi ô, không phải
    // `0.5`.
    const f = computeField("walkable", batchOf(2, 2, { drop: [0, 0, 0, 0] }), []);
    expect(Array.from(f.v)).toEqual([1, 1, 1, 1]);
    expect(f.min).toBe(0);
    expect(f.max).toBe(1);
  });

  it("đơn vị rỗng — nhị phân không có đơn vị thật", () => {
    expect(computeField("walkable", batchOf(1, 1), []).unit).toBe("");
  });
});

describe("crowd", () => {
  it("ô không có ai trong bán kính là NaN", () => {
    const f = computeField("crowd", batchOf(9, 1), []);
    expect(Array.from(f.v).every((x) => Number.isNaN(x))).toBe(true);
  });

  it("ô có người đứng đúng đó đậm nhất; càng xa càng nhạt trong bán kính 3", () => {
    // Một lô 9×1, người đứng ở x = 4 (giữa lô).
    const f = computeField("crowd", batchOf(9, 1), [beingAt(4, 0)]);
    // Ô đúng vị trí người đứng có mật độ thô lớn nhất trong lô, nên sau chuẩn
    // hóa min/max nó luôn là 1 — đây không phải trùng hợp, đó là tính chất của
    // chính phép chuẩn hóa.
    expect(f.v[4]).toBe(1);
    // Ngoài bán kính 3 (x = 0 và x = 8, cách 4 và 4 ô) là NaN.
    expect(Number.isNaN(f.v[0]!)).toBe(true);
    expect(Number.isNaN(f.v[8]!)).toBe(true);
    // Trong bán kính, càng gần người thì giá trị chuẩn hóa càng lớn.
    expect(f.v[3]!).toBeGreaterThan(f.v[2]!);
    expect(f.v[5]!).toBeGreaterThan(f.v[6]!);
  });

  it("chỉ đếm kind === 'being', bỏ qua 'item'", () => {
    const item: Entity = {
      id: "i1",
      name: "táo",
      x: 4,
      y: 0,
      kind: "item",
        hunger: null,
      role: null,
      intent: null,
    };
    const f = computeField("crowd", batchOf(9, 1), [item]);
    expect(Array.from(f.v).every((x) => Number.isNaN(x))).toBe(true);
  });

  it("đơn vị là người", () => {
    expect(computeField("crowd", batchOf(1, 1), [beingAt(0, 0)]).unit).toBe("người");
  });
});

describe("paintField", () => {
  function fieldOf(v: number[], w = v.length, h = 1): Field {
    return { w, h, v: Float32Array.from(v), unit: "m", min: 0, max: 1 };
  }

  it("kích thước buffer là w*h*4", () => {
    const buf = paintField(fieldOf([0, 0.5, 1]), "dark", 1);
    expect(buf.length).toBe(3 * 1 * 4);
  });

  it("NaN luôn ra alpha 0, bất kể alpha truyền vào", () => {
    const buf = paintField(fieldOf([NaN]), "dark", 1);
    expect(buf[3]).toBe(0);
    const buf2 = paintField(fieldOf([NaN]), "dark", 0.5);
    expect(buf2[3]).toBe(0);
  });

  it("ô có dữ liệu dùng đúng alpha truyền vào", () => {
    const buf = paintField(fieldOf([1]), "dark", 0.4);
    expect(buf[3]).toBe(Math.round(0.4 * 255));
  });

  it("v = 0 và v = 1 khớp đúng hai đầu thang màu của accessibility.ts", () => {
    // Không bịa thang riêng: đầu 0 và đầu 1 phải khớp đúng SCALES.dark.
    const buf = paintField(fieldOf([0, 1]), "dark", 1);
    // #08306b
    expect([buf[0], buf[1], buf[2]]).toEqual([0x08, 0x30, 0x6b]);
    // #f7fbff
    expect([buf[4], buf[5], buf[6]]).toEqual([0xf7, 0xfb, 0xff]);
  });

  it("chế độ sáng và tối cho hai màu khác nhau ở cùng giá trị", () => {
    const light = paintField(fieldOf([0]), "light", 1);
    const dark = paintField(fieldOf([0]), "dark", 1);
    expect([light[0], light[1], light[2]]).not.toEqual([dark[0], dark[1], dark[2]]);
  });
});
