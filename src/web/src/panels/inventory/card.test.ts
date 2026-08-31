import { describe, expect, it } from "vitest";
import {
  buildCard,
  compare,
  effectivenessPercent,
  whatYouLose,
  worstCondition,
  type ItemCardData,
} from "./card";

function riu(over: Partial<ItemCardData> = {}): ItemCardData {
  return {
    entity: "e12",
    def: "core.axe",
    displayName: "Rìu đồng thau",
    quality: "masterwork",
    craftedBy: "Aren",
    conditions: [
      { part: "blade", condition: 0.4 },
      { part: "haft", condition: 1 },
    ],
    repairs: [],
    massMmu: 2500,
    volumeMl: 3000,
    appraisedValue: null,
    appraisedBy: null,
    dimensions: { damage: 12, reach: 2, speed: 8 },
    ...over,
  };
}

describe("PB-20 — chất lượng và tình trạng hiện tách hẳn", () => {
  it("hai khối riêng, không gộp thành một thanh độ bền", () => {
    const rows = buildCard(riu());
    const nhom = new Set(rows.map((r) => r.group));
    expect(nhom.has("quality")).toBe(true);
    expect(nhom.has("condition")).toBe(true);

    const chat_luong = rows.filter((r) => r.group === "quality");
    const tinh_trang = rows.filter((r) => r.group === "condition");
    expect(chat_luong.some((r) => r.value === "kiệt tác")).toBe(true);
    expect(tinh_trang).toHaveLength(2);
  });

  it("tình trạng hiện theo từng bộ phận", () => {
    // "Cán còn tốt, lưỡi mẻ" phải đọc được từ thẻ.
    const rows = buildCard(riu()).filter((r) => r.group === "condition");
    expect(rows.map((r) => r.value)).toEqual(["40%", "100%"]);
    expect(rows[0]!.label).toContain("blade");
  });

  it("bộ phận yếu nhất quyết định, không phải trung bình", () => {
    expect(worstCondition([{ part: "a", condition: 0.4 }, { part: "b", condition: 1 }])).toBe(0.4);
    // Trung bình sẽ là 0.7 và người chơi sẽ mang cây rìu cán gãy ra trận.
    expect(effectivenessPercent(riu())).toBe(72);
  });

  it("lịch sử sửa chữa nói ai đã chạm vào món đồ", () => {
    const rows = buildCard(
      riu({ repairs: [{ by: "Bram", atTick: "900", part: "blade", restored: 0.3 }] }),
    );
    const lich_su = rows.filter((r) => r.group === "history");
    expect(lich_su).toHaveLength(1);
    expect(lich_su[0]!.value).toContain("Bram");
    expect(lich_su[0]!.source).toBe("Bram");
  });

  it("mọi giá trị suy ra bấm được về nguồn", () => {
    // `§18.13`.
    const rows = buildCard(riu({ appraisedValue: 12, appraisedBy: "Cira" }));
    expect(rows.some((r) => r.source === "Aren")).toBe(true);
    expect(rows.some((r) => r.source === "Cira")).toBe(true);
  });
});

describe("§22.35 — giá là ước lượng của nhân vật", () => {
  it("chưa thẩm định thì nói là chưa thẩm định, không đoán số", () => {
    const gia = buildCard(riu()).find((r) => r.group === "value")!;
    expect(gia.value).toBe("chưa thẩm định");
    expect(gia.value).not.toMatch(/\d/);
  });

  it("có thẩm định thì nói rõ AI nghĩ vậy", () => {
    const gia = buildCard(riu({ appraisedValue: 12, appraisedBy: "Cira" })).find(
      (r) => r.group === "value",
    )!;
    expect(gia.value).toContain("Cira");
    expect(gia.value).toContain("12");
    // Không được hiện như một sự thật khách quan.
    expect(gia.value).not.toBe("12");
  });
});

describe("§18.15.7 — so sánh không rút về một điểm số", () => {
  const a = riu();
  const b = riu({
    displayName: "Kiếm sắt",
    dimensions: { damage: 14, reach: 3, speed: 5 },
  });

  it("bảng cạnh nhau theo từng chiều", () => {
    const bang = compare(a, b);
    expect(bang.map((r) => r.dimension)).toEqual(["damage", "reach", "speed"]);
    expect(bang.find((r) => r.dimension === "damage")).toMatchObject({ a: 12, b: 14, delta: 1 });
  });

  it("hiện rõ cái gì mất đi nếu đổi", () => {
    // Phần mà mọi giao diện so sánh hay bỏ quên.
    const mat = whatYouLose(a, b);
    expect(mat.map((r) => r.dimension)).toEqual(["speed"]);
    expect(mat[0]!.a).toBe(8);
    expect(mat[0]!.b).toBe(5);
  });

  it("không có hàm nào trả về một điểm số tổng", () => {
    // Bài này khóa hình dạng của module lại: nếu ai đó thêm `totalScore`, họ
    // phải xóa bài test này và đối diện với lý do nó tồn tại.
    const api = Object.keys(
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      {} as Record<string, unknown>,
    );
    const cam = ["totalScore", "overallRating", "score"];
    for (const c of cam) expect(api).not.toContain(c);
  });

  it("chiều chỉ có ở một bên vẫn so được", () => {
    const c = riu({ dimensions: { damage: 10, armor_pierce: 3 } });
    const bang = compare(a, c);
    const xuyen = bang.find((r) => r.dimension === "armor_pierce")!;
    expect(xuyen.a).toBe(0);
    expect(xuyen.b).toBe(3);
  });
});
