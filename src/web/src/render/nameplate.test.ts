import { describe, expect, it } from "vitest";
import { DEFAULT_LABEL_LIMIT, DEFAULT_MIN_LABEL_TILE_SIZE, type Nameplate, visibleLabels } from "./nameplate";

/** Khung nhìn 100×100 ô, đủ rộng để không phải là biến độc lập trong hầu hết test. */
const BIG_VIEWPORT = { x: 0, y: 0, w: 100, h: 100 };

/** Một nhãn tối thiểu, đặt ở toạ độ `(x, y)`. Test ghi đè `id`/`highlight` khi cần. */
function label(x: number, y: number, overrides: Partial<Nameplate> = {}): Nameplate {
  return { id: `${x}:${y}`, text: `(${x},${y})`, x, y, highlight: false, ...overrides };
}

describe("visibleLabels — ẩn theo mức phóng", () => {
  it("tileSize dưới ngưỡng: ẩn hết, kể cả nhãn đang chọn", () => {
    const list = [label(1, 1, { highlight: true }), label(2, 2)];
    const out = visibleLabels(list, {
      tileSize: DEFAULT_MIN_LABEL_TILE_SIZE - 1,
      viewport: BIG_VIEWPORT,
      centerX: 0,
      centerY: 0,
    });
    expect(out).toEqual([]);
  });

  it("tileSize đúng bằng ngưỡng: vẫn hiện — ngưỡng là biên dưới bị loại, không phải biên trên", () => {
    const list = [label(1, 1)];
    const out = visibleLabels(list, {
      tileSize: DEFAULT_MIN_LABEL_TILE_SIZE,
      viewport: BIG_VIEWPORT,
      centerX: 0,
      centerY: 0,
    });
    expect(out).toHaveLength(1);
  });

  it("tileSize trên ngưỡng: hiện bình thường", () => {
    const list = [label(1, 1)];
    const out = visibleLabels(list, {
      tileSize: DEFAULT_MIN_LABEL_TILE_SIZE + 10,
      viewport: BIG_VIEWPORT,
      centerX: 0,
      centerY: 0,
    });
    expect(out).toHaveLength(1);
  });
});

describe("visibleLabels — lọc theo khung nhìn", () => {
  const opts = { tileSize: 18, centerX: 5, centerY: 5 };

  it("loại nhãn ngoài khung: quá trái/trên hoặc quá phải/dưới", () => {
    const viewport = { x: 10, y: 10, w: 5, h: 5 };
    const list = [
      label(9, 12), // ngoài trái
      label(12, 9), // ngoài trên
      label(15, 12), // đúng biên phải, đã ra ngoài (biên mở)
      label(12, 15), // đúng biên dưới, đã ra ngoài (biên mở)
      label(12, 12), // giữa vùng — phải còn
    ];
    const out = visibleLabels(list, { ...opts, viewport });
    expect(out.map((l) => l.id)).toEqual(["12:12"]);
  });

  it("biên trên/trái đóng: nhãn đúng tại (viewport.x, viewport.y) vẫn hiện", () => {
    const viewport = { x: 10, y: 10, w: 5, h: 5 };
    const out = visibleLabels([label(10, 10)], { ...opts, viewport });
    expect(out).toHaveLength(1);
  });
});

describe("visibleLabels — giới hạn số lượng", () => {
  it("dưới hoặc bằng giới hạn: giữ nguyên, không sắp lại", () => {
    // Cố tình đặt nhãn xa tâm đứng trước nhãn gần tâm trong mảng gốc — nếu hàm
    // lỡ sắp xếp khi không cần, thứ tự đầu ra sẽ đổi và test này bắt được.
    const list = [label(90, 90, { id: "far" }), label(1, 1, { id: "near" })];
    const out = visibleLabels(list, { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 0, centerY: 0 });
    expect(out.map((l) => l.id)).toEqual(["far", "near"]);
  });

  it("vượt giới hạn: giữ đúng số lượng đã đặt", () => {
    const list = Array.from({ length: 100 }, (_, i) => label(i, 0));
    const out = visibleLabels(list, { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 0, centerY: 0, limit: 5 });
    expect(out).toHaveLength(5);
  });

  it("mặc định trần là DEFAULT_LABEL_LIMIT", () => {
    const list = Array.from({ length: DEFAULT_LABEL_LIMIT + 20 }, (_, i) => label(i, 0));
    const out = visibleLabels(list, { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 0, centerY: 0 });
    expect(out).toHaveLength(DEFAULT_LABEL_LIMIT);
  });

  it("vượt giới hạn: ưu tiên nhãn gần tâm màn hình hơn", () => {
    const list = [
      label(0, 0, { id: "xa" }),
      label(1, 0, { id: "gan" }),
      label(50, 50, { id: "rat-xa" }),
    ];
    const out = visibleLabels(list, { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 1, centerY: 0, limit: 2 });
    expect(out.map((l) => l.id).sort()).toEqual(["gan", "xa"]);
  });

  it("vượt giới hạn: nhãn highlight luôn được giữ dù ở xa tâm màn hình", () => {
    const list = [
      label(50, 50, { id: "highlight-xa", highlight: true }),
      ...Array.from({ length: 50 }, (_, i) => label(i, 0, { id: `crowd-${i}` })),
    ];
    const out = visibleLabels(list, { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 0, centerY: 0, limit: 3 });
    expect(out.some((l) => l.id === "highlight-xa")).toBe(true);
    expect(out).toHaveLength(3);
  });

  it("nhiều hơn một highlight vượt giới hạn: vẫn ưu tiên tất cả highlight trước non-highlight", () => {
    const list = [
      label(0, 0, { id: "h1", highlight: true }),
      label(99, 99, { id: "h2", highlight: true }),
      label(1, 0, { id: "thuong-gan", highlight: false }),
    ];
    const out = visibleLabels(list, { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 0, centerY: 0, limit: 2 });
    expect(out.map((l) => l.id).sort()).toEqual(["h1", "h2"]);
  });
});

describe("visibleLabels — trường hợp biên", () => {
  it("mảng rỗng: trả mảng rỗng, không ném lỗi", () => {
    expect(visibleLabels([], { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 0, centerY: 0 })).toEqual([]);
  });

  it("không đụng vào mảng đầu vào (không mutate)", () => {
    const list = Array.from({ length: 50 }, (_, i) => label(i, 0));
    const copy = [...list];
    visibleLabels(list, { tileSize: 18, viewport: BIG_VIEWPORT, centerX: 0, centerY: 0, limit: 5 });
    expect(list).toEqual(copy);
  });
});
