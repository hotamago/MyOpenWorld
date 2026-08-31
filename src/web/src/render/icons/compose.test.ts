import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  bakeAtlas,
  compose,
  IconError,
  iconKey,
  LAYERS,
  PrimitiveRegistry,
  validateAllSpecs,
  type IconSpec,
  type Layer,
  type Primitive,
} from "./compose";

const PRIMS = join(import.meta.dirname, "../../../../content/core/icons/primitives.json");

function soDangKy(): PrimitiveRegistry {
  const r = new PrimitiveRegistry();
  for (const p of JSON.parse(readFileSync(PRIMS, "utf-8")) as Primitive[]) {
    r.add(p);
  }
  return r;
}

describe("bóng nguyên thủy thật của dự án", () => {
  const reg = soDangKy();

  it("có khoảng một trăm bóng, phủ đủ năm lớp", () => {
    expect(reg.size).toBeGreaterThanOrEqual(100);
    for (const layer of LAYERS) {
      expect(reg.idsOfLayer(layer).length).toBeGreaterThan(0);
    }
  });

  it("mọi bóng đều có namespace", () => {
    for (const id of reg.ids()) expect(id).toContain(".");
  });

  it("mọi bóng đều là SVG hợp lệ tối thiểu", () => {
    for (const id of reg.ids()) {
      const svg = reg.get(id)!.svg;
      expect(svg.length).toBeGreaterThan(0);
      // Đếm thẻ mở và thẻ đóng khớp nhau: một thẻ hở sẽ nuốt mọi lớp vẽ sau nó.
      const mo = (svg.match(/<(path|rect|circle|g)\b/g) ?? []).length;
      const dong = (svg.match(/\/>|<\/(path|rect|circle|g)>/g) ?? []).length;
      expect(dong, `${id}: ${svg}`).toBe(mo);
    }
  });

  it("bóng nguyên thủy đăng ký trùng id là lỗi", () => {
    const r = new PrimitiveRegistry();
    const p: Primitive = { id: "core.x", layer: "silhouette", svg: "<path/>" };
    r.add(p);
    expect(() => r.add(p)).toThrow(/trùng/);
  });

  it("bóng thiếu namespace bị từ chối", () => {
    const r = new PrimitiveRegistry();
    expect(() => r.add({ id: "x", layer: "silhouette", svg: "<path/>" })).toThrow(
      /namespace/,
    );
  });
});

describe("§18.14.2 — khóa là hàm thuần của dữ liệu", () => {
  it("cùng đặc tả cho cùng khóa", () => {
    const a: IconSpec = { silhouette: "core.axe", material: "core.brass" };
    const b: IconSpec = { silhouette: "core.axe", material: "core.brass" };
    expect(iconKey(a)).toBe(iconKey(b));
  });

  it("thứ tự trạng thái không đổi khóa", () => {
    // Đây là bài quan trọng nhất của file. Nếu thứ tự đổi khóa, thì cùng một
    // món đồ sẽ chiếm hai ô atlas — tùy vào thứ tự effect được áp, và thứ tự
    // đó thay đổi. Atlas phình lên mà không ai hiểu vì sao.
    const a: IconSpec = { silhouette: "core.axe", states: ["core.wet", "core.burnt"] };
    const b: IconSpec = { silhouette: "core.axe", states: ["core.burnt", "core.wet"] };
    expect(iconKey(a)).toBe(iconKey(b));
  });

  it("đặc tả khác thì khóa khác", () => {
    const goc: IconSpec = { silhouette: "core.axe" };
    expect(iconKey(goc)).not.toBe(iconKey({ ...goc, material: "core.iron" }));
    expect(iconKey(goc)).not.toBe(iconKey({ ...goc, states: ["core.broken"] }));
    expect(iconKey(goc)).not.toBe(iconKey({ ...goc, marker: "core.owned" }));
    expect(iconKey(goc)).not.toBe(iconKey({ silhouette: "core.sword" }));
  });

  it("khóa không phụ thuộc thời điểm hay số lần gọi", () => {
    const s: IconSpec = { silhouette: "core.book", material: "core.leather" };
    const ds = Array.from({ length: 50 }, () => iconKey(s));
    expect(new Set(ds).size).toBe(1);
  });
});

describe("hợp thành", () => {
  const reg = soDangKy();

  it("rìu đồng thau bị mẻ của phe xanh — không ai phải vẽ gì cả", () => {
    const icon = compose(
      {
        silhouette: "core.axe",
        material: "core.brass",
        states: ["core.chipped"],
        marker: "core.faction_blue",
        annotation: "core.quality_fine",
      },
      reg,
    );
    expect(icon.used).toEqual([
      "core.axe",
      "core.brass",
      "core.chipped",
      "core.faction_blue",
      "core.quality_fine",
    ]);
    expect(icon.svg).toContain("<svg");
    expect(icon.svg).toContain('data-layer="silhouette"');
    expect(icon.svg).toContain('data-layer="marker"');
  });

  it("thứ tự vẽ khớp thứ tự trong khóa", () => {
    // Nếu hai thứ tự lệch nhau, hai icon có cùng khóa sẽ trông khác nhau, và
    // cái nào vào atlas là tùy cái nào tới trước.
    const icon = compose(
      { silhouette: "core.axe", states: ["core.wet", "core.burnt"] },
      reg,
    );
    expect(icon.used).toEqual(["core.axe", "core.burnt", "core.wet"]);
  });

  it("bóng không tồn tại là LỖI, không phải dấu hỏi", () => {
    // Dấu hỏi đã có nghĩa khác: `§18.14.5` dùng nó cho "chưa thẩm định". Vẽ nó
    // khi hợp thành lỗi sẽ khiến người chơi đọc một lỗi hiển thị thành một sự
    // thật về thế giới.
    expect(() => compose({ silhouette: "core.khong_co" }, reg)).toThrow(IconError);
    expect(() => compose({ silhouette: "core.khong_co" }, reg)).toThrow(/dấu hỏi/);
  });

  it("dùng bóng sai lớp là lỗi", () => {
    expect(() =>
      compose({ silhouette: "core.brass" }, reg),
    ).toThrow(/thuộc lớp/);
  });

  it("modder thêm vật liệu là có icon ngay", () => {
    // Đây là toàn bộ lý do hệ này tồn tại.
    const r = soDangKy();
    r.add({
      id: "mypack.mithril",
      layer: "material" as Layer,
      svg: '<rect width="32" height="32" fill="#c8e8f0" opacity="0.5"/>',
    });
    // Không sửa một dòng engine nào, mọi hình bóng sẵn có đều dùng được vật
    // liệu mới.
    for (const s of ["core.axe", "core.sword", "core.helmet", "core.pot"]) {
      const icon = compose({ silhouette: s, material: "mypack.mithril" }, r);
      expect(icon.used).toContain("mypack.mithril");
    }
  });
});

describe("PA-14 — bước CI", () => {
  const reg = soDangKy();

  it("mọi tổ hợp hình bóng × vật liệu đều giải ra được icon", () => {
    // Đây là bước CI mà `PA-14` yêu cầu: "CI kiểm tra mọi def giải ra được icon".
    const specs: IconSpec[] = [];
    for (const s of reg.idsOfLayer("silhouette")) {
      for (const m of reg.idsOfLayer("material")) {
        specs.push({ silhouette: s, material: m });
      }
    }
    expect(specs.length).toBeGreaterThan(1000);
    expect(validateAllSpecs(specs, reg)).toEqual([]);
  });

  it("một def thiếu bóng thì CI báo đúng cái nào thiếu", () => {
    const loi = validateAllSpecs(
      [{ silhouette: "core.axe" }, { silhouette: "core.chua_ton_tai" }],
      reg,
    );
    expect(loi).toHaveLength(1);
    expect(loi[0]).toContain("core.chua_ton_tai");
  });
});

describe("nướng atlas", () => {
  it("khóa trùng chỉ chiếm một ô", () => {
    // Một nghìn cái rìu đồng thau dùng chung một ô. Nếu không, atlas nổ tung.
    const keys = Array.from({ length: 1000 }, () =>
      iconKey({ silhouette: "core.axe", material: "core.brass" }),
    );
    const atlas = bakeAtlas(keys);
    expect(atlas.slots.size).toBe(1);
  });

  it("bố cục không phụ thuộc thứ tự gặp", () => {
    // Nhờ vậy hai lần chạy cho cùng một atlas, và ảnh chụp màn hình so sánh
    // được giữa các phiên bản.
    const keys = ["c", "a", "b", "d"];
    const a = bakeAtlas(keys);
    const b = bakeAtlas([...keys].reverse());
    expect([...a.slots]).toEqual([...b.slots]);
  });

  it("atlas đủ chỗ cho mọi khóa", () => {
    const keys = Array.from({ length: 37 }, (_, i) => `k${i}`);
    const atlas = bakeAtlas(keys);
    expect(atlas.slots.size).toBe(37);
    expect(atlas.columns * atlas.rows).toBeGreaterThanOrEqual(37);
  });
});
