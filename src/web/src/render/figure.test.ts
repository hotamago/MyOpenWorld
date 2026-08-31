import { describe, expect, it } from "vitest";
import type { Entity } from "@/api/game";
import { figureOf, type FigureSpec } from "./figure";

/**
 * Một cư dân tối thiểu, đủ trường cho `figureOf`. Test ghi đè từng trường cần
 * — giữ phần còn lại trung tính để mỗi test chỉ nói về đúng một điều nó kiểm.
 */
function entity(overrides: Partial<Entity> = {}): Entity {
  return {
    id: "e1",
    name: "Ai đó",
    x: 10,
    y: 10,
    kind: "being",
    is_avatar: false,
    hunger: null,
    role: null,
    intent: null,
    ...overrides,
  };
}

/** Năm vai cư dân mà `idea.md` đặt tên tường minh. */
const ROLES = ["elder", "smith", "hunter", "farmer", "child"] as const;

describe("figureOf — mỗi vai một hình riêng", () => {
  it("không hai vai nào trùng cả tool lẫn head cùng lúc", () => {
    // Đây là bất biến mà `§18.6` đòi: hai vai được phép chia sẻ MỘT kênh hình
    // (elder và hunter cùng đội hood), nhưng chia sẻ CẢ HAI thì cặp hình dạng
    // hết còn phân biệt được hai vai đó với nhau.
    const specs = ROLES.map((role) => [role, figureOf(entity({ role }))] as const);
    for (const [roleA, a] of specs) {
      for (const [roleB, b] of specs) {
        if (roleA === roleB) continue;
        const collide = a.tool === b.tool && a.head === b.head;
        expect(collide, `${roleA} và ${roleB} trùng cả tool lẫn head`).toBe(false);
      }
    }
  });

  it("elder cầm staff, đội hood", () => {
    const s = figureOf(entity({ role: "elder" }));
    expect(s.tool).toBe("staff");
    expect(s.head).toBe("hood");
  });

  it("smith cầm hammer, đầu trần", () => {
    const s = figureOf(entity({ role: "smith" }));
    expect(s.tool).toBe("hammer");
    expect(s.head).toBe("bare");
  });

  it("hunter cầm bow, đội hood", () => {
    const s = figureOf(entity({ role: "hunter" }));
    expect(s.tool).toBe("bow");
    expect(s.head).toBe("hood");
  });

  it("farmer cầm hoe, đội hat", () => {
    const s = figureOf(entity({ role: "farmer" }));
    expect(s.tool).toBe("hoe");
    expect(s.head).toBe("hat");
  });

  it("child tay không (none_child), đầu trần, và nhỏ hơn người lớn rõ rệt (~0.72×)", () => {
    const child = figureOf(entity({ role: "child" }));
    const adult = figureOf(entity({ role: "farmer" }));
    expect(child.tool).toBe("none_child");
    expect(child.head).toBe("bare");
    expect(child.scale).toBeLessThan(adult.scale);
    // Kiểm tỉ lệ, không kiểm một hằng số tuyệt đối — để test không vỡ nếu sau
    // này cỡ người lớn đổi một chút mà tỉ lệ trẻ con vẫn giữ nguyên.
    expect(child.scale / adult.scale).toBeCloseTo(0.72, 1);
  });

  it("vai lạ (một content pack chưa biết) không ném lỗi, và không lẫn với avatar", () => {
    expect(() => figureOf(entity({ role: "vai-tu-mod-la" }))).not.toThrow();
    const s = figureOf(entity({ role: "vai-tu-mod-la" }));
    expect(s.head).not.toBe("crown");
  });
});

describe("figureOf — avatar luôn khác biệt", () => {
  it("avatar đội crown — không vai cư dân nào khác có crown", () => {
    const avatar = figureOf(entity({ is_avatar: true }));
    expect(avatar.head).toBe("crown");
    for (const role of ROLES) {
      expect(figureOf(entity({ role })).head).not.toBe("crown");
    }
  });

  it("avatar khác cư dân thường kể cả khi mang chung một vai nền", () => {
    // Đây là trường hợp dễ vỡ nhất: một vị thần vẫn có thể đứng tên "farmer"
    // trong dữ liệu định cư. Avatar phải ghi đè, không hợp nhất một phần.
    for (const role of ROLES) {
      const god = figureOf(entity({ is_avatar: true, role }));
      const resident = figureOf(entity({ is_avatar: false, role }));
      const collide = god.tool === resident.tool && god.head === resident.head;
      expect(collide, `avatar mang vai ${role} trùng hết tool+head với cư dân thường`).toBe(false);
    }
  });
});

describe("figureOf — vật phẩm không bị ép thành người", () => {
  it("kind 'item' trả về shape riêng, không phải 'being'", () => {
    const s = figureOf(entity({ kind: "item", role: null }));
    expect(s.shape).toBe("item");
  });

  it("vật phẩm không mang ý định — mark luôn 'none'", () => {
    const s = figureOf(entity({ kind: "item", intent: "work" }));
    expect(s.mark).toBe("none");
  });
});

describe("figureOf — ý định suy ra dấu hình học", () => {
  const cases: Array<[string | null, FigureSpec["mark"]]> = [
    ["eat", "eat"],
    ["sleep", "sleep"],
    ["work", "work"],
    ["socialize", "talk"],
    ["goto.field", "walk"],
    ["goto.well", "walk"],
    ["goto.home", "walk"],
    ["idle", "none"],
    [null, "none"],
    ["", "none"],
    ["mot.khoa.tuong.lai", "none"],
    // Định dạng `Debug` cũ **không** còn được chấp nhận. Giữ nó ở đây như một
    // cái chốt: nếu ai đó lỡ tay trả `format!("{:?}")` về lại thì dấu trên đầu
    // biến mất, và bài này nói ra điều đó thay vì để nó lặng lẽ mất.
    ["GoTo { place: Field }", "none"],
  ];

  for (const [intent, expected] of cases) {
    it(`intent ${JSON.stringify(intent)} → mark "${expected}", và không ném lỗi`, () => {
      expect(() => figureOf(entity({ intent }))).not.toThrow();
      expect(figureOf(entity({ intent })).mark).toBe(expected);
    });
  }
});

describe("figureOf — hướng nhìn suy từ bước đi trước", () => {
  it("x giảm so với prev → left", () => {
    const s = figureOf(entity({ x: 5, y: 5 }), { x: 10, y: 5 });
    expect(s.facing).toBe("left");
  });

  it("x tăng so với prev → right", () => {
    const s = figureOf(entity({ x: 10, y: 5 }), { x: 5, y: 5 });
    expect(s.facing).toBe("right");
  });

  it("x đứng yên (chỉ đổi y) → right, không phải một lỗi", () => {
    const s = figureOf(entity({ x: 5, y: 9 }), { x: 5, y: 5 });
    expect(s.facing).toBe("right");
  });

  it("không có prev (lần đầu thấy thực thể) → right", () => {
    const s = figureOf(entity({ x: 5, y: 5 }));
    expect(s.facing).toBe("right");
  });
});
