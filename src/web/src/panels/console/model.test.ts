import { describe, expect, it } from "vitest";
import {
  auditView,
  buildDiff,
  DESTRUCTIVE_SCOPE,
  filterByProvenance,
  preview,
  rollbackPoints,
  wasIntervention,
  type AuditEvent,
  type Proposal,
} from "./model";

const nho: Proposal = {
  summary: "đặt lại cơn đói một người",
  intervention: "administrative",
  ops: [
    {
      kind: "set_attr",
      target: "need.hunger",
      count: 1,
      before: "120",
      after: "800",
    },
  ],
  lawsTouched: [],
  cost: 0,
  risks: [],
};

const lon: Proposal = {
  summary: "xóa một thành phố",
  intervention: "hard_override",
  ops: [{ kind: "despawn", target: "entity", count: 5000 }],
  lawsTouched: ["core.inheritance", "core.taxation"],
  cost: 0,
  risks: [],
};

describe("console Yuu và True God (PF-15, §18.12)", () => {
  // ─────────── diff dữ liệu, không phải diff văn bản ───────────

  it("một đề xuất chạm 5000 thực thể vẫn ra một dòng đọc được", () => {
    const d = buildDiff(lon);
    expect(d.lines).toHaveLength(1);
    expect(d.totalScope).toBe(5000);
    expect(d.lines[0]!.summary).toContain("5000");
  });

  it("dòng diff nói rõ từ đâu tới đâu khi là sửa giá trị", () => {
    const d = buildDiff(nho);
    expect(d.lines[0]!.summary).toContain("120 → 800");
  });

  it("phá hủy diện rộng đo bằng phạm vi, không bằng tên gọi", () => {
    const don_dep: Proposal = {
      ...lon,
      summary: "dọn dẹp nhẹ nhàng",
      ops: [{ kind: "despawn", target: "entity", count: DESTRUCTIVE_SCOPE }],
    };
    expect(buildDiff(don_dep).destructive).toBe(true);
  });

  it("xóa ít không phải phá hủy diện rộng", () => {
    const it_thoi: Proposal = {
      ...lon,
      ops: [{ kind: "despawn", target: "entity", count: 5 }],
    };
    expect(buildDiff(it_thoi).destructive).toBe(false);
  });

  it("sửa thuộc tính trên nhiều thực thể không phải phá hủy", () => {
    const nhieu: Proposal = {
      ...nho,
      ops: [{ kind: "set_attr", target: "need.hunger", count: 9000 }],
    };
    expect(buildDiff(nhieu).destructive).toBe(false);
  });

  // ─────────── preview ───────────

  it("preview mang đủ phạm vi, chi phí, luật bị chạm, rủi ro", () => {
    const p = preview(lon);
    expect(p.diff.totalScope).toBe(5000);
    expect(p.lawsTouched).toEqual(["core.inheritance", "core.taxation"]);
    expect(p.willSnapshot).toBe(true);
    expect(p.commitEnabled).toBe(true);
  });

  it("rủi ro chặn thì TẮT nút commit, không chỉ cảnh báo", () => {
    const nguy_hiem: Proposal = {
      ...lon,
      risks: [
        { code: "pack.missing", detail: "thiếu pack", blocking: true },
        { code: "cosmetic", detail: "chỉ là cảnh báo", blocking: false },
      ],
    };
    const p = preview(nguy_hiem);
    expect(p.commitEnabled).toBe(false);
    // Nhưng vẫn hiện đủ rủi ro, kể cả cái không chặn.
    expect(p.risks).toHaveLength(2);
  });

  it("chỉ có cảnh báo thì vẫn bấm được", () => {
    const p = preview({
      ...nho,
      risks: [{ code: "x", detail: "cẩn thận", blocking: false }],
    });
    expect(p.commitEnabled).toBe(true);
  });

  it("kế hoạch nhỏ không cần ảnh chụp", () => {
    expect(preview(nho).willSnapshot).toBe(false);
  });

  it("luật bị chạm sắp theo thứ tự ổn định", () => {
    const a = preview({ ...lon, lawsTouched: ["b", "a", "c"] });
    expect(a.lawsTouched).toEqual(["a", "b", "c"]);
  });

  // ─────────── audit view ───────────

  const nhat_ky: AuditEvent[] = [
    { seq: 1, provenance: "simulation", summary: "mùa màng thất bát" },
    { seq: 2, provenance: "llm_intent", summary: "Aren quyết định rời làng" },
    { seq: 3, provenance: "yuu_proposal", summary: "Yuu đề xuất hạn hán" },
    {
      seq: 4,
      provenance: "true_god",
      intervention: "diegetic",
      summary: "một cơn bão bất thường",
    },
  ];

  it('trả lời được "cái gì tự nhiên, cái gì do Yuu, cái gì do tôi"', () => {
    expect(auditView(nhat_ky)).toEqual({
      simulation: 1,
      llm_intent: 1,
      yuu_proposal: 1,
      true_god: 1,
    });
  });

  it("KỂ CẢ khi True God giả vờ đó là chuyện tự nhiên, audit view vẫn phân biệt được", () => {
    // Event số 4 là `diegetic` — cư dân trong thế giới không phân biệt được nó
    // với một cơn bão thật. Audit view thì phân biệt được.
    const con_nguoi = nhat_ky.find((e) => e.seq === 4)!;
    expect(con_nguoi.intervention).toBe("diegetic");
    expect(con_nguoi.provenance).toBe("true_god");
    expect(wasIntervention(con_nguoi)).toBe(true);
    expect(filterByProvenance(nhat_ky, ["true_god"])).toHaveLength(1);
  });

  it("chuyện thật sự tự nhiên không bị đếm là can thiệp", () => {
    expect(wasIntervention(nhat_ky[0]!)).toBe(false);
    expect(wasIntervention(nhat_ky[1]!)).toBe(false);
  });

  it("lọc nhiều nguồn cùng lúc", () => {
    const r = filterByProvenance(nhat_ky, ["yuu_proposal", "true_god"]);
    expect(r.map((e) => e.seq)).toEqual([3, 4]);
  });

  it("lọc rỗng cho danh sách rỗng, không cho tất cả", () => {
    expect(filterByProvenance(nhat_ky, [])).toEqual([]);
  });

  // ─────────── rollback ───────────

  it("chỉ về được những chỗ có ảnh chụp", () => {
    const r = rollbackPoints([
      { event: 1, atTick: 10, snapshot: false, automatic: false },
      { event: 2, atTick: 20, snapshot: true, automatic: true },
      { event: 3, atTick: 30, snapshot: true, automatic: false },
    ]);
    expect(r.map((x) => x.event)).toEqual([2, 3]);
    expect(r[0]!.automatic).toBe(true);
  });

  it("không có ảnh chụp nào thì không hứa rollback được", () => {
    expect(
      rollbackPoints([{ event: 1, atTick: 10, snapshot: false, automatic: false }]),
    ).toEqual([]);
  });
});
