import { describe, expect, it } from "vitest";
import {
  AXES,
  compare,
  gains,
  HIGHER_IS_BETTER,
  isTradeoff,
  losses,
  worstPart,
  type Item,
} from "./model";

function giapNhe(): Item {
  return {
    id: "item.leather",
    displayName: "giáp da",
    axes: { weight: 200, coverage: 300, mobility: 800, durability: 400, noise: 200 },
    affordances: ["swim", "sneak", "climb"],
    effects: [],
    parts: [
      { part: "chest", condition: 900 },
      { part: "arms", condition: 700 },
    ],
    requirements: [],
  };
}

function giapNang(): Item {
  return {
    id: "item.plate",
    displayName: "giáp tấm",
    axes: { weight: 900, coverage: 900, mobility: 200, durability: 900, noise: 800 },
    affordances: [],
    effects: ["effect.warmth"],
    parts: [
      { part: "chest", condition: 1000 },
      { part: "arms", condition: 200 },
    ],
    requirements: ["skill.armour_training"],
  };
}

describe("không rút về một điểm số duy nhất", () => {
  it("không có hàm nào trả về 'cái nào tốt hơn'", () => {
    const c = compare(giapNhe(), giapNang());
    // Mọi khóa của kết quả — không được có `overallWinner` hay `score`.
    const khoa = Object.keys(c);
    for (const cam of ["overallWinner", "score", "total", "rating"]) {
      expect(khoa).not.toContain(cam);
    }
  });

  it("mỗi chiều có người thắng riêng, và hai bên chia nhau", () => {
    const c = compare(giapNhe(), giapNang());
    const thangA = c.axes.filter((x) => x.winner === "a").map((x) => x.axis);
    const thangB = c.axes.filter((x) => x.winner === "b").map((x) => x.axis);

    expect(thangA).toContain("mobility");
    expect(thangA).toContain("weight"); // nhẹ hơn là tốt hơn
    expect(thangB).toContain("coverage");
    expect(thangB).toContain("durability");
  });
});

describe("hướng của từng chiều", () => {
  it("không phải chiều nào cũng cao hơn là tốt hơn", () => {
    expect(HIGHER_IS_BETTER.coverage).toBe(true);
    expect(HIGHER_IS_BETTER.weight).toBe(false);
    expect(HIGHER_IS_BETTER.noise).toBe(false);
  });

  it("nhẹ hơn thắng ở chiều khối lượng", () => {
    const c = compare(giapNhe(), giapNang());
    expect(c.axes.find((x) => x.axis === "weight")?.winner).toBe("a");
  });

  it("mọi chiều đều khai hướng", () => {
    for (const a of AXES) {
      expect(HIGHER_IS_BETTER[a]).toBeTypeOf("boolean");
    }
  });
});

describe("cái gì mất đi nếu đổi", () => {
  it("bỏ mất khả năng bơi — thứ mà một bảng số không nói", () => {
    const mat = losses(giapNhe(), giapNang());
    expect(mat).toContain("mất khả năng: swim");
    expect(mat).toContain("mất khả năng: sneak");
  });

  it("và đòi thêm điều kiện mới", () => {
    expect(losses(giapNhe(), giapNang())).toContain(
      "đòi thêm điều kiện: skill.armour_training",
    );
  });

  it("cũng nói cái được thêm", () => {
    expect(gains(giapNhe(), giapNang())).toContain("thêm hiệu ứng: effect.warmth");
  });

  it("đổi ngược lại thì mất và được đảo chiều", () => {
    const xuoi = losses(giapNhe(), giapNang());
    const nguoc = gains(giapNang(), giapNhe());
    expect(nguoc).toContain("thêm khả năng: swim");
    expect(xuoi.length).toBeGreaterThan(0);
  });
});

describe("nhận ra đánh đổi thật", () => {
  it("giáp nhẹ và giáp nặng là một đánh đổi", () => {
    expect(isTradeoff(compare(giapNhe(), giapNang()))).toBe(true);
  });

  it("một bên hơn hẳn ở mọi chiều thì không phải đánh đổi", () => {
    const te: Item = {
      ...giapNhe(),
      id: "item.rags",
      axes: { weight: 200, coverage: 100, mobility: 800, durability: 100, noise: 200 },
    };
    const tot: Item = {
      ...giapNhe(),
      id: "item.good_leather",
      axes: { weight: 200, coverage: 400, mobility: 800, durability: 600, noise: 200 },
    };
    const c = compare(te, tot);
    expect(c.lostBySwitching).toEqual([]);
    expect(isTradeoff(c)).toBe(false);
  });

  it("hơn ở mọi chiều nhưng mất một khả năng thì VẪN là đánh đổi", () => {
    const a: Item = { ...giapNhe(), affordances: ["swim"] };
    const b: Item = {
      ...giapNhe(),
      id: "item.b",
      axes: { weight: 100, coverage: 500, mobility: 900, durability: 900, noise: 100 },
      affordances: [],
    };
    const c = compare(a, b);
    expect(c.axes.every((x) => x.winner !== "a")).toBe(true);
    expect(isTradeoff(c)).toBe(true);
  });
});

describe("chiều không so được", () => {
  it("một bên thiếu số liệu thì không tuyên bố ai thắng", () => {
    const thieu: Item = { ...giapNhe(), axes: { weight: 200 } };
    const c = compare(thieu, giapNang());
    const cov = c.axes.find((x) => x.axis === "coverage");
    expect(cov?.a).toBeNull();
    expect(cov?.winner).toBeNull();
  });

  it("chiều không bên nào có thì không xuất hiện", () => {
    const c = compare(giapNhe(), giapNang());
    expect(c.axes.find((x) => x.axis === "warmth")).toBeUndefined();
  });
});

describe("bộ phận yếu nhất quyết định", () => {
  it("một cây rìu cán gãy thì không dùng được dù lưỡi hoàn hảo", () => {
    expect(worstPart(giapNang())?.part).toBe("arms");
    expect(worstPart(giapNang())?.condition).toBe(200);
  });

  it("không có bộ phận nào thì trả undefined", () => {
    expect(worstPart({ ...giapNhe(), parts: [] })).toBeUndefined();
  });
});
