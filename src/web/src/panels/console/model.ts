/**
 * Console Yuu và True God (`idea.md §18.12`, `§15.5`, `PF-15`).
 *
 * Ba thứ, và cả ba nói cùng một chuyện: **người chơi phải thấy trước khi
 * chuyện xảy ra, và thấy lại sau khi nó đã xảy ra**.
 *
 * | Thứ | `§18.12` đòi gì |
 * |---|---|
 * | Yuu console | đề xuất hiện dưới dạng **diff dữ liệu có preview**, kèm phạm vi, chi phí, thực thể bị ảnh hưởng, luật bị chạm, báo cáo rủi ro |
 * | True God console | transaction editor, snapshot, branch; **mọi can thiệp ghi provenance** |
 * | Audit view | lọc event theo provenance: *"cái gì tự nhiên, cái gì do Yuu, cái gì do tôi"* |
 *
 * ## Diff dữ liệu, không phải diff văn bản
 *
 * Một đề xuất hiện ra dưới dạng `-old_value +new_value` trên một khối JSON là
 * thứ không ai đọc nổi khi nó chạm 4000 thực thể. [`Diff`] gộp theo **loại
 * thay đổi**, và mỗi dòng trả lời một câu người chơi thật sự hỏi: *bao nhiêu
 * thực thể, thuộc tính nào, từ đâu tới đâu*.
 *
 * ## "Kể cả khi True God chọn giả vờ đó là chuyện tự nhiên"
 *
 * `§18.12` viết đúng câu đó, và nó là yêu cầu cứng nhất của module. Người chơi
 * được phép làm một can thiệp trông như tự nhiên **trong thế giới** — cư dân
 * không phân biệt được. Nhưng [`auditView`] thì phân biệt được, luôn luôn, vì
 * provenance nằm trên event chứ không nằm trên cách nó hiển thị.
 *
 * Nên [`Intervention`] không có biến thể nào tên `Hidden`, và
 * [`filterByProvenance`] không có tham số nào ẩn một nguồn khỏi audit view.
 */

/** Nguồn của một thay đổi (`§17.1` provenance). */
export type Provenance = "simulation" | "llm_intent" | "yuu_proposal" | "true_god";

/** Mức can thiệp (`§16.2`). */
export type Intervention = "diegetic" | "administrative" | "hard_override";

/** Một thao tác trong đề xuất. */
export interface Op {
  /** Loại: `set_attr`, `spawn`, `despawn`, `redefine_content`. */
  kind: string;
  /** Thuộc tính hoặc định nghĩa bị chạm. */
  target: string;
  /** Bao nhiêu thực thể. */
  count: number;
  /** Giá trị trước, nếu là sửa. */
  before?: string;
  /** Giá trị sau, nếu là sửa. */
  after?: string;
}

/** Một rủi ro trong báo cáo (`§15.5`). */
export interface Risk {
  code: string;
  detail: string;
  /** Chặn hẳn việc commit, hay chỉ cảnh báo. */
  blocking: boolean;
}

/** Một đề xuất chưa commit. */
export interface Proposal {
  summary: string;
  intervention: Intervention;
  ops: Op[];
  /** Luật bị chạm — `§18.12` liệt riêng vì nó khác với thực thể bị chạm. */
  lawsTouched: string[];
  /** Chi phí, đơn vị nhỏ nhất của tiền tệ. */
  cost: number;
  risks: Risk[];
}

/** Một dòng trong diff dữ liệu. */
export interface DiffLine {
  /** Câu mô tả, viết cho người đọc — `§18.13` nguyên tắc 1. */
  summary: string;
  /** Bao nhiêu thực thể dòng này chạm. */
  scope: number;
  /** Có phá hủy không.  */
  destructive: boolean;
}

/** Diff của một đề xuất. */
export interface Diff {
  lines: DiffLine[];
  /** Tổng thực thể bị chạm. */
  totalScope: number;
  /** Có phá hủy diện rộng không — quyết định việc tự snapshot (`§15.5`). */
  destructive: boolean;
}

/** Ngưỡng phá hủy diện rộng, khớp với `NGUONG_PHA_HUY_DIEN_RONG` ở Rust. */
export const DESTRUCTIVE_SCOPE = 1000;

/** Thao tác nào là phá hủy. */
function laPhaHuy(kind: string): boolean {
  return kind === "despawn" || kind === "redefine_content";
}

/**
 * Dựng diff **dữ liệu** từ một đề xuất (`§18.12`).
 *
 * Mỗi dòng là một câu, không phải một khối JSON. Một đề xuất chạm 4000 thực
 * thể vẫn ra vài dòng đọc được — đó là toàn bộ khác biệt giữa diff dữ liệu và
 * diff văn bản.
 */
export function buildDiff(p: Proposal): Diff {
  const lines: DiffLine[] = p.ops.map((o) => {
    const doi =
      o.before !== undefined && o.after !== undefined
        ? `: ${o.before} → ${o.after}`
        : "";
    return {
      summary: `${o.kind} ${o.target} trên ${o.count} thực thể${doi}`,
      scope: o.count,
      destructive: laPhaHuy(o.kind),
    };
  });
  const totalScope = lines.reduce((a, l) => a + l.scope, 0);
  return {
    lines,
    totalScope,
    destructive: lines.some((l) => l.destructive) && totalScope >= DESTRUCTIVE_SCOPE,
  };
}

/** Preview hiện ra trước khi người chơi bấm. */
export interface Preview {
  diff: Diff;
  lawsTouched: string[];
  cost: number;
  risks: Risk[];
  /** Sẽ tự chụp ảnh trước không (`§15.5`). */
  willSnapshot: boolean;
  /** Bấm commit được không. */
  commitEnabled: boolean;
}

/**
 * Dựng preview.
 *
 * `commitEnabled` tắt khi có rủi ro chặn. Không phải "cảnh báo rồi vẫn cho
 * bấm": một nút bấm được là một nút sẽ được bấm, và một hộp thoại cảnh báo là
 * thứ người ta học cách bấm qua trong ba ngày.
 */
export function preview(p: Proposal): Preview {
  const diff = buildDiff(p);
  return {
    diff,
    lawsTouched: [...p.lawsTouched].sort(),
    cost: p.cost,
    risks: p.risks,
    willSnapshot: diff.destructive,
    commitEnabled: !p.risks.some((r) => r.blocking),
  };
}

/** Một event trong audit view. */
export interface AuditEvent {
  seq: number;
  /** Nguồn — **luôn có**, không tùy chọn. */
  provenance: Provenance;
  /** Mức can thiệp, nếu là can thiệp. */
  intervention?: Intervention;
  /** Câu mô tả, dựng từ dữ liệu. */
  summary: string;
}

/**
 * Lọc event theo provenance (`§18.12`).
 *
 * Trả lời đúng ba câu hỏi mà `§18.12` nêu: *"cái gì tự nhiên, cái gì do Yuu,
 * cái gì do tôi"*.
 *
 * Không có tham số nào ẩn một nguồn. Một can thiệp mà True God chọn cho trông
 * như tự nhiên vẫn hiện ở đây với `provenance: "true_god"` — vì lời "giả vờ" ở
 * `§18.12` là giả vờ **với cư dân trong thế giới**, không phải với công cụ
 * kiểm toán.
 */
export function filterByProvenance(
  events: readonly AuditEvent[],
  want: readonly Provenance[],
): AuditEvent[] {
  const tap = new Set(want);
  return events.filter((e) => tap.has(e.provenance));
}

/** Phân rã audit view theo nguồn — bảng tóm tắt của `§18.12`. */
export function auditView(events: readonly AuditEvent[]): Record<Provenance, number> {
  const ra: Record<Provenance, number> = {
    simulation: 0,
    llm_intent: 0,
    yuu_proposal: 0,
    true_god: 0,
  };
  for (const e of events) ra[e.provenance] += 1;
  return ra;
}

/**
 * Event này có phải do ai đó can thiệp không — đối lại "thế giới tự vận hành".
 *
 * Đây là câu hỏi mà audit view tồn tại để trả lời, và nó đọc `provenance` chứ
 * không đọc `intervention`: một can thiệp `diegetic` — qua avatar, qua phép —
 * trông y hệt chuyện tự nhiên **với cư dân**, nhưng nguồn của nó vẫn là
 * `true_god` và audit view vẫn phải nói ra.
 */
export function wasIntervention(e: AuditEvent): boolean {
  return e.provenance === "yuu_proposal" || e.provenance === "true_god";
}

/** Một điểm rollback. */
export interface RollbackPoint {
  event: number;
  atTick: number;
  /** Ảnh chụp tự động hay do người yêu cầu. */
  automatic: boolean;
}

/**
 * Những chỗ rollback được.
 *
 * Chỉ những event **có ảnh chụp**. Không hứa dựng lại từ event log: dựng lại
 * được thì tốt, nhưng *"có lẽ dựng lại được"* không phải thứ để hứa với người
 * vừa xóa nhầm một lục địa.
 */
export function rollbackPoints(
  commits: readonly { event: number; atTick: number; snapshot: boolean; automatic: boolean }[],
): RollbackPoint[] {
  return commits
    .filter((c) => c.snapshot)
    .map((c) => ({ event: c.event, atTick: c.atTick, automatic: c.automatic }));
}
