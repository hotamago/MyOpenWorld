/**
 * Trường vô hướng phủ lên bản đồ (`idea.md §18.5`, `PG-07`).
 *
 * ## Vì sao chỉ một cách nhìn là không đủ
 *
 * Người chơi là một vị thần quan sát, và `terrain.ts` chỉ trả lời được một
 * câu: "ô này là chất gì". Nó không trả lời được "chỗ nào cao", "chỗ nào ướt",
 * "chỗ nào đi được", "chỗ nào đông người" — bốn câu đó cần bốn lớp riêng, bật
 * tắt được, không trộn vào màu vật liệu. Trộn vào sẽ vi phạm đúng ràng buộc
 * `§18.5` đã đặt ra cho `terrain.ts`: màu nền ô chỉ chở vật liệu.
 *
 * ## Vì sao module này không đụng DOM hay Pixi
 *
 * Cùng lý do `ambient.ts` không đụng: một lỗi ở đây phải là lỗi *tính toán*,
 * kiểm được bằng `vitest` thuần trong Node, không phải lỗi *đồ họa* cần mở
 * trình duyệt mới thấy. Phần phủ lên canvas là việc của `world.ts`.
 *
 * ## Vì sao `NaN` chứ không phải `0` cho "không có dữ liệu"
 *
 * `0` là một giá trị hợp lệ trong hầu hết các trường ở đây — sông ngay dưới
 * chân là `drop = 0`, đồng bằng thấp nhất trong lô cũng chuẩn hóa về `0`. Nếu
 * "khô" hay "không ai" cũng là `0`, `paintField` sẽ tô cả hai bằng đúng một
 * màu: màu "thấp nhất" của thang. Kết quả là một overlay phủ kín toàn bản đồ
 * kể cả nơi không có gì để nói, che mất địa hình bên dưới. `NaN` không nằm
 * trong `[0, 1]` của bất cứ thang màu nào — nó chỉ có thể có nghĩa là "trong
 * suốt", và `paintField` xử lý đúng vậy: alpha 0.
 */

import type { Entity, TileBatch } from "@/api/game";
import { SCALES, type Scheme } from "../accessibility";
import { parseHex, type Rgb } from "../palette/color";

/** Bốn lớp dữ liệu của lát cắt này. Thêm lớp mới thì thêm vào đây trước. */
export const LAYERS = ["elevation", "water", "walkable", "crowd"] as const;

/** Định danh một lớp. */
export type LayerId = (typeof LAYERS)[number];

/** Một trường vô hướng trên đúng khung của lô ô. */
export interface Field {
  w: number;
  h: number;
  /** Giá trị đã chuẩn hóa về `[0, 1]`; `NaN` nghĩa là "không có dữ liệu". */
  v: Float32Array;
  /** Đơn vị thật để hiện trong chú giải, ví dụ `"m"`, `"người"`, `""`. */
  unit: string;
  /** Khoảng giá trị thật trước khi chuẩn hóa. */
  min: number;
  max: number;
}

/**
 * Chuẩn hóa một mảng giá trị thật (có thể chứa `NaN`) về `[0, 1]`, theo
 * min/max **thực tế của chính mảng đó**, không theo hằng số cố định.
 *
 * Một lô toàn đồng bằng phải vẫn đọc được chênh lệch centimét — chuẩn hóa theo
 * một thang cố định (ví dụ "0–4000 m" cho cao độ cả thế giới) sẽ dồn cả lô về
 * cùng một màu và bản đồ trông chết cứng.
 *
 * `max === min` (mọi ô hợp lệ bằng nhau hệt nhau, kể cả khi chỉ có đúng một ô
 * hợp lệ) là phép chia cho 0 kinh điển. Ở đây nó **không** được phép biến cả
 * trường thành `NaN` — một lô bằng phẳng vẫn là dữ liệu thật, chỉ là không có
 * chênh lệch để tô đậm nhạt, nên nó nhận một màu **hằng** ở giữa thang (`0.5`).
 * Đây đúng là loại lỗi im lặng đã cắn dự án này nhiều lần: quên nhánh này thì
 * mọi lô bằng phẳng đều biến mất khỏi overlay mà không ném lỗi nào cả.
 */
function normalize(raw: Float32Array): { v: Float32Array; min: number; max: number } {
  let min = Infinity;
  let max = -Infinity;
  for (const x of raw) {
    if (Number.isNaN(x)) continue;
    if (x < min) min = x;
    if (x > max) max = x;
  }
  // Không ô nào hợp lệ: quy min/max về 0 để chú giải không hiện "Infinity".
  if (!Number.isFinite(min)) {
    min = 0;
    max = 0;
  }
  const span = max - min;
  const v = new Float32Array(raw.length);
  for (let i = 0; i < raw.length; i++) {
    const x = raw[i]!;
    v[i] = Number.isNaN(x) ? NaN : span === 0 ? 0.5 : (x - min) / span;
  }
  return { v, min, max };
}

/**
 * Giá trị nước: chỉ ô có sông (`river`) mới có dữ liệu; ô khô là `NaN`, không
 * phải `0` — lý do chung ở đầu file.
 *
 * Giá trị thật là `drop`: sông ngay tại lát đang đứng (`drop = 0`) đọc rõ nhất,
 * sông nhìn xuyên qua không khí từ trên cao (`drop` lớn) đọc mờ dần. Đây là
 * đúng cách `terrain.ts` đã làm mờ các ô không rắn theo khoảng cách tới mặt
 * đất — dùng lại quy ước đó thay vì bịa một thang riêng cho overlay.
 */
function computeWaterRaw(batch: TileBatch): Float32Array {
  const n = batch.w * batch.h;
  const raw = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    raw[i] = (batch.river[i] ?? 0) === 1 ? (batch.drop[i] ?? 0) : NaN;
  }
  return raw;
}

/**
 * Giá trị đi lại được: nhị phân, `1` đi được, `0` không.
 *
 * `TileBatch` không mang bảng vật liệu — walkable theo vật liệu là việc của
 * `BlockPalette`, không nạp ở hàm thuần này. Nên "đi được" ở đây suy trực tiếp
 * từ hình học của lát đang xem: `drop === 0` nghĩa là mặt đất nằm ngay tại lát
 * này, chỗ nhân vật đứng được; `drop > 0` nghĩa là đang nhìn xuyên không khí
 * phía trên mặt đất, không có gì đỡ chân. Overlay này không phân biệt được vật
 * liệu nguy hiểm (dung nham, băng trơn…) vì thông tin đó không có trong lô ô —
 * nó chỉ trả lời "có nền hay không", không thay được kết quả thật của
 * `api.goto`.
 */
function computeWalkableRaw(batch: TileBatch): Float32Array {
  const n = batch.w * batch.h;
  const raw = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    raw[i] = (batch.drop[i] ?? 0) === 0 ? 1 : 0;
  }
  return raw;
}

/** Bán kính đếm mật độ người, tính bằng ô. */
const CROWD_RADIUS = 3;

/**
 * Giá trị mật độ người: đếm thực thể `being` trong bán kính `CROWD_RADIUS`,
 * trọng số giảm tuyến tính theo khoảng cách.
 *
 * Trọng số giảm dần **chính là** bước "làm mượt": đếm cứng trong một bán kính
 * cố định sẽ vẽ ra một đường viền tròn sắc nét đúng ở bán kính đó — một ranh
 * giới không ai chủ đích tạo ra, chỉ là tác dụng phụ của phép đếm rời rạc. Cho
 * trọng số hạ dần về 0 tại rìa bán kính xóa luôn cạnh đó.
 *
 * Ô không có ai trong tầm là `NaN`, không phải `0` — cùng lý do như nước: `0`
 * vẫn là một màu hợp lệ trên thang, và một lô vắng người sẽ bị tô kín bằng
 * chính màu "không ai" đó thay vì để trống.
 */
function computeCrowdRaw(batch: TileBatch, entities: Entity[]): Float32Array {
  const { w, h, x: ox, y: oy } = batch;
  const raw = new Float32Array(w * h).fill(NaN);
  const beings = entities.filter((e) => e.kind === "being");
  if (beings.length === 0) return raw;
  for (let gy = 0; gy < h; gy++) {
    for (let gx = 0; gx < w; gx++) {
      const wx = ox + gx;
      const wy = oy + gy;
      let density = 0;
      for (const e of beings) {
        const d = Math.hypot(e.x - wx, e.y - wy);
        if (d <= CROWD_RADIUS) density += 1 - d / CROWD_RADIUS;
      }
      if (density > 0) raw[gy * w + gx] = density;
    }
  }
  return raw;
}

/** Tính một trường theo định danh lớp, trên đúng khung của lô ô đang xem. */
export function computeField(id: LayerId, batch: TileBatch, entities: Entity[]): Field {
  const { w, h } = batch;
  switch (id) {
    case "elevation": {
      // Cao độ chuẩn hóa theo min/max **thực tế của lô**, không theo hằng số
      // — lý do đầy đủ ở tài liệu của `normalize`.
      const { v, min, max } = normalize(Float32Array.from(batch.height));
      return { w, h, v, unit: "m", min, max };
    }
    case "water": {
      const { v, min, max } = normalize(computeWaterRaw(batch));
      return { w, h, v, unit: "m", min, max };
    }
    case "walkable": {
      // Nhị phân sẵn: KHÔNG đi qua `normalize`. Nếu một lô toàn đi được (hoặc
      // toàn không đi được), `normalize` sẽ đọc `max === min` và dồn mọi ô về
      // `0.5` — đúng cho một trường liên tục, nhưng phá nghĩa nhị phân "đi
      // được"/"không" thành một màu xám lửng lơ chẳng còn là gì cả.
      const v = computeWalkableRaw(batch);
      return { w, h, v, unit: "", min: 0, max: 1 };
    }
    case "crowd": {
      const { v, min, max } = normalize(computeCrowdRaw(batch, entities));
      return { w, h, v, unit: "người", min, max };
    }
    default: {
      // Bảo vệ khi kiểu bị ép qua ranh giới thời gian chạy (ví dụ từ một giá
      // trị `v-model` bên ngoài). `LayerId` tại biên dịch đã bao hết `LAYERS`.
      const _exhaustive: never = id;
      throw new Error(`không có lớp \`${String(_exhaustive)}\``);
    }
  }
}

/** Nội suy tuyến tính giữa hai màu, `t` trong `[0, 1]`. */
function mixRgb(a: Rgb, b: Rgb, t: number): Rgb {
  return {
    r: a.r + (b.r - a.r) * t,
    g: a.g + (b.g - a.g) * t,
    b: a.b + (b.b - a.b) * t,
  };
}

/**
 * Màu tại vị trí `t` trên một thang nhiều bậc, nội suy tuyến tính giữa hai
 * bậc liền kề. `SCALES` chỉ cho năm mốc rời rạc; bản đồ cần một dải liên tục.
 */
function rampColor(stops: readonly Rgb[], t: number): Rgb {
  const clamped = Math.min(1, Math.max(0, t));
  const seg = clamped * (stops.length - 1);
  const i0 = Math.min(stops.length - 2, Math.floor(seg));
  return mixRgb(stops[i0]!, stops[i0 + 1]!, seg - i0);
}

/**
 * Tô một trường thành RGBA, dùng thang màu đã kiểm tương phản của
 * `accessibility.ts` — **không** bịa thang mới, vì thang ở đó đã qua
 * `validateBothSchemes` cho cả `§18.6.2` lẫn `§18.6.4`.
 *
 * `alpha` là độ mờ chung của cả lớp, không phải theo ô. `NaN` luôn ra alpha
 * `0` bất kể `alpha` truyền vào là bao nhiêu — "không có dữ liệu" không có độ
 * mờ, nó là trong suốt tuyệt đối.
 */
export function paintField(f: Field, scheme: Scheme, alpha: number): Uint8ClampedArray {
  const stops = SCALES[scheme].map(parseHex);
  const a8 = Math.min(255, Math.max(0, Math.round(alpha * 255)));
  const out = new Uint8ClampedArray(f.w * f.h * 4);
  for (let i = 0; i < f.v.length; i++) {
    const t = f.v[i]!;
    if (Number.isNaN(t)) {
      out[i * 4 + 3] = 0;
      continue;
    }
    const c = rampColor(stops, t);
    out[i * 4] = c.r;
    out[i * 4 + 1] = c.g;
    out[i * 4 + 2] = c.b;
    out[i * 4 + 3] = a8;
  }
  return out;
}
