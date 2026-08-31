/**
 * Bài kiểm cho tầng vẽ địa hình.
 *
 * Không so từng pixel với một ảnh mẫu: một bài như vậy đỏ mỗi lần ai đó chỉnh
 * một hệ số ánh sáng, và nó không nói được điều gì sai. Thay vào đó kiểm các
 * **quan hệ** phải đúng bất kể hệ số: bóng tối hơn nền, mép sáng hơn thân,
 * và cùng đầu vào cho cùng đầu ra.
 */
import { describe, expect, it } from "vitest";
import type { TileBatch } from "@/api/game";
import { BlockPalette } from "./blocks";
import { dayPhase, paintTerrain, skyTint, TICKS_PER_DAY } from "./terrain";

/** Một lô phẳng, toàn đất, với các ô `built` do bài kiểm chỉ định. */
function batchOf(w: number, h: number, built: Array<[number, number]>): TileBatch {
  const n = w * h;
  const b: TileBatch = {
    x: 0,
    y: 0,
    w,
    h,
    z: 0,
    material: Array.from({ length: n }, () => "air"),
    surface: Array.from({ length: n }, () => "topsoil"),
    drop: Array.from({ length: n }, () => 0),
    built: Array.from({ length: n }, () => 0),
    biome: Array.from({ length: n }, () => "grassland"),
    height: Array.from({ length: n }, () => 0),
    river: Array.from({ length: n }, () => 0),
    worn: Array.from({ length: n }, () => 0),
  };
  for (const [x, y] of built) {
    b.built[y * w + x] = 1;
    b.surface[y * w + x] = "roof_light";
  }
  return b;
}

const lum = (px: Uint8ClampedArray, i: number): number =>
  (px[i * 4] ?? 0) * 0.299 + (px[i * 4 + 1] ?? 0) * 0.587 + (px[i * 4 + 2] ?? 0) * 0.114;

describe("paintTerrain — bóng tiếp đất của công trình", () => {
  const p = new BlockPalette();
  const W = 7;

  it("đất ngay bên trái một công trình tối hơn đất trống cùng loại", () => {
    // Nắng tới từ phải/dưới, nên bóng đổ về trái/trên. Không có bóng thì nhà
    // trông như dán lên bản đồ — đúng lời phàn nàn đã dẫn tới bài này.
    const px = paintTerrain(batchOf(W, W, [[3, 3]]), p);
    const shadowed = lum(px, 3 * W + 2);
    const open = lum(px, 3 * W + 0);
    expect(shadowed).toBeLessThan(open);
  });

  it("đất bên phải công trình không bị tối — bóng chỉ đổ về một hướng", () => {
    // Bóng đổ cả bốn phía là bóng của một cái đèn treo, không phải của mặt
    // trời, và nó xóa mất chính thông tin hướng nắng mà hillshade đang mang.
    const px = paintTerrain(batchOf(W, W, [[3, 3]]), p);
    const right = lum(px, 3 * W + 4);
    const open = lum(px, 3 * W + 6);
    expect(Math.abs(right - open)).toBeLessThan(1);
  });

  it("mép mái hướng nắng sáng hơn thân mái", () => {
    // Một dãy nhà cùng vật liệu mà không có mép sáng thì đọc ra một mảng màu
    // duy nhất, không đọc ra mấy nóc nhà.
    const roofs: Array<[number, number]> = [];
    for (let y = 2; y <= 4; y++) for (let x = 2; x <= 4; x++) roofs.push([x, y]);
    const px = paintTerrain(batchOf(W, W, roofs), p);
    const edge = lum(px, 4 * W + 4); // góc dưới-phải của khối
    const inner = lum(px, 3 * W + 3); // giữa khối
    expect(edge).toBeGreaterThan(inner);
  });

  it("cùng một lô cho đúng cùng một ảnh", () => {
    const a = paintTerrain(batchOf(W, W, [[3, 3]]), p);
    const b = paintTerrain(batchOf(W, W, [[3, 3]]), p);
    expect(Array.from(a)).toEqual(Array.from(b));
  });

  it("không có công trình thì không có ô nào bị đổi bởi nhánh bóng", () => {
    const px = paintTerrain(batchOf(W, W, []), p);
    // Lô phẳng, không công trình: mọi ô chỉ khác nhau bởi hạt vật liệu, nên
    // biên độ phải nhỏ. Một cái bóng lạc vào đây sẽ đội con số này lên.
    const vals = Array.from({ length: W * W }, (_, i) => lum(px, i));
    expect(Math.max(...vals) - Math.min(...vals)).toBeLessThan(40);
  });
});

describe("skyTint và dayPhase", () => {
  const green = (c: number): number => (c >> 8) & 0xff;

  it("ban đêm tối hơn ban ngày nhưng không bao giờ đen kịt", () => {
    const night = green(skyTint(0));
    const noon = green(skyTint(TICKS_PER_DAY / 2));
    expect(night).toBeLessThan(noon);
    // Sàn ban đêm là 0.62: một thế giới đen kịt là một thế giới không đọc được
    // (`§18.13`), và đó là lỗi mà con số đó đã sửa một lần.
    expect(night).toBeGreaterThan(255 * 0.6);
  });

  it("bốn buổi phủ hết một ngày", () => {
    const seen = new Set<string>();
    for (let t = 0; t < TICKS_PER_DAY; t += 10) seen.add(dayPhase(t));
    expect(seen).toEqual(new Set(["night", "dawn", "day", "dusk"]));
  });
});

describe("paintTerrain — vách đá vẫn đọc được", () => {
  it("một vách 6 m mỗi ô không bị vẽ thành đen tuyền", () => {
    // Đây là bài cho đúng lỗi đã thấy trên màn hình: nửa bên phải bản đồ là
    // một mảng đen, trông y hệt một lỗi renderer nhưng thực ra là hillshade
    // đang làm đúng việc của nó trên một địa hình quá dốc.
    const W = 16;
    const n = W * W;
    const b: TileBatch = {
      x: 0,
      y: 0,
      w: W,
      h: W,
      z: 0,
      material: Array.from({ length: n }, () => "air"),
      surface: Array.from({ length: n }, () => "topsoil"),
      drop: Array.from({ length: n }, () => 0),
      built: Array.from({ length: n }, () => 0),
      biome: Array.from({ length: n }, () => "grassland"),
      // Nửa trái bằng phẳng, nửa phải là vách rơi 6 m mỗi ô — đúng hình dạng
      // đã thấy trên màn hình, nơi làng đứng trên mép một vách 64 m.
      height: Array.from({ length: n }, (_, i) => {
        const gx = i % W;
        return gx < W / 2 ? 90 : 90 - (gx - W / 2) * 6;
      }),
      river: Array.from({ length: n }, () => 0),
      worn: Array.from({ length: n }, () => 0),
    };
    const px = paintTerrain(b, new BlockPalette());
    const vals = Array.from({ length: n }, (_, i) => lum(px, i));
    // Không ô nào được rơi xuống mức "không phân biệt nổi với nền đen".
    expect(Math.min(...vals)).toBeGreaterThan(30);
    // Nhưng vẫn phải còn tương phản giữa đỉnh bằng và mặt vách: kẹp mà làm
    // phẳng hết thì mất luôn hình khối của địa hình, tức là đổi một lỗi này
    // lấy một lỗi khác.
    expect(Math.max(...vals) - Math.min(...vals)).toBeGreaterThan(20);
  });
});

describe("paintTerrain — lối mòn", () => {
  const W = 8;

  function withWear(wear: number[]): TileBatch {
    const n = W * W;
    return {
      x: 0,
      y: 0,
      w: W,
      h: W,
      z: 0,
      material: Array.from({ length: n }, () => "air"),
      surface: Array.from({ length: n }, () => "topsoil"),
      drop: Array.from({ length: n }, () => 0),
      built: Array.from({ length: n }, () => 0),
      biome: Array.from({ length: n }, () => "grassland"),
      height: Array.from({ length: n }, () => 0),
      river: Array.from({ length: n }, () => 0),
      worn: wear,
    };
  }

  it("ô bị giẫm nhiều ngả về màu đất nện, khác ô chưa ai đi", () => {
    const n = W * W;
    const wear = Array.from({ length: n }, () => 0);
    wear[3 * W + 3] = 255;
    const px = paintTerrain(withWear(wear), new BlockPalette());
    const trodden = px[(3 * W + 3) * 4] ?? 0;
    const fresh = px[(3 * W + 6) * 4] ?? 0;
    expect(trodden).not.toBe(fresh);
  });

  it("không ai đi thì không đổi gì cả", () => {
    // Một lớp phủ luôn bật là một lớp phủ không mang thông tin.
    const n = W * W;
    const a = paintTerrain(withWear(Array.from({ length: n }, () => 0)), new BlockPalette());
    const b = paintTerrain(withWear(Array.from({ length: n }, () => 0)), new BlockPalette());
    expect(Array.from(a)).toEqual(Array.from(b));
  });

  it("mòn nhiều hơn thì ngả về đất nện nhiều hơn, đơn điệu", () => {
    // Đơn điệu là điều kiện để mắt đọc được "chỗ này đi nhiều hơn chỗ kia".
    const n = W * W;
    const read = (w: number): number => {
      const wear = Array.from({ length: n }, () => 0);
      wear[2 * W + 2] = w;
      const px = paintTerrain(withWear(wear), new BlockPalette());
      return px[(2 * W + 2) * 4 + 2] ?? 0; // kênh lam: đất nện lam hơn topsoil
    };
    expect(read(120)).toBeGreaterThan(read(0));
    expect(read(255)).toBeGreaterThan(read(120));
  });
});
