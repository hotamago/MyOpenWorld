/**
 * Test cho chữ hiển thị riêng của bảng quyền năng (`strings.ts`).
 *
 * Ba việc: `vi`/`en` phủ đúng cùng một tập khóa (lưới an toàn thứ hai cạnh
 * kiểm bằng kiểu ở `PowerCatalog`), `tp()` đổi theo `locale()` dùng chung với
 * `@/i18n` (đúng lý do module này không tự giữ biến ngôn ngữ riêng — xem chú
 * thích đầu `strings.ts`), và `tpRaw()` có đường lui thấy được cho khóa lạ.
 */

import { afterEach, describe, expect, it } from "vitest";
import { locale, setLocale, type Locale } from "@/i18n";
import { POWER_CATALOGS, tp, tpRaw, type PowerMessageKey } from "./strings";

describe("POWER_CATALOGS — vi/en phủ đúng cùng một tập khóa", () => {
  it("en có đúng và đủ tập khóa của vi", () => {
    const viKeys = Object.keys(POWER_CATALOGS.vi).sort();
    const enKeys = Object.keys(POWER_CATALOGS.en).sort();
    expect(enKeys).toEqual(viKeys);
  });

  it("không khóa nào rỗng ở cả hai ngôn ngữ", () => {
    for (const cat of [POWER_CATALOGS.vi, POWER_CATALOGS.en]) {
      for (const [key, value] of Object.entries(cat)) {
        expect(value.length, `khóa rỗng: ${key}`).toBeGreaterThan(0);
      }
    }
  });
});

describe("tp — dùng chung locale() với @/i18n, không giữ trạng thái ngôn ngữ riêng", () => {
  const original: Locale = locale();
  afterEach(() => setLocale(original));

  it("đổi ngôn ngữ qua setLocale() của @/i18n thì tp() đổi theo ngay", () => {
    setLocale("vi");
    expect(tp("dock.title")).toBe("Quyền năng");
    setLocale("en");
    expect(tp("dock.title")).toBe("Powers");
  });

  it("mọi khóa của vi đều tra được qua tp() ở cả hai ngôn ngữ, không rỗng", () => {
    const keys = Object.keys(POWER_CATALOGS.vi) as PowerMessageKey[];
    for (const l of ["vi", "en"] as const) {
      setLocale(l);
      for (const key of keys) {
        expect(tp(key).length, `${l}["${key}"]`).toBeGreaterThan(0);
      }
    }
  });
});

describe("tpRaw — tra khóa ghép từ Power.id lúc chạy", () => {
  const original: Locale = locale();
  afterEach(() => setLocale(original));

  it("khóa tồn tại thì trả đúng chữ đã dịch, theo đúng ngôn ngữ hiện tại", () => {
    setLocale("vi");
    expect(tpRaw("power.body.feed.label")).toBe("Ban no đủ");
    setLocale("en");
    expect(tpRaw("power.body.feed.label")).toBe("Grant Fullness");
  });

  it("khóa lạ rơi về chính khóa — không chuỗi rỗng, không throw", () => {
    expect(tpRaw("power.nonexistent.label")).toBe("power.nonexistent.label");
  });
});
