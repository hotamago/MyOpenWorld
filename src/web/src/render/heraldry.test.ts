import { describe, expect, it } from "vitest";
import {
  blazon,
  cadetArms,
  CADENCY,
  classOf,
  contrasts,
  generateArms,
  generationsFromMain,
  sameLineage,
  violatesTincture,
  type Arms,
} from "./heraldry";

describe("luật màu", () => {
  it("kim loại trên màu và màu trên kim loại thì được", () => {
    expect(contrasts("or", "gules")).toBe(true);
    expect(contrasts("azure", "argent")).toBe(true);
  });

  it("cùng nhóm thì không — đó chính là ràng buộc tương phản", () => {
    expect(contrasts("or", "argent")).toBe(false);
    expect(contrasts("gules", "azure")).toBe(false);
  });

  it("phân nhóm đúng", () => {
    expect(classOf("or")).toBe("metal");
    expect(classOf("sable")).toBe("colour");
  });
});

describe("sinh huy hiệu", () => {
  it("không bao giờ vi phạm luật màu — với hàng nghìn seed", () => {
    for (let i = 0n; i < 2000n; i++) {
      const a = generateArms(i * 7919n + 13n);
      expect(violatesTincture(a), `seed ${i}`).toEqual([]);
    }
  });

  it("xác định: cùng seed cho cùng lá cờ", () => {
    const a = generateArms(0xdead_beefn);
    const b = generateArms(0xdead_beefn);
    expect(a).toEqual(b);
  });

  it("seed khác thì lá cờ thường khác", () => {
    const thay = new Set<string>();
    for (let i = 0n; i < 200n; i++) thay.add(blazon(generateArms(i)));
    expect(thay.size).toBeGreaterThan(50);
  });

  it("dùng BigInt nên hai seed cách nhau 1 ở trên 2^53 vẫn khác nhau", () => {
    const a = generateArms(2n ** 53n + 1n);
    const b = generateArms(2n ** 53n + 2n);
    expect(blazon(a)).not.toBe(blazon(b));
  });

  it("trường không chia thì chỉ có một màu", () => {
    let thay = false;
    for (let i = 0n; i < 200n; i++) {
      const a = generateArms(i);
      if (a.division === "plain") {
        expect(a.field).toHaveLength(1);
        thay = true;
      } else {
        expect(a.field).toHaveLength(2);
      }
    }
    expect(thay, "phải có ít nhất một lá cờ không chia trường").toBe(true);
  });
});

describe("nhánh thứ mã hóa huyết thống", () => {
  it("thừa kế nguyên vẹn, cộng đúng một dấu", () => {
    const chinh = generateArms(42n);
    const thu = cadetArms(chinh, 1);

    expect(sameLineage(chinh, thu)).toBe(true);
    expect(thu.cadency).toEqual([CADENCY[1]]);
    expect(thu.charge).toBe(chinh.charge);
    expect(thu.field).toEqual(chinh.field);
  });

  it("nhìn hai lá cờ là biết bên nào là nhánh thứ", () => {
    const chinh = generateArms(42n);
    const thu = cadetArms(chinh, 2);
    expect(generationsFromMain(chinh)).toBe(0);
    expect(generationsFromMain(thu)).toBe(1);
  });

  it("nhánh thứ của nhánh thứ mang hai dấu", () => {
    const chinh = generateArms(42n);
    const doi2 = cadetArms(cadetArms(chinh, 1), 3);
    expect(doi2.cadency).toHaveLength(2);
    expect(generationsFromMain(doi2)).toBe(2);
    expect(sameLineage(chinh, doi2)).toBe(true);
  });

  it("hai dòng họ khác nhau không bị nhận nhầm là cùng máu", () => {
    const a = generateArms(1n);
    let b = generateArms(2n);
    // Tìm một seed thật sự khác.
    for (let i = 2n; sameLineage(a, b) && i < 100n; i++) b = generateArms(i);
    expect(sameLineage(a, b)).toBe(false);
  });

  it("thêm dấu không phá luật màu", () => {
    for (let i = 0n; i < 200n; i++) {
      const thu = cadetArms(generateArms(i), Number(i % 8n));
      expect(violatesTincture(thu)).toEqual([]);
    }
  });
});

describe("bộ kiểm luật màu", () => {
  it("bắt được hai nửa trường cùng nhóm", () => {
    const xau: Arms = {
      division: "per_pale",
      field: ["or", "argent"],
      charge: "lion",
      chargeTincture: "gules",
      cadency: [],
    };
    expect(violatesTincture(xau).length).toBeGreaterThan(0);
  });

  it("bắt được hình không tương phản với trường", () => {
    const xau: Arms = {
      division: "plain",
      field: ["gules"],
      charge: "lion",
      chargeTincture: "azure",
      cadency: [],
    };
    expect(violatesTincture(xau)[0]).toContain("không tương phản");
  });
});

describe("mô tả bằng lời", () => {
  it("một lá cờ phải nói được thành lời", () => {
    const a = generateArms(7n);
    const s = blazon(a);
    expect(s).toContain(a.charge);
    expect(s.length).toBeGreaterThan(5);
  });

  it("nhánh thứ nói rõ dấu khác biệt", () => {
    const thu = cadetArms(generateArms(7n), 1);
    expect(blazon(thu)).toContain("khác biệt bởi");
  });
});
