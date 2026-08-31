/**
 * Bài kiểm cho `strings.ts`.
 *
 * Phần kiểm bằng kiểu (`ChronicleCatalog = Record<ChronicleMessageKey, string>`)
 * đã bắt lỗi thiếu khóa lúc biên dịch, nhưng lát cắt này còn đòi một lưới an
 * toàn thứ hai chạy được ở `vitest` — đối chiếu trực tiếp tập khóa của hai
 * catalog — cộng bài kiểm cho phần `tc()` không kiểm được bằng kiểu: khóa lạ,
 * slot thiếu, và đồng bộ theo `locale()` của `@/i18n`.
 */
import { afterEach, describe, expect, it } from "vitest";
import { locale, setLocale } from "@/i18n";
import { CHRONICLE_CATALOGS, t, tc } from "./strings";

describe("mọi khóa `vi` đều có trong `en`, và ngược lại", () => {
  it("hai catalog có đúng cùng một tập khóa", () => {
    const viKeys = Object.keys(CHRONICLE_CATALOGS.vi).sort();
    const enKeys = Object.keys(CHRONICLE_CATALOGS.en).sort();
    expect(enKeys).toEqual(viKeys);
  });

  it("không có khóa nào tra ra chuỗi rỗng", () => {
    // Một khóa map sang chuỗi rỗng là dấu hiệu quên viết bản dịch, không
    // phải một bản dịch hợp lệ dài 0 ký tự.
    for (const cat of [CHRONICLE_CATALOGS.vi, CHRONICLE_CATALOGS.en]) {
      for (const [key, value] of Object.entries(cat)) {
        expect(value.length, `khóa "${key}" rỗng`).toBeGreaterThan(0);
      }
    }
  });
});

describe("t/tc — tra chữ theo ngôn ngữ hiện tại của @/i18n", () => {
  const original = locale();
  afterEach(() => setLocale(original));

  it("mặc định vi thì tra ra bản tiếng Việt", () => {
    setLocale("vi");
    expect(t("chronicle.title")).toBe("Biên niên sử");
    expect(tc("chronicle.intervened", { key: "need.hunger" })).toBe(
      "Bàn tay thần chạm vào need.hunger",
    );
  });

  it("đổi sang en ở @/i18n thì cũng đổi theo, không cần state riêng", () => {
    setLocale("en");
    expect(t("chronicle.title")).toBe("The Chronicle");
    expect(tc("chronicle.intervened", { key: "need.hunger" })).toBe(
      "The hand of a god touched need.hunger",
    );
    setLocale("vi");
    expect(t("chronicle.title")).toBe("Biên niên sử");
  });
});

describe("tc — điền slot, và chịu được dữ liệu thiếu/khóa lạ", () => {
  it("điền nhiều slot cùng lúc, đúng khớp cả số lẫn chuỗi", () => {
    setLocale("vi");
    expect(tc("chronicle.journey", { who: "Linnea", count: 12 })).toBe(
      "Linnea rời bước, qua 12 chặng chân",
    );
  });

  it("khóa lạ (không có trong catalog) trả về chính khóa, không ném lỗi", () => {
    expect(tc("chronicle.not.a.real.key")).toBe("chronicle.not.a.real.key");
  });

  it("thiếu slot thì giữ nguyên `{tên_slot}`, không in ra chữ `undefined`", () => {
    setLocale("vi");
    const out = tc("chronicle.journey", {});
    expect(out).not.toMatch(/undefined/);
    expect(out).toContain("{who}");
    expect(out).toContain("{count}");
  });

  it("không truyền slot cho một khóa không có `{}` thì trả nguyên mẫu chữ", () => {
    setLocale("vi");
    expect(tc("chronicle.empty")).toBe("Sử xanh chưa chép một dòng nào");
  });
});
