/**
 * Bài kiểm cho `strings.ts`.
 *
 * Phần kiểm bằng kiểu (`MenuCatalog = Record<MenuMessageKey, string>`) đã bắt
 * lỗi thiếu khóa lúc biên dịch — giống hệt cách `i18n/index.test.ts` giải
 * thích lý do không cần kiểm lại `t()` bằng runtime. Nhưng đề bài của lát cắt
 * này còn đòi một lưới an toàn thứ hai chạy được ở `vitest`, không chỉ ở
 * `tsc`, nên bài kiểm dưới đây đối chiếu trực tiếp tập khóa của hai catalog —
 * và kiểm `tm()` đổi đúng theo ngôn ngữ hiện tại của `@/i18n`.
 */
import { afterEach, describe, expect, it } from "vitest";
import { locale, setLocale } from "@/i18n";
import { MENU_CATALOGS, tm } from "./strings";

describe("mọi khóa `vi` đều có trong `en`, và ngược lại", () => {
  it("hai catalog có đúng cùng một tập khóa", () => {
    const viKeys = Object.keys(MENU_CATALOGS.vi).sort();
    const enKeys = Object.keys(MENU_CATALOGS.en).sort();
    expect(enKeys).toEqual(viKeys);
  });

  it("không có khóa nào tra ra chuỗi rỗng", () => {
    // Một khóa map sang chuỗi rỗng là dấu hiệu quên viết bản dịch, không
    // phải một bản dịch hợp lệ dài 0 ký tự.
    for (const cat of [MENU_CATALOGS.vi, MENU_CATALOGS.en]) {
      for (const [key, value] of Object.entries(cat)) {
        expect(value.length, `khóa "${key}" rỗng`).toBeGreaterThan(0);
      }
    }
  });
});

describe("tm — tra chữ theo ngôn ngữ hiện tại của @/i18n", () => {
  const original = locale();
  afterEach(() => setLocale(original));

  it("mặc định vi thì tra ra bản tiếng Việt", () => {
    setLocale("vi");
    expect(tm("menu.resume")).toBe("Tiếp tục");
    expect(tm("title.play")).toBe("Bước vào thế giới");
  });

  it("đổi sang en ở @/i18n thì tm() cũng đổi theo, không cần state riêng", () => {
    setLocale("en");
    expect(tm("menu.resume")).toBe("Resume");
    expect(tm("title.play")).toBe("Step into the world");
    setLocale("vi");
    expect(tm("menu.resume")).toBe("Tiếp tục");
  });
});
