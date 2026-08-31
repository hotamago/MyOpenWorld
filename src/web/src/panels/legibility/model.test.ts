import { describe, expect, it } from "vitest";
import {
  isTraceable,
  MAX_SYMPTOMS,
  relevantFields,
  symptoms,
  traceDerived,
  why,
  type Factor,
  type RawState,
} from "./model";

const khoe: RawState = {
  hunger: 800,
  fatigue: 200,
  effects: {},
  injuries: {},
};

describe("đọc được mà không cần đọc bảng số (PF-16, §18.13)", () => {
  // ───────── nguyên tắc 1: triệu chứng trước, con số sau ─────────

  it('hiện "gầy trơ xương, ho ra máu, đi khập khiễng" thay vì hunger: 0.12', () => {
    const r = symptoms({
      hunger: 120,
      fatigue: 200,
      effects: { "effect.grey_lung": 340 },
      injuries: { leg: 500 },
    });
    const chu = r.symptoms.map((s) => s.text);
    expect(chu).toContain("gầy trơ xương");
    expect(chu).toContain("ho ra máu");
    expect(chu).toContain("đi khập khiễng");
  });

  it("nhưng số đầy đủ LUÔN CÓ, không phải một chế độ riêng", () => {
    const r = symptoms({
      hunger: 120,
      fatigue: 200,
      effects: { "effect.grey_lung": 340 },
      injuries: { leg: 500 },
    });
    expect(r.numbers.hunger).toBe(120);
    expect(r.numbers["effect.grey_lung"]).toBe(340);
    expect(r.numbers["injury.leg"]).toBe(500);
  });

  it("mỗi triệu chứng chỉ về đúng con số sinh ra nó", () => {
    const r = symptoms({ ...khoe, hunger: 100 });
    expect(r.symptoms[0]!.source).toEqual({ field: "hunger", value: 100 });
  });

  it("người khỏe không có triệu chứng nào — và đó là một câu trả lời", () => {
    const r = symptoms(khoe);
    expect(r.symptoms).toEqual([]);
    expect(r.moreCount).toBe(0);
    // Nhưng số vẫn có.
    expect(r.numbers.hunger).toBe(800);
  });

  it("nặng nhất hiện trước", () => {
    const r = symptoms({
      hunger: 100, // severity 900
      fatigue: 700, // severity 700
      effects: {},
      injuries: {},
    });
    expect(r.symptoms[0]!.text).toBe("gầy trơ xương");
    expect(r.symptoms[1]!.text).toBe("mệt rã rời");
  });

  it("thứ tự ổn định giữa hai lần vẽ khi mức nặng bằng nhau", () => {
    const s: RawState = {
      ...khoe,
      effects: { "effect.fever": 500, "effect.poison": 500 },
    };
    expect(symptoms(s).symptoms).toEqual(symptoms(s).symptoms);
  });

  // ───────── nguyên tắc 4: không đổ tường số ─────────

  it("chỉ hiện ba triệu chứng; phần còn lại sau một cú bấm", () => {
    const r = symptoms({
      hunger: 50,
      fatigue: 950,
      effects: {
        "effect.grey_lung": 800,
        "effect.fever": 700,
        "effect.poison": 600,
      },
      injuries: { leg: 500, arm: 400 },
    });
    expect(r.symptoms).toHaveLength(MAX_SYMPTOMS);
    expect(r.moreCount).toBeGreaterThan(0);
  });

  it("trường không liên quan bị ẩn, không bị xóa", () => {
    const tat_ca = ["hunger", "fatigue", "skill.smithing", "lineage", "faith"];
    const r = relevantFields(tat_ca, "crafting", {
      crafting: ["skill.smithing", "fatigue"],
    });
    expect(r.shown).toEqual(["fatigue", "skill.smithing"]);
    expect(r.behindTabs).toEqual(["hunger", "lineage", "faith"]);
    // Không mất trường nào.
    expect(r.shown.length + r.behindTabs.length).toBe(tat_ca.length);
  });

  it("ngữ cảnh không có trong bảng thì ẩn hết, không hiện hết", () => {
    const r = relevantFields(["a", "b"], "khong_biet", {});
    expect(r.shown).toEqual([]);
    expect(r.behindTabs).toEqual(["a", "b"]);
  });

  // ───────── nguyên tắc 2: bấm được về nguồn ─────────

  it("can_fly: false chỉ ra được là vì cánh gãy hay vì quá tải", () => {
    const canh_gay = traceDerived([
      {
        field: "body.wing.left.condition",
        value: "broken",
        contribution: "không tạo được lực nâng",
      },
    ]);
    const qua_tai = traceDerived([
      { field: "load.mass_g", value: "42000", contribution: "vượt tải trọng" },
      { field: "strength", value: "300", contribution: "sức nâng tối đa 30 kg" },
    ]);
    expect(canh_gay[0]!.field).toContain("wing");
    expect(qua_tai[0]!.field).toContain("load");
    expect(canh_gay).not.toEqual(qua_tai);
  });

  it("giữ nguyên thứ tự tính, không sắp theo mức đóng góp", () => {
    const b = [
      { field: "a", value: "1", contribution: "nhỏ" },
      { field: "b", value: "2", contribution: "lớn" },
    ];
    expect(traceDerived(b).map((s) => s.field)).toEqual(["a", "b"]);
  });

  it("một giá trị suy ra không có nguồn là LỖI DỮ LIỆU, không phải câu trả lời", () => {
    expect(isTraceable([])).toBe(false);
    expect(isTraceable([{ field: "x", value: "1", contribution: "y" }])).toBe(true);
  });

  // ───────── nguyên tắc 3: "vì sao?" dựng từ dữ liệu ─────────

  const yeu_to: Factor[] = [
    { field: "belief.wage_there", value: "900", weight: 600, eventSeq: 41 },
    { field: "belief.safety_here", value: "200", weight: -800, eventSeq: 42 },
    { field: "social.has_contact", value: "true", weight: 100, eventSeq: 43 },
  ];

  it("sắp theo ĐỘ LỚN, không theo dấu — vì sao không cũng quan trọng", () => {
    const e = why(yeu_to);
    expect(e.factors[0]!.field).toBe("belief.safety_here");
    expect(e.factors[0]!.weight).toBe(-800);
    expect(e.factors[1]!.field).toBe("belief.wage_there");
  });

  it("mọi yếu tố trỏ về event thật", () => {
    const e = why(yeu_to);
    expect(e.fabricated).toBe(false);
    expect(e.factors.every((f) => f.eventSeq > 0)).toBe(true);
  });

  it("một yếu tố không có event làm cả lời giải thích thành bịa", () => {
    const e = why([
      ...yeu_to,
      { field: "vibes", value: "tốt", weight: 900, eventSeq: 0 },
    ]);
    expect(e.fabricated).toBe(true);
  });

  it("không có yếu tố nào thì không bịa ra lý do", () => {
    const e = why([]);
    expect(e.factors).toEqual([]);
    expect(e.fabricated).toBe(false);
  });

  it("thứ tự ổn định khi hai yếu tố cùng độ lớn", () => {
    const a: Factor[] = [
      { field: "b", value: "1", weight: 100, eventSeq: 1 },
      { field: "a", value: "1", weight: -100, eventSeq: 2 },
    ];
    expect(why(a).factors.map((f) => f.field)).toEqual(["a", "b"]);
  });
});
