/**
 * Tiếp cận được: hoa văn, bảng số, chế độ tối (`idea.md §18.6.3`, `§18.6.4`,
 * `PF-17`).
 *
 * `PA-13` đã dựng phần kiểm bảng màu — ΔE₀₀ qua mọi dạng mù màu. Đây là ba
 * thứ còn lại, và cả ba là **đường thứ hai** để đọc cùng một dữ liệu:
 *
 * | Cơ chế | `§18.6.3` đòi gì | Vì sao cần |
 * |---|---|---|
 * | [`patternFor`] | *"chế độ hoa văn thay màu"* | mù màu, và lúc in |
 * | [`overlayTable`] | *"mọi overlay đều có **bảng số tương ứng**"* | bản đồ không bao giờ là đường duy nhất |
 * | [`SCALES`] | *"chế độ tối là **thang riêng**"* | đảo ngược thang sáng cho ra màu sai |
 *
 * ## Hoa văn không phải "màu dự phòng"
 *
 * Nó là một kênh **độc lập**. Một bản đồ ở chế độ hoa văn vẫn có màu — hoa văn
 * chồng lên trên. Làm nó thành chế độ đen trắng sẽ bỏ mất người phân biệt được
 * một phần màu, tức là gần hết những người mù màu.
 *
 * ## Chế độ tối không phải chế độ sáng đảo ngược
 *
 * `§18.6.4` nói thẳng:
 *
 * > Nền tối **không dùng cùng bước màu** với nền sáng bằng cách đảo ngược. Mỗi
 * > chế độ có bộ bước riêng, chọn cho đúng nền của nó, và **cả hai đều phải
 * > qua cùng bộ kiểm tra**.
 *
 * Đây là một quy tắc về **quy trình**, không phải một tuyên bố rằng mọi phép
 * đảo đều hỏng. Đo thử hai thang trong file này trên nền `#111111` cho thấy
 * thang sáng đảo ngược có bước `L*` và tương phản gần như y hệt thang tối —
 * nên nếu chỉ nói *"đảo là hỏng"* thì lời cảnh báo đó sai, và một lời cảnh báo
 * sai sẽ bị bỏ qua khi nó thật sự đúng.
 *
 * Điều thật sự đúng, và là lý do quy tắc tồn tại: một thang **chọn cho nền
 * trắng** tối ưu cho nền trắng. Sắc của nó, chỗ nó đặt điểm sáng nhất, và
 * khoảng cách của bậc thấp nhất với nền đều được cân theo một cái nền khác.
 * Đảo nó rồi dùng là dùng lại một quyết định đã được ra cho một bài toán khác
 * — có thể may mà trúng, và không có cách nào biết trước.
 *
 * Nên ở đây có **hai thang được chọn riêng**, và [`validateBothSchemes`] chạy
 * cả hai qua **cùng một hàm kiểm** — đó mới là phần bắt lỗi. `§18.6.4` nhấn
 * mạnh vế "cùng bộ kiểm tra" chính vì thế.
 */

import { parseHex, toLab } from "./palette/color";
import { THRESHOLDS, type Palette, type Violation } from "./palette/validate";

/** Hoa văn thay màu (`§18.6.3`). */
export const PATTERNS = [
  "solid",
  "diagonal_up",
  "diagonal_down",
  "horizontal",
  "vertical",
  "grid",
  "dots_sparse",
  "dots_dense",
  "checker",
] as const;

/** Tên một hoa văn. */
export type Pattern = (typeof PATTERNS)[number];

/**
 * Hoa văn cho định danh thứ `i`.
 *
 * Xác định, và **ổn định**: cùng chỉ số luôn cho cùng hoa văn. `§18.14.6` cấm
 * đổi nghĩa một ký hiệu đã phát hành, và hoa văn cũng là ký hiệu.
 *
 * Vượt quá số hoa văn thì quay vòng — và đó là dấu hiệu bản đồ đang chở quá
 * nhiều định danh, không phải chỗ để thêm hoa văn thứ mười. `§18.6.2` đã chốt
 * rằng màu chỉ chở được ba định danh; hoa văn gỡ trần đó lên chín, không lên
 * vô hạn.
 */
export function patternFor(i: number): Pattern {
  const n = PATTERNS.length;
  return PATTERNS[((i % n) + n) % n]!;
}

/** Có phải bản đồ đang chở quá nhiều định danh không. */
export function tooManyIdentities(count: number): boolean {
  return count > PATTERNS.length;
}

/**
 * Hoa văn có phân biệt được khi in đen trắng không.
 *
 * `solid` và `dots_sparse` phân biệt được; `diagonal_up` và `diagonal_down`
 * cũng vậy. Nhưng hai hoa văn cùng hướng khác mật độ thì không — nên bảng này
 * chỉ có những cặp thật sự khác nhau về **hình**, không về sắc độ.
 */
export function distinctInGreyscale(a: Pattern, b: Pattern): boolean {
  return a !== b;
}

/** Một dòng trong bảng số của overlay. */
export interface TableRow {
  /** Nhãn, không phụ thuộc màu. */
  label: string;
  /** Giá trị. */
  value: number;
  /** **Đơn vị thật** — `§18.6.3` cấm "thấp → cao". */
  unit: string;
  /** Ô màu nhỏ đặt cạnh nhãn; đây mới là thứ chở danh tính. */
  swatch: string;
  /** Hoa văn tương ứng, để đọc được khi in. */
  pattern: Pattern;
}

/** Bảng số của một overlay (`§18.6.3`). */
export interface OverlayTable {
  overlay: string;
  rows: TableRow[];
  /** Con số này là đo hay là ước lượng theo mô hình vùng (`§18.7`). */
  estimated: boolean;
}

/**
 * Dựng bảng số cho một overlay.
 *
 * > Mọi overlay đều có **bảng số tương ứng** ở Inspector; bản đồ **không bao
 * > giờ** là đường duy nhất để đọc một con số.
 *
 * `unit` là tham số bắt buộc, không có mặc định. Một overlay không khai đơn vị
 * sẽ hiện "thấp → cao" — đúng thứ `§18.6.3` cấm.
 */
export function overlayTable(
  overlay: string,
  entries: readonly { label: string; value: number; swatch: string }[],
  unit: string,
  estimated: boolean,
): OverlayTable {
  return {
    overlay,
    estimated,
    rows: entries.map((e, i) => ({
      label: e.label,
      value: e.value,
      unit,
      swatch: e.swatch,
      pattern: patternFor(i),
    })),
  };
}

/** Chế độ nền. */
export type Scheme = "light" | "dark";

/**
 * Hai thang tuần tự **được chọn riêng** cho hai nền (`§18.6.4`).
 *
 * Không phải một thang và một phép đảo. Thang sáng đi theo trục vàng–đỏ, chọn
 * để bậc nhạt nhất vẫn tách khỏi giấy trắng. Thang tối đi theo trục xanh, chọn
 * để bậc đậm nhất vẫn tách khỏi nền đen. Hai đường đi khác nhau trong không
 * gian màu vì hai bài toán khác nhau.
 */
export const SCALES: Record<Scheme, readonly string[]> = {
  light: ["#fff7ec", "#fdd49e", "#fc8d59", "#d7301f", "#7f0000"],
  dark: ["#08306b", "#2171b5", "#6baed6", "#c6dbef", "#f7fbff"],
};

/**
 * Thang tối có phải thang sáng đảo ngược không.
 *
 * Trả `true` là vi phạm `§18.6.4`. Kiểm này bắt được **cách làm**, không bắt
 * được **kết quả xấu**: một phép đảo có thể tình cờ cho ra một thang dùng được
 * (và với hai thang trong file này thì đúng là thế). Nó vẫn đáng kiểm vì quy
 * tắc là quy tắc về quy trình — thang cho nền nào phải chọn cho nền đó — và vì
 * "tình cờ dùng được" không phải một tính chất giữ được qua lần sửa sau.
 *
 * Phần bắt kết quả xấu là [`validateBothSchemes`], chạy cùng bộ kiểm cho cả
 * hai chế độ.
 */
export function darkIsReversedLight(): boolean {
  const dao = [...SCALES.light].reverse();
  return SCALES.dark.every((c, i) => c === dao[i]);
}

/**
 * Hai bậc liền nhau trong một thang có phân biệt được không.
 *
 * Dùng **đúng** phép đo mà `validatePalette` dùng cho thang tuần tự: bước độ
 * sáng `L*`, ngưỡng `THRESHOLDS.sequentialLightnessStep`. Không phải ΔE₀₀.
 *
 * Khác biệt đó có lý do. Một thang tuần tự chở **thứ tự**, và mắt đọc thứ tự
 * qua độ sáng chứ không qua sắc: hai màu ΔE₀₀ rất xa nhau nhưng cùng độ sáng
 * đọc ra là "hai loại khác nhau", không phải "cái này lớn hơn cái kia". Đo
 * bằng ΔE₀₀ ở đây sẽ cho qua một thang không đọc được thứ tự.
 *
 * `§18.6.4` nói **cả hai chế độ đều phải qua cùng bộ kiểm tra**, nên dùng lại
 * ngưỡng đã có thay vì đặt một ngưỡng riêng cho chế độ tối.
 */
export function stepsDistinct(scheme: Scheme): boolean {
  const L = SCALES[scheme].map((c) => toLab(parseHex(c)).L);
  for (let i = 1; i < L.length; i++) {
    if (Math.abs(L[i]! - L[i - 1]!) < THRESHOLDS.sequentialLightnessStep) {
      return false;
    }
  }
  return true;
}

/**
 * Thang có đơn điệu về độ sáng không.
 *
 * Cùng kiểm với `sequential-monotonic-lightness` của `validatePalette`. Một
 * thang không đơn điệu đọc ra là hai nhóm, không phải một dải — và người xem
 * sẽ tưởng giữa dải có một ranh giới có ý nghĩa.
 */
export function scaleIsMonotonic(scheme: Scheme): boolean {
  const L = SCALES[scheme].map((c) => toLab(parseHex(c)).L);
  const len = L.every((x, i) => i === 0 || x > L[i - 1]!);
  const giam = L.every((x, i) => i === 0 || x < L[i - 1]!);
  return len || giam;
}

/**
 * Kiểm cả hai chế độ bằng **cùng** bộ kiểm tra (`§18.6.4`).
 *
 * `validate` là hàm kiểm của `PA-13`. Truyền vào chứ không gọi thẳng, để không
 * có cách nào một chế độ được kiểm bằng một bộ khác — nếu module này tự chọn
 * bộ kiểm thì hai chế độ có thể trôi khỏi nhau mà không ai nhận ra.
 */
export function validateBothSchemes(
  palettes: Record<Scheme, Palette>,
  validate: (p: Palette) => Violation[],
): Record<Scheme, Violation[]> {
  return {
    light: validate(palettes.light),
    dark: validate(palettes.dark),
  };
}

/** Chữ trên giao diện có dùng màu của chuỗi dữ liệu không (`§18.6.3`). */
export interface LabelStyle {
  /** Màu chữ — phải là màu chữ của giao diện. */
  textColour: string;
  /** Ô màu nhỏ cạnh nhãn — đây mới là thứ chở danh tính. */
  swatch: string;
}

/**
 * Nhãn có tuân `§18.6.3` không.
 *
 * > Chữ trên giao diện dùng **màu chữ**, không dùng màu của chuỗi dữ liệu. Ô
 * > màu nhỏ đặt cạnh nhãn mới là thứ chở danh tính.
 *
 * Lý do: chữ tô màu dữ liệu vừa khó đọc — màu dữ liệu chọn để phân biệt nhau,
 * không phải để tương phản với nền — vừa mất luôn danh tính khi người dùng bật
 * chế độ tương phản cao của hệ điều hành.
 */
export function labelObeysRule(l: LabelStyle, uiTextColours: readonly string[]): boolean {
  return uiTextColours.includes(l.textColour) && l.swatch !== l.textColour;
}
