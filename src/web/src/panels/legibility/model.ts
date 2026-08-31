/**
 * Đọc được mà không cần đọc bảng số (`idea.md §18.13`, `PF-16`).
 *
 * `§25` xếp *"siêu thực tế thành khó hiểu"* là một rủi ro thật, và bốn nguyên
 * tắc ở `§18.13` là thứ chống lại nó. Module này cài cả bốn, và mỗi cái là một
 * hàm chứ không phải một lời khuyên trong tài liệu:
 *
 * | Nguyên tắc | Hàm |
 * |---|---|
 * | 1. Triệu chứng trước, con số sau | [`symptoms`] |
 * | 2. Mọi con số bấm được về nguồn | [`traceDerived`] |
 * | 3. "Vì sao?" ở khắp nơi | [`why`] |
 * | 4. Không đổ tường số | [`relevantFields`] |
 *
 * ## Nguyên tắc 1 là nguyên tắc dễ làm ngược nhất
 *
 * > Mặc định hiển thị trạng thái bằng **ngôn ngữ người**: *"gầy trơ xương, ho
 * > ra máu, đi khập khiễng"* — không phải `hunger: 0.12, effect.grey_lung:
 * > 340`. Số đầy đủ **luôn có, sau một cú bấm**.
 *
 * Hai vế, và bỏ vế nào cũng hỏng. Chỉ có triệu chứng thì người chơi không gỡ
 * được lỗi và không so sánh được hai nhân vật; chỉ có số thì không ai đọc.
 * Nên [`Reading`] mang **cả hai**, và số nằm ở một trường riêng chứ không phải
 * ở một chế độ riêng — một chế độ là thứ người ta bật rồi quên tắt.
 *
 * ## Nguyên tắc 3: câu trả lời dựng từ dữ liệu, không từ model
 *
 * > Mọi quyết định của NPC, mọi thay đổi giá, mọi bản án đều có affordance hỏi
 * > lý do, và câu trả lời **dựng từ dữ liệu chứ không từ model**.
 *
 * Nên [`why`] nhận một chuỗi [`Factor`] đã tính sẵn và chỉ sắp xếp chúng. Nó
 * không nhận một hàm sinh văn bản, và không có đường nào để một câu không có
 * dữ liệu đằng sau lọt vào — cùng bất biến với `§22.17` ở Historian.
 */

/** Một triệu chứng, viết bằng ngôn ngữ người. */
export interface Symptom {
  /** Câu mô tả: `"gầy trơ xương"`. */
  text: string;
  /** Mức nặng `0`–`1000`, để sắp thứ tự. */
  severity: number;
  /** Con số đằng sau — **luôn có**, hiện sau một cú bấm. */
  source: { field: string; value: number };
}

/** Một trạng thái thô của thực thể. */
export interface RawState {
  /** Đói `0`–`1000`; thấp là đói. */
  hunger: number;
  /** Mệt `0`–`1000`; cao là mệt. */
  fatigue: number;
  /** Hiệu ứng đang mang: id → cường độ. */
  effects: Record<string, number>;
  /** Bộ phận đang hỏng: tên → mức hỏng `0`–`1000`. */
  injuries: Record<string, number>;
}

/** Bảng dịch hiệu ứng sang triệu chứng — dữ liệu, không phải văn model sinh. */
export const EFFECT_SYMPTOMS: Record<string, string> = {
  "effect.grey_lung": "ho ra máu",
  "effect.fever": "sốt cao",
  "effect.frostbite": "tê cóng đầu ngón",
  "effect.poison": "nôn và vã mồ hôi",
};

/** Bảng dịch thương tích sang triệu chứng. */
export const INJURY_SYMPTOMS: Record<string, string> = {
  leg: "đi khập khiễng",
  arm: "không nhấc nổi tay",
  eye: "nhìn một bên",
  hand: "cầm nắm khó",
};

/**
 * Số triệu chứng hiện mặc định.
 *
 * Ba. Nguyên tắc 4 nói *"không đổ tường số"*, và một danh sách triệu chứng dài
 * cũng là một bức tường — chỉ là bằng chữ. Ba câu là thứ đọc được bằng liếc
 * mắt; cái thứ tư trở đi nằm sau một cú bấm.
 */
export const MAX_SYMPTOMS = 3;

/** Một cách đọc trạng thái: triệu chứng trước, số sau. */
export interface Reading {
  /** Những gì hiện mặc định, nặng nhất trước. */
  symptoms: Symptom[];
  /** Còn bao nhiêu triệu chứng nữa sau một cú bấm. */
  moreCount: number;
  /** **Số đầy đủ, luôn có.** Không phải một chế độ. */
  numbers: Record<string, number>;
}

/**
 * Dịch trạng thái thô thành cách đọc (`§18.13` nguyên tắc 1 và 4).
 *
 * Trả về **cả hai** — triệu chứng và số. Trả về một cái rồi để chỗ gọi tự lấy
 * cái kia là cách mà vế thứ hai bị quên.
 */
export function symptoms(s: RawState): Reading {
  const tat_ca: Symptom[] = [];

  if (s.hunger < 450) {
    tat_ca.push({
      text: s.hunger < 200 ? "gầy trơ xương" : "hốc hác",
      severity: 1000 - s.hunger,
      source: { field: "hunger", value: s.hunger },
    });
  }
  if (s.fatigue > 600) {
    tat_ca.push({
      text: s.fatigue > 850 ? "kiệt sức" : "mệt rã rời",
      severity: s.fatigue,
      source: { field: "fatigue", value: s.fatigue },
    });
  }
  for (const [id, muc] of Object.entries(s.effects)) {
    const chu = EFFECT_SYMPTOMS[id];
    if (chu === undefined) continue;
    tat_ca.push({ text: chu, severity: muc, source: { field: id, value: muc } });
  }
  for (const [bo_phan, muc] of Object.entries(s.injuries)) {
    const chu = INJURY_SYMPTOMS[bo_phan];
    if (chu === undefined) continue;
    tat_ca.push({
      text: chu,
      severity: muc,
      source: { field: `injury.${bo_phan}`, value: muc },
    });
  }

  // Nặng nhất trước; hòa thì theo tên để thứ tự ổn định giữa hai lần vẽ.
  tat_ca.sort((a, b) => b.severity - a.severity || a.text.localeCompare(b.text));

  const numbers: Record<string, number> = {
    hunger: s.hunger,
    fatigue: s.fatigue,
  };
  for (const [k, v] of Object.entries(s.effects)) numbers[k] = v;
  for (const [k, v] of Object.entries(s.injuries)) numbers[`injury.${k}`] = v;

  return {
    symptoms: tat_ca.slice(0, MAX_SYMPTOMS),
    moreCount: Math.max(0, tat_ca.length - MAX_SYMPTOMS),
    numbers,
  };
}

/** Một mắt trong chuỗi suy ra của một giá trị. */
export interface DerivationStep {
  /** Thuộc tính hoặc điều kiện: `"wing.left.broken"`. */
  field: string;
  /** Giá trị của nó. */
  value: string;
  /** Nó đóng góp gì vào kết quả. */
  contribution: string;
}

/**
 * Truy một giá trị suy ra về nguồn (`§18.13` nguyên tắc 2, `§9.2`).
 *
 * > Người chơi thấy `can_fly: false` thì phải xem được là **vì cánh gãy hay vì
 * > quá tải**.
 *
 * Trả về danh sách rỗng nghĩa là *"không có gì làm nó thành thế"* — và với một
 * giá trị suy ra thì đó là một **lỗi dữ liệu**, không phải một câu trả lời.
 * [`isTraceable`] phân biệt hai chuyện đó.
 */
export function traceDerived(
  steps: readonly DerivationStep[],
): DerivationStep[] {
  // Giữ nguyên thứ tự tính: người đọc cần thấy chuỗi theo đúng chiều nó chạy,
  // không theo mức đóng góp.
  return [...steps];
}

/** Một giá trị suy ra có truy được về nguồn không. */
export function isTraceable(steps: readonly DerivationStep[]): boolean {
  return steps.length > 0;
}

/** Một yếu tố dẫn tới một quyết định. */
export interface Factor {
  /** Dữ liệu nào: `"belief.wage_there"`, `"norm.theft.severity"`. */
  field: string;
  /** Giá trị. */
  value: string;
  /** Trọng số đóng góp, có dấu. */
  weight: number;
  /** Event nào chứng minh — **bắt buộc**, `§18.13` nguyên tắc 3. */
  eventSeq: number;
}

/** Câu trả lời cho "vì sao?". */
export interface Explanation {
  /** Yếu tố, ảnh hưởng lớn nhất trước. */
  factors: Factor[];
  /** Có yếu tố nào không trỏ về event thật không. */
  fabricated: boolean;
}

/**
 * Affordance "vì sao?" (`§18.13` nguyên tắc 3).
 *
 * Sắp theo **độ lớn** của trọng số, không theo dấu: một yếu tố kéo mạnh về
 * phía "không" cũng quan trọng ngang một yếu tố kéo mạnh về phía "có", và sắp
 * theo giá trị có dấu sẽ đẩy hết phần "vì sao không" xuống cuối.
 *
 * `fabricated` bật khi có yếu tố không trỏ về event — chỗ gọi phải **không**
 * hiện lời giải thích đó. Cùng bất biến với `§22.17`: một câu không có dữ liệu
 * đằng sau thì không được hiện, kể cả khi nó nghe hợp lý.
 */
export function why(factors: readonly Factor[]): Explanation {
  const sap = [...factors].sort(
    (a, b) => Math.abs(b.weight) - Math.abs(a.weight) || a.field.localeCompare(b.field),
  );
  return {
    factors: sap,
    fabricated: sap.some((f) => f.eventSeq <= 0),
  };
}

/**
 * Chọn trường nào hiện mặc định (`§18.13` nguyên tắc 4).
 *
 * > Một entity có **hàng trăm trường**; hiện hết cùng lúc là cách chắc chắn
 * > nhất khiến không ai đọc gì.
 *
 * `context` là việc người chơi đang làm. Trường không liên quan **vẫn có**, ở
 * `behindTabs` — ẩn khác với xóa.
 */
export function relevantFields(
  all: readonly string[],
  context: string,
  relevance: Record<string, readonly string[]>,
): { shown: string[]; behindTabs: string[] } {
  const lien_quan = new Set(relevance[context] ?? []);
  const shown = all.filter((f) => lien_quan.has(f));
  const behindTabs = all.filter((f) => !lien_quan.has(f));
  return { shown, behindTabs };
}
