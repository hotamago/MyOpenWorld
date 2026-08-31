import { describe, expect, it } from "vitest";
import {
  bakeTile,
  TILE_SIZE,
  TileAtlas,
  tileKey,
  VARIANTS,
  variantAt,
  type MaterialDef,
} from "./atlas";

const DA: MaterialDef = { id: "core.stone", color: "#7d7f82", roughness: 2 };
const NUOC: MaterialDef = { id: "core.water", color: "#2f7fa8", roughness: 0, liquid: true };
const BANG: MaterialDef = { id: "core.ice", color: "#dfeef5", roughness: 0 };

describe("PA-11 — tile suy ra từ dữ liệu vật liệu", () => {
  it("modder thêm vật liệu là có tile ngay", () => {
    // Đây là toàn bộ lý do hệ này tồn tại.
    const bazan: MaterialDef = { id: "mypack.basalt", color: "#3a3a42", roughness: 3 };
    const a = new TileAtlas();
    a.bake([DA, bazan]);
    expect(a.size).toBe(2 * VARIANTS);
    expect(a.get(tileKey(bazan, 0))).toBeDefined();
  });

  it("mỗi vật liệu có nhiều biến thể để mắt không thấy hoa văn lặp", () => {
    const a = new TileAtlas();
    a.bake([DA]);
    const px = [0, 1, 2, 3].map((v) => a.get(tileKey(DA, v))!.pixels.join(","));
    expect(new Set(px).size).toBe(VARIANTS);
  });

  it("biến thể là hàm thuần của tọa độ, không phải ngẫu nhiên", () => {
    // Nếu ngẫu nhiên, cùng một ô sẽ đổi diện mạo mỗi khi chunk được nạp lại —
    // mặt đất nhấp nháy khi người chơi đi qua đi lại.
    expect(variantAt(100n, 200n)).toBe(variantAt(100n, 200n));
    expect(variantAt(0n, 0n)).toBeGreaterThanOrEqual(0);
    expect(variantAt(0n, 0n)).toBeLessThan(VARIANTS);
  });

  it("biến thể phân bố đều, không dính một giá trị", () => {
    const dem = [0, 0, 0, 0];
    for (let x = 0n; x < 400n; x++) {
      for (let y = 0n; y < 5n; y++) dem[variantAt(x, y)]!++;
    }
    // 2000 mẫu, kỳ vọng 500 mỗi loại. Biên độ rộng nhưng vẫn bắt được hàm băm
    // hỏng theo kiểu "luôn trả về 0".
    for (const n of dem) expect(n).toBeGreaterThan(300);
  });

  it("biến thể ổn định ở tọa độ vượt 2^53", () => {
    const xa = 1n << 55n;
    expect(variantAt(xa, xa)).toBe(variantAt(xa, xa));
    expect(variantAt(xa, xa)).toBeLessThan(VARIANTS);
  });

  it("chỉ số trong atlas ổn định giữa các lần chạy", () => {
    // Không ổn định thì ảnh chụp màn hình so sánh giữa hai phiên bản sẽ khác
    // nhau chỉ vì thứ tự đọc thư mục khác nhau.
    const mk = (dao: boolean) => {
      const a = new TileAtlas();
      const ds = [DA, NUOC, BANG];
      a.bake(dao ? [...ds].reverse() : ds);
      return a.keys();
    };
    expect(mk(false)).toEqual(mk(true));
  });

  it("chất lỏng được vẽ với gợn sóng, không chỉ khác màu", () => {
    // Người mù màu vẫn phải phân biệt nước với đồng cỏ.
    const kho = bakeTile({ ...NUOC, liquid: false }, 0);
    const uot = bakeTile(NUOC, 0);
    expect(uot.pixels.join(",")).not.toBe(kho.pixels.join(","));
  });

  it("độ nhám 0 cho bề mặt phẳng tuyệt đối", () => {
    const t = bakeTile(BANG, 0);
    const dau = [t.pixels[0], t.pixels[1], t.pixels[2]];
    for (let i = 0; i < t.pixels.length; i += 4) {
      expect([t.pixels[i], t.pixels[i + 1], t.pixels[i + 2]]).toEqual(dau);
    }
  });

  it("độ nhám cao cho nhiều hạt hơn", () => {
    const dem_khac = (r: number) => {
      const t = bakeTile({ id: "x.y", color: "#808080", roughness: r }, 0);
      return new Set(
        Array.from({ length: TILE_SIZE * TILE_SIZE }, (_, i) => t.pixels[i * 4]),
      ).size;
    };
    expect(dem_khac(3)).toBeGreaterThan(dem_khac(1));
    expect(dem_khac(1)).toBeGreaterThan(dem_khac(0));
  });

  it("vật liệu chưa nướng thì báo lỗi rõ ràng", () => {
    const a = new TileAtlas();
    a.bake([DA]);
    expect(() => a.tileAt(NUOC, 0n, 0n)).toThrow(/chưa được nướng/);
  });

  it("màu không hợp lệ bị bắt", () => {
    expect(() =>
      bakeTile({ id: "x.y", color: "khong-phai-mau", roughness: 1 }, 0),
    ).toThrow();
  });

  it("ô có đúng kích thước và kênh alpha đầy", () => {
    const t = bakeTile(DA, 0);
    expect(t.width).toBe(TILE_SIZE);
    expect(t.pixels.length).toBe(TILE_SIZE * TILE_SIZE * 4);
    for (let i = 3; i < t.pixels.length; i += 4) expect(t.pixels[i]).toBe(255);
  });
});
