/**
 * Panel Entity Mind (`idea.md §18.3`, `§18.13`, `§22.17`, `PC-14`).
 *
 * > Observation hiện tại, goal, plan, belief, ký ức được truy xuất và **lý do
 * > chọn action**.
 *
 * ## Panel này là lời hứa khó nhất trong `§18.13`
 *
 * Nguyên tắc 3 viết: *"Mọi quyết định của NPC ... đều có affordance hỏi lý do,
 * và câu trả lời dựng **từ dữ liệu** chứ không từ model."*
 *
 * Cám dỗ ở đây rất mạnh và rất dễ chiều: hỏi mô hình *"vì sao Aren làm thế?"*
 * rồi hiện câu trả lời. Nó đọc hay hơn hẳn một bảng phân rã điểm số. Và nó sai
 * theo một cách đặc biệt độc: câu trả lời sinh ra **sau** khi đã biết kết cục,
 * nên nó luôn mạch lạc — kể cả khi quyết định thật ra do một yếu tố hoàn toàn
 * khác. Người chơi học được một mô hình nhân quả không tồn tại, rồi dựa vào đó
 * mà chơi, rồi thua mà không hiểu vì sao.
 *
 * Nên [`Decision.factors`] là **bắt buộc** và [`explain`] chỉ ráp chữ quanh
 * những con số có thật. Không có trường nào ở đây nhận văn bản tự do từ model.
 *
 * ## Bốn khối, theo đúng thứ tự của một chu trình nhận thức
 *
 * ```text
 *   thấy gì      →  nhớ gì       →  muốn gì      →  nên làm gì
 *  observations     memories        goals           decision
 *  (§10.4 b2)      (§10.4 b3)      (§9.9)          (§10.4 b5-7)
 * ```
 *
 * Thứ tự đó không phải thẩm mỹ. Nó là thứ khiến người chơi đọc được *chuỗi* chứ
 * không chỉ đọc *kết quả* — và chuỗi mới là thứ dạy được cách thế giới vận hành.
 */

/**
 * Sự thật của thế giới, hay điều nhân vật tin.
 *
 * `§18.9` ràng buộc 1: **belief và sự thật không bao giờ được vẽ giống nhau.**
 * Sự thật vẽ đặc, belief vẽ viền đứt kèm mức tin cậy.
 */
export type Certainty = "truth" | "belief";

/** Một quan sát mà nhân vật đang có. */
export interface MindObservation {
  id: string;
  /** Mô tả bằng ngôn ngữ người — triệu chứng, không phải chỉ số (`§18.13`). */
  summary: string;
  /** Giác quan nào bắt được. */
  channel: string;
  /**
   * Danh tính, **nếu nhận ra**. `null` nghĩa là "có ai đó" chứ không phải
   * "không có ai" — và giao diện phải vẽ hai thứ đó khác nhau.
   */
  identity: string | null;
  tick: string;
}

/** Một ký ức đã được truy xuất cho lần nghĩ này. */
export interface MindMemory {
  id: string;
  content: string;
  /** Điểm liên quan `0`–`1000`. */
  relevance: number;
  /** Ký ức này dựa trên thấy tận mắt hay nghe kể. */
  firsthand: boolean;
  tick: string;
}

/** Một mục tiêu đang hoạt động. */
export interface MindGoal {
  id: string;
  label: string;
  /** Ưu tiên `0`–`1000`. */
  priority: number;
  /** Hết hạn ở tick nào, nếu có (`§20.5`). */
  deadline: string | null;
}

/** Một phần đóng góp vào điểm của một lựa chọn. */
export interface Factor {
  /** Tên đọc được: "đang đói", "đường xa", "sợ Bram". */
  label: string;
  /** Đóng góp vào điểm; âm là chống lại. */
  weight: number;
  /**
   * Trỏ về nguồn: một `MindObservation.id` hoặc `MindMemory.id`.
   *
   * `§18.13` nguyên tắc 2: mọi con số đều bấm được về nguồn. `null` chỉ dành
   * cho những yếu tố nội tại thuần túy — đói, mệt — không đến từ một quan sát
   * nào cả.
   */
  evidence: string | null;
}

/** Một lựa chọn mà nhân vật đã cân nhắc. */
export interface Option {
  action: string;
  score: number;
  factors: Factor[];
}

/** Quyết định đã chọn, kèm những gì đã bị loại. */
export interface Decision {
  /** Action được chọn. */
  chosen: string;
  /**
   * Mọi lựa chọn đã cân nhắc, **kể cả những cái bị loại**.
   *
   * Chỉ hiện cái được chọn thì người chơi thấy một quyết định; hiện cả những cái
   * bị loại thì họ thấy một *cân nhắc*, và đó là thứ dạy được luật chơi. Nó cũng
   * là cách duy nhất trả lời được câu hỏi hay gặp nhất: "sao nó không chạy đi?"
   */
  options: Option[];
  /**
   * Model đã được gọi hay đây là policy.
   *
   * Hiện ra vì `§20.10` đòi phân biệt được, và vì một quyết định do policy đưa
   * ra trong lúc provider hỏng **trông y hệt** một quyết định bình thường nếu
   * không nói.
   */
  source: "model" | "policy" | "fallback";
  /** Model thật sự đã dùng, nếu có. */
  model: string | null;
}

/** Toàn bộ nội dung panel. */
export interface MindView {
  entity: string;
  tick: string;
  observations: MindObservation[];
  memories: MindMemory[];
  goals: MindGoal[];
  decision: Decision | null;
  /**
   * Panel này đang nhìn qua ống kính nào.
   *
   * Ở chế độ hóa thân, panel chỉ mở được cho **chính avatar**. Mở nó cho người
   * khác là đọc trộm ý định — đúng cái mà `§18.9` cấm, và cái mà read model đã
   * chặn ở phía server. Trường này để giao diện không dựng một nút bấm dẫn tới
   * một request chắc chắn bị từ chối.
   */
  lens: "embodied" | "observer" | "true_god";
}

/** Lựa chọn đã chọn, nếu tìm được trong danh sách. */
export function chosenOption(d: Decision): Option | undefined {
  return d.options.find((o) => o.action === d.chosen);
}

/**
 * Những lựa chọn đã bị loại, xếp theo điểm giảm dần.
 *
 * Đây là dữ liệu của câu "sao nó không làm X?".
 */
export function rejectedOptions(d: Decision): Option[] {
  return d.options
    .filter((o) => o.action !== d.chosen)
    .sort((a, b) => b.score - a.score);
}

/**
 * Yếu tố quyết định nhất của một lựa chọn.
 *
 * Lấy theo **giá trị tuyệt đối**: một yếu tố cản trở mạnh cũng giải thích được
 * quyết định y như một yếu tố thúc đẩy mạnh. Lấy theo giá trị có dấu sẽ luôn trả
 * về lý do "vì sao nên làm" và không bao giờ trả về "vì sao suýt không làm".
 */
export function dominantFactor(o: Option): Factor | undefined {
  let best: Factor | undefined;
  for (const f of o.factors) {
    if (!best || Math.abs(f.weight) > Math.abs(best.weight)) best = f;
    // Hòa thì lấy cái đứng trước: thứ tự đầu vào do server quyết, và nó ổn định.
  }
  return best;
}

/**
 * Dựng câu trả lời cho "vì sao?" — **từ dữ liệu**.
 *
 * Mọi mảnh chữ ở đây đến từ `factors`. Không có lời gọi model, không có mẫu câu
 * nào chứa thông tin mà `factors` không nói. Nếu một ngày ai đó muốn câu văn hay
 * hơn, chỗ để sửa là cách *ráp chữ*, không phải nguồn *sự thật*.
 */
export function explain(d: Decision): string {
  const o = chosenOption(d);
  if (!o) return `Đã chọn ${d.chosen}.`;

  const thuc_day = o.factors
    .filter((f) => f.weight > 0)
    .sort((a, b) => b.weight - a.weight);
  const can_tro = o.factors
    .filter((f) => f.weight < 0)
    .sort((a, b) => a.weight - b.weight);

  const phan: string[] = [`Đã chọn ${d.chosen}`];
  if (thuc_day.length > 0) {
    phan.push(`vì ${thuc_day.map((f) => f.label).join(", ")}`);
  }
  if (can_tro.length > 0) {
    phan.push(`dù ${can_tro.map((f) => f.label).join(", ")}`);
  }

  const gan_nhat = rejectedOptions(d)[0];
  if (gan_nhat !== undefined) {
    phan.push(`thay vì ${gan_nhat.action} (${o.score - gan_nhat.score} điểm)`);
  }

  if (d.source !== "model") {
    // Nói ra, luôn luôn. Một quyết định do policy trong lúc provider hỏng trông
    // y hệt một quyết định bình thường nếu không nói.
    phan.push(`[${d.source}]`);
  }
  return `${phan.join(" ")}.`;
}

/**
 * Panel này có mở được cho thực thể `target` không.
 *
 * Giao diện gọi hàm này **trước khi** dựng nút bấm. Không phải vì server sẽ cho
 * qua — server sẽ từ chối — mà vì một nút bấm luôn báo lỗi là một nút bấm dạy
 * người chơi rằng phần mềm hỏng.
 */
export function canOpenFor(view: Pick<MindView, "lens" | "entity">, target: string): boolean {
  return view.lens === "embodied" ? view.entity === target : true;
}

/**
 * Kiểm rằng mọi yếu tố trỏ về một nguồn có thật trong chính view này.
 *
 * Cùng tinh thần với validator ở `§22.4`: một yếu tố trỏ ra ngoài là một yếu tố
 * người chơi không kiểm chứng được, và một lời giải thích không kiểm chứng được
 * thì không hơn gì một câu do model bịa.
 */
export function danglingEvidence(v: MindView): string[] {
  const hop_le = new Set<string>([
    ...v.observations.map((o) => o.id),
    ...v.memories.map((m) => m.id),
  ]);
  const treo: string[] = [];
  for (const o of v.decision?.options ?? []) {
    for (const f of o.factors) {
      if (f.evidence !== null && !hop_le.has(f.evidence)) treo.push(f.evidence);
    }
  }
  return treo;
}

/**
 * Ký ức, xếp theo thứ tự panel hiển thị.
 *
 * Liên quan nhất trước — đó là thứ tự mà bộ truy xuất đã dùng để chọn, nên hiện
 * theo thứ tự khác sẽ làm người chơi hiểu sai vì sao những ký ức này có mặt.
 * Phá hòa bằng `id` để hai lần mở panel cho cùng một thứ tự.
 */
export function orderedMemories(v: MindView): MindMemory[] {
  return [...v.memories].sort(
    (a, b) => b.relevance - a.relevance || a.id.localeCompare(b.id),
  );
}

/** Nhãn chắc chắn cho một ký ức — thấy tận mắt hay nghe kể. */
export function memoryCertainty(m: MindMemory): Certainty {
  return m.firsthand ? "truth" : "belief";
}
