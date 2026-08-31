/**
 * Panel xã hội, tri thức và kinh tế (`idea.md §18.3`, `§18.7`, `PD-19`).
 *
 * ## Điều khó nhất ở đây là **thang zoom nói thật**
 *
 * `§18.7`: thang zoom có chỉ báo *"ước lượng theo mô hình vùng"*.
 *
 * Một biểu đồ dân số ở mức lục địa **không** được dựng từ việc đếm từng người —
 * ở LOD `far` không có từng người để mà đếm (`§22.14`). Nó được dựng từ thống kê
 * gộp. Con số đó **đúng theo nghĩa bảo toàn**, nhưng nó không phải một phép đếm,
 * và hai chuyện đó khác nhau ở đúng chỗ người chơi hay dựa vào:
 *
 * - "9 240 người" ở mức `active` là **đếm được**, bấm vào xem được từng người;
 * - "≈9 200 người" ở mức `far` là **suy ra**, và bấm vào chỉ có mô hình vùng.
 *
 * Vẽ hai thứ đó giống nhau là nói dối một cách rất khó phát hiện: người chơi lập
 * kế hoạch dựa trên một con số họ tưởng là đếm được. Nên [`Metric`] mang
 * [`Provenance`] **bắt buộc**, không có mặc định.
 *
 * ## Đồ thị tri thức hiển thị **cái đang chặn**, không phải nút bị làm mờ
 *
 * Cùng nguyên tắc với `blockers()` ở `mow-knowledge`: một nút xám không nói cho
 * ai biết phải làm gì.
 */

/** Một con số ở panel này đến từ đâu. */
export type Provenance =
  /** Đếm thật, từng thực thể một. Bấm vào là ra danh sách. */
  | "counted"
  /** Suy ra từ mô hình vùng ở LOD thấp. Bảo toàn, nhưng không phải phép đếm. */
  | "modelled"
  /** Ước đoán của một nhân vật, không phải của thế giới (`§18.9`). */
  | "believed";

/** Nhãn hiển thị cho từng nguồn — người chơi phải đọc được, không phải đoán. */
export const PROVENANCE_LABEL: Record<Provenance, string> = {
  counted: "đếm được",
  modelled: "ước lượng theo mô hình vùng",
  believed: "theo lời người trong vùng",
};

/** Một số liệu trên panel. */
export interface Metric {
  key: string;
  label: string;
  value: number;
  /** **Bắt buộc.** Xem docstring của module. */
  provenance: Provenance;
  /**
   * Sai số, nếu là ước lượng.
   *
   * `null` cho `counted` — một phép đếm không có sai số. Bắt buộc khác `null`
   * cho `modelled`: một ước lượng không nói sai số là một ước lượng giả vờ làm
   * phép đếm.
   */
  uncertainty: number | null;
}

/** Một số liệu có hợp lệ không, và nếu không thì vì sao. */
export function validateMetric(m: Metric): string[] {
  const loi: string[] = [];
  if (m.provenance === "counted" && m.uncertainty !== null) {
    loi.push(`\`${m.key}\`: đã đếm được thì không có sai số`);
  }
  if (m.provenance !== "counted" && m.uncertainty === null) {
    loi.push(
      `\`${m.key}\`: ước lượng phải nói sai số, nếu không nó đang giả vờ là phép đếm`,
    );
  }
  return loi;
}

/** Một quan hệ trong society view. */
export interface Relation {
  from: string;
  to: string;
  kind: "kin" | "ally" | "rival" | "vassal" | "trade" | "creditor";
  /** `-1000`..`1000`. */
  strength: number;
  /**
   * Người chơi **biết** quan hệ này không.
   *
   * Ở chế độ hóa thân, một liên minh bí mật không có mặt trên đồ thị. Trường này
   * để giao diện không vẽ một cạnh mà read model đã lọc bỏ — nếu nó có mặt ở
   * client thì `§18.9` đã hỏng từ trước rồi.
   */
  known: boolean;
}

/** Đồ thị xã hội đã lọc theo những gì người xem được biết. */
export function visibleRelations(all: Relation[]): Relation[] {
  return all
    .filter((r) => r.known)
    .sort(
      (a, b) =>
        a.from.localeCompare(b.from) ||
        a.to.localeCompare(b.to) ||
        a.kind.localeCompare(b.kind),
    );
}

/** Một nút trên knowledge graph, ở dạng để vẽ. */
export interface KnowledgeNodeView {
  id: string;
  label: string;
  /** `unknown` … `mastered`. */
  level: string;
  /** **Cái đang chặn**, không phải một nút xám. */
  blockers: string[];
}

/**
 * Nút này có bấm được không.
 *
 * Bấm được nghĩa là *"bắt đầu nghiên cứu ngay"*. Không chặn gì thì bấm được.
 */
export function isActionable(n: KnowledgeNodeView): boolean {
  return n.blockers.length === 0;
}

/**
 * Câu trả lời cho "vì sao chưa nghiên cứu được".
 *
 * Trả về chuỗi rỗng khi không có gì chặn — chứ không phải một câu chung chung.
 * `§18.13` nguyên tắc 3: câu trả lời dựng từ dữ liệu.
 */
export function whyBlocked(n: KnowledgeNodeView): string {
  if (n.blockers.length === 0) return "";
  return `Chưa nghiên cứu được: ${n.blockers.join("; ")}.`;
}

/** Một điểm trên biểu đồ kinh tế. */
export interface EconPoint {
  tick: string;
  value: number;
  provenance: Provenance;
}

/**
 * Một chuỗi thời gian có **trộn nguồn** không.
 *
 * Đây là cái bẫy đặc trưng của biểu đồ đường: mấy trăm tick đầu là số đếm được
 * ở LOD `active`, rồi người chơi đi chỗ khác, vùng tụt xuống `far`, và phần còn
 * lại là số suy ra. Đường vẽ liền một mạch, và chỗ đổi bản chất **không nhìn
 * thấy được** — trong khi độ tin cậy của hai nửa khác hẳn nhau.
 *
 * Trả về các chỉ số mà nguồn thay đổi, để giao diện đổi nét vẽ ở đúng chỗ đó.
 */
export function provenanceBreaks(series: EconPoint[]): number[] {
  const cho: number[] = [];
  for (let i = 1; i < series.length; i++) {
    if (series[i]!.provenance !== series[i - 1]!.provenance) cho.push(i);
  }
  return cho;
}

/**
 * Toàn bộ chuỗi có đáng tin ở cùng một mức không.
 *
 * `false` nghĩa là giao diện **phải** cho thấy chỗ đổi — không được vẽ một đường
 * liền.
 */
export function isUniformProvenance(series: EconPoint[]): boolean {
  return provenanceBreaks(series).length === 0;
}

/** Toàn bộ nội dung panel. */
export interface SocietyView {
  metrics: Metric[];
  relations: Relation[];
  knowledge: KnowledgeNodeView[];
  economy: EconPoint[];
  /** Mức zoom hiện tại. */
  lod: "active" | "near" | "far";
}

/**
 * Panel này có đang hiện số liệu **mà nó không nói rõ nguồn** không.
 *
 * Đây là bộ kiểm chạy trong test và trong dev build. Nó tồn tại vì lỗi ở đây
 * không bao giờ trông như lỗi: một con số thiếu nhãn vẫn hiện ra bình thường, và
 * người chơi vẫn tin nó.
 */
export function unlabelledMetrics(v: SocietyView): string[] {
  return v.metrics.flatMap(validateMetric);
}
