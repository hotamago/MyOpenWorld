import { describe, expect, it } from "vitest";
import type { TileBatch } from "@/api/game";
import { BlockPalette } from "./blocks";
import { AMBIENT_CYCLE, AMBIENT_DENSITY, ambientSprites, type AmbientSprite } from "./ambient";

/** Bảng dự phòng của pack lõi: có nước, cát, đất mặt, băng, quặng. */
const PALETTE = new BlockPalette();

// Gốc lô cố tình âm ở một trục và lớn ở trục kia: hạt phải bám **tọa độ thế
// giới**, nên một gốc lô nằm ở `(0, 0)` sẽ giấu đúng loại lỗi cần bắt.
const X0 = -137;
const Y0 = 5_012;

/** Dựng một lô từ một hàm "ô này là vật liệu gì". */
function batchOf(
  w: number,
  h: number,
  pick: (gx: number, gy: number) => string,
  river: (gx: number, gy: number) => number = () => 0,
  origin: { x: number; y: number } = { x: X0, y: Y0 },
): TileBatch {
  const n = w * h;
  const material: string[] = [];
  const surface: string[] = [];
  const riverCol: number[] = [];
  for (let gy = 0; gy < h; gy++) {
    for (let gx = 0; gx < w; gx++) {
      // Lát đang xem là không khí và mặt đất nằm bên dưới — đúng trường hợp
      // thường gặp nhất khi nhìn thế giới từ trên xuống.
      material.push("air");
      surface.push(pick(gx, gy));
      riverCol.push(river(gx, gy));
    }
  }
  return {
    x: origin.x,
    y: origin.y,
    w,
    h,
    z: 0,
    material,
    surface,
    drop: new Array<number>(n).fill(0),
    built: new Array<number>(n).fill(0),
    biome: new Array<string>(n).fill("temperate"),
    height: new Array<number>(n).fill(12),
    river: riverCol,
  };
}

/** Đủ mọi loại địa hình trong một lô: biển, bãi cát, đồng, băng, quặng, sông. */
function mixed(w = 64, h = 44): TileBatch {
  return batchOf(
    w,
    h,
    (gx) => {
      if (gx < 18) return "water";
      if (gx < 24) return "sand";
      if (gx < 46) return "topsoil";
      if (gx < 54) return "ice";
      return "ore";
    },
    (gx, gy) => (gy === 15 && gx >= 26 && gx < 42 ? 1 : 0),
  );
}

const uniform = (w: number, h: number, id: string) => batchOf(w, h, () => id);

/** Vật liệu ở ô chứa một hạt. */
function materialUnder(batch: TileBatch, s: AmbientSprite): string {
  const gx = Math.floor(s.x) - batch.x;
  const gy = Math.floor(s.y) - batch.y;
  return batch.surface[gy * batch.w + gx] ?? "air";
}

const kindsOf = (list: AmbientSprite[]) => list.map((s) => s.kind).join(",");

describe("hạt môi trường — xác định", () => {
  it("100 lần gọi cùng đầu vào cho cùng một mảng, từng byte", () => {
    // Đây là toàn bộ lý do module này không dùng ngẫu nhiên: một hạt nhảy chỗ
    // giữa hai khung hình trông y hệt một lỗi đồng bộ trạng thái.
    const batch = mixed();
    const first = JSON.stringify(ambientSprites(batch, PALETTE, 731, 400));
    for (let i = 0; i < 100; i++) {
      expect(JSON.stringify(ambientSprites(batch, PALETTE, 731, 400))).toBe(first);
    }
  });

  it("không gọi Math.random", () => {
    const real = Math.random;
    Math.random = () => {
      throw new Error("ambient.ts không được phép dùng Math.random");
    };
    try {
      expect(ambientSprites(mixed(), PALETTE, 9, 500).length).toBeGreaterThan(0);
    } finally {
      Math.random = real;
    }
  });

  it("hạt bám tọa độ thế giới, không trôi khi camera dịch", () => {
    // Hai khung nhìn lệch nhau, cùng phủ một vùng thế giới. Một ô nằm sâu trong
    // cả hai phải cho cùng một hạt — nếu không, cả lớp hạt sẽ "bò" theo camera.
    const pickWorld = (wx: number) => (wx % 7 === 0 ? "water" : "sand");
    const a = batchOf(40, 30, (gx) => pickWorld(X0 + gx), () => 0, { x: X0, y: Y0 });
    const b = batchOf(40, 30, (gx) => pickWorld(X0 + 6 + gx), () => 0, { x: X0 + 6, y: Y0 });

    const inA = ambientSprites(a, PALETTE, 40, 100_000).filter(
      (s) => s.x >= X0 + 10 && s.x < X0 + 30,
    );
    const inB = ambientSprites(b, PALETTE, 40, 100_000).filter(
      (s) => s.x >= X0 + 10 && s.x < X0 + 30,
    );
    expect(inA.length).toBeGreaterThan(0);
    expect(JSON.stringify(inB)).toBe(JSON.stringify(inA));
  });

  it("tick âm vẫn xác định và không ném", () => {
    const batch = mixed();
    const a = ambientSprites(batch, PALETTE, -1_234, 400);
    expect(JSON.stringify(ambientSprites(batch, PALETTE, -1_234, 400))).toBe(JSON.stringify(a));
    expect(a.length).toBeGreaterThan(0);
  });
});

describe("hạt môi trường — chuyển động", () => {
  it("đổi tick thì mọi hạt dịch chỗ, nhưng số lượng và loại không đổi", () => {
    // Số lượng đổi theo tick nghĩa là hạt chớp tắt — mắt đọc đó là nhiễu, không
    // phải chuyển động.
    const batch = mixed();
    const base = ambientSprites(batch, PALETTE, 0, 400);
    expect(base.length).toBeGreaterThan(30);

    for (const t of [1, 7, 13, 55, 9_999]) {
      const next = ambientSprites(batch, PALETTE, t, 400);
      expect(next.length).toBe(base.length);
      expect(kindsOf(next)).toBe(kindsOf(base));
      const moved = next.filter((s, i) => {
        const p = base[i];
        return p !== undefined && (s.x !== p.x || s.y !== p.y);
      });
      expect(moved.length).toBe(base.length);
    }
  });

  it("chuyển động tuần hoàn: sau AMBIENT_CYCLE tick cảnh lặp lại đúng nguyên", () => {
    // Pha tính bằng `%` trên số nguyên nên không tích lũy sai số; nếu ai đó đổi
    // sang cộng dồn delta, bài này hỏng ngay.
    const batch = mixed();
    for (const t of [0, 17, 401]) {
      expect(JSON.stringify(ambientSprites(batch, PALETTE, t + AMBIENT_CYCLE, 400))).toBe(
        JSON.stringify(ambientSprites(batch, PALETTE, t, 400)),
      );
    }
  });

  it("alpha không bao giờ chạm 0 và scale luôn dương", () => {
    // Alpha 0 là hạt biến mất; hạt biến mất rồi hiện lại là nhấp nháy.
    for (const t of [0, 3, 11, 29, 60, 120]) {
      for (const s of ambientSprites(mixed(), PALETTE, t, 1_000)) {
        expect(s.alpha).toBeGreaterThan(0);
        expect(s.alpha).toBeLessThanOrEqual(1);
        expect(s.scale).toBeGreaterThan(0);
        expect(Number.isFinite(s.rotation)).toBe(true);
      }
    }
  });
});

describe("hạt môi trường — trần số lượng", () => {
  it("không bao giờ vượt budget", () => {
    const batch = mixed();
    for (const cap of [1, 2, 5, 17, 50, 137, 400, 5_000]) {
      expect(ambientSprites(batch, PALETTE, 88, cap).length).toBeLessThanOrEqual(cap);
    }
  });

  it("hạ trần chỉ làm thưa đều, không xóa sạch một mảng khung nhìn", () => {
    // Chế độ hỏng cần bắt: cắt đuôi mảng theo thứ tự duyệt. Khi đó nửa trên đầy
    // hạt, nửa dưới trơ, và đường ranh giới chạy theo camera.
    const batch = mixed(200, 160);
    const full = ambientSprites(batch, PALETTE, 12, 1_000_000);
    expect(full.length).toBeGreaterThan(600);

    const half = Math.floor(full.length / 2);
    const thinned = ambientSprites(batch, PALETTE, 12, half);
    expect(thinned.length).toBe(half);

    const quad = (s: AmbientSprite) =>
      (s.x < batch.x + batch.w / 2 ? 0 : 1) + (s.y < batch.y + batch.h / 2 ? 0 : 2);
    const before = [0, 0, 0, 0];
    const after = [0, 0, 0, 0];
    for (const s of full) before[quad(s)] = (before[quad(s)] ?? 0) + 1;
    for (const s of thinned) after[quad(s)] = (after[quad(s)] ?? 0) + 1;

    for (let q = 0; q < 4; q++) {
      const n = before[q] ?? 0;
      expect(n).toBeGreaterThan(40);
      const kept = (after[q] ?? 0) / n;
      expect(Math.abs(kept - 0.5)).toBeLessThan(0.15);
    }
  });

  it("nới trần chỉ thêm hạt, không xáo lại những hạt đã có", () => {
    // Nếu tỉa phụ thuộc số lượng ứng viên thì mỗi lần đổi mức phóng cả lớp hạt
    // sẽ nhảy sang chỗ khác.
    const batch = mixed();
    const small = ambientSprites(batch, PALETTE, 5, 60);
    const big = new Set(ambientSprites(batch, PALETTE, 5, 90).map((s) => JSON.stringify(s)));
    for (const s of small) expect(big.has(JSON.stringify(s))).toBe(true);
  });

  it("tỉa không phụ thuộc tick: cùng trần thì cùng tập ô ở mọi tick", () => {
    const batch = mixed();
    const cells = (t: number) =>
      ambientSprites(batch, PALETTE, t, 40)
        .map((s) => `${Math.floor(s.x)},${Math.floor(s.y)},${s.kind}`)
        .join("|");
    expect(cells(3)).toBe(cells(0));
    expect(cells(777)).toBe(cells(0));
  });
});

describe("hạt môi trường — chỗ nào được sinh hạt", () => {
  it("foam chỉ ở ô nước giáp đất", () => {
    const batch = mixed();
    const foam = ambientSprites(batch, PALETTE, 21, 100_000).filter((s) => s.kind === "foam");
    expect(foam.length).toBeGreaterThan(0);

    for (const s of foam) {
      const gx = Math.floor(s.x) - batch.x;
      const gy = Math.floor(s.y) - batch.y;
      expect(materialUnder(batch, s)).toBe("water");
      const land = ([
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ] as const).some(([ox, oy]) => {
        const nx = gx + ox;
        const ny = gy + oy;
        if (nx < 0 || ny < 0 || nx >= batch.w || ny >= batch.h) return false;
        return !PALETTE.isLiquid(batch.surface[ny * batch.w + nx] ?? "air");
      });
      expect(land).toBe(true);
    }
  });

  it("giữa đại dương không có một bọt sóng nào", () => {
    const ocean = uniform(60, 40, "water");
    const list = ambientSprites(ocean, PALETTE, 21, 100_000);
    expect(list.some((s) => s.kind === "foam")).toBe(false);
    // Nhưng mặt biển vẫn phải sống: gợn nước thì có.
    expect(list.filter((s) => s.kind === "ripple").length).toBeGreaterThan(0);
  });

  it("dust không bao giờ nằm trên nước", () => {
    const batch = mixed();
    for (const t of [0, 6, 31, 200]) {
      for (const s of ambientSprites(batch, PALETTE, t, 100_000)) {
        if (s.kind !== "dust") continue;
        expect(PALETTE.isLiquid(materialUnder(batch, s))).toBe(false);
      }
    }
    expect(ambientSprites(uniform(50, 50, "water"), PALETTE, 4, 100_000).some((s) => s.kind === "dust")).toBe(
      false,
    );
  });

  it("sparkle chỉ trên băng và quặng", () => {
    const batch = mixed();
    const glint = ambientSprites(batch, PALETTE, 33, 100_000).filter((s) => s.kind === "sparkle");
    expect(glint.length).toBeGreaterThan(0);
    for (const s of glint) expect(["ice", "ore"]).toContain(materialUnder(batch, s));
  });

  it("lòng sông có gợn nước dù vật liệu vẫn là đất", () => {
    // Sông chảy trên nền đất: nếu chỉ nhìn bảng vật liệu thì nó khô, và một con
    // sông khô là một vệt sơn.
    const river = batchOf(40, 9, () => "topsoil", (_gx, gy) => (gy === 4 ? 1 : 0));
    const list = ambientSprites(river, PALETTE, 7, 100_000);
    const ripples = list.filter((s) => s.kind === "ripple");
    expect(ripples.length).toBeGreaterThan(0);
    for (const s of ripples) expect(Math.floor(s.y) - river.y).toBe(4);
  });

  it("mật độ nằm trong 3–8% số ô đủ điều kiện", () => {
    // Dày hơn là nhiễu: hạt bắt đầu cạnh tranh với thực thể, mà thực thể mới là
    // thứ người chơi cần thấy.
    const cases: [string, number][] = [
      ["water", AMBIENT_DENSITY.ripple],
      ["sand", AMBIENT_DENSITY.dustSand],
      ["topsoil", AMBIENT_DENSITY.dustSoil],
      ["ore", AMBIENT_DENSITY.sparkle],
    ];
    for (const [id, target] of cases) {
      const batch = uniform(120, 100, id);
      const ratio = ambientSprites(batch, PALETTE, 1, 1_000_000).length / (120 * 100);
      expect(target).toBeGreaterThanOrEqual(0.03);
      expect(target).toBeLessThanOrEqual(0.08);
      expect(Math.abs(ratio - target)).toBeLessThan(0.015);
    }
  });

  it("cát bốc bụi nhiều hơn đất mặt", () => {
    const n = 120 * 100;
    const sand = ambientSprites(uniform(120, 100, "sand"), PALETTE, 1, 1_000_000).length / n;
    const soil = ambientSprites(uniform(120, 100, "topsoil"), PALETTE, 1, 1_000_000).length / n;
    expect(sand).toBeGreaterThan(soil * 1.5);
  });
});

describe("hạt môi trường — biên", () => {
  it("lô rỗng, budget 0 hoặc âm đều trả mảng rỗng và không ném", () => {
    const batch = mixed();
    expect(ambientSprites(batch, PALETTE, 3, 0)).toEqual([]);
    expect(ambientSprites(batch, PALETTE, 3, -5)).toEqual([]);
    expect(ambientSprites(batch, PALETTE, 3, Number.NaN)).toEqual([]);
    expect(ambientSprites(uniform(0, 0, "water"), PALETTE, 3, 500)).toEqual([]);
    expect(ambientSprites(uniform(10, 0, "water"), PALETTE, 3, 500)).toEqual([]);
  });

  it("lô thiếu cột dữ liệu không làm sập, chỉ không sinh hạt", () => {
    const broken: TileBatch = {
      x: 0,
      y: 0,
      w: 8,
      h: 8,
      z: 0,
      material: [],
      surface: [],
      drop: [],
      built: [],
      biome: [],
      height: [],
      river: [],
    };
    expect(() => ambientSprites(broken, PALETTE, 5, 100)).not.toThrow();
    expect(ambientSprites(broken, PALETTE, 5, 100)).toEqual([]);
  });

  it("tọa độ luôn nằm trong phạm vi lô, và mỗi hạt nằm trong ô sinh ra nó", () => {
    // Hạt lọt ra ngoài lô sẽ được vẽ ở chỗ chưa có dữ liệu nền — một chấm lơ
    // lửng trên nền trống.
    const batch = mixed();
    for (const t of [0, 5, 23, 47, 91, 480]) {
      const list = ambientSprites(batch, PALETTE, t, 100_000);
      expect(list.length).toBeGreaterThan(0);
      for (const s of list) {
        expect(s.x).toBeGreaterThanOrEqual(batch.x);
        expect(s.x).toBeLessThan(batch.x + batch.w);
        expect(s.y).toBeGreaterThanOrEqual(batch.y);
        expect(s.y).toBeLessThan(batch.y + batch.h);
      }
    }
  });

  it("lô một ô không có hàng xóm nào vẫn an toàn", () => {
    expect(() => ambientSprites(uniform(1, 1, "water"), PALETTE, 9, 10)).not.toThrow();
    expect(() => ambientSprites(uniform(1, 1, "sand"), PALETTE, 9, 10)).not.toThrow();
  });
});
