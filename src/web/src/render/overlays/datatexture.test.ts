import { describe, expect, it } from "vitest";
import {
  buildLegend,
  formatValue,
  OverlayChannel,
  OverlayGroup,
  OVERLAY_FRAGMENT_SHADER,
} from "./datatexture";

function kenh(): OverlayChannel {
  return new OverlayChannel({
    id: "temperature",
    unit: "°C",
    domain: [-40, 50],
    width: 32,
    height: 32,
  });
}

describe("PA-12 — overlay là data texture một kênh", () => {
  it("một byte mỗi ô, không hơn", () => {
    // Sprite mỗi ô sụp đổ ở 100 000 ô. Một texture 32×32 phải là đúng 1024 byte.
    expect(kenh().byteLength).toBe(32 * 32);
  });

  it("giá trị thật đi và về qua lượng tử hóa", () => {
    const c = kenh();
    c.set(5, 5, 20);
    // Lượng tử hóa 8 bit trên miền 90 độ: bước ~0.35 độ.
    expect(c.get(5, 5)).toBeCloseTo(20, 0);
  });

  it("giá trị ngoài miền bị kẹp, không quay vòng", () => {
    const c = kenh();
    c.set(0, 0, 1000);
    expect(c.raw(0, 0)).toBe(255);
    c.set(1, 0, -1000);
    expect(c.raw(1, 0)).toBe(0);
  });

  it("đổi thang màu không đụng vào dữ liệu", () => {
    // Đây là lợi ích chính của data texture: thang nằm trong shader, không
    // trong dữ liệu.
    const c = kenh();
    c.set(3, 3, 10);
    const truoc = c.raw(3, 3);
    expect(OVERLAY_FRAGMENT_SHADER).toContain("uRamp");
    expect(c.raw(3, 3)).toBe(truoc);
  });

  it("shader lấy mẫu một kênh, không phải RGBA", () => {
    // RGBA tốn gấp bốn băng thông cho ba kênh không ai đọc.
    expect(OVERLAY_FRAGMENT_SHADER).toContain(".r;");
  });
});

describe("§18.6 — đơn vị là bắt buộc", () => {
  it("không thể tạo overlay thiếu đơn vị", () => {
    expect(
      () =>
        new OverlayChannel({ id: "x", unit: "", domain: [0, 1], width: 4, height: 4 }),
    ).toThrow(/đơn vị/);
  });

  it("miền suy biến bị từ chối", () => {
    expect(
      () =>
        new OverlayChannel({ id: "x", unit: "m", domain: [5, 5], width: 4, height: 4 }),
    ).toThrow(/suy biến/);
  });

  it("legend luôn kèm đơn vị thật", () => {
    const l = buildLegend(kenh());
    expect(l).toHaveLength(5);
    for (const s of l) expect(s.label).toContain("°C");
    expect(l[0]!.value).toBe(-40);
    expect(l[4]!.value).toBe(50);
    expect(l[0]!.byte).toBe(0);
    expect(l[4]!.byte).toBe(255);
  });

  it("định dạng số theo độ lớn, không cố định số chữ số", () => {
    // Quy tắc "hai chữ số thập phân" sẽ biến 0.00042 thành 0.00.
    expect(formatValue(0)).toBe("0");
    expect(formatValue(0.00042)).toContain("e-");
    expect(formatValue(0.5)).toBe("0.5");
    expect(formatValue(42)).toBe("42");
    expect(formatValue(1500)).toBe("1.5k");
    expect(formatValue(2_500_000)).toBe("2.5M");
  });
});

describe("§18.5 — overlay là nhóm loại trừ", () => {
  it("bật cái này thì cái kia tự tắt", () => {
    // Chồng hai bản đồ nhiệt cho ra một màu thứ ba không có nghĩa gì, và người
    // xem sẽ đọc nó như một dữ liệu thật.
    const g = new OverlayGroup();
    g.register(kenh());
    g.register(
      new OverlayChannel({
        id: "moisture",
        unit: "%",
        domain: [0, 100],
        width: 8,
        height: 8,
      }),
    );

    g.activate("temperature");
    expect(g.active?.id).toBe("temperature");
    g.activate("moisture");
    expect(g.active?.id).toBe("moisture");
    g.activate(null);
    expect(g.active).toBeNull();
  });

  it("legend có khi và chỉ khi có overlay bật", () => {
    const g = new OverlayGroup();
    g.register(kenh());
    expect(g.legend()).toBeNull();
    g.activate("temperature");
    expect(g.legend()).toHaveLength(5);
  });

  it("bật overlay không tồn tại là lỗi", () => {
    expect(() => new OverlayGroup().activate("khong-co")).toThrow();
  });
});
