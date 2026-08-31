/**
 * Toán màu cho bộ kiểm bảng màu (`§18.6.2`, `§18.6.3`, `PA-13`).
 *
 * Mọi hàm ở đây là hàm thuần trên `number` — đây là **tầng hiển thị**, không
 * phải đường commit, nên số thực hoàn toàn hợp lệ ở đây. Ranh giới đó đáng nói
 * ra: `plan.md §P10.2` cấm số thực trong mô phỏng, không cấm chúng khi tính
 * xem hai màu có phân biệt được không.
 */

/** Màu RGB, mỗi kênh trong `[0, 255]`. */
export interface Rgb {
  r: number;
  g: number;
  b: number;
}

/** Màu trong không gian CIE L*a*b*. */
export interface Lab {
  L: number;
  a: number;
  b: number;
}

/** Đọc một màu hex `#rrggbb`. */
export function parseHex(hex: string): Rgb {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) throw new Error(`màu không hợp lệ: ${hex} (cần dạng #rrggbb)`);
  const v = Number.parseInt(m[1]!, 16);
  return { r: (v >> 16) & 0xff, g: (v >> 8) & 0xff, b: v & 0xff };
}

/** Ghi một màu thành hex. */
export function toHex({ r, g, b }: Rgb): string {
  const c = (n: number) =>
    Math.round(Math.min(255, Math.max(0, n)))
      .toString(16)
      .padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}

/** sRGB → tuyến tính. */
function toLinear(c: number): number {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

/** Tuyến tính → sRGB. */
function fromLinear(c: number): number {
  const s = c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055;
  return s * 255;
}

/** RGB → XYZ, chiếu sáng D65. */
function toXyz({ r, g, b }: Rgb): [number, number, number] {
  const R = toLinear(r);
  const G = toLinear(g);
  const B = toLinear(b);
  return [
    R * 0.4124 + G * 0.3576 + B * 0.1805,
    R * 0.2126 + G * 0.7152 + B * 0.0722,
    R * 0.0193 + G * 0.1192 + B * 0.9505,
  ];
}

/** RGB → Lab. */
export function toLab(c: Rgb): Lab {
  const [X, Y, Z] = toXyz(c);
  // Điểm trắng D65.
  const xn = 0.95047;
  const yn = 1.0;
  const zn = 1.08883;
  const f = (t: number) => (t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116);
  const fx = f(X / xn);
  const fy = f(Y / yn);
  const fz = f(Z / zn);
  return { L: 116 * fy - 16, a: 500 * (fx - fy), b: 200 * (fy - fz) };
}

/**
 * Khoảng cách màu **ΔE₀₀ (CIEDE2000)**.
 *
 * Phải là CIEDE2000, không phải CIE76, và lý do là một con số cụ thể. Ngưỡng 15
 * ở [`THRESHOLDS.identityAllPairs`] bắt nguồn từ phép đo:
 *
 * ```text
 * #eda100 ↔ #eb6834    ΔE₀₀  13.7   ← dưới ngưỡng, đúng như luật nói
 *                      ΔE₇₆  39.6   ← trên ngưỡng, luật mất hiệu lực
 * ```
 *
 * Hai cam khác nhau lệch nhau chủ yếu ở **sắc độ**, và CIE76 phóng đại chênh
 * lệch sắc độ tới gần ba lần. Dùng CIE76 với cùng ngưỡng 15 nghĩa là ngưỡng đó
 * không còn cấm gì cả — bảng bốn màu sẽ qua, và luật "tối đa 3 màu định danh"
 * mất cơ sở.
 *
 * CIEDE2000 phức tạp hơn, nhưng nó vẫn là **hàm thuần**: cùng đầu vào cho cùng
 * kết quả. Nỗi lo về nhánh điều kiện gần góc 275° là có thật về mặt lý thuyết
 * nhưng không đủ để lật một quyết định 13.7-so-với-15.
 *
 * [`THRESHOLDS.identityAllPairs`]: ./validate.ts
 */
export function deltaE(a: Rgb, b: Rgb): number {
  const la = toLab(a);
  const lb = toLab(b);

  const kL = 1;
  const kC = 1;
  const kH = 1;
  const deg = Math.PI / 180;

  const C1 = Math.hypot(la.a, la.b);
  const C2 = Math.hypot(lb.a, lb.b);
  const Cbar = (C1 + C2) / 2;

  // Hiệu chỉnh trục a* để vùng xám không bị đánh giá sai sắc độ.
  const G = 0.5 * (1 - Math.sqrt(Cbar ** 7 / (Cbar ** 7 + 25 ** 7)));
  const a1p = (1 + G) * la.a;
  const a2p = (1 + G) * lb.a;

  const C1p = Math.hypot(a1p, la.b);
  const C2p = Math.hypot(a2p, lb.b);

  const h = (ap: number, bp: number) => {
    if (ap === 0 && bp === 0) return 0;
    const x = Math.atan2(bp, ap) / deg;
    return x < 0 ? x + 360 : x;
  };
  const h1p = h(a1p, la.b);
  const h2p = h(a2p, lb.b);

  const dLp = lb.L - la.L;
  const dCp = C2p - C1p;

  let dhp: number;
  if (C1p * C2p === 0) dhp = 0;
  else if (Math.abs(h2p - h1p) <= 180) dhp = h2p - h1p;
  else if (h2p - h1p > 180) dhp = h2p - h1p - 360;
  else dhp = h2p - h1p + 360;
  const dHp = 2 * Math.sqrt(C1p * C2p) * Math.sin((dhp * deg) / 2);

  const Lbarp = (la.L + lb.L) / 2;
  const Cbarp = (C1p + C2p) / 2;

  let hbarp: number;
  if (C1p * C2p === 0) hbarp = h1p + h2p;
  else if (Math.abs(h1p - h2p) <= 180) hbarp = (h1p + h2p) / 2;
  else if (h1p + h2p < 360) hbarp = (h1p + h2p + 360) / 2;
  else hbarp = (h1p + h2p - 360) / 2;

  const T =
    1 -
    0.17 * Math.cos((hbarp - 30) * deg) +
    0.24 * Math.cos(2 * hbarp * deg) +
    0.32 * Math.cos((3 * hbarp + 6) * deg) -
    0.2 * Math.cos((4 * hbarp - 63) * deg);

  const dTheta = 30 * Math.exp(-(((hbarp - 275) / 25) ** 2));
  const RC = 2 * Math.sqrt(Cbarp ** 7 / (Cbarp ** 7 + 25 ** 7));
  const SL = 1 + (0.015 * (Lbarp - 50) ** 2) / Math.sqrt(20 + (Lbarp - 50) ** 2);
  const SC = 1 + 0.045 * Cbarp;
  const SH = 1 + 0.015 * Cbarp * T;
  const RT = -Math.sin(2 * dTheta * deg) * RC;

  return Math.sqrt(
    (dLp / (kL * SL)) ** 2 +
      (dCp / (kC * SC)) ** 2 +
      (dHp / (kH * SH)) ** 2 +
      RT * (dCp / (kC * SC)) * (dHp / (kH * SH)),
  );
}

/** Độ sáng tương đối theo WCAG. */
export function relativeLuminance({ r, g, b }: Rgb): number {
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b);
}

/** Tỉ lệ tương phản theo WCAG, trong `[1, 21]`. */
export function contrastRatio(a: Rgb, b: Rgb): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

/** Ba dạng mù màu được mô phỏng. */
export type Cvd = "protanopia" | "deuteranopia" | "tritanopia";

/**
 * Mô phỏng mù màu bằng ma trận Machado–Oliveira–Fernandes (2009), mức nặng nhất.
 *
 * Mô phỏng ở mức nặng nhất chứ không phải mức trung bình: nếu bảng màu đọc
 * được ở mức nặng thì nó đọc được với mọi mức nhẹ hơn. Kiểm ở mức trung bình
 * sẽ để lọt đúng những người cần nó nhất.
 */
const CVD_MATRIX: Record<Cvd, number[]> = {
  protanopia: [0.152286, 1.052583, -0.204868, 0.114503, 0.786281, 0.099216, -0.003882, -0.048116, 1.051998],
  deuteranopia: [0.367322, 0.860646, -0.227968, 0.280085, 0.672501, 0.047413, -0.01182, 0.04294, 0.968881],
  tritanopia: [1.255528, -0.076749, -0.178779, -0.078411, 0.930809, 0.147602, 0.004733, 0.691367, 0.3039],
};

/** Mô phỏng một màu như người mù màu nhìn thấy. */
export function simulateCvd(c: Rgb, kind: Cvd): Rgb {
  const m = CVD_MATRIX[kind];
  const R = toLinear(c.r);
  const G = toLinear(c.g);
  const B = toLinear(c.b);
  return {
    r: fromLinear(m[0]! * R + m[1]! * G + m[2]! * B),
    g: fromLinear(m[3]! * R + m[4]! * G + m[5]! * B),
    b: fromLinear(m[6]! * R + m[7]! * G + m[8]! * B),
  };
}

/** Khoảng cách nhỏ nhất giữa hai màu qua **mọi** dạng mù màu, kể cả thị giác thường. */
export function minDeltaEAcrossVision(a: Rgb, b: Rgb): { value: number; vision: string } {
  let value = deltaE(a, b);
  let vision = "normal";
  for (const k of ["protanopia", "deuteranopia", "tritanopia"] as const) {
    const d = deltaE(simulateCvd(a, k), simulateCvd(b, k));
    if (d < value) {
      value = d;
      vision = k;
    }
  }
  return { value, vision };
}
