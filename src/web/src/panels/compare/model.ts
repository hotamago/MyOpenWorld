/**
 * So sánh vật phẩm (`idea.md §18.15.7`, `PD-21`).
 *
 * > Khi so hai món, giao diện **không được rút về một điểm số duy nhất** — điều
 * > đó mâu thuẫn trực tiếp với `§2.1` và làm mất mọi đánh đổi.
 *
 * ## Vì sao một điểm số là sai, không chỉ là kém
 *
 * Với một con số, câu hỏi *"cái nào tốt hơn"* luôn có đáp án, và người chơi học
 * được rằng chơi giỏi nghĩa là chọn số lớn hơn. Nhưng:
 *
 * > Đổi giáp nhẹ lấy giáp nặng là **đánh đổi tốc độ lấy che phủ**, không phải
 * > một con số lớn hơn.
 *
 * Không có bối cảnh thì không có "tốt hơn". Một bộ giáp nặng tốt hơn trong trận
 * tuyến và tệ hơn khi phải chạy — và cùng một người chơi cần cả hai câu trả lời
 * ở hai lúc khác nhau.
 *
 * Nên module này **không có** hàm nào trả về một con số tổng, và
 * [`compare`] trả về [`Comparison`] với `winner: null` ở mọi chiều mà hai món
 * ngang nhau. Không có `overallWinner`.
 *
 * ## Cái đáng giá nhất là cột "mất gì"
 *
 * > ... và **cái gì sẽ mất đi nếu đổi**.
 *
 * Đó là thứ một bảng số không nói: người chơi thấy giáp mới hơn ở bốn chiều và
 * không nhận ra nó bỏ mất khả năng bơi. [`losses`] tồn tại để câu hỏi đó luôn
 * được trả lời trước khi đổi, không phải sau.
 */

/** Một chiều so sánh. Không có "điểm tổng". */
export type Axis =
  | "weight"
  | "coverage"
  | "mobility"
  | "durability"
  | "noise"
  | "warmth"
  | "value";

/** Mọi chiều, để lặp. */
export const AXES: Axis[] = [
  "weight",
  "coverage",
  "mobility",
  "durability",
  "noise",
  "warmth",
  "value",
];

/**
 * Chiều này **cao hơn là tốt hơn**, hay ngược lại.
 *
 * Cần bảng này vì không phải chiều nào cũng cùng hướng: che phủ cao là tốt,
 * tiếng ồn cao là xấu. Không có nó thì "cái nào hơn" tính sai ở đúng những chiều
 * mà người chơi ít để ý nhất.
 */
export const HIGHER_IS_BETTER: Record<Axis, boolean> = {
  weight: false,
  coverage: true,
  mobility: true,
  durability: true,
  noise: false,
  warmth: true,
  value: true,
};

/** Nhãn đọc được. */
export const AXIS_LABEL: Record<Axis, string> = {
  weight: "khối lượng",
  coverage: "che phủ",
  mobility: "cơ động",
  durability: "độ bền",
  noise: "tiếng động",
  warmth: "giữ ấm",
  value: "giá trị",
};

/** Tình trạng của một bộ phận (`§18.15`). */
export interface PartCondition {
  part: string;
  /** `0`–`1000`. */
  condition: number;
}

/** Một món đồ, ở dạng đủ để so. */
export interface Item {
  id: string;
  displayName: string;
  /** Giá trị theo từng chiều. */
  axes: Partial<Record<Axis, number>>;
  /** Chức năng nó cho phép: `swim`, `climb`, `sneak`. */
  affordances: string[];
  /** Effect nó mang. */
  effects: string[];
  /** Tình trạng theo bộ phận. */
  parts: PartCondition[];
  /** Cổng sử dụng: cần gì mới dùng được. */
  requirements: string[];
}

/** Kết quả so một chiều. */
export interface AxisComparison {
  axis: Axis;
  label: string;
  a: number | null;
  b: number | null;
  /**
   * Bên nào hơn ở **chiều này**. `null` khi ngang nhau hoặc không so được.
   *
   * Không có trường tương ứng cho "tổng thể" — xem docstring của module.
   */
  winner: "a" | "b" | null;
}

/** Toàn bộ so sánh. */
export interface Comparison {
  a: Item;
  b: Item;
  axes: AxisComparison[];
  /** Đổi từ `a` sang `b` thì **mất** gì. */
  lostBySwitching: string[];
  /** Và **được** gì. */
  gainedBySwitching: string[];
}

/** So một chiều. */
function compareAxis(axis: Axis, a: Item, b: Item): AxisComparison {
  const va = a.axes[axis] ?? null;
  const vb = b.axes[axis] ?? null;

  let winner: "a" | "b" | null = null;
  if (va !== null && vb !== null && va !== vb) {
    const aHon = HIGHER_IS_BETTER[axis] ? va > vb : va < vb;
    winner = aHon ? "a" : "b";
  }
  return { axis, label: AXIS_LABEL[axis], a: va, b: vb, winner };
}

/**
 * Những gì **mất đi** khi đổi từ `a` sang `b`.
 *
 * Ba nguồn mất mát, và cả ba đều vô hình trên một bảng số:
 *
 * - **chức năng** biến mất — bỏ mất khả năng bơi;
 * - **effect** biến mất — mất kháng lạnh;
 * - **cổng sử dụng mới** — món mới đòi thứ mình chưa có, nên chưa dùng được.
 */
export function losses(a: Item, b: Item): string[] {
  const mat: string[] = [];
  for (const f of a.affordances) {
    if (!b.affordances.includes(f)) mat.push(`mất khả năng: ${f}`);
  }
  for (const e of a.effects) {
    if (!b.effects.includes(e)) mat.push(`mất hiệu ứng: ${e}`);
  }
  for (const r of b.requirements) {
    if (!a.requirements.includes(r)) mat.push(`đòi thêm điều kiện: ${r}`);
  }
  return mat;
}

/** Những gì **được thêm** khi đổi. */
export function gains(a: Item, b: Item): string[] {
  const duoc: string[] = [];
  for (const f of b.affordances) {
    if (!a.affordances.includes(f)) duoc.push(`thêm khả năng: ${f}`);
  }
  for (const e of b.effects) {
    if (!a.effects.includes(e)) duoc.push(`thêm hiệu ứng: ${e}`);
  }
  return duoc;
}

/**
 * So hai món.
 *
 * **Không trả về "cái nào tốt hơn".** Nó trả về một bảng, và người chơi quyết
 * định — vì chỉ họ mới biết lát nữa mình sẽ đứng trong trận tuyến hay phải bơi
 * qua sông.
 */
export function compare(a: Item, b: Item): Comparison {
  return {
    a,
    b,
    axes: AXES.filter((x) => a.axes[x] !== undefined || b.axes[x] !== undefined).map(
      (x) => compareAxis(x, a, b),
    ),
    lostBySwitching: losses(a, b),
    gainedBySwitching: gains(a, b),
  };
}

/**
 * Đổi này có **đánh đổi thật** không — tức là được cái này mất cái kia.
 *
 * Nếu `false` thì một bên hơn hẳn ở mọi chiều, và lúc đó chọn là dễ. Hàm này để
 * giao diện biết khi nào cần nhấn mạnh cột "mất gì" — chính là khi có đánh đổi.
 */
export function isTradeoff(c: Comparison): boolean {
  const coA = c.axes.some((x) => x.winner === "a");
  const coB = c.axes.some((x) => x.winner === "b");
  return (coA && coB) || c.lostBySwitching.length > 0;
}

/**
 * Bộ phận yếu nhất quyết định, không phải trung bình.
 *
 * Cùng quy tắc với thẻ vật phẩm ở `PB-24`: một cây rìu cán gãy thì không dùng
 * được, dù lưỡi hoàn hảo. Lặp lại ở đây thay vì gọi chéo module để hai panel
 * không bị buộc vào nhau — nhưng quy tắc thì phải giống, nên có test giữ.
 */
export function worstPart(item: Item): PartCondition | undefined {
  let te = item.parts[0];
  for (const p of item.parts) {
    if (te === undefined || p.condition < te.condition) te = p;
  }
  return te;
}
