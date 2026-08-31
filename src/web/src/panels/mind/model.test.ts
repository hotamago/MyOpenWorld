import { describe, expect, it } from "vitest";
import {
  canOpenFor,
  chosenOption,
  danglingEvidence,
  dominantFactor,
  explain,
  memoryCertainty,
  orderedMemories,
  rejectedOptions,
  type Decision,
  type MindView,
} from "./model";

function quyetDinh(): Decision {
  return {
    chosen: "core.steal",
    source: "model",
    model: "test-model",
    options: [
      {
        action: "core.steal",
        score: 120,
        factors: [
          { label: "đang đói", weight: 200, evidence: null },
          { label: "kho thóc không ai canh", weight: 60, evidence: "obs-1" },
          { label: "sợ bị đuổi khỏi làng", weight: -140, evidence: "mem-1" },
        ],
      },
      { action: "core.beg", score: 90, factors: [{ label: "còn người quen", weight: 90, evidence: "mem-1" }] },
      { action: "core.wait", score: 10, factors: [{ label: "an toàn", weight: 10, evidence: null }] },
    ],
  };
}

function view(over: Partial<MindView> = {}): MindView {
  return {
    entity: "e#1",
    tick: "1000",
    observations: [
      { id: "obs-1", summary: "kho thóc không ai canh", channel: "sight", identity: null, tick: "999" },
    ],
    memories: [
      { id: "mem-1", content: "năm ngoái Bram bị đuổi vì trộm", relevance: 700, firsthand: false, tick: "10" },
    ],
    goals: [{ id: "g1", label: "sống qua mùa đông", priority: 900, deadline: null }],
    decision: quyetDinh(),
    lens: "embodied",
    ...over,
  };
}

describe("lý do chọn action", () => {
  it("dựng từ dữ liệu, và tổng các phần bằng đúng những gì được hiện", () => {
    const d = quyetDinh();
    const s = explain(d);
    // Mọi mảnh chữ phải truy được về một `factor`.
    for (const f of chosenOption(d)!.factors) {
      expect(s).toContain(f.label);
    }
  });

  it("nói cả cái đã bị loại — đó là câu trả lời cho 'sao nó không làm X?'", () => {
    const s = explain(quyetDinh());
    expect(s).toContain("core.beg");
    expect(s).toContain("30 điểm");
  });

  it("phân biệt thúc đẩy với cản trở", () => {
    const s = explain(quyetDinh());
    expect(s).toMatch(/vì .*đang đói/);
    expect(s).toMatch(/dù .*sợ bị đuổi/);
  });

  it("nói ra khi quyết định đến từ policy, không phải model", () => {
    const d = { ...quyetDinh(), source: "fallback" as const, model: null };
    expect(explain(d)).toContain("[fallback]");
    // Còn quyết định bình thường thì không dán nhãn gì.
    expect(explain(quyetDinh())).not.toContain("[");
  });

  it("lựa chọn bị loại xếp theo điểm giảm dần", () => {
    expect(rejectedOptions(quyetDinh()).map((o) => o.action)).toEqual([
      "core.beg",
      "core.wait",
    ]);
  });

  it("yếu tố quyết định nhất lấy theo giá trị tuyệt đối", () => {
    // `đang đói` +200 thắng, nhưng nếu đảo dấu thì `-240` phải thắng.
    const o = chosenOption(quyetDinh())!;
    expect(dominantFactor(o)?.label).toBe("đang đói");

    const can_tro = {
      ...o,
      factors: [
        { label: "đang đói", weight: 200, evidence: null },
        { label: "sợ chết", weight: -240, evidence: null },
      ],
    };
    expect(dominantFactor(can_tro)?.label).toBe("sợ chết");
  });

  it("không có lựa chọn nào khớp thì vẫn nói được cái đã chọn", () => {
    const d = { ...quyetDinh(), chosen: "core.fly" };
    expect(explain(d)).toContain("core.fly");
  });
});

describe("bằng chứng phải truy được", () => {
  it("mọi yếu tố trỏ về nguồn có thật trong chính view này", () => {
    expect(danglingEvidence(view())).toEqual([]);
  });

  it("bắt được yếu tố trỏ ra ngoài context", () => {
    const v = view();
    v.decision!.options[0]!.factors.push({
      label: "nghe nói kho bên kia dễ hơn",
      weight: 50,
      evidence: "obs-999",
    });
    expect(danglingEvidence(v)).toEqual(["obs-999"]);
  });

  it("yếu tố nội tại được phép không có nguồn", () => {
    const v = view();
    expect(v.decision!.options[0]!.factors[0]!.evidence).toBeNull();
    expect(danglingEvidence(v)).toEqual([]);
  });
});

describe("ống kính", () => {
  it("hóa thân chỉ mở được panel của chính mình", () => {
    const v = view({ lens: "embodied" });
    expect(canOpenFor(v, "e#1")).toBe(true);
    expect(canOpenFor(v, "e#2")).toBe(false);
  });

  it("quan sát và True God mở được của người khác", () => {
    expect(canOpenFor(view({ lens: "observer" }), "e#2")).toBe(true);
    expect(canOpenFor(view({ lens: "true_god" }), "e#2")).toBe(true);
  });
});

describe("ký ức", () => {
  it("xếp theo liên quan, phá hòa ổn định", () => {
    const v = view({
      memories: [
        { id: "b", content: "x", relevance: 500, firsthand: true, tick: "1" },
        { id: "a", content: "y", relevance: 500, firsthand: true, tick: "2" },
        { id: "c", content: "z", relevance: 900, firsthand: true, tick: "3" },
      ],
    });
    expect(orderedMemories(v).map((m) => m.id)).toEqual(["c", "a", "b"]);
  });

  it("nghe kể và thấy tận mắt không được vẽ giống nhau", () => {
    expect(memoryCertainty({ id: "a", content: "", relevance: 0, firsthand: true, tick: "0" })).toBe("truth");
    expect(memoryCertainty({ id: "b", content: "", relevance: 0, firsthand: false, tick: "0" })).toBe("belief");
  });
});

describe("quan sát không danh tính", () => {
  it("'có ai đó' khác 'không có ai'", () => {
    const v = view();
    // `identity: null` là một quan sát có thật, chỉ là chưa nhận ra ai.
    expect(v.observations).toHaveLength(1);
    expect(v.observations[0]!.identity).toBeNull();
  });
});
