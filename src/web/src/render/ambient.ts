/**
 * Hạt môi trường: bọt sóng, gợn nước, bụi và lấp lánh.
 *
 * ## Vì sao một bản đồ vẽ đúng vẫn trông như ảnh chụp
 *
 * `terrain.ts` đã cho địa hình hình khối, bờ biển và ngày đêm — và kết quả vẫn
 * là một **bức tranh tĩnh**. Mắt người đọc "còn sống" qua *chuyển động nhỏ, có
 * quy luật*: sóng vỗ vào bờ, bụi bay qua bãi cát, một tia sáng chớp trên băng.
 * Không có lớp đó, người chơi tạm dừng thế giới hay để nó chạy ×100 đều thấy y
 * hệt nhau, và cảm giác "thế giới này đang vận hành" biến mất.
 *
 * Module này **không đụng Pixi**: nó trả về một danh sách hạt ở tọa độ thế giới
 * và để phía vẽ tự quyết dùng `Graphics`, `Sprite` hay particle container. Nhờ
 * vậy nó chạy được trong test Node thuần, và một lỗi ở đây là lỗi *tính toán*
 * chứ không phải lỗi *đồ họa* — hai loại lỗi cần hai cách gỡ khác nhau.
 *
 * ## Xác định tuyệt đối, không `Math.random()`
 *
 * Đây là **ràng buộc cứng**, không phải sở thích. Nếu hạt lấy ngẫu nhiên lúc
 * chạy thì mỗi lần dựng lại khung hình chúng nhảy sang chỗ khác, và khi kéo
 * camera cả lớp hạt "bò" theo viewport. Triệu chứng đó trông **y hệt một lỗi
 * đồng bộ trạng thái** — và người ta sẽ đi soi netcode hàng giờ trước khi nghi
 * ngờ lớp trang trí. Cùng `(batch, tick)` phải cho cùng một mảng, từng byte.
 *
 * Mọi thứ vì thế suy ra từ `hash(tọa độ thế giới, salt)` và `tick`. Tọa độ thế
 * giới chứ không phải chỉ số trong lô: chỉ số đổi khi camera dịch, tọa độ thì
 * không — đó chính là điều giữ cho hạt đứng yên trên mặt đất khi màn hình trượt.
 *
 * ## Chuyển động tuần hoàn, không tích lũy
 *
 * Pha là `((tick + offset) % period) / period` — số nguyên chia số nguyên. Cách
 * "cộng dồn delta mỗi khung" thay thế được về mặt hình ảnh nhưng sai về bản
 * chất: nó biến hạt thành *trạng thái*, mà trạng thái thì trôi khỏi `tick` sau
 * vài phút và không tua lại được khi xem replay.
 *
 * ## Bốn loại, mỗi ô nhiều nhất một hạt
 *
 * | Loại | Ô đủ điều kiện | Chở cảm giác gì |
 * |---|---|---|
 * | `foam` | nước **giáp đất** | sóng vỗ bờ — đường bờ động chứ không phải một nét vẽ |
 * | `ripple` | nước xa bờ, và lòng sông | mặt nước có bề mặt, không phải mảng màu |
 * | `dust` | đất khô, cát nhiều hơn | có gió, và gió có hướng |
 * | `sparkle` | băng và quặng | thứ đáng chú ý thì *tự* gọi mắt tới |
 *
 * Một ô chỉ thuộc **một** lớp. Cho phép chồng lớp làm mật độ thật nhân lên theo
 * số loại đủ điều kiện, và "3–8%" trong cấu hình sẽ không còn là 3–8% trên màn
 * hình — mật độ trở thành thứ không ai kiểm được bằng mắt.
 */

import type { TileBatch } from "@/api/game";
import type { BlockPalette } from "./blocks";

export type AmbientKind = "foam" | "ripple" | "dust" | "sparkle";

/** Thứ tự cố định, cho test và cho phía vẽ gom theo lớp. */
export const AMBIENT_KINDS = ["foam", "ripple", "dust", "sparkle"] as const;

/**
 * Một hạt, ở **tọa độ thế giới dạng số thực**.
 *
 * Số thực chứ không phải chỉ số ô: một hạt nằm đúng tâm ô trông như một lỗi
 * lưới, không phải một hạt. Hạt luôn nằm **trong ô sinh ra nó**, nên phía vẽ cắt
 * theo ô vẫn đúng và `Math.floor(x)` luôn lần ngược được về ô nguồn.
 */
export interface AmbientSprite {
  x: number;
  y: number;
  kind: AmbientKind;
  /** `(0, 1]`. Không bao giờ chạm 0: hạt tắt hẳn rồi bật lại là nhấp nháy. */
  alpha: number;
  /** Bội số kích thước, phía vẽ nhân với kích thước cơ sở của nó. */
  scale: number;
  /** Radian. `0` với hạt đối xứng tròn. */
  rotation: number;
}

/**
 * Tỉ lệ ô đủ điều kiện thực sự sinh hạt.
 *
 * Dày hơn khoảng này là nhiễu: hạt thôi làm nền sống và bắt đầu cạnh tranh với
 * thực thể — mà thực thể mới là thứ người chơi cần thấy. Ngưỡng trên 8% là nơi
 * mặt nước bắt đầu trông như bị lỗi hiển thị.
 */
export const AMBIENT_DENSITY = {
  foam: 0.07,
  ripple: 0.045,
  dustSoil: 0.035,
  /** Cát bay nhiều hơn đất mặt — đó là khác biệt vật chất, không phải trang trí. */
  dustSand: 0.075,
  sparkle: 0.055,
} as const;

/**
 * Chu kỳ từng loại, tính bằng tick. Nguyên tố cùng nhau theo cặp ở mức đủ để
 * cả lớp không **đập cùng nhịp** — nhịp chung biến bụi và sóng thành một hiệu
 * ứng nhấp nháy toàn màn hình, thứ mắt bắt ngay là "giả".
 */
const PERIOD = { foam: 24, ripple: 40, dust: 96, sparkle: 60 } as const;

/**
 * Bội chung nhỏ nhất của các chu kỳ: sau bấy nhiêu tick toàn cảnh lặp lại
 * chính xác. Xuất ra vì đây là bất biến đáng kiểm — nó chứng minh pha không
 * trôi, và một hạt trôi pha là một hạt đang giữ trạng thái ẩn.
 */
export const AMBIENT_CYCLE = 480;

/** Salt tách các kênh băm. Cùng ô, khác salt, phải ra số không liên quan nhau. */
const SALT = {
  gate: 0x1f3d_5b79,
  rank: 0x2a6c_9e11,
  px: 0x3b81_c4d7,
  py: 0x4c95_2fa3,
  phase: 0x5da7_63e9,
  rot: 0x6eb3_87c5,
  scale: 0x7fc1_49bd,
  wind: 0x11d7_ab63,
} as const;

const TAU = Math.PI * 2;

/**
 * Chừa mép ô. Hạt sát mép ô bị phía vẽ cắt mất một nửa khi cull theo ô, và một
 * nửa hạt trông như một lỗi vẽ chứ không như một hạt.
 */
const INSET = 0.04;

/** Bốn hướng, cùng thứ tự với đường bờ ở `terrain.ts` để hai lớp khớp nhau. */
const NEIGHBORS = [
  [1, 0],
  [-1, 0],
  [0, 1],
  [0, -1],
] as const;

/**
 * Băm ba số thành `[0, 1)`.
 *
 * Cùng họ với `hash2` của `terrain.ts` — cố ý, để hạt và hạt vật liệu không
 * "bắt" vào nhau thành hoa văn. Thêm một vòng trộn so với `hash2` vì ở đây một ô
 * phải cho ra **nhiều** số độc lập (vị trí, pha, hạng); một vòng là đủ trắng cho
 * một kênh nhưng để lại tương quan nhìn thấy được giữa các salt gần nhau.
 */
function hash3(x: number, y: number, salt: number): number {
  let h = Math.imul(x | 0, 0x27d4_eb2d) ^ Math.imul(y | 0, 0x1656_67b1);
  h = (h ^ Math.imul(salt | 0, 0x85eb_ca6b)) >>> 0;
  h = (h ^ (h >>> 15)) >>> 0;
  h = Math.imul(h, 0x2545_f491) >>> 0;
  h = (h ^ (h >>> 13)) >>> 0;
  h = Math.imul(h, 0x27d4_eb2d) >>> 0;
  return ((h ^ (h >>> 16)) >>> 0) / 4_294_967_296;
}

/** Ánh xạ `[0, 1)` sang `[lo, hi)`. */
function span(u: number, lo: number, hi: number): number {
  return lo + u * (hi - lo);
}

/** Giữ hạt trong ô của nó. Ràng buộc cấu trúc, không phải hệ quả của số học. */
function inCell(v: number): number {
  return v < INSET ? INSET : v > 1 - INSET ? 1 - INSET : v;
}

/**
 * Pha trong `[0, 1)`.
 *
 * `%` trên số nguyên nên `tick` có chạy tới hàng triệu cũng không mất độ chính
 * xác, và pha không bao giờ trôi. Chuẩn hóa hai lần vì `tick` âm hợp lệ (một
 * replay tua ngược) và `%` của JS giữ dấu của toán hạng trái.
 */
function phaseOf(tick: number, period: number, offset: number): number {
  return ((((tick + offset) % period) + period) % period) / period;
}

/** Nhãn của một vật liệu, rỗng nếu bảng không biết nó. */
function tagsOf(palette: BlockPalette, id: string): readonly string[] {
  return palette.get(id)?.tags ?? [];
}

/**
 * Vật liệu có lấp lánh không.
 *
 * Ưu tiên **nhãn** chứ không phải id: `§8.2` coi vật liệu là dữ liệu, nên một
 * pack thêm "thạch anh" phải lấp lánh được mà không sửa mã. Nhưng pack lõi chưa
 * gắn nhãn nào cho băng — nó chỉ là `solid` — nên vẫn phải nhận id kết thúc bằng
 * `ice` làm lối thoát. Chỗ này sẽ bỏ được ngay khi pack có nhãn `crystal`.
 */
export function isGlint(palette: BlockPalette, id: string): boolean {
  const tags = tagsOf(palette, id);
  if (tags.includes("valuable") || tags.includes("crystal")) return true;
  return /(?:^|[.:])ice$/.test(id);
}

/** Đất khô đủ tơi để có bụi. Đá không sinh bụi — nó sinh vụn, việc khác. */
function isDusty(palette: BlockPalette, id: string): boolean {
  const tags = tagsOf(palette, id);
  return tags.includes("soil") || tags.includes("loose");
}

/**
 * Danh sách hạt môi trường cho một lô ô tại một tick.
 *
 * `budget` là **trần cứng** số hạt trả về. Vượt trần thì tỉa theo *hạng băm của
 * tọa độ* chứ không cắt cuối mảng: cắt theo thứ tự duyệt xóa sạch nửa dưới khung
 * nhìn — nửa trên đầy sóng, nửa dưới chết trơ, và đường ranh giới chạy theo
 * camera. Hạng băm phân bố trắng trong không gian nên hạ trần chỉ làm **thưa
 * đều**. Hạng cũng không phụ thuộc `tick`, nên hạt không chớp tắt qua các khung.
 */
export function ambientSprites(
  batch: TileBatch,
  palette: BlockPalette,
  tick: number,
  budget: number,
): AmbientSprite[] {
  const w = Math.floor(batch.w);
  const h = Math.floor(batch.h);
  const cap = Number.isFinite(budget) ? Math.floor(budget) : 0;
  if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0 || cap <= 0) return [];

  const t = Number.isFinite(tick) ? Math.floor(tick) : 0;
  const n = w * h;

  // Một lượt quét trước để biết ô nào là nước. Hỏi `isLiquid` lại cho từng ô
  // hàng xóm là bốn lần tra `Map` cho mỗi ô — 14 nghìn lần tra trên một khung
  // nhìn, mỗi khung hình, cho một câu trả lời không đổi.
  const wet = new Uint8Array(n);
  const visible = new Array<string>(n);
  for (let i = 0; i < n; i++) {
    const m = batch.material[i] ?? "air";
    // Cùng quy tắc "ghost lớp dưới" của `terrain.ts §18.1`: khi lát đang xem là
    // không khí, thứ người chơi *thấy* là mặt đất bên dưới — và hạt phải bám vào
    // thứ được thấy, không phải thứ nằm ở lát.
    const id = m !== "air" ? m : (batch.surface[i] ?? "air");
    visible[i] = id;
    wet[i] = palette.isLiquid(id) ? 1 : 0;
  }

  const sprites: AmbientSprite[] = [];
  const ranks: number[] = [];

  for (let gy = 0; gy < h; gy++) {
    for (let gx = 0; gx < w; gx++) {
      const i = gy * w + gx;
      const wx = batch.x + gx;
      const wy = batch.y + gy;
      const id = visible[i] ?? "air";

      // ── Phân lớp: mỗi ô nhiều nhất một loại ────────────────────────────
      let kind: AmbientKind | null = null;
      let density = 0;
      // Hướng ra phía đất, chỉ có nghĩa với `foam`.
      let landX = 0;
      let landY = 0;

      if (wet[i] === 1) {
        for (const [ox, oy] of NEIGHBORS) {
          const nx = gx + ox;
          const ny = gy + oy;
          // Ngoài lô thì **bỏ qua**, không coi là đất. Coi là đất sẽ vẽ một vành
          // bọt giả chạy dọc mép khung nhìn mỗi khi camera dịch — đúng thứ mà
          // toàn bộ module này tồn tại để tránh.
          if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
          if (wet[ny * w + nx] === 0) {
            landX = ox;
            landY = oy;
            kind = "foam";
            density = AMBIENT_DENSITY.foam;
            break;
          }
        }
        if (kind === null) {
          kind = "ripple";
          density = AMBIENT_DENSITY.ripple;
        }
      } else if ((batch.river[i] ?? 0) === 1) {
        // Lòng sông là nước chảy trên nền đất: vật liệu vẫn là đất nên nó không
        // "ướt" theo bảng, nhưng để nó khô là để sông thành một vệt sơn.
        kind = "ripple";
        density = AMBIENT_DENSITY.ripple;
      } else if (isGlint(palette, id)) {
        kind = "sparkle";
        density = AMBIENT_DENSITY.sparkle;
      } else if (isDusty(palette, id)) {
        kind = "dust";
        density = tagsOf(palette, id).includes("loose")
          ? AMBIENT_DENSITY.dustSand
          : AMBIENT_DENSITY.dustSoil;
      }

      if (kind === null) continue;
      if (hash3(wx, wy, SALT.gate) >= density) continue;

      // ── Các đại lượng chỉ phụ thuộc tọa độ ─────────────────────────────
      const period = PERIOD[kind];
      const offset = Math.floor(hash3(wx, wy, SALT.phase) * period);
      const phase = phaseOf(t, period, offset);
      const wave = Math.sin(TAU * phase);
      const swirl = Math.cos(TAU * phase);
      const jitter = span(hash3(wx, wy, SALT.scale), 0.85, 1.15);

      let fx: number;
      let fy: number;
      let alpha: number;
      let scale: number;
      let rotation: number;

      switch (kind) {
        case "foam": {
          // Bọt ngồi ở **mép giáp đất**, không ở tâm ô: bọt ở tâm ô đọc thành
          // "có gì đó nổi trên nước", không đọc thành sóng vỗ vào bờ.
          const alongX = -landY;
          const alongY = landX;
          const slide = span(hash3(wx, wy, SALT.px), -0.26, 0.26) + 0.05 * swirl;
          const lap = 0.3 + 0.1 * wave;
          fx = 0.5 + landX * lap + alongX * slide;
          fy = 0.5 + landY * lap + alongY * slide;
          const swell = 0.5 + 0.5 * wave;
          alpha = 0.3 + 0.35 * swell;
          scale = jitter * (0.75 + 0.35 * swell);
          // Quay theo hướng bờ: một vệt bọt nằm ngang trên một bờ dọc là chi
          // tiết duy nhất phá hỏng cả lớp.
          rotation = Math.atan2(landY, landX);
          break;
        }
        case "ripple": {
          // Vòng lan ra rồi mờ đi. Bao hình `sin(pi * phase)` bằng 0 ở **cả hai
          // đầu** chu kỳ, nên lúc bán kính nhảy về 0 thì hạt gần như trong suốt
          // — không có cú giật nào lộ ra.
          const bx = span(hash3(wx, wy, SALT.px), 0.2, 0.8);
          const by = span(hash3(wx, wy, SALT.py), 0.2, 0.8);
          fx = bx + 0.05 * swirl;
          fy = by + 0.05 * wave;
          const env = Math.sin(Math.PI * phase);
          alpha = 0.08 + 0.34 * env;
          scale = jitter * (0.35 + 1.0 * phase);
          rotation = 0;
          break;
        }
        case "dust": {
          // Hướng gió băm theo **ô 16×16**, không theo từng ô: gió mỗi ô một
          // hướng là chuyển động Brown, và mắt đọc nó thành nhiễu chứ không
          // thành gió. Một vùng chung hướng mới thành cơn.
          const gust = hash3(wx >> 4, wy >> 4, SALT.wind) * TAU;
          const gx0 = Math.cos(gust);
          const gy0 = Math.sin(gust);
          const bx = span(hash3(wx, wy, SALT.px), 0.32, 0.68);
          const by = span(hash3(wx, wy, SALT.py), 0.32, 0.68);
          const swing = 0.2 * wave;
          const flutter = 0.06 * swirl;
          fx = bx + gx0 * swing - gy0 * flutter;
          fy = by + gy0 * swing + gx0 * flutter;
          const env = 0.5 + 0.5 * wave;
          // Rất mờ: bụi phải ở **dưới ngưỡng chú ý**. Thấy rõ từng hạt bụi là
          // lúc nó thôi làm không khí và thành đồ trang trí.
          alpha = 0.12 + 0.22 * env;
          scale = jitter * (0.3 + 0.25 * env);
          rotation = gust;
          break;
        }
        default: {
          // Chớp nhọn: lũy thừa 4 dồn gần hết chu kỳ vào mức tối, chỉ để lại
          // một tia ngắn. Một nhịp sin trơn cho ra "đèn nhấp nháy", không cho ra
          // "ánh nắng bắt vào tinh thể".
          const bx = span(hash3(wx, wy, SALT.px), 0.2, 0.8);
          const by = span(hash3(wx, wy, SALT.py), 0.2, 0.8);
          fx = bx + 0.05 * swirl;
          fy = by + 0.05 * wave;
          const env = 0.5 + 0.5 * wave;
          const twinkle = env * env * env * env;
          alpha = 0.14 + 0.76 * twinkle;
          scale = jitter * (0.28 + 0.55 * twinkle);
          rotation = hash3(wx, wy, SALT.rot) * TAU;
          break;
        }
      }

      sprites.push({
        x: wx + inCell(fx),
        y: wy + inCell(fy),
        kind,
        alpha,
        scale,
        rotation,
      });
      ranks.push(hash3(wx, wy, SALT.rank));
    }
  }

  if (sprites.length <= cap) return sprites;

  // Giữ `cap` hạt có hạng nhỏ nhất. Phá hòa bằng chỉ số để thứ tự là **toàn
  // phần**: hai hạng bằng nhau tuyệt đối là hiếm, nhưng "hiếm" cộng với
  // `Array.sort` không ổn định là đủ để cùng đầu vào cho hai mảng khác nhau.
  const order = sprites.map((_, i) => ({ i, rank: ranks[i] ?? 0 }));
  order.sort((a, b) => a.rank - b.rank || a.i - b.i);
  const keep = new Uint8Array(sprites.length);
  for (let k = 0; k < cap; k++) {
    const o = order[k];
    if (o) keep[o.i] = 1;
  }
  // Trả theo thứ tự quét chứ không theo thứ tự hạng: phía vẽ duyệt mảng này
  // theo hàng thì chạm bộ nhớ tuần tự, và ảnh chụp màn hình so được với nhau.
  return sprites.filter((_, i) => keep[i] === 1);
}
