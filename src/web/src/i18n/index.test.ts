/**
 * Bài kiểm cho tra chữ lúc chạy.
 *
 * Phần kiểm bằng kiểu (`t`) không cần test — `tsc` đã là bài kiểm đó, và viết
 * lại nó bằng runtime chỉ nhân đôi công việc. Thứ đáng kiểm là nhánh **không**
 * kiểm được bằng kiểu: khóa tới từ engine.
 */
import { describe, expect, it } from "vitest";
import { setLocale, t, tRuntime } from "./index";

describe("tRuntime", () => {
  it("tra được khóa engine gửi lên", () => {
    setLocale("vi");
    expect(tRuntime("intent", "goto.field")).toBe("đang ra đồng");
    expect(tRuntime("role", "smith")).toBe("thợ rèn");
  });

  it("khóa lạ hiện ra chính nó chứ không biến mất", () => {
    // Một content pack thêm vai mới là chuyện bình thường; giao diện phải nói
    // được "chưa dịch" thay vì để một ô trống.
    expect(tRuntime("role", "alchemist")).toBe("alchemist");
  });

  it("không có ý định thì hiện dấu gạch, không phải chữ `undefined`", () => {
    expect(tRuntime("intent", null)).toBe("—");
    expect(tRuntime("intent", undefined)).toBe("—");
    expect(tRuntime("intent", "")).toBe("—");
  });

  it("đổi ngôn ngữ thì đổi cả chữ tra lúc chạy", () => {
    setLocale("en");
    expect(tRuntime("intent", "goto.field")).toBe("heading out to the fields");
    expect(t("panel.who.role")).toBe("role");
    setLocale("vi");
  });
});
