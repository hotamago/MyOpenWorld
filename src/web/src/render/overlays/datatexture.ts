/**
 * Overlay là **data texture một kênh**, không phải sprite mỗi ô
 * (`idea.md §18.6`, `plan.md §P6.9.2`, `PA-12`).
 *
 * ## Vì sao không phải sprite mỗi ô
 *
 * Cách tự nhiên nhất để vẽ bản đồ nhiệt là tạo một sprite màu cho mỗi ô. Nó
 * hoạt động với một trăm ô và sụp đổ với một trăm nghìn: mỗi sprite là một
 * object, một lần cập nhật transform, và một khả năng phá vỡ batch.
 *
 * Cách ở đây: **một texture, một byte mỗi ô**, và thang màu được áp trong
 * shader. Đổi overlay là đổi một texture 64 KB, không phải tạo lại 100 000
 * object. Đổi thang màu không đụng vào dữ liệu chút nào.
 *
 * ## Vì sao đơn vị là bắt buộc
 *
 * `§18.6` đòi legend phải kèm **đơn vị thật**. Byte trong texture là `0`–`255`,
 * một thang tùy tiện không mang nghĩa gì. Nếu không đi kèm miền giá trị thật,
 * người xem sẽ tự bịa ra một thang trong đầu — và họ sẽ bịa sai. Một bản đồ
 * nhiệt "đỏ hơn" mà không nói đỏ nghĩa là bao nhiêu độ thì tệ hơn không có bản
 * đồ, vì nó tạo cảm giác đã hiểu.
 *
 * Nên [`OverlayChannel`] **không có** hàm dựng nào thiếu `unit` và `domain`.
 */

/** Một kênh overlay. */
export class OverlayChannel {
  readonly id: string;
  readonly unit: string;
  readonly domain: readonly [number, number];
  readonly width: number;
  readonly height: number;
  readonly data: Uint8Array;

  /**
   * Dựng một kênh.
   *
   * @throws nếu thiếu đơn vị, hoặc miền giá trị suy biến. Cả hai đều là lỗi
   * lập trình, và cả hai đều dẫn tới một legend nói dối.
   */
  constructor(opts: {
    id: string;
    unit: string;
    domain: [number, number];
    width: number;
    height: number;
    data?: Uint8Array;
  }) {
    if (!opts.unit) {
      throw new Error(
        `overlay \`${opts.id}\` thiếu đơn vị. §18.6 cấm overlay không có đơn vị ` +
          `thật — người xem sẽ tự bịa một thang trong đầu, và họ sẽ bịa sai.`,
      );
    }
    if (opts.domain[0] === opts.domain[1]) {
      throw new Error(
        `overlay \`${opts.id}\` có miền suy biến [${opts.domain[0]}, ${opts.domain[1]}] — ` +
          `mọi ô sẽ ánh xạ về cùng một màu và bản đồ trở nên vô nghĩa.`,
      );
    }
    this.id = opts.id;
    this.unit = opts.unit;
    this.domain = opts.domain;
    this.width = opts.width;
    this.height = opts.height;
    this.data = opts.data ?? new Uint8Array(opts.width * opts.height);
  }

  /** Đặt giá trị **thật** tại một ô; nó được lượng tử hóa vào byte. */
  set(x: number, y: number, value: number): void {
    const [lo, hi] = this.domain;
    const t = (value - lo) / (hi - lo);
    this.data[y * this.width + x] = Math.round(Math.min(1, Math.max(0, t)) * 255);
  }

  /** Đọc giá trị **thật** tại một ô, đã giải lượng tử. */
  get(x: number, y: number): number {
    const [lo, hi] = this.domain;
    const b = this.data[y * this.width + x] ?? 0;
    return lo + (b / 255) * (hi - lo);
  }

  /** Byte thô tại một ô — thứ đi vào texture. */
  raw(x: number, y: number): number {
    return this.data[y * this.width + x] ?? 0;
  }

  /** Số byte của kênh này. Một byte mỗi ô, không hơn. */
  get byteLength(): number {
    return this.data.byteLength;
  }
}

/** Một mục trong legend. */
export interface LegendStop {
  /** Giá trị thật. */
  value: number;
  /** Nhãn đã định dạng, kèm đơn vị. */
  label: string;
  /** Byte tương ứng, để đối chiếu với texture. */
  byte: number;
}

/**
 * Sinh legend cho một kênh (`§18.6.3`).
 *
 * Legend là **bắt buộc khi overlay bật**, không phải tùy chọn. Hàm này không
 * thể tạo ra một legend thiếu đơn vị, vì đơn vị nằm trong chính kênh.
 */
export function buildLegend(ch: OverlayChannel, stops = 5): LegendStop[] {
  const [lo, hi] = ch.domain;
  return Array.from({ length: stops }, (_, i) => {
    const t = i / (stops - 1);
    const value = lo + t * (hi - lo);
    return {
      value,
      label: `${formatValue(value)} ${ch.unit}`,
      byte: Math.round(t * 255),
    };
  });
}

/**
 * Định dạng một giá trị cho legend.
 *
 * Số chữ số theo độ lớn, không cố định. `0.00042` và `1200000` đều phải đọc
 * được, và một quy tắc "hai chữ số thập phân" sẽ biến cái đầu thành `0.00`.
 */
export function formatValue(v: number): string {
  const a = Math.abs(v);
  if (a === 0) return "0";
  if (a >= 1e6) return `${trim((v / 1e6).toFixed(1))}M`;
  if (a >= 1e3) return `${trim((v / 1e3).toFixed(1))}k`;
  if (a >= 10) return v.toFixed(0);
  if (a >= 1) return trim(v.toFixed(1));
  if (a >= 0.01) return trim(v.toFixed(3));
  return v.toExponential(1);
}

/**
 * Bỏ số 0 thừa ở đuôi.
 *
 * `0.500` và `0.5` là cùng một con số, nhưng cái đầu chiếm nhiều chỗ hơn trong
 * một legend vốn đã chật, và mắt phải đọc thêm hai ký tự không mang tin. Ở một
 * legend năm bậc thì đó là mười ký tự thừa.
 */
function trim(s: string): string {
  return s.includes(".") ? s.replace(/\.?0+$/, "") : s;
}

/**
 * Nhóm loại trừ của overlay (`§18.5`).
 *
 * Chỉ **một** overlay nền được bật cùng lúc. Chồng hai bản đồ nhiệt lên nhau
 * cho ra một màu thứ ba không có nghĩa gì, và người xem sẽ đọc nó như một dữ
 * liệu thật. Lớp này biến quy tắc đó thành thứ không thể vi phạm.
 */
export class OverlayGroup {
  #channels = new Map<string, OverlayChannel>();
  #active: string | null = null;

  /** Đăng ký một kênh. */
  register(ch: OverlayChannel): void {
    this.#channels.set(ch.id, ch);
  }

  /** Bật một overlay; overlay đang bật tự tắt. `null` để tắt hết. */
  activate(id: string | null): void {
    if (id !== null && !this.#channels.has(id)) {
      throw new Error(`không có overlay \`${id}\``);
    }
    this.#active = id;
  }

  /** Overlay đang bật. */
  get active(): OverlayChannel | null {
    return this.#active === null ? null : (this.#channels.get(this.#active) ?? null);
  }

  /** Legend của overlay đang bật, `null` nếu không có overlay nào bật. */
  legend(): LegendStop[] | null {
    const a = this.active;
    return a ? buildLegend(a) : null;
  }

  /** Mọi kênh đã đăng ký, theo thứ tự id. */
  ids(): string[] {
    return [...this.#channels.keys()].sort();
  }
}

/**
 * Mã shader lấy mẫu data texture và áp thang màu.
 *
 * Giữ ở đây, cạnh cấu trúc dữ liệu, để hai thứ không lệch nhau: nếu định dạng
 * texture đổi mà shader không đổi, bản đồ sẽ hiện màu sai chứ không báo lỗi.
 */
export const OVERLAY_FRAGMENT_SHADER = `#version 300 es
precision mediump float;

// Một kênh, một byte mỗi ô. KHÔNG phải RGBA — dùng RGBA sẽ tốn gấp bốn băng
// thông cho ba kênh không ai đọc.
uniform sampler2D uData;
// Thang màu, tra cứu 256 bậc. Đổi thang không đụng vào dữ liệu.
uniform sampler2D uRamp;
uniform float uOpacity;

in vec2 vUv;
out vec4 fragColor;

void main() {
  float v = texture(uData, vUv).r;
  vec3 c = texture(uRamp, vec2(v, 0.5)).rgb;
  fragColor = vec4(c, uOpacity);
}
`;
