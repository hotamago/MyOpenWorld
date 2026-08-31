/**
 * Test cho bản đồ thu nhỏ (`§18.3`).
 *
 * Bài test trung tâm ở đây là ["mode thắng"](#) và ["không sinh màu bùn"](#):
 * chúng mã hóa lý do module này không dùng `canvas.drawImage` để thu nhỏ. Nếu
 * ai đó sau này thay phần gộp bằng phép lấy trung bình vì "nhanh hơn", hai bài
 * đó phải đỏ ngay.
 */

import { describe, expect, it } from "vitest";
import type { TileBatch } from "@/api/game";
import { BlockPalette } from "./blocks";
import { minimapMarker, minimapToWorld, paintMinimap } from "./minimap";

const PALETTE = new BlockPalette();

/** Màu bảng dự phòng, viết thẳng ra để test không phụ thuộc thứ tự đọc bảng. */
const WATER: readonly [number, number, number] = [0x2c, 0x5c, 0x8a];
const SOIL: readonly [number, number, number] = [0x6b, 0x5a, 0x3e];

function fill<T>(n: number, v: T): T[] {
  return Array.from({ length: n }, () => v);
}

interface BatchOptions {
  x?: number;
  y?: number;
  surface?: string[];
  river?: number[];
  height?: number[];
}

/** Dựng một lô ô tối thiểu: mặc định đất bằng, không sông, cao độ 0. */
function batchOf(w: number, h: number, material: string[], opt: BatchOptions = {}): TileBatch {
  const n = w * h;
  return {
    x: opt.x ?? 0,
    y: opt.y ?? 0,
    w,
    h,
    z: 0,
    material,
    surface: opt.surface ?? material,
    drop: fill(n, 0),
    built: new Array<number>(n).fill(0),
    biome: fill(n, "plains"),
    height: opt.height ?? fill(n, 0),
    river: opt.river ?? fill(n, 0),
    worn: fill(n, 0),
  };
}

/**
 * Màu RGB của một pixel. Trả `-1` khi đọc ngoài buffer, chứ không `0`: `0` là
 * một màu hợp lệ và sẽ giấu mất lỗi chỉ số.
 */
function rgbAt(buf: Uint8ClampedArray, index: number): [number, number, number] {
  return [buf[index * 4] ?? -1, buf[index * 4 + 1] ?? -1, buf[index * 4 + 2] ?? -1];
}

function rgbXY(buf: Uint8ClampedArray, size: number, x: number, y: number) {
  return rgbAt(buf, y * size + x);
}

/** Khoảng cách bình phương trong không gian RGB. Đủ để so "gần màu nào hơn". */
function dist2(a: readonly [number, number, number], b: readonly [number, number, number]): number {
  return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2;
}

describe("§18.3 — bản đồ thu nhỏ", () => {
  it("buffer đúng size*size*4 và alpha luôn đầy", () => {
    // Alpha thiếu thì panel ở dưới lộ qua, và người chơi đọc màu panel thành
    // màu địa hình.
    const batch = batchOf(9, 5, fill(45, "topsoil"));
    for (const size of [1, 2, 7, 16, 33]) {
      const buf = paintMinimap(batch, PALETTE, size);
      expect(buf.length).toBe(size * size * 4);
      for (let i = 3; i < buf.length; i += 4) expect(buf[i]).toBe(255);
    }
  });

  it("mode thắng: 5 ô nước và 4 ô đất trong một pixel cho ra màu NƯỚC", () => {
    // Đây là bài test lý do tồn tại của module. Trung bình RGB cho (72, 91, 104)
    // — một màu bùn không có trong bảng vật liệu. Mode cho đúng #2c5c8a.
    const material = [
      "water", "water", "water",
      "water", "water", "topsoil",
      "topsoil", "topsoil", "topsoil",
    ];
    const buf = paintMinimap(batchOf(3, 3, material), PALETTE, 1);
    expect(rgbAt(buf, 0)).toEqual([...WATER]);
  });

  it("mode không phải trung bình: kết quả xa hẳn màu trung bình cộng", () => {
    const material = [
      "water", "water", "water",
      "water", "water", "topsoil",
      "topsoil", "topsoil", "topsoil",
    ];
    const averaged: [number, number, number] = [
      Math.round((5 * WATER[0] + 4 * SOIL[0]) / 9),
      Math.round((5 * WATER[1] + 4 * SOIL[1]) / 9),
      Math.round((5 * WATER[2] + 4 * SOIL[2]) / 9),
    ];
    const got = rgbAt(paintMinimap(batchOf(3, 3, material), PALETTE, 1), 0);

    expect(got).not.toEqual(averaged);
    expect(dist2(got, WATER)).toBe(0);
    // Màu bùn nằm cách cả hai vật liệu một quãng đáng kể — đó chính là vấn đề.
    expect(dist2(averaged, WATER)).toBeGreaterThan(500);
    expect(dist2(averaged, SOIL)).toBeGreaterThan(500);
  });

  it("thiểu số 4/9 vẫn thua, kể cả khi nó nằm ở ô đầu tiên được quét", () => {
    // Thứ tự quét không được ảnh hưởng kết quả khi có đa số rõ ràng.
    const material = [
      "topsoil", "topsoil", "water",
      "topsoil", "topsoil", "water",
      "water", "water", "water",
    ];
    expect(rgbAt(paintMinimap(batchOf(3, 3, material), PALETTE, 1), 0)).toEqual([...WATER]);
  });

  it("không sinh màu bùn: mọi pixel bờ biển đều là một màu có thật trong bảng", () => {
    // 16×16 nửa nước nửa đất, thu về 4×4 — mọi pixel đều nằm trên đường bờ.
    const material: string[] = [];
    for (let y = 0; y < 16; y++) {
      for (let x = 0; x < 16; x++) material.push(x + y < 16 ? "water" : "topsoil");
    }
    const buf = paintMinimap(batchOf(16, 16, material), PALETTE, 4);
    for (let i = 0; i < 16; i++) {
      const c = rgbAt(buf, i);
      const ok = dist2(c, WATER) === 0 || dist2(c, SOIL) === 0;
      expect(ok).toBe(true);
    }
  });

  it("một ô sông duy nhất trong nhóm gộp vẫn hiện ra là sông", () => {
    // Sông rộng một ô là 1/9 nhóm này: mode loại nó ngay. Luật `any` giữ nó lại,
    // vì sông là thứ người chơi dùng để định hướng.
    const material = fill(9, "topsoil");
    const plain = rgbAt(paintMinimap(batchOf(3, 3, material), PALETTE, 1), 0);

    for (let k = 0; k < 9; k++) {
      const river = fill(9, 0);
      river[k] = 1;
      const got = rgbAt(paintMinimap(batchOf(3, 3, material, { river }), PALETTE, 1), 0);

      expect(got).not.toEqual(plain);
      // Ngả lam rõ rệt: kênh lam vượt hẳn kênh đỏ, ngược với đất mặt.
      expect(got[2]).toBeGreaterThan(got[0] + 40);
      // Vị trí ô sông trong nhóm không đổi kết quả — `any` là `any`.
      const first = rgbAt(
        paintMinimap(batchOf(3, 3, material, { river: [1, 0, 0, 0, 0, 0, 0, 0, 0] }), PALETTE, 1),
        0,
      );
      expect(got).toEqual(first);
    }
  });

  it("sông không lan sang pixel hàng xóm", () => {
    // Luật `any` làm sông dày lên, nhưng chỉ trong đúng nhóm ô của nó.
    const river = fill(4, 0);
    river[0] = 1;
    const buf = paintMinimap(batchOf(2, 2, fill(4, "topsoil"), { river }), PALETTE, 2);
    expect(rgbXY(buf, 2, 0, 0)[2]).toBeGreaterThan(rgbXY(buf, 2, 1, 1)[2] + 40);
    expect(rgbXY(buf, 2, 1, 1)).toEqual([...SOIL]);
  });

  it("size lớn hơn batch.w không ném lỗi và không đọc ngoài mảng", () => {
    // Đọc ngoài mảng cho `undefined` → vật liệu rơi về "air" (#0d1014). Nếu
    // màu đó xuất hiện thì đã có chỉ số vượt biên ở đâu đó.
    const batch = batchOf(2, 2, ["water", "topsoil", "topsoil", "water"]);
    const size = 8;
    const buf = paintMinimap(batch, PALETTE, size);

    expect(buf.length).toBe(size * size * 4);
    const seen = new Set<string>();
    for (let i = 0; i < size * size; i++) seen.add(rgbAt(buf, i).join(","));
    expect(seen).toEqual(new Set([WATER.join(","), SOIL.join(",")]));

    // Bốn góc phóng to phải khớp bốn ô gốc.
    expect(rgbXY(buf, size, 0, 0)).toEqual([...WATER]);
    expect(rgbXY(buf, size, 7, 0)).toEqual([...SOIL]);
    expect(rgbXY(buf, size, 0, 7)).toEqual([...SOIL]);
    expect(rgbXY(buf, size, 7, 7)).toEqual([...WATER]);
  });

  it("phóng to cực đoan (1 ô, 64 pixel) vẫn cho ảnh đặc một màu", () => {
    const buf = paintMinimap(batchOf(1, 1, ["water"]), PALETTE, 64);
    for (let i = 0; i < 64 * 64; i++) expect(rgbAt(buf, i)).toEqual([...WATER]);
  });

  it("ô không khí lấy màu mặt đất bên dưới, không phải màu không khí", () => {
    // Cùng luật ghost lớp dưới với `terrain.ts` (`§18.1`); nếu không, đứng ở
    // lát cao thì bản đồ thu nhỏ là một hình chữ nhật đen.
    const batch = batchOf(1, 1, ["air"], { surface: ["water"] });
    expect(rgbAt(paintMinimap(batch, PALETTE, 1), 0)).toEqual([...WATER]);
  });

  it("đổ bóng theo độ cao nằm trong 0.90–1.10 và không đổi phân loại", () => {
    const flat = rgbAt(paintMinimap(batchOf(1, 1, ["topsoil"]), PALETTE, 1), 0);
    const peak = rgbAt(
      paintMinimap(batchOf(1, 1, ["topsoil"], { height: [9000] }), PALETTE, 1),
      0,
    );
    const abyss = rgbAt(
      paintMinimap(batchOf(1, 1, ["topsoil"], { height: [-9000] }), PALETTE, 1),
      0,
    );

    expect(flat).toEqual([...SOIL]);
    expect(peak[0]).toBeGreaterThan(flat[0]);
    expect(abyss[0]).toBeLessThan(flat[0]);
    for (let k = 0; k < 3; k++) {
      const base = SOIL[k] ?? 0;
      expect(peak[k] ?? 0).toBeLessThanOrEqual(Math.round(base * 1.1));
      expect(abyss[k] ?? 0).toBeGreaterThanOrEqual(Math.round(base * 0.9));
    }

    // Ràng buộc thật sự: bóng không được nuốt kênh vật liệu. Nước trên đỉnh núi
    // vẫn phải gần màu nước hơn màu đất.
    const highWater = rgbAt(
      paintMinimap(batchOf(1, 1, ["water"], { height: [9000] }), PALETTE, 1),
      0,
    );
    expect(dist2(highWater, WATER)).toBeLessThan(dist2(highWater, SOIL));
  });

  it("minimapMarker: bốn góc lô rơi đúng bốn góc ảnh", () => {
    const batch = batchOf(64, 64, fill(64 * 64, "topsoil"), { x: 1000, y: -500 });
    expect(minimapMarker(batch, 16, 1000, -500)).toEqual({ x: 0, y: 0 });
    expect(minimapMarker(batch, 16, 1063, -437)).toEqual({ x: 15, y: 15 });
  });

  it("minimapMarker trả null cho toạ độ ngoài lô", () => {
    // Kẹp về mép sẽ vẽ một dấu trông y hệt dấu thật ở mép, và người chơi sẽ đi
    // tới một chỗ không có gì.
    const batch = batchOf(64, 64, fill(64 * 64, "topsoil"), { x: 1000, y: -500 });
    expect(minimapMarker(batch, 16, 999, -500)).toBeNull();
    expect(minimapMarker(batch, 16, 1064, -500)).toBeNull();
    expect(minimapMarker(batch, 16, 1000, -501)).toBeNull();
    expect(minimapMarker(batch, 16, 1000, -436)).toBeNull();
    expect(minimapMarker(batch, 16, -99999, 99999)).toBeNull();
  });

  it("minimapMarker nhận toạ độ phân số, lấy ô chứa điểm", () => {
    const batch = batchOf(64, 64, fill(64 * 64, "topsoil"), { x: 0, y: 0 });
    expect(minimapMarker(batch, 16, 7.9, 7.9)).toEqual(minimapMarker(batch, 16, 7, 7));
  });

  it("khứ hồi pixel → thế giới → pixel đúng trong sai số một pixel", () => {
    for (const [w, h, size] of [
      [64, 64, 16],
      [70, 41, 16],
      [87, 41, 32],
      [16, 16, 16],
    ] as const) {
      const batch = batchOf(w, h, fill(w * h, "topsoil"), { x: 1000, y: -500 });
      for (let py = 0; py < size; py++) {
        for (let px = 0; px < size; px++) {
          const world = minimapToWorld(batch, size, px, py);
          const back = minimapMarker(batch, size, world.x, world.y);
          if (back === null) throw new Error(`ô ${world.x},${world.y} rơi ra ngoài lô ${w}×${h}`);
          expect(Math.abs(back.x - px)).toBeLessThanOrEqual(1);
          expect(Math.abs(back.y - py)).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  it("khứ hồi thế giới → pixel → thế giới sai lệch dưới một nhóm ô", () => {
    const w = 80;
    const size = 20;
    const batch = batchOf(w, w, fill(w * w, "topsoil"), { x: -7, y: 3 });
    const tilesPerPixel = w / size;
    for (let t = 0; t < w; t++) {
      const p = minimapMarker(batch, size, -7 + t, 3 + t);
      if (p === null) throw new Error(`ô ${t} rơi ra ngoài lô`);
      const world = minimapToWorld(batch, size, p.x, p.y);
      expect(Math.abs(world.x - (-7 + t))).toBeLessThanOrEqual(tilesPerPixel);
      expect(Math.abs(world.y - (3 + t))).toBeLessThanOrEqual(tilesPerPixel);
    }
  });

  it("minimapToWorld trả ô nguyên, ở giữa nhóm, và kẹp pixel ngoài ảnh", () => {
    const batch = batchOf(80, 80, fill(6400, "topsoil"), { x: 100, y: 200 });
    const mid = minimapToWorld(batch, 20, 0, 0);
    // Nhóm là ô 0..3; lấy góc sẽ cho 100, lấy giữa cho 101.
    expect(mid).toEqual({ x: 101, y: 201 });
    expect(Number.isInteger(mid.x) && Number.isInteger(mid.y)).toBe(true);

    // Chuột ở cạnh phải canvas cho ra đúng `size` sau khi chia tỉ lệ.
    expect(minimapToWorld(batch, 20, 20, 20)).toEqual(minimapToWorld(batch, 20, 19, 19));
    expect(minimapToWorld(batch, 20, -3, -3)).toEqual(minimapToWorld(batch, 20, 0, 0));
  });

  it("xác định: cùng đầu vào cho cùng buffer", () => {
    // Không xác định thì ảnh chụp màn hình so sánh giữa hai bản sẽ khác nhau vì
    // lý do không liên quan tới thay đổi đang xét.
    const material: string[] = [];
    const river: number[] = [];
    const height: number[] = [];
    for (let i = 0; i < 40 * 24; i++) {
      material.push(["water", "topsoil", "sand", "igneous"][i % 4] ?? "air");
      river.push(i % 17 === 0 ? 1 : 0);
      height.push(((i * 37) % 500) - 150);
    }
    const batch = batchOf(40, 24, material, { river, height });
    const a = paintMinimap(batch, PALETTE, 24);
    const b = paintMinimap(batch, PALETTE, 24);
    expect(Array.from(a)).toEqual(Array.from(b));
  });

  it("lô rỗng cho ảnh đặc, không sập và không có pixel trong suốt", () => {
    const empty = batchOf(0, 0, []);
    const buf = paintMinimap(empty, PALETTE, 4);
    expect(buf.length).toBe(4 * 4 * 4);
    for (let i = 3; i < buf.length; i += 4) expect(buf[i]).toBe(255);
    expect(minimapMarker(empty, 4, 0, 0)).toBeNull();
    expect(minimapToWorld(empty, 4, 2, 2)).toEqual({ x: 0, y: 0 });
  });

  it("size vô nghĩa báo lỗi thay vì trả buffer rỗng", () => {
    // Buffer rỗng sẽ đi tiếp vào `ImageData` và ném ở đó, cách chỗ sai một tầng.
    const batch = batchOf(4, 4, fill(16, "topsoil"));
    expect(() => paintMinimap(batch, PALETTE, 0)).toThrow(RangeError);
    expect(() => paintMinimap(batch, PALETTE, -8)).toThrow(RangeError);
    expect(() => paintMinimap(batch, PALETTE, Number.NaN)).toThrow(RangeError);
    expect(minimapMarker(batch, 0, 0, 0)).toBeNull();
  });

  it("cột dữ liệu ngắn hơn w*h không làm sập panel", () => {
    // Server hỏng hoặc JSON bị cắt: giao diện phải vẽ được một cái gì đó thay
    // vì ném ở giữa vòng lặp và để lại canvas vẽ dở.
    const batch: TileBatch = {
      ...batchOf(4, 4, fill(16, "topsoil")),
      material: ["topsoil"],
      surface: ["topsoil"],
      river: [],
      worn: [],
      height: [],
    };
    const buf = paintMinimap(batch, PALETTE, 4);
    expect(buf.length).toBe(4 * 4 * 4);
    for (let i = 3; i < buf.length; i += 4) expect(buf[i]).toBe(255);
    // Ô thiếu rơi về "air", tức màu của chính vật liệu air trong bảng.
    expect(rgbXY(buf, 4, 3, 3)).toEqual([0x0d, 0x10, 0x14]);
  });
});
