import { describe, expect, it } from "vitest";
import {
  darkIsReversedLight,
  distinctInGreyscale,
  labelObeysRule,
  overlayTable,
  patternFor,
  PATTERNS,
  SCALES,
  scaleIsMonotonic,
  stepsDistinct,
  tooManyIdentities,
  validateBothSchemes,
  type Scheme,
} from "./accessibility";
import { validatePalette, type Palette } from "./palette/validate";

describe("tiếp cận được (PF-17, §18.6.3, §18.6.4)", () => {
  // ───────────── hoa văn thay màu ─────────────

  it("mỗi định danh có một hoa văn, ổn định qua mọi lần gọi", () => {
    for (let i = 0; i < PATTERNS.length; i++) {
      expect(patternFor(i)).toBe(patternFor(i));
    }
  });

  it("chín định danh đầu có chín hoa văn khác nhau", () => {
    const tap = new Set(PATTERNS.map((_, i) => patternFor(i)));
    expect(tap.size).toBe(PATTERNS.length);
  });

  it("hoa văn gỡ trần ba định danh của màu lên chín, không lên vô hạn", () => {
    // §18.6.2: màu chỉ chở được ba định danh.
    expect(PATTERNS.length).toBeGreaterThan(3);
    expect(tooManyIdentities(PATTERNS.length)).toBe(false);
    expect(tooManyIdentities(PATTERNS.length + 1)).toBe(true);
  });

  it("chỉ số âm vẫn cho một hoa văn hợp lệ, không cho undefined", () => {
    expect(PATTERNS).toContain(patternFor(-1));
    expect(PATTERNS).toContain(patternFor(-100));
  });

  it("hai hoa văn khác nhau phân biệt được khi in đen trắng", () => {
    expect(distinctInGreyscale("solid", "grid")).toBe(true);
    expect(distinctInGreyscale("solid", "solid")).toBe(false);
  });

  // ───────────── bảng số cho mọi overlay ─────────────

  it("mọi overlay có bảng số kèm ĐƠN VỊ THẬT, không phải thấp → cao", () => {
    const b = overlayTable(
      "mana_density",
      [
        { label: "Thung lũng Veskar", value: 3400, swatch: "#2171b5" },
        { label: "Rừng Tolm", value: 900, swatch: "#6baed6" },
      ],
      "mMU",
      false,
    );
    expect(b.rows).toHaveLength(2);
    expect(b.rows.every((r) => r.unit === "mMU")).toBe(true);
    expect(b.rows.map((r) => r.value)).toEqual([3400, 900]);
  });

  it("mỗi dòng bảng có cả ô màu lẫn hoa văn", () => {
    const b = overlayTable("x", [{ label: "a", value: 1, swatch: "#fff" }], "°C", false);
    expect(b.rows[0]!.swatch).toBe("#fff");
    expect(PATTERNS).toContain(b.rows[0]!.pattern);
  });

  it("bảng nói rõ con số là đo hay là ước lượng theo mô hình vùng", () => {
    const do_that = overlayTable("x", [], "người/ô", false);
    const uoc_luong = overlayTable("x", [], "người/ô", true);
    expect(do_that.estimated).toBe(false);
    expect(uoc_luong.estimated).toBe(true);
  });

  it("overlay rỗng vẫn ra một bảng, không ra undefined", () => {
    expect(overlayTable("x", [], "mMU", false).rows).toEqual([]);
  });

  // ───────────── chế độ tối là thang riêng ─────────────

  it("thang tối KHÔNG phải thang sáng đảo ngược", () => {
    expect(darkIsReversedLight()).toBe(false);
  });

  it("hai thang có cùng số bậc nhưng đi hai đường khác nhau", () => {
    expect(SCALES.dark).toHaveLength(SCALES.light.length);
    expect([...SCALES.dark]).not.toEqual([...SCALES.light]);
    expect([...SCALES.dark]).not.toEqual([...SCALES.light].reverse());
  });

  it("cả hai thang đều đơn điệu về độ sáng", () => {
    for (const s of ["light", "dark"] as Scheme[]) {
      expect(scaleIsMonotonic(s)).toBe(true);
    }
  });

  it("cả hai thang đều qua CÙNG bộ kiểm bước độ sáng", () => {
    for (const s of ["light", "dark"] as Scheme[]) {
      expect(stepsDistinct(s)).toBe(true);
    }
  });

  it("cả hai chế độ đi qua cùng một hàm kiểm bảng màu", () => {
    const dung = (mode: Scheme, background: string, colours: string[]): Palette => ({
      id: `seq.${mode}`,
      kind: "sequential",
      mode,
      background,
      noData: "#808080",
      entries: colours.map((c, i) => ({ id: `b${i}`, color: c })),
    });
    const ra = validateBothSchemes(
      {
        light: dung("light", "#ffffff", [...SCALES.light]),
        dark: dung("dark", "#111111", [...SCALES.dark]),
      },
      validatePalette,
    );
    expect(ra.light).toEqual([]);
    expect(ra.dark).toEqual([]);
  });

  it("bộ kiểm có răng: một thang tối chọn ẩu thì bị bắt", () => {
    // Đây là vế "cả hai đều phải qua cùng bộ kiểm tra" của `§18.6.4`. Thang
    // dưới đây dồn ba bậc vào cùng một vùng độ sáng — đúng cách hỏng mà một
    // người chọn màu bằng mắt trên nền đen dễ mắc.
    const don: Palette = {
      id: "seq.dark.bunched",
      kind: "sequential",
      mode: "dark",
      background: "#111111",
      noData: "#808080",
      entries: ["#08306b", "#0a3570", "#0c3a75", "#c6dbef", "#f7fbff"].map(
        (c, i) => ({ id: `b${i}`, color: c }),
      ),
    };
    expect(validatePalette(don).length).toBeGreaterThan(0);
    // Còn thang tối thật thì qua.
    expect(stepsDistinct("dark")).toBe(true);
  });

  it("kiểm đảo ngược bắt CÁCH LÀM, không bắt kết quả xấu", () => {
    // Nói rõ giới hạn của `darkIsReversedLight`, để không ai tưởng nó là một
    // bảo đảm về chất lượng. Thang sáng đảo ngược trong dự án này thật ra vẫn
    // qua được bộ kiểm — nó bị cấm vì quy trình, không vì nó hỏng.
    const dao: Palette = {
      id: "seq.dark.lazy",
      kind: "sequential",
      mode: "dark",
      background: "#111111",
      noData: "#808080",
      entries: [...SCALES.light]
        .reverse()
        .map((c, i) => ({ id: `b${i}`, color: c })),
    };
    expect(validatePalette(dao)).toEqual([]);
    // Nhưng nó vẫn là một phép đảo, và `§18.6.4` cấm đúng điều đó.
    expect(darkIsReversedLight()).toBe(false);
  });

  // ───────────── nhãn không phụ thuộc màu ─────────────

  it("chữ dùng màu chữ của giao diện, ô màu nhỏ mới chở danh tính", () => {
    const mau_chu = ["#1a1a1a", "#f0f0f0"];
    expect(
      labelObeysRule({ textColour: "#1a1a1a", swatch: "#d7301f" }, mau_chu),
    ).toBe(true);
  });

  it("tô chữ bằng màu dữ liệu là vi phạm", () => {
    const mau_chu = ["#1a1a1a"];
    expect(
      labelObeysRule({ textColour: "#d7301f", swatch: "#d7301f" }, mau_chu),
    ).toBe(false);
  });

  it("ô màu trùng màu chữ cũng vi phạm — không còn phân biệt được kênh nào chở gì", () => {
    const mau_chu = ["#1a1a1a"];
    expect(
      labelObeysRule({ textColour: "#1a1a1a", swatch: "#1a1a1a" }, mau_chu),
    ).toBe(false);
  });
});
