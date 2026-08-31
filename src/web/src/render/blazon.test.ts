import { describe, expect, it } from "vitest";
import { parseBlazon, roundTrips } from "./blazon";
import {
  blazon,
  cadetArms,
  generateArms,
  sameLineage,
  violatesTincture,
} from "./heraldry";

describe("bộ giải văn phạm blazon (PF-18, §18.14.3)", () => {
  it("giải được trường một màu", () => {
    const r = parseBlazon("gules, một lion or");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.arms.division).toBe("plain");
    expect(r.arms.field).toEqual(["gules"]);
    expect(r.arms.charge).toBe("lion");
    expect(r.arms.chargeTincture).toBe("or");
    expect(r.arms.cadency).toEqual([]);
  });

  it("giải được trường chia đôi với counterchanged", () => {
    const r = parseBlazon("per pale or và azure, một eagle counterchanged");
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.arms.division).toBe("per_pale");
    expect(r.arms.field).toEqual(["or", "azure"]);
    expect(r.arms.chargeTincture).toBe("counterchanged");
  });

  it("giải được dấu nhánh thứ, giữ nguyên thứ tự", () => {
    const r = parseBlazon(
      "azure, một tower or, khác biệt bởi label rồi crescent",
    );
    expect(r.ok).toBe(true);
    if (!r.ok) return;
    expect(r.arms.cadency).toEqual(["label", "crescent"]);
  });

  // ───────────── từ chối, không đoán ─────────────

  it("từ chối chuỗi rỗng thay vì trả một lá cờ trống", () => {
    const r = parseBlazon("   ");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors[0]!.code).toBe("empty");
  });

  it("từ chối tincture không có thật thay vì đoán màu gần đúng", () => {
    const r = parseBlazon("crimson, một lion or");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors.some((e) => e.code === "unknown_division")).toBe(true);
  });

  it("từ chối hình không có trong bộ nguyên thủy", () => {
    const r = parseBlazon("gules, một dragon or");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors[0]!.code).toBe("unknown_charge");
    // Thông báo phải liệt ra cái hợp lệ, không chỉ nói "sai".
    expect(r.errors[0]!.detail).toContain("lion");
  });

  it("từ chối trường chia thiếu vế thứ hai", () => {
    const r = parseBlazon("per pale or, một lion azure");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors.some((e) => e.code === "field_count")).toBe(true);
  });

  it("từ chối khi không có phần hình", () => {
    const r = parseBlazon("gules");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors.some((e) => e.code === "missing_charge")).toBe(true);
  });

  it("từ chối dấu nhánh thứ không có thật", () => {
    const r = parseBlazon("gules, một lion or, khác biệt bởi bông_hoa_lạ");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors.some((e) => e.code === "unknown_cadency")).toBe(true);
  });

  // ───────────── luật màu ─────────────

  it("TỪ CHỐI lion đỏ trên nền đỏ — luật màu là chuẩn tương phản", () => {
    const r = parseBlazon("gules, một lion gules");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    const loi = r.errors.find((e) => e.code === "tincture_rule");
    expect(loi).toBeDefined();
    expect(loi!.detail).toContain("tương phản");
  });

  it("từ chối kim loại trên kim loại", () => {
    const r = parseBlazon("or, một star argent");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors.some((e) => e.code === "tincture_rule")).toBe(true);
  });

  it("counterchanged không bị luật màu chặn — đó là lý do nó tồn tại", () => {
    const r = parseBlazon("per fess argent và sable, một wheat counterchanged");
    expect(r.ok).toBe(true);
  });

  it("kiểm luật màu SAU cú pháp — không báo lạc hướng", () => {
    // Chuỗi vừa sai hình vừa sẽ sai luật màu nếu đọc bừa.
    const r = parseBlazon("gules, một dragon gules");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors.some((e) => e.code === "unknown_charge")).toBe(true);
    expect(r.errors.some((e) => e.code === "tincture_rule")).toBe(false);
  });

  it("báo mọi lỗi cùng lúc để người viết sửa một lần", () => {
    const r = parseBlazon("per pale crimson và mauve, một lion or");
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.errors.filter((e) => e.code === "unknown_tincture").length).toBe(2);
  });

  // ───────────── vòng khép kín ─────────────

  it("giải rồi viết lại ra đúng chuỗi ban đầu", () => {
    for (const s of [
      "gules, một lion or",
      "per pale or và azure, một eagle counterchanged",
      "azure, một tower or, khác biệt bởi label",
      "sable, một hammer argent, khác biệt bởi label rồi crescent rồi mullet",
    ]) {
      expect(roundTrips(s)).toBe(true);
    }
  });

  it("mọi huy hiệu sinh ra đều đi qua văn bản mà không mất gì", () => {
    for (let i = 0n; i < 200n; i++) {
      const a = generateArms(i * 7919n + 13n);
      const r = parseBlazon(blazon(a));
      expect(r.ok).toBe(true);
      if (!r.ok) continue;
      expect(r.arms).toEqual(a);
    }
  });

  it("huy hiệu nhánh thứ cũng khép vòng, kể cả sau nhiều đời", () => {
    let a = generateArms(4242n);
    for (let doi = 0; doi < 5; doi++) {
      a = cadetArms(a, doi);
      const r = parseBlazon(blazon(a));
      expect(r.ok).toBe(true);
      if (!r.ok) continue;
      expect(r.arms).toEqual(a);
      // Và huyết thống vẫn đọc được sau khi đi qua văn bản.
      expect(sameLineage(r.arms, a)).toBe(true);
    }
  });

  it("khoảng trắng thừa không đổi kết quả", () => {
    const a = parseBlazon("gules, một lion or");
    const b = parseBlazon("  gules,   một   lion   or  ");
    expect(a).toEqual(b);
  });

  // ───────────── nối với phần đã có ─────────────

  it("mọi lá cờ giải được đều không vi phạm luật màu", () => {
    for (const s of [
      "gules, một lion or",
      "per bend argent và vert, một fish counterchanged",
      "purpure, một oak argent",
    ]) {
      const r = parseBlazon(s);
      expect(r.ok).toBe(true);
      if (!r.ok) continue;
      expect(violatesTincture(r.arms)).toEqual([]);
    }
  });

  it("một lá cờ do content pack khai mà không khép vòng thì không phải cái nó nghĩ", () => {
    // Thiếu chữ "một": khép vòng thất bại, và pack phải sửa.
    expect(roundTrips("gules, lion or")).toBe(false);
  });
});
