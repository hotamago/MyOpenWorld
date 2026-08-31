/**
 * Thẻ vật phẩm (`idea.md §18.15.3`, `§18.15.6`, `§18.15.7`, `PB-20`, `PD-21`).
 *
 * ## Ba quy tắc, và cả ba đều là quyết định thiết kế có thể sai
 *
 * **1. Chất lượng chế tác và tình trạng hiện **tách hẳn**.**
 * Chúng là hai thứ khác nhau (`§22.34`) và giao diện phải nói điều đó. Gộp
 * chúng thành một thanh "độ bền" sẽ khiến người chơi không hiểu vì sao mài lại
 * một thanh kiếm cùn của bậc thầy thì tốt hơn mài một thanh kiếm rẻ.
 *
 * **2. Giá là ước lượng của nhân vật, không phải giá trị của vật.**
 * `§22.35`: vật phẩm không lưu giá trị. Thẻ hiện *"Aren nghĩ nó đáng khoảng 12
 * đồng"*, không phải *"giá: 12"*. Khác biệt đó là toàn bộ nền của thương mại:
 * một người bán và một người mua định giá khác nhau, và chênh lệch đó là lợi
 * nhuận.
 *
 * **3. So sánh không rút về một điểm số.**
 * `§18.15.7` cấm điều đó. Một điểm số duy nhất giấu đi cái được và cái mất, và
 * nó biến một quyết định có ý nghĩa thành một phép so sánh số học. Thẻ so sánh
 * hiện **từng chiều**, và hiện rõ cái gì mất đi nếu đổi.
 */

/** Chất lượng chế tác — bất biến. */
export type CraftQuality = "crude" | "plain" | "fine" | "superior" | "masterwork";

/** Tình trạng một bộ phận. */
export interface PartCondition {
  part: string;
  /** `0`–`1`. */
  condition: number;
}

/** Một lần sửa chữa. */
export interface RepairRecord {
  by: string;
  atTick: string;
  part: string;
  restored: number;
}

/** Dữ liệu một thẻ vật phẩm. */
export interface ItemCardData {
  entity: string;
  def: string;
  displayName: string;
  /** **Bất biến.** Hiện riêng, không trộn với tình trạng. */
  quality: CraftQuality;
  craftedBy: string | null;
  /** Tình trạng theo bộ phận, không phải một con số. */
  conditions: PartCondition[];
  repairs: RepairRecord[];
  massMmu: number;
  volumeMl: number;
  /**
   * Ước lượng giá của **người đang xem**, không phải giá trị của vật.
   *
   * `null` nghĩa là chưa thẩm định — và thẻ phải nói ra điều đó chứ không đoán
   * bừa một con số (`§18.15.6`).
   */
  appraisedValue: number | null;
  /** Ai đã thẩm định. */
  appraisedBy: string | null;
  /** Các chiều so sánh được: sát thương, tầm, tốc độ. */
  dimensions: Record<string, number>;
}

/** Nhãn tiếng Việt của chất lượng. */
export const QUALITY_LABEL: Record<CraftQuality, string> = {
  crude: "vụng về",
  plain: "bình thường",
  fine: "khéo",
  superior: "tinh xảo",
  masterwork: "kiệt tác",
};

/** Hệ số nhân của chất lượng, khớp với `mow-items`. */
export const QUALITY_MULTIPLIER: Record<CraftQuality, number> = {
  crude: 70,
  plain: 100,
  fine: 120,
  superior: 145,
  masterwork: 180,
};

/** Một dòng trong thẻ. */
export interface CardRow {
  label: string;
  value: string;
  /** Nhóm để giao diện tách khối. */
  group: "identity" | "quality" | "condition" | "physical" | "value" | "history";
  /**
   * Bấm được về nguồn nào (`§18.13`).
   *
   * Cho phép `undefined` tường minh vì `exactOptionalPropertyTypes` phân biệt
   * "không có khóa" với "khóa mang `undefined`", và chỗ dựng dòng ở dưới dùng
   * `?? undefined` — một cách viết gọn và đúng nghĩa: *nguồn có tồn tại trên
   * thẻ này, chỉ là không biết là ai*.
   */
  source?: string | undefined;
}

/**
 * Bộ phận yếu nhất quyết định, không phải trung bình.
 *
 * Một cây rìu cán gãy thì không dùng được, dù lưỡi hoàn hảo. Trung bình sẽ nói
 * nó còn 50% và người chơi sẽ mang nó ra trận.
 */
export function worstCondition(conditions: PartCondition[]): number {
  if (conditions.length === 0) return 1;
  return Math.min(...conditions.map((c) => c.condition));
}

/** Hiệu quả thực tế, phần trăm. */
export function effectivenessPercent(card: ItemCardData): number {
  return Math.round(QUALITY_MULTIPLIER[card.quality] * worstCondition(card.conditions));
}

/** Dựng các dòng của thẻ. */
export function buildCard(card: ItemCardData): CardRow[] {
  const rows: CardRow[] = [
    { label: "Tên", value: card.displayName, group: "identity" },
  ];

  // ── Chất lượng chế tác: khối riêng, có người làm ra ────────────────────────
  rows.push({
    label: "Chế tác",
    value: QUALITY_LABEL[card.quality],
    group: "quality",
    source: card.craftedBy ?? undefined,
  });
  if (card.craftedBy) {
    rows.push({ label: "Người làm", value: card.craftedBy, group: "quality", source: card.craftedBy });
  }

  // ── Tình trạng: khối riêng, theo từng bộ phận ─────────────────────────────
  for (const c of [...card.conditions].sort((a, b) => a.part.localeCompare(b.part))) {
    rows.push({
      label: `Tình trạng · ${c.part}`,
      value: `${Math.round(c.condition * 100)}%`,
      group: "condition",
    });
  }

  rows.push(
    { label: "Khối lượng", value: `${card.massMmu} mMU`, group: "physical" },
    { label: "Thể tích", value: `${card.volumeMl} mL`, group: "physical" },
  );

  // ── Giá: ước lượng của ai đó, hoặc chưa biết ──────────────────────────────
  if (card.appraisedValue === null) {
    rows.push({
      label: "Giá",
      // Không đoán một con số. `§18.15.6`: chưa thẩm định thì nói là chưa thẩm định.
      value: "chưa thẩm định",
      group: "value",
    });
  } else {
    rows.push({
      label: "Giá",
      value: `${card.appraisedBy ?? "ai đó"} nghĩ khoảng ${card.appraisedValue}`,
      group: "value",
      source: card.appraisedBy ?? undefined,
    });
  }

  // ── Lịch sử sửa chữa: ai đã chạm vào món đồ này ───────────────────────────
  for (const r of card.repairs) {
    rows.push({
      label: `Sửa · ${r.part}`,
      value: `${r.by} tại t${r.atTick} (+${Math.round(r.restored * 100)}%)`,
      group: "history",
      source: r.by,
    });
  }

  return rows;
}

/** Một chiều trong bảng so sánh. */
export interface ComparisonRow {
  dimension: string;
  a: number;
  b: number;
  /** `+1` nếu `b` tốt hơn, `-1` nếu tệ hơn, `0` nếu bằng. */
  delta: number;
}

/**
 * So sánh hai vật phẩm **theo từng chiều** (`§18.15.7`, `PD-21`).
 *
 * > Bảng cạnh nhau theo từng chiều, **không rút về một điểm số duy nhất**;
 * > hiện rõ cái gì mất đi nếu đổi.
 *
 * Hàm này cố ý **không** trả về một con số tổng. Nếu có ai cần một con số tổng
 * thì họ đang muốn tránh phải quyết định, và tránh quyết định là thứ làm trò
 * chơi nhạt đi.
 */
export function compare(a: ItemCardData, b: ItemCardData): ComparisonRow[] {
  const chieu = new Set([...Object.keys(a.dimensions), ...Object.keys(b.dimensions)]);
  return [...chieu].sort().map((dimension) => {
    const va = a.dimensions[dimension] ?? 0;
    const vb = b.dimensions[dimension] ?? 0;
    return { dimension, a: va, b: vb, delta: Math.sign(vb - va) };
  });
}

/**
 * Những chiều mà đổi sang `b` sẽ **mất đi**.
 *
 * Đây là phần mà `§18.15.7` nhấn mạnh và là phần mà mọi giao diện so sánh hay
 * bỏ quên: cái được thì dễ thấy, cái mất thì phải chỉ ra.
 */
export function whatYouLose(a: ItemCardData, b: ItemCardData): ComparisonRow[] {
  return compare(a, b).filter((r) => r.delta < 0);
}
