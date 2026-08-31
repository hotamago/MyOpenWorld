import { describe, expect, it } from "vitest";
import type { TileBatch } from "@/api/game";
import {
  CHIMNEY_GATE_RATIO,
  SMOKE_LIFE_MS,
  SWAY_ACTIVE_RATIO,
  chimneys,
  phaseAt,
  smokePuff,
  sway,
} from "./liveliness";
import { TICKS_PER_DAY, dayPhase } from "./terrain";

/** Gốc lô cố tình lệch khỏi `(0, 0)` — hạt/pha phải bám tọa độ thế giới, và một
 * gốc ở đúng gốc toạ độ sẽ giấu đúng loại lỗi cần bắt (xem `ambient.test.ts`). */
const X0 = -211;
const Y0 = 7_003;

function batchOf(
  w: number,
  h: number,
  opts: {
    material?: (gx: number, gy: number) => string;
    surface?: (gx: number, gy: number) => string;
    built?: (gx: number, gy: number) => number;
    origin?: { x: number; y: number };
  } = {},
): TileBatch {
  const n = w * h;
  const material: string[] = [];
  const surface: string[] = [];
  const built: number[] = [];
  for (let gy = 0; gy < h; gy++) {
    for (let gx = 0; gx < w; gx++) {
      material.push(opts.material ? opts.material(gx, gy) : "air");
      surface.push(opts.surface ? opts.surface(gx, gy) : "topsoil");
      built.push(opts.built ? opts.built(gx, gy) : 0);
    }
  }
  const origin = opts.origin ?? { x: X0, y: Y0 };
  return {
    x: origin.x,
    y: origin.y,
    w,
    h,
    z: 0,
    material,
    surface,
    drop: new Array<number>(n).fill(0),
    built,
    biome: new Array<string>(n).fill("temperate"),
    height: new Array<number>(n).fill(10),
    river: new Array<number>(n).fill(0),
    worn: new Array<number>(n).fill(0),
  };
}

/** Một tick chắc chắn rơi vào buổi cho trước, dùng `dayPhase` để không đoán mò biên. */
function tickAt(target: "night" | "dawn" | "day" | "dusk"): number {
  for (let t = 0; t < TICKS_PER_DAY; t++) {
    if (dayPhase(t) === target) return t;
  }
  throw new Error(`no tick found for ${target}`);
}

/** Một lô mái lớn, xây hết, đủ để gần như chắc chắn có ống khói đủ điều kiện. */
function roofField(w = 80, h = 80): TileBatch {
  return batchOf(w, h, {
    material: () => "roof_light",
    surface: () => "roof_light",
    built: () => 1,
  });
}

describe("liveliness — xác định", () => {
  it("phaseAt: cùng ô luôn cho cùng pha", () => {
    const a = phaseAt(X0 + 12, Y0 - 7);
    for (let i = 0; i < 50; i++) expect(phaseAt(X0 + 12, Y0 - 7)).toBe(a);
  });

  it("sway: cùng (x, y, ms) luôn cho cùng số", () => {
    // Quét một dải toạ độ để chắc chắn bắt được vài ô đang "được chọn" lay.
    for (let x = X0; x < X0 + 40; x++) {
      const a = sway(x, Y0, 1_234.5);
      expect(sway(x, Y0, 1_234.5)).toBe(a);
    }
  });

  it("chimneys: cùng đầu vào cho cùng mảng, từng phần tử", () => {
    const batch = roofField();
    const tick = tickAt("night");
    const first = JSON.stringify(chimneys(batch, tick, 20));
    for (let i = 0; i < 20; i++) {
      expect(JSON.stringify(chimneys(batch, tick, 20))).toBe(first);
    }
  });

  it("smokePuff: cùng (seed, ageMs) luôn cho cùng số", () => {
    const a = smokePuff(42, 900);
    for (let i = 0; i < 20; i++) expect(smokePuff(42, 900)).toEqual(a);
  });

  it("không gọi Math.random ở đâu cả", () => {
    const real = Math.random;
    Math.random = () => {
      throw new Error("liveliness.ts không được phép dùng Math.random");
    };
    try {
      for (let x = X0; x < X0 + 30; x++) {
        phaseAt(x, Y0);
        sway(x, Y0, 500);
      }
      expect(chimneys(roofField(), tickAt("night"), 10).length).toBeGreaterThanOrEqual(0);
      expect(smokePuff(7, 300)).not.toBeNull();
    } finally {
      Math.random = real;
    }
  });
});

describe("liveliness — pha lệch theo ô", () => {
  it("các ô khác nhau cho pha thật sự khác nhau", () => {
    const phases = new Set<number>();
    for (let x = X0; x < X0 + 200; x++) phases.add(phaseAt(x, Y0));
    // Không đòi trắng tuyệt đối, chỉ đòi rõ ràng không phải một hằng số dùng
    // chung — nếu `phaseAt` lỡ bỏ sót toạ độ trong hash, số giá trị riêng biệt
    // sẽ sụp xuống rất thấp so với 200 mẫu.
    expect(phases.size).toBeGreaterThan(150);
  });

  it("đổi trục y cũng đổi pha — không chỉ đọc x", () => {
    const row = phaseAt(X0, Y0);
    const col = phaseAt(X0, Y0 + 1);
    expect(row).not.toBe(col);
  });

  it("pha nằm trong [0, 2π)", () => {
    for (let x = X0; x < X0 + 50; x++) {
      const p = phaseAt(x, Y0 + 3);
      expect(p).toBeGreaterThanOrEqual(0);
      expect(p).toBeLessThan(Math.PI * 2);
    }
  });
});

describe("liveliness — cỏ lay (sway)", () => {
  it("biên độ luôn bị chặn trong [-amplitude, amplitude]", () => {
    for (let x = X0; x < X0 + 300; x++) {
      const v = sway(x, Y0, 999);
      expect(Math.abs(v)).toBeLessThanOrEqual(2 + 1e-9);
    }
  });

  it("tỉ lệ ô thật sự dao động nằm quanh SWAY_ACTIVE_RATIO (10–20%)", () => {
    // Một ô được coi là "đang lay" nếu nó khác 0 tại ít nhất một thời điểm —
    // dùng hai `ms` lệch pha xa nhau (1/4 chu kỳ) để một ô hiếm khi rơi đúng
    // điểm 0 ở CẢ HAI, tránh đánh giá thấp tỉ lệ cổng thật.
    const n = 20_000;
    let active = 0;
    for (let x = X0; x < X0 + n; x++) {
      const a = sway(x, Y0, 0);
      const b = sway(x, Y0, 625); // period mặc định 2500ms => 1/4 chu kỳ
      if (a !== 0 || b !== 0) active++;
    }
    const ratio = active / n;
    expect(SWAY_ACTIVE_RATIO).toBeGreaterThanOrEqual(0.1);
    expect(SWAY_ACTIVE_RATIO).toBeLessThanOrEqual(0.2);
    expect(Math.abs(ratio - SWAY_ACTIVE_RATIO)).toBeLessThan(0.02);
  });

  it("phần lớn ô không bao giờ lay, dù đổi ms", () => {
    let neverActive = 0;
    const n = 2_000;
    for (let x = X0; x < X0 + n; x++) {
      let allZero = true;
      for (const ms of [0, 400, 900, 1_600, 2_400]) {
        if (sway(x, Y0 + 9, ms) !== 0) {
          allZero = false;
          break;
        }
      }
      if (allZero) neverActive++;
    }
    expect(neverActive / n).toBeGreaterThan(0.7);
  });

  it("period/amplitude tuỳ chỉnh được áp dụng cho ô đang lay", () => {
    // Dò một ô chắc chắn thuộc nhóm 15% đang lay.
    let x = X0;
    for (; x < X0 + 5_000; x++) {
      if (sway(x, Y0 + 1, 300) !== 0) break;
    }
    const base = sway(x, Y0 + 1, 300);
    const scaled = sway(x, Y0 + 1, 300, { amplitude: base === 0 ? 1 : 10 });
    if (base !== 0) {
      expect(scaled / base).toBeCloseTo(10 / 2, 5); // biên độ mặc định là 2
    }
  });

  it("ms không hữu hạn hoặc period <= 0 trả về 0, không ném lỗi", () => {
    expect(() => sway(X0, Y0, Number.NaN)).not.toThrow();
    expect(sway(X0, Y0, Number.NaN)).toBe(0);
    expect(sway(X0, Y0, 100, { periodMs: 0 })).toBe(0);
    expect(sway(X0, Y0, 100, { periodMs: -50 })).toBe(0);
  });
});

describe("liveliness — khói bếp (chimneys)", () => {
  it("không mái, không xây thì không có ống khói nào, kể cả ban đêm", () => {
    const bare = batchOf(40, 40, { surface: () => "topsoil", built: () => 0 });
    expect(chimneys(bare, tickAt("night"), 10)).toEqual([]);
  });

  it("mái có xây nhưng đang ban ngày thì im lặng", () => {
    const batch = roofField();
    expect(chimneys(batch, tickAt("day"), 20)).toEqual([]);
    expect(chimneys(batch, tickAt("dusk"), 20)).toEqual([]);
  });

  it("mái có xây, ban đêm hoặc sáng sớm: có ống khói, đúng vị trí là ô mái", () => {
    const batch = roofField();
    for (const phase of ["night", "dawn"] as const) {
      const list = chimneys(batch, tickAt(phase), 30);
      expect(list.length).toBeGreaterThan(0);
      for (const { x, y } of list) {
        expect(x).toBeGreaterThanOrEqual(batch.x);
        expect(x).toBeLessThan(batch.x + batch.w);
        expect(y).toBeGreaterThanOrEqual(batch.y);
        expect(y).toBeLessThan(batch.y + batch.h);
        const gx = x - batch.x;
        const gy = y - batch.y;
        const i = gy * batch.w + gx;
        expect(batch.built[i]).toBe(1);
        expect(["roof_light", "roof_dark"]).toContain(batch.material[i]);
      }
    }
  });

  it("vật liệu không phải mái thì không được chọn dù có xây và ban đêm", () => {
    const batch = batchOf(60, 60, {
      material: () => "path_gravel",
      surface: () => "path_gravel",
      built: () => 1,
    });
    expect(chimneys(batch, tickAt("night"), 30)).toEqual([]);
  });

  it("mái dưới lát không khí vẫn được nhận qua `surface` — ghost lớp dưới", () => {
    const batch = batchOf(60, 60, {
      material: () => "air",
      surface: () => "roof_dark",
      built: () => 1,
    });
    expect(chimneys(batch, tickAt("night"), 30).length).toBeGreaterThan(0);
  });

  it("không bao giờ vượt trần", () => {
    const batch = roofField(100, 100);
    const tick = tickAt("night");
    for (const cap of [0, 1, 2, 5, 9, 50]) {
      expect(chimneys(batch, tick, cap).length).toBeLessThanOrEqual(cap);
    }
  });

  it("trần 0 hoặc âm trả về mảng rỗng, không ném lỗi", () => {
    const batch = roofField();
    expect(() => chimneys(batch, tickAt("night"), 0)).not.toThrow();
    expect(chimneys(batch, tickAt("night"), 0)).toEqual([]);
    expect(chimneys(batch, tickAt("night"), -3)).toEqual([]);
  });

  it("nới trần chỉ thêm ống khói, không xáo lại tập cũ", () => {
    const batch = roofField(100, 100);
    const tick = tickAt("night");
    const small = chimneys(batch, tick, 5);
    const big = new Set(chimneys(batch, tick, 12).map((c) => `${c.x},${c.y}`));
    for (const c of small) expect(big.has(`${c.x},${c.y}`)).toBe(true);
  });

  it("tick âm không ném lỗi và vẫn xác định", () => {
    const batch = roofField();
    const a = chimneys(batch, -500, 10);
    expect(() => chimneys(batch, -500, 10)).not.toThrow();
    expect(JSON.stringify(chimneys(batch, -500, 10))).toBe(JSON.stringify(a));
  });

  it("tỉ lệ nhà từng có thể là ống khói khớp CHIMNEY_GATE_RATIO (10–20%)", () => {
    expect(CHIMNEY_GATE_RATIO).toBeGreaterThanOrEqual(0.1);
    expect(CHIMNEY_GATE_RATIO).toBeLessThanOrEqual(0.2);
  });
});

describe("liveliness — hạt khói (smokePuff)", () => {
  it("ageMs vượt vòng đời trả null", () => {
    expect(smokePuff(1, SMOKE_LIFE_MS + 1)).toBeNull();
    expect(smokePuff(1, SMOKE_LIFE_MS + 5_000)).toBeNull();
  });

  it("ageMs âm (hạt chưa sinh) trả null", () => {
    expect(smokePuff(1, -1)).toBeNull();
  });

  it("ageMs không hữu hạn trả null, không ném lỗi", () => {
    expect(smokePuff(1, Number.NaN)).toBeNull();
    expect(smokePuff(Number.NaN, 100)).toBeNull();
  });

  it("lúc mới sinh: gần vị trí gốc, bán kính nhỏ nhất, alpha cao nhất", () => {
    const p = smokePuff(3, 0);
    expect(p).not.toBeNull();
    if (!p) return;
    expect(p.dx).toBeCloseTo(0, 9);
    expect(p.dy).toBeCloseTo(0, 9);
    expect(p.alpha).toBeCloseTo(0.4, 9);
    expect(p.r).toBeGreaterThan(0);
  });

  it("cuối vòng đời: bay lên cao nhất, alpha gần 0", () => {
    const p = smokePuff(3, SMOKE_LIFE_MS);
    expect(p).not.toBeNull();
    if (!p) return;
    expect(p.dy).toBeLessThan(0); // "bay lên" — âm là hướng lên theo quy ước ô
    expect(p.alpha).toBeCloseTo(0, 9);
  });

  it("bán kính nở dần, alpha mờ dần — đơn điệu theo tuổi", () => {
    const ages = [0, 400, 800, 1_200, 1_600, 2_000, SMOKE_LIFE_MS];
    let prevR = -Infinity;
    let prevAlpha = Infinity;
    for (const age of ages) {
      const p = smokePuff(11, age);
      expect(p).not.toBeNull();
      if (!p) continue;
      expect(p.r).toBeGreaterThanOrEqual(prevR);
      expect(p.alpha).toBeLessThanOrEqual(prevAlpha + 1e-9);
      prevR = p.r;
      prevAlpha = p.alpha;
    }
  });

  it("seed khác nhau cho hướng trôi khác nhau", () => {
    const dxs = new Set<number>();
    for (let seed = 0; seed < 30; seed++) {
      const p = smokePuff(seed, 1_000);
      if (p) dxs.add(Math.round(p.dx * 1_000));
    }
    expect(dxs.size).toBeGreaterThan(10);
  });
});
