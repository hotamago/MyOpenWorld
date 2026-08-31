/**
 * Tô nền địa hình: màu vật liệu, đổ bóng theo độ dốc, hạt, đường bờ, đường đồng mức.
 *
 * ## Vì sao một bảng màu đúng vẫn cho ra bản đồ chết
 *
 * Bản đầu tiên tô mỗi ô đúng màu vật liệu và cho ra **một mảng nâu phẳng**. Màu
 * không sai — thông tin thì thiếu: mắt người đọc địa hình bằng **bóng**, không
 * bằng sắc độ. Một sườn núi và một cánh đồng cùng là `topsoil` trong cùng biome
 * nên chúng cùng một màu, và bản đồ mất sạch hình khối.
 *
 * Năm lớp, theo đúng thứ tự áp:
 *
 * | Lớp | Chở thông tin gì | Vì sao cần |
 * |---|---|---|
 * | màu vật liệu | ô này là chất gì | `§18.5`, kênh bắt buộc |
 * | đổ bóng | độ dốc và hướng dốc | biến mảng phẳng thành địa hình |
 * | hạt | không gì cả | phá cảm giác "tô bằng xô sơn" |
 * | đường bờ | ranh giới nước–đất | đường nét duy nhất mắt bám được |
 * | đồng mức | bậc độ sâu dưới lát đang xem | đọc được mình đang ở trên hay dưới |
 *
 * Bốn lớp sau **chỉ đổi độ sáng**, không đổi sắc. Đó là ràng buộc của `§18.5`:
 * màu nền ô chở vật liệu và chỉ chở vật liệu. Nếu đổ bóng cũng đổi sắc thì
 * người chơi không phân biệt được "đá tối" với "đất trong bóng râm".
 *
 * ## Xác định, không ngẫu nhiên
 *
 * Hạt lấy từ hash của **tọa độ thế giới**, không phải `Math.random()`. Lấy ngẫu
 * nhiên lúc chạy làm hoa văn nhấp nháy mỗi lần vẽ lại và bò đi khi kéo camera —
 * trông y hệt một lỗi đồng bộ, và tốn nhiều giờ để loại trừ.
 */

import type { TileBatch } from "@/api/game";
import type { BlockPalette } from "./blocks";

/**
 * Hệ số phóng đại độ dốc trước khi tính bóng.
 *
 * Cao độ tính bằng mét, ô tính bằng ô, nên tỉ lệ thật giữa hai trục không phải
 * 1:1. `0.35` làm đồi thấy rõ mà vách núi không cháy trắng.
 */
const SLOPE_EXAGGERATION = 0.35;

/** Hướng nắng: trên–trái. Quy ước bản đồ, không phải thiên văn. */
const SUN: readonly [number, number, number] = (() => {
  const d = Math.hypot(-0.55, -0.62, 0.56);
  return [-0.55 / d, -0.62 / d, 0.56 / d] as const;
})();

/**
 * Sàn và trần cho hệ số ánh sáng của một ô.
 *
 * Sàn 0.5 chứ không phải 0: nhân thêm sắc trời ban đêm (0.62) thì ô tối nhất
 * vẫn còn khoảng 31% độ sáng — đủ để phân biệt vật liệu, không đủ để hết cảm
 * giác là bóng tối. Trần 1.45 chặn mặt dốc đón nắng cháy trắng thành một mảng
 * không còn màu vật liệu.
 */
const SHADE_FLOOR = 0.5;
const SHADE_CEIL = 1.45;

/** Băm hai tọa độ thành `[0, 1)`. Rẻ, xác định, đủ trắng cho việc này. */
function hash2(x: number, y: number): number {
  let h = Math.imul(x | 0, 0x27d4_eb2d) ^ Math.imul(y | 0, 0x1656_67b1);
  h = (h ^ (h >>> 15)) >>> 0;
  h = Math.imul(h, 0x2545_f491) >>> 0;
  return ((h ^ (h >>> 13)) >>> 0) / 4_294_967_296;
}

const CLAMP8 = (v: number) => (v < 0 ? 0 : v > 255 ? 255 : v);

/**
 * Màu của một ô nhìn từ trên xuống, đã tính ghost lớp dưới (`§18.1`).
 *
 * Một lát `z` thuần túy cho ra bản đồ đen kịt: đứng ở 85 m thì mọi thứ thấp hơn
 * đều là không khí. Nên khi ô ở lát hiện tại là không khí, ta lấy màu **mặt đất
 * bên dưới**, tối dần theo khoảng cách.
 */
function topDownColor(
  palette: BlockPalette,
  material: string,
  surface: string,
  drop: number,
  elevation: number,
): number {
  const solid = material !== "air";
  const base = palette.color(solid ? material : surface);

  let r = (base >> 16) & 0xff;
  let g = (base >> 8) & 0xff;
  let b = base & 0xff;

  // Nước sâu tối hơn nước nông. Không có bậc này thì cả đại dương là một mảng
  // xanh phẳng, và người chơi mất manh mối duy nhất về chỗ nào lội được.
  if (palette.isLiquid(solid ? material : surface)) {
    const depth = Math.min(1, Math.max(0, -elevation / 400));
    const k = 1 - depth * 0.55;
    r *= k;
    g *= k;
    b *= k;
  }

  if (!solid) {
    // 0 m ngay dưới chân giữ nguyên sáng; 120 m trở xuống chạm sàn 30%.
    const k = Math.max(0.3, 1 - Math.min(1, drop / 120) * 0.7);
    r *= k;
    g *= k;
    b *= k;
  }
  return (CLAMP8(Math.round(r)) << 16) | (CLAMP8(Math.round(g)) << 8) | CLAMP8(Math.round(b));
}

/**
 * Dựng buffer RGBA cho cả lô ô. Dài `w * h * 4`, hợp với `ImageData`.
 */
export function paintTerrain(batch: TileBatch, palette: BlockPalette): Uint8ClampedArray {
  const { w, h } = batch;
  const out = new Uint8ClampedArray(w * h * 4);
  // Cao độ dùng để đổ bóng là cao độ **mặt đất**, không phải của lát đang xem:
  // người chơi cần thấy hình khối của địa hình bên dưới.
  const H = batch.height;

  const heightAt = (gx: number, gy: number): number => {
    const cx = gx < 0 ? 0 : gx >= w ? w - 1 : gx;
    const cy = gy < 0 ? 0 : gy >= h ? h - 1 : gy;
    return H[cy * w + cx] ?? 0;
  };

  /** Bậc độ sâu, dùng cho đường đồng mức. Log để bậc thưa dần khi xuống sâu. */
  const depthBand = (i: number): number => Math.floor(Math.log2(1 + Math.max(0, batch.drop[i] ?? 0)));

  for (let gy = 0; gy < h; gy++) {
    for (let gx = 0; gx < w; gx++) {
      const i = gy * w + gx;
      const material = batch.material[i] ?? "air";
      const surface = batch.surface[i] ?? "air";
      const visible = material !== "air" ? material : surface;

      const base = topDownColor(palette, material, surface, batch.drop[i] ?? 0, H[i] ?? 0);
      let r = (base >> 16) & 0xff;
      let g = (base >> 8) & 0xff;
      let b = base & 0xff;

      // ── Đổ bóng ────────────────────────────────────────────────────────
      // Sai phân trung tâm; kẹp ở biên lô để mép không có viền tối giả.
      const dzdx = (heightAt(gx + 1, gy) - heightAt(gx - 1, gy)) * 0.5 * SLOPE_EXAGGERATION;
      const dzdy = (heightAt(gx, gy + 1) - heightAt(gx, gy - 1)) * 0.5 * SLOPE_EXAGGERATION;
      const len = Math.hypot(dzdx, dzdy, 1);
      const ndotl = (-dzdx * SUN[0] + -dzdy * SUN[1] + SUN[2]) / len;
      // Mặt phẳng cho `ndotl ≈ SUN[2]`, nên chuẩn hóa quanh đó: đất bằng giữ
      // đúng màu bảng, chỉ sườn dốc mới sáng/tối đi.
      let light = 1 + (ndotl - SUN[2]) * 1.35;

      // Dưới nước thì bóng phải nhẹ: đáy biển nhìn qua nước không có nắng gắt.
      if (palette.isLiquid(visible)) light = 1 + (light - 1) * 0.35;

      // ── Hạt vật liệu ───────────────────────────────────────────────────
      // Biên độ theo vật liệu: nước gần phẳng, cát rất hạt. Một biên độ chung
      // làm mặt nước lấm tấm như giấy nhám — sai về cảm giác vật chất.
      const grain = palette.grain(visible);
      light *= 1 + (hash2(batch.x + gx, batch.y + gy) - 0.5) * grain;

      // ── Đường bờ ───────────────────────────────────────────────────────
      // Chỉ 4 hướng: 8 hướng làm bờ dày lên và nuốt mất các eo nhỏ.
      const wet = palette.isLiquid(surface);
      let onEdge = false;
      for (const [ox, oy] of [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ] as const) {
        const nx = gx + ox;
        const ny = gy + oy;
        if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
        if (palette.isLiquid(batch.surface[ny * w + nx] ?? "air") !== wet) {
          onEdge = true;
          break;
        }
      }
      // Đất giáp nước sáng lên (bãi bồi), nước giáp đất tối đi (bóng bờ). Hai
      // chiều ngược nhau tạo một đường viền đọc được ở mọi mức phóng.
      if (onEdge) light *= wet ? 0.78 : 1.14;

      // ── Bóng tiếp đất của công trình ────────────────────────────────────
      //
      // Đây là cách sửa cho đúng lời phàn nàn "mái nhà nổi như sticker". Nguyên
      // nhân không chỉ ở màu: một khối nhà không có bóng thì mắt không có bằng
      // chứng nào rằng nó **đứng trên** mặt đất chứ không phải dán lên ảnh.
      //
      // `SUN` hướng sao cho sườn dốc lên phía `+x`/`+y` thì sáng — tức nắng tới
      // từ phải và dưới. Vậy bóng đổ về trái và trên, và một ô đất có công
      // trình ở `(+1, 0)` hoặc `(0, +1)` là ô nằm trong bóng của nó.
      //
      // Làm ở đây chứ không phải bằng một sprite bóng riêng vì bóng phải **theo
      // đúng lưới ô** — một ellipse mờ đặt lên trên sẽ trôi lệch khi phóng to.
      {
        const isBuilt = (nx: number, ny: number): boolean =>
          nx >= 0 && ny >= 0 && nx < w && ny < h && (batch.built[ny * w + nx] ?? 0) === 1;
        const here = (batch.built[i] ?? 0) === 1;
        const lit = isBuilt(gx + 1, gy) || isBuilt(gx, gy + 1);
        const shade = isBuilt(gx - 1, gy) || isBuilt(gx, gy - 1);
        if (!here && lit) {
          // Đất ngay sát chân tường phía khuất nắng.
          light *= 0.82;
        } else if (here && !lit) {
          // Mép mái hướng về phía nắng: bắt sáng, và đó là thứ tách mái ra khỏi
          // mái nhà bên cạnh khi cả dãy cùng một vật liệu.
          light *= 1.1;
        } else if (here && shade) {
          // Mép mái phía khuất: một vạch tối mỏng đọc ra là độ dày của mái.
          light *= 0.9;
        }
      }

      // ── Đường đồng mức theo bậc độ sâu ─────────────────────────────────
      // Chỉ vẽ khi đang nhìn từ trên xuống (`material === "air"`): trong lòng
      // đất thì mọi ô cùng bậc, và một lưới đồng mức ở đó chỉ là nhiễu.
      if (material === "air") {
        const band = depthBand(i);
        const rightDiff = gx + 1 < w && depthBand(i + 1) !== band;
        const downDiff = gy + 1 < h && depthBand(i + w) !== band;
        if (rightDiff || downDiff) light *= 0.9;
      }

      // Lòng sông: sáng và lệch lam, đủ để lần theo bằng mắt. Đây là **overlay
      // riêng**, không phải đổi màu vật liệu — sông vẫn nằm trên đất.
      if ((batch.river[i] ?? 0) === 1) {
        r = r * 0.55 + 0x35 * 0.45;
        g = g * 0.55 + 0x7a * 0.45;
        b = b * 0.55 + 0xb8 * 0.45;
      }

      // ── Sàn và trần độ sáng ────────────────────────────────────────────
      //
      // Đổ bóng theo độ dốc là **đúng** về vật lý và **sai** về mục đích: một
      // vách 6 mét mỗi ô cho `ndotl` gần 0, nhân với sắc trời rạng sáng thì ra
      // đen tuyền, và cả một sườn núi biến mất khỏi bản đồ. `§18.13` đòi bản đồ
      // đọc được mà không cần bảng số — một mảng đen thì không đọc được gì, và
      // tệ hơn, nó trông y hệt một lỗi renderer.
      //
      // Kẹp ở đây chứ không hạ `SLOPE_EXAGGERATION`: hạ hệ số sẽ làm phẳng cả
      // những gợn đồi nhẹ vốn là thứ hillshade sinh ra để cho thấy. Kẹp chỉ cắt
      // đúng phần đuôi mà mắt không đọc được nữa.
      const lit = light < SHADE_FLOOR ? SHADE_FLOOR : light > SHADE_CEIL ? SHADE_CEIL : light;

      out[i * 4] = CLAMP8(Math.round(r * lit));
      out[i * 4 + 1] = CLAMP8(Math.round(g * lit));
      out[i * 4 + 2] = CLAMP8(Math.round(b * lit));
      out[i * 4 + 3] = 255;
    }
  }
  return out;
}

/** Bốn buổi trong ngày, cho nhãn hiển thị. */
export type DayPhase = "night" | "dawn" | "day" | "dusk";

/** Số tick một ngày. Đặt ở đây vì cả sắc trời lẫn nhãn giờ đều cần. */
export const TICKS_PER_DAY = 2_400;

/** Buổi trong ngày tại một tick. */
export function dayPhase(tick: number): DayPhase {
  const p = (tick % TICKS_PER_DAY) / TICKS_PER_DAY;
  if (p < 0.2) return "night";
  if (p < 0.32) return "dawn";
  if (p < 0.72) return "day";
  if (p < 0.84) return "dusk";
  return "night";
}

/**
 * Sắc trời nhân lên toàn cảnh, `0xRRGGBB`.
 *
 * `tick` là thời gian của **thế giới**, nên ngày đêm là hàm của trạng thái chứ
 * không phải của đồng hồ máy người chơi. Hai người xem cùng một world thấy cùng
 * một buổi chiều — và một bản replay cũng vậy.
 */
export function skyTint(tick: number): number {
  const p = (tick % TICKS_PER_DAY) / TICKS_PER_DAY; // 0 = nửa đêm
  const sun = 0.5 - 0.5 * Math.cos(p * Math.PI * 2);
  // Đêm không đen kịt: `§18.13` đòi bản đồ đọc được mà không cần bảng số, và
  // một màn hình đen thì không đọc được gì.
  // Sàn 0.62 chứ không phải 0.5. Bản trước dùng 0.5 và ban đêm bản đồ tối tới
  // mức không đọc được địa hình — `§18.13` đòi đọc được mà không cần bảng số,
  // và một màn hình gần đen thì không đọc được gì. Đêm phải *cảm thấy* là đêm,
  // không phải *biến mất*.
  const k = 0.62 + 0.38 * sun;
  // Rạng sáng và hoàng hôn ngả cam; trưa trung tính; đêm ngả lam.
  const warm = Math.max(0, Math.sin(p * Math.PI * 2)) * 0.1;
  const r = CLAMP8(Math.round(255 * Math.min(1, k * (1 + warm))));
  const g = CLAMP8(Math.round(255 * Math.min(1, k * (1 + warm * 0.3))));
  const b = CLAMP8(Math.round(255 * Math.min(1, k * 0.94 + 0.08)));
  return (r << 16) | (g << 8) | b;
}
