import { describe, expect, it } from "vitest";
import {
  isActionable,
  isUniformProvenance,
  PROVENANCE_LABEL,
  provenanceBreaks,
  unlabelledMetrics,
  validateMetric,
  visibleRelations,
  whyBlocked,
  type EconPoint,
  type Metric,
  type Relation,
  type SocietyView,
} from "./model";

function demDuoc(): Metric {
  return {
    key: "population",
    label: "dân số",
    value: 9_240,
    provenance: "counted",
    uncertainty: null,
  };
}

function uocLuong(): Metric {
  return {
    key: "population",
    label: "dân số",
    value: 9_200,
    provenance: "modelled",
    uncertainty: 400,
  };
}

describe("thang zoom nói thật", () => {
  it("số đếm được và số suy ra không được trông giống nhau", () => {
    expect(demDuoc().provenance).not.toBe(uocLuong().provenance);
    expect(PROVENANCE_LABEL.modelled).toContain("ước lượng theo mô hình vùng");
  });

  it("phép đếm không có sai số", () => {
    const sai: Metric = { ...demDuoc(), uncertainty: 50 };
    expect(validateMetric(sai)[0]).toContain("không có sai số");
  });

  it("ước lượng KHÔNG nói sai số là đang giả vờ làm phép đếm", () => {
    const sai: Metric = { ...uocLuong(), uncertainty: null };
    expect(validateMetric(sai)[0]).toContain("giả vờ");
  });

  it("khai đúng thì không có lỗi nào", () => {
    expect(validateMetric(demDuoc())).toEqual([]);
    expect(validateMetric(uocLuong())).toEqual([]);
  });

  it("ước đoán của nhân vật là một nguồn riêng, không lẫn với sự thật", () => {
    const theo_loi: Metric = {
      ...uocLuong(),
      provenance: "believed",
      uncertainty: 900,
    };
    expect(validateMetric(theo_loi)).toEqual([]);
    expect(PROVENANCE_LABEL.believed).toContain("người trong vùng");
  });
});

describe("biểu đồ trộn nguồn", () => {
  const chuoi: EconPoint[] = [
    { tick: "0", value: 10, provenance: "counted" },
    { tick: "1", value: 11, provenance: "counted" },
    { tick: "2", value: 12, provenance: "modelled" },
    { tick: "3", value: 13, provenance: "modelled" },
  ];

  it("tìm được chỗ nguồn đổi bản chất", () => {
    expect(provenanceBreaks(chuoi)).toEqual([2]);
    expect(isUniformProvenance(chuoi)).toBe(false);
  });

  it("chuỗi cùng nguồn thì vẽ liền được", () => {
    const deu = chuoi.map((p) => ({ ...p, provenance: "counted" as const }));
    expect(provenanceBreaks(deu)).toEqual([]);
    expect(isUniformProvenance(deu)).toBe(true);
  });

  it("chuỗi rỗng và một điểm không làm hỏng gì", () => {
    expect(provenanceBreaks([])).toEqual([]);
    expect(isUniformProvenance([chuoi[0]!])).toBe(true);
  });
});

describe("đồ thị xã hội lọc theo cái người xem biết", () => {
  const quan_he: Relation[] = [
    { from: "b", to: "c", kind: "ally", strength: 500, known: true },
    { from: "a", to: "b", kind: "kin", strength: 900, known: true },
    { from: "a", to: "z", kind: "ally", strength: 800, known: false },
  ];

  it("liên minh bí mật không có mặt trên đồ thị", () => {
    const thay = visibleRelations(quan_he);
    expect(thay).toHaveLength(2);
    expect(thay.some((r) => r.to === "z")).toBe(false);
  });

  it("thứ tự ổn định, không phụ thuộc thứ tự nạp", () => {
    const a = visibleRelations(quan_he).map((r) => `${r.from}->${r.to}`);
    const b = visibleRelations([...quan_he].reverse()).map(
      (r) => `${r.from}->${r.to}`,
    );
    expect(a).toEqual(b);
    expect(a[0]).toBe("a->b");
  });
});

describe("knowledge graph hiện cái đang chặn", () => {
  it("nút không chặn gì thì bấm được", () => {
    const n = { id: "n", label: "thép", level: "conceptual", blockers: [] };
    expect(isActionable(n)).toBe(true);
    expect(whyBlocked(n)).toBe("");
  });

  it("nút bị chặn nói rõ thiếu gì, không phải một nút xám", () => {
    const n = {
      id: "n",
      label: "thép",
      level: "unknown",
      blockers: ["thiếu core.iron_ore", "cần 2 chuyên môn, có 1"],
    };
    expect(isActionable(n)).toBe(false);
    const s = whyBlocked(n);
    expect(s).toContain("core.iron_ore");
    expect(s).toContain("2 chuyên môn");
  });
});

describe("bộ kiểm panel", () => {
  it("bắt được số liệu không nói rõ nguồn", () => {
    const v: SocietyView = {
      metrics: [demDuoc(), { ...uocLuong(), uncertainty: null }],
      relations: [],
      knowledge: [],
      economy: [],
      lod: "far",
    };
    expect(unlabelledMetrics(v)).toHaveLength(1);
  });

  it("panel khai đúng thì sạch", () => {
    const v: SocietyView = {
      metrics: [demDuoc(), uocLuong()],
      relations: [],
      knowledge: [],
      economy: [],
      lod: "active",
    };
    expect(unlabelledMetrics(v)).toEqual([]);
  });
});
