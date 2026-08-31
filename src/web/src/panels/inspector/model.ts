/**
 * Panel Inspector và Timeline (`idea.md §18.3`, `§18.10`, `PB-16`).
 *
 * ## Hai quy tắc, và cả hai đều về việc **không bịa**
 *
 * **1. Đọc state thật, không đọc bản sao đã diễn giải.**
 * Inspector hiện những gì server gửi xuống, và không tự tính thêm gì. Nếu nó
 * tính "chắc là nhân vật này đang đói vì đã lâu không ăn", nó đang mô phỏng lại
 * thế giới ở client — và hai bản mô phỏng sẽ trôi khỏi nhau.
 *
 * **2. Chuỗi nhân quả chỉ hiện event có thật.**
 * `§22.17`: tường thuật không được thêm sự kiện không có trong nhật ký. Một mắt
 * xích thiếu là một mắt xích thiếu; vẽ một mũi tên đứt nét ghi "có lẽ vì..."
 * thì tệ hơn không vẽ gì, vì người đọc sẽ tin nó.
 */

/** Một trường trong Inspector. */
export interface Field {
  key: string;
  value: string;
  /**
   * Giá trị này đến từ đâu.
   *
   * `"state"` là giá trị lưu trực tiếp. `"derived"` là suy ra, và khi đó
   * `steps` phải giải thích được (`§18.13`).
   */
  origin: "state" | "derived";
  /** Các bước suy ra, nếu `origin === "derived"`. */
  steps?: DerivationStep[];
  /** Nhóm để giao diện tách khối. */
  group: string;
}

/** Một bước trong lời giải thích một giá trị suy ra. */
export interface DerivationStep {
  source: string;
  before: string;
  after: string;
}

/** Một mắt xích trong chuỗi nhân quả. */
export interface CauseNode {
  seq: string;
  tick: string;
  kind: string;
  actor: string | null;
  subject: string | null;
  /** Sự kiện nào đã dẫn tới sự kiện này. */
  cause: string | null;
  /** Phiên bản luật lúc đó (`§22.49`). */
  lawVersion: number | null;
}

/** Chuỗi nhân quả đã dựng. */
export interface CauseChain {
  nodes: CauseNode[];
  /**
   * Chuỗi có bị cắt không, và vì sao.
   *
   * `"root"` là tới gốc thật; `"depth"` là chạm trần độ sâu; `"missing"` là
   * mắt xích tiếp theo **không có trong nhật ký**.
   *
   * `"missing"` là trường hợp quan trọng nhất: nó nghĩa là một sự kiện được
   * tạo ra mà không ghi `cause`, và đó là một bug — nhưng giao diện phải nói
   * ra chứ không lấp liếm.
   */
  terminated: "root" | "depth" | "missing";
}

/** Dựng chuỗi nhân quả từ một tập sự kiện. */
export function buildCauseChain(
  events: Map<string, CauseNode>,
  from: string,
  maxDepth = 256,
): CauseChain {
  const nodes: CauseNode[] = [];
  const seen = new Set<string>();
  let cur: string | null = from;

  while (cur !== null) {
    if (nodes.length >= maxDepth) return { nodes, terminated: "depth" };
    // Vòng lặp trong dữ liệu: cắt thay vì treo giao diện. Một giao diện treo
    // thì không ai gỡ lỗi bằng nó được nữa.
    if (seen.has(cur)) return { nodes, terminated: "depth" };
    seen.add(cur);

    const n = events.get(cur);
    if (!n) {
      // Mắt xích không có trong nhật ký. Nói ra, không bịa.
      return { nodes, terminated: "missing" };
    }
    nodes.push(n);
    cur = n.cause;
  }
  return { nodes, terminated: "root" };
}

/**
 * Kiểm chuỗi chỉ chứa event có thật (`§22.17`).
 *
 * Dùng trong test và trong Auditor. Trả về những `seq` được nhắc tới mà không
 * tồn tại trong nhật ký.
 */
export function phantomEvents(chain: CauseChain, log: Set<string>): string[] {
  return chain.nodes.filter((n) => !log.has(n.seq)).map((n) => n.seq);
}

/** Một mục trên dòng thời gian. */
export interface TimelineEntry {
  tick: string;
  kind: string;
  /** Nhãn ngắn cho người đọc. */
  label: string;
  /** Có phải mốc đáng dừng lại không (`§18.8`). */
  notable: boolean;
  /** `seq` của event, để bấm sang chuỗi nhân quả. */
  seq: string;
}

/**
 * Gom nhật ký thành dòng thời gian.
 *
 * Lọc theo `kinds` nếu có; **không** tóm tắt, không gộp, không diễn giải. Dòng
 * thời gian là nhật ký, chỉ được lọc và định dạng.
 */
export function buildTimeline(
  events: CauseNode[],
  opts: { kinds?: string[]; notableKinds?: string[] } = {},
): TimelineEntry[] {
  const loc = opts.kinds;
  const dang_chu_y = new Set(opts.notableKinds ?? []);
  return events
    .filter((e) => !loc || loc.includes(e.kind))
    .map((e) => ({
      tick: e.tick,
      kind: e.kind,
      label: e.kind.split(".").at(-1) ?? e.kind,
      notable: dang_chu_y.has(e.kind),
      seq: e.seq,
    }));
}

/** Dữ liệu một thực thể như Inspector nhận được. */
export interface EntityInspection {
  entity: string;
  /** Thuộc tính lưu trực tiếp. */
  attrs: Record<string, string | number | boolean>;
  /** Giá trị suy ra, kèm lời giải thích. */
  derived: Record<string, { value: string; steps: DerivationStep[] }>;
}

/**
 * Dựng danh sách trường cho Inspector.
 *
 * Mọi trường suy ra **phải** có `steps`. Đó là ràng buộc `§18.13` dưới dạng
 * kiểu: không có cách nào tạo một trường `derived` mà không kèm lời giải thích.
 */
export function buildInspector(e: EntityInspection): Field[] {
  const fields: Field[] = [];

  for (const [key, value] of Object.entries(e.attrs).sort(([a], [b]) => a.localeCompare(b))) {
    fields.push({
      key,
      value: String(value),
      origin: "state",
      group: key.split(".")[0] ?? "core",
    });
  }

  for (const [key, d] of Object.entries(e.derived).sort(([a], [b]) => a.localeCompare(b))) {
    fields.push({
      key,
      value: d.value,
      origin: "derived",
      steps: d.steps,
      group: key.split(".")[0] ?? "core",
    });
  }

  return fields;
}

/** Những trường suy ra mà thiếu lời giải thích — luôn là bug. */
export function unexplainedFields(fields: Field[]): string[] {
  return fields
    .filter((f) => f.origin === "derived" && (f.steps === undefined || f.steps.length === 0))
    .map((f) => f.key);
}
