import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  contrastRatio,
  deltaE,
  minDeltaEAcrossVision,
  parseHex,
  simulateCvd,
} from "./color";
import {
  formatReport,
  THRESHOLDS,
  validateAll,
  validatePalette,
  type Palette,
} from "./validate";

const DIR = join(import.meta.dirname, "../../../../content/core/palettes");

function tatCaBangMau(): Palette[] {
  return readdirSync(DIR)
    .filter((f: string) => f.endsWith(".json"))
    .sort()
    .map((f: string) => JSON.parse(readFileSync(join(DIR, f), "utf-8")) as Palette);
}

describe("bảng màu thật của dự án", () => {
  it("không vi phạm luật nào", () => {
    const v = validateAll(tatCaBangMau());
    expect(formatReport(v)).toBe("bảng màu: không vi phạm");
  });

  it("có đủ cả bốn loại bảng", () => {
    const loai = new Set(tatCaBangMau().map((p) => p.kind));
    expect(loai).toEqual(
      new Set(["identity_critical", "environment", "sequential", "diverging"]),
    );
  });

  it("mọi bảng đều có cả chế độ sáng và tối", () => {
    const p = tatCaBangMau();
    const ids = new Set(p.map((x) => x.id));
    for (const id of ids) {
      const modes = p.filter((x) => x.id === id).map((x) => x.mode).sort();
      expect(modes).toEqual(["dark", "light"]);
    }
  });

  it("bảng biome phủ đủ 16 quần xã sinh vật của engine", () => {
    // Nếu `mow-worldgen::Biome::ALL` thêm một biến thể mà bảng màu không theo,
    // bản đồ sẽ vẽ nó bằng màu mặc định và không ai nhận ra cho tới khi thấy
    // một mảng xám lạ giữa bản đồ.
    const b = tatCaBangMau().filter((p) => p.id === "biome");
    for (const p of b) expect(p.entries).toHaveLength(16);
  });
});

describe("PA-13 — hai luật cho hai loại bảng", () => {
  const nen = "#f5f2ea";

  it("luật là NGƯỠNG, không phải số đếm — bốn màu tách đủ thì hợp lệ", () => {
    // Kết luận cũ chốt "quá 3 màu là fail". Phép đo ở bài dưới cho thấy con số
    // 3 là một biến thay thế cho ngưỡng, và biến thay thế sai ở cả hai phía.
    // Đây là phía thứ nhất: một bộ bốn màu tốt bị cấm oan.
    const p: Palette = {
      id: "test",
      kind: "identity_critical",
      mode: "light",
      background: nen,
      entries: [
        { id: "a", color: "#0072b2" },
        { id: "b", color: "#d55e00" },
        { id: "c", color: "#009e73" },
        { id: "d", color: "#000000" },
      ],
    };
    expect(validatePalette(p)).toHaveLength(0);
  });

  it("ba màu quá giống nhau thì vẫn fail", () => {
    // Và đây là phía thứ hai: một bộ ba màu tồi được cho qua bởi luật đếm.
    const p: Palette = {
      id: "test",
      kind: "identity_critical",
      mode: "light",
      background: nen,
      entries: [
        { id: "a", color: "#1a4fa0" },
        { id: "b", color: "#eda100" },
        { id: "c", color: "#eb6834" },
      ],
    };
    expect(
      validatePalette(p).some((x) => x.rule === "identity-all-pairs"),
    ).toBe(true);
  });

  it("phép đo nền tảng: ràng buộc nằm ở mù màu, không ở thị giác thường", () => {
    // Cặp cam này cách nhau rõ ràng với hầu hết mọi người và gần như trùng nhau
    // với người protanopia. Một bộ kiểm chỉ nhìn thị giác thường sẽ cho nó qua.
    const thuong = deltaE(parseHex("#eda100"), parseHex("#eb6834"));
    const qua_mu_mau = minDeltaEAcrossVision(
      parseHex("#eda100"),
      parseHex("#eb6834"),
    );

    expect(thuong).toBeGreaterThan(THRESHOLDS.identityAllPairs);
    expect(qua_mu_mau.value).toBeLessThan(THRESHOLDS.identityAllPairs);
    expect(qua_mu_mau.vision).toBe("deuteranopia");
  });

  it("bảng an toàn kinh điển giữ được ngưỡng tới tám màu", () => {
    // Okabe–Ito. Đây là dữ liệu đã đặt ngưỡng ở 10: nó nằm giữa cặp dễ nhầm
    // (9.6) và bảng này (10.9). Khoảng đó hẹp, và bài test này giữ nó — nếu ai
    // đó nâng ngưỡng, bài sẽ đỏ và buộc họ đối diện với việc họ vừa loại thứ
    // tốt nhất mà ngành từng tìm ra.
    const okabe = [
      "#e69f00",
      "#56b4e9",
      "#009e73",
      "#f0e442",
      "#0072b2",
      "#d55e00",
      "#cc79a7",
      "#000000",
    ];
    let min = Infinity;
    for (let i = 0; i < okabe.length; i++) {
      for (let j = i + 1; j < okabe.length; j++) {
        min = Math.min(
          min,
          minDeltaEAcrossVision(parseHex(okabe[i]!), parseHex(okabe[j]!)).value,
        );
      }
    }
    expect(min).toBeGreaterThanOrEqual(THRESHOLDS.identityAllPairs);
    expect(min).toBeLessThan(THRESHOLDS.identityAllPairs + 2);
  });

  it("bảng môi trường 16 màu thì HỢP LỆ — luật khác nhau vì hậu quả khác nhau", () => {
    // Đây là bản sửa của lần review trước: luật ≤ 3 màu áp cho mọi thứ sẽ làm
    // vỡ bảng biome, và thế giới không thể chỉ có ba quần xã sinh vật.
    const bang = tatCaBangMau().find((p) => p.id === "biome" && p.mode === "light")!;
    expect(bang.entries.length).toBeGreaterThan(THRESHOLDS.identityMaxColors);
    expect(validatePalette(bang)).toHaveLength(0);
  });

  it("bảng môi trường thiếu hoa văn thì fail", () => {
    // Hoa văn là ĐIỀU KIỆN để nới ngưỡng ΔE. Thiếu nó thì màu là tín hiệu duy
    // nhất, và ngưỡng lỏng thành một lời hứa suông với người mù màu.
    const p: Palette = {
      id: "test",
      kind: "environment",
      mode: "light",
      background: nen,
      entries: [
        { id: "a", color: "#12395e" },
        { id: "b", color: "#e3d5a8" },
      ],
    };
    expect(validatePalette(p).some((x) => x.rule === "environment-pattern-required")).toBe(
      true,
    );
  });

  it("hai biome kề nhau quá giống thì fail", () => {
    const p: Palette = {
      id: "test",
      kind: "environment",
      mode: "light",
      background: nen,
      entries: [
        { id: "a", color: "#4f8442", pattern: "x" },
        { id: "b", color: "#528745", pattern: "y" },
      ],
    };
    expect(validatePalette(p).some((x) => x.rule === "environment-adjacent")).toBe(true);
  });
});

describe("§18.6.3 — mù màu", () => {
  it("bảng định danh phân biệt được qua cả ba dạng mù màu", () => {
    const bang = tatCaBangMau().filter((p) => p.kind === "identity_critical");
    for (const p of bang) {
      for (const kind of ["protanopia", "deuteranopia", "tritanopia"] as const) {
        for (let i = 0; i < p.entries.length; i++) {
          for (let j = i + 1; j < p.entries.length; j++) {
            const a = simulateCvd(parseHex(p.entries[i]!.color), kind);
            const b = simulateCvd(parseHex(p.entries[j]!.color), kind);
            expect(deltaE(a, b)).toBeGreaterThanOrEqual(THRESHOLDS.identityAllPairs);
          }
        }
      }
    }
  });

  it("mô phỏng ở mức nặng nhất, không phải trung bình", () => {
    // Kiểm ở mức trung bình sẽ để lọt đúng những người cần nó nhất.
    const do_ = parseHex("#d02020");
    const xanh = parseHex("#20a020");
    // Với protanopia nặng, đỏ và xanh lá phải sập lại gần nhau.
    const d_thuong = deltaE(do_, xanh);
    const d_mu = deltaE(simulateCvd(do_, "protanopia"), simulateCvd(xanh, "protanopia"));
    expect(d_mu).toBeLessThan(d_thuong / 2);
  });
});

describe("thang tuần tự và phân kỳ", () => {
  it("thang tuần tự có độ sáng đơn điệu", () => {
    // Đây là thứ khiến thang đọc được **mà không cần legend**, và khiến nó còn
    // đọc được khi in đen trắng.
    const p: Palette = {
      id: "test",
      kind: "sequential",
      mode: "light",
      background: "#f5f2ea",
      entries: [
        { id: "a", color: "#1d3a55" },
        { id: "b", color: "#e2efdc" },
        { id: "c", color: "#4b8a93" },
      ],
    };
    expect(
      validatePalette(p).some((x) => x.rule === "sequential-monotonic-lightness"),
    ).toBe(true);
  });

  it("thang phân kỳ số bậc chẵn thì fail", () => {
    const p: Palette = {
      id: "test",
      kind: "diverging",
      mode: "light",
      background: "#f5f2ea",
      entries: [
        { id: "a", color: "#1f4d8f" },
        { id: "b", color: "#e8e4d8" },
        { id: "c", color: "#d99257" },
        { id: "d", color: "#8f4213" },
      ],
    };
    expect(validatePalette(p).some((x) => x.rule === "diverging-odd")).toBe(true);
  });
});

describe("toán màu", () => {
  it("hex đi và về nguyên vẹn", () => {
    expect(parseHex("#1a4fa0")).toEqual({ r: 0x1a, g: 0x4f, b: 0xa0 });
    expect(() => parseHex("#zzz")).toThrow();
  });

  it("tương phản trắng–đen là 21:1", () => {
    expect(contrastRatio(parseHex("#ffffff"), parseHex("#000000"))).toBeCloseTo(21, 1);
  });

  it("ΔE của một màu với chính nó là 0", () => {
    expect(deltaE(parseHex("#4f8442"), parseHex("#4f8442"))).toBe(0);
  });
});
