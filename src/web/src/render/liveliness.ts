/**
 * Chuyển động vi mô: khói bếp, cỏ lay, và các con số thuần đứng sau chúng.
 *
 * ## Vì sao module này tồn tại tách khỏi `ambient.ts`
 *
 * `ambient.ts` đã giải quyết "mặt nước có bề mặt, có gió" bằng hạt tuần hoàn
 * theo `tick` — đúng cho hiện tượng **vật lý của thế giới** (sóng, gió), thứ
 * phải dừng lại khi thế giới tạm dừng và tua lại y hệt khi xem replay.
 *
 * Ở đây là một lớp khác: **sự sống của con người** — khói bếp, lúa lay, nhịp
 * bước chân. Những thứ này cần mượt ở 60 khung/giây bất kể server đang trả lời
 * mỗi 400ms hay đang tạm dừng mô phỏng, giống hệt lý do `motion.ts` nội suy
 * bước đi bằng đồng hồ thật (`performance.now()`) thay vì đợi tick tiếp theo.
 * Vì vậy `sway` và `smokePuff` nhận `ms` — thời gian thực — chứ không phải
 * `tick`, đúng khuôn `MotionTrack.at(id, nowMs)` đã đặt ra. `chimneys` thì
 * khác: nó quyết định **ống khói nào đang cháy**, một câu hỏi về trạng thái
 * thế giới (ban đêm hay không, nhà có ai không) nên vẫn phải là hàm của
 * `tick`, giữ đúng nguyên tắc "cùng tick, cùng thế giới" của `ambient.ts`.
 *
 * ## Vẫn không `Math.random()`, vẫn không đụng Pixi
 *
 * Cùng lý do đã nói ở `ambient.ts`: ngẫu nhiên lúc chạy trông y hệt lỗi đồng
 * bộ, và tách thuần khỏi vẽ nghĩa là một hiệu ứng sai là lỗi *tính toán*, kiểm
 * bằng `vitest` thẳng trong Node, không cần mở trình duyệt.
 *
 * ## Lệch pha, không đồng loạt
 *
 * Mọi ô lấy một góc pha riêng từ `phaseAt` — băm tọa độ thế giới, không phải
 * chỉ số trong lô, cùng lý do `ambient.ts` đã giải thích: chỉ số đổi khi camera
 * dịch, tọa độ thì không. Nhờ pha riêng, hàng trăm ô lay cùng một chu kỳ không
 * bao giờ lay *cùng lúc* — thứ mắt đọc ngay ra là hoạt ảnh lặp, không phải sự
 * sống.
 *
 * ## Chỉ một phần nhỏ chuyển động
 *
 * `sway` không trả một dao động nhỏ cho **mọi** ô — nó cổng (gate) trước bằng
 * một kênh băm riêng (`SALT.swayGate`), tách khỏi kênh cho pha
 * (`SALT.phase`): nếu dùng chung một kênh, tập ô "được lay" sẽ luôn rơi vào
 * đúng dải pha thấp, và cả đám sẽ bắt đầu dao động ở góc pha gần giống nhau —
 * lại quay về đúng thứ "đồng loạt" mà việc cổng này sinh ra để tránh.
 * `SWAY_ACTIVE_RATIO` giữ tỉ lệ đó trong khoảng 10–20% mà buổi tư vấn đề ra:
 * dày hơn thì cỏ lay khắp màn hình là nhiễu, thưa hơn thì cánh đồng lại đứng
 * hình.
 */

import type { TileBatch } from "@/api/game";
import { dayPhase } from "./terrain";

const TAU = Math.PI * 2;

/** Salt tách các kênh băm — cùng kỹ thuật `ambient.ts` dùng cho `SALT`. */
const SALT = {
  phase: 0x1a2b_3c4d,
  swayGate: 0x5e6f_7081,
  chimneyGate: 0x92a3_b4c5,
  chimneyDuty: 0xd6e7_f809,
  chimneyRank: 0x1b2c_3d4e,
  smokeDrift: 0x5f60_718a,
} as const;

/**
 * Băm hai tọa độ cộng một salt thành `[0, 1)`.
 *
 * Cùng công thức `hash3` của `ambient.ts` (bản thân nó cùng họ với `hash2` của
 * `terrain.ts`) — tái dùng công thức đã kiểm chứng thay vì phát minh một hàm
 * băm mới, nhưng viết lại tại đây vì cả hai hàm gốc đều là hàm riêng (`private`
 * theo nghĩa không export) của module chủ, và `liveliness.ts` không được sửa
 * chúng để export ra.
 */
function hash2(x: number, y: number, salt: number): number {
  let h = Math.imul(x | 0, 0x27d4_eb2d) ^ Math.imul(y | 0, 0x1656_67b1);
  h = (h ^ Math.imul(salt | 0, 0x85eb_ca6b)) >>> 0;
  h = (h ^ (h >>> 15)) >>> 0;
  h = Math.imul(h, 0x2545_f491) >>> 0;
  h = (h ^ (h >>> 13)) >>> 0;
  h = Math.imul(h, 0x27d4_eb2d) >>> 0;
  return ((h ^ (h >>> 16)) >>> 0) / 4_294_967_296;
}

/** `%` giữ dấu toán hạng trái trong JS; chuẩn hóa hai lần cho `tick`/`ms` âm hợp lệ. */
function safeMod(a: number, m: number): number {
  return ((a % m) + m) % m;
}

/**
 * Pha dao động của một ô, xác định theo toạ độ thế giới.
 *
 * Trả về radian trong `[0, 2π)`, dùng thẳng làm số hạng cộng trong một
 * `Math.sin`. Cùng một ô luôn cho cùng một pha ở mọi lần gọi, mọi phiên chơi —
 * đó là toàn bộ lý do hiệu ứng "thở" trông giống nhau giữa hai lần mở game.
 */
export function phaseAt(x: number, y: number): number {
  return hash2(Math.trunc(x), Math.trunc(y), SALT.phase) * TAU;
}

/**
 * Tỉ lệ ô đủ điều kiện thực sự dao động tại một thời điểm.
 *
 * 15% — giữa dải 10–20% buổi tư vấn chốt. Đây là kênh băm **riêng** với
 * `SALT.phase` (xem lời giải đầu file) để tập ô được chọn không thiên về một
 * dải pha hẹp.
 */
export const SWAY_ACTIVE_RATIO = 0.15;

/** Chu kỳ và biên độ mặc định — khớp đúng con số cho lúa/cỏ trong buổi tư vấn. */
export const SWAY_DEFAULT_PERIOD_MS = 2_500;
export const SWAY_DEFAULT_AMPLITUDE = 2;

function swayGateActive(x: number, y: number): boolean {
  return hash2(Math.trunc(x), Math.trunc(y), SALT.swayGate) < SWAY_ACTIVE_RATIO;
}

/**
 * Biên độ dao động tại thời điểm `ms`, cho một ô.
 *
 * Trả `0` cho phần lớn ô (85%) — chưa từng "được chọn" để lay, không phải vì
 * `ms` chưa tới lượt. Với ô được chọn, kết quả là một dao động hình sin bị
 * chặn trong `[-amplitude, amplitude]`, lệch một góc riêng theo `phaseAt`.
 *
 * `ms` cố ý là thời gian **thực** (kiểu `performance.now()`), không phải
 * `tick`: xem lời giải "vì sao module này tồn tại" ở đầu file. Vì vậy gọi hai
 * lần với cùng `(x, y, ms)` — kể cả ở hai tick thế giới khác nhau, hay khi thế
 * giới tạm dừng — luôn cho đúng cùng một số; đó là tất cả những gì cần cho
 * "xác định" ở một hàm hoạt hình theo đồng hồ thật.
 */
export function sway(
  x: number,
  y: number,
  ms: number,
  opts?: { periodMs?: number; amplitude?: number },
): number {
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(ms)) return 0;
  if (!swayGateActive(x, y)) return 0;

  const period = opts?.periodMs ?? SWAY_DEFAULT_PERIOD_MS;
  const amplitude = opts?.amplitude ?? SWAY_DEFAULT_AMPLITUDE;
  if (!(period > 0) || !Number.isFinite(amplitude)) return 0;

  const phase = phaseAt(x, y);
  return amplitude * Math.sin((ms / period) * TAU + phase);
}

/**
 * Vật liệu mái — cố tình chép trực tiếp `id`, không tra `tags` qua
 * `BlockPalette`.
 *
 * `chimneys` chỉ nhận `TileBatch`, không nhận bảng vật liệu (đúng chữ ký buổi
 * tư vấn chốt): nó không cần biết "mái" là gì về mặt vật lý, chỉ cần biết
 * đúng hai `id` mà pack lõi dùng cho ngói (`content/core/blocks/roof_*`). Nếu
 * sau này một content pack thêm một vật liệu mái khác, khói bếp đơn giản là
 * chưa vẽ ở đó — không sập, không sai, chỉ thiếu một hiệu ứng trang trí.
 */
const ROOF_MATERIALS = new Set(["roof_light", "roof_dark"]);

/** Tỉ lệ ô mái đủ điều kiện từng có cơ hội là ống khói của nhà nó. */
export const CHIMNEY_GATE_RATIO = 0.15;

/**
 * Chu kỳ "thỉnh thoảng cháy" của một ống khói, tính bằng tick.
 *
 * Theo `tick`, không theo `ms`: đây là quyết định "ống khói này đang hoạt
 * động hay không", một sự thật về **thế giới** — nó phải đứng yên khi thế
 * giới tạm dừng và lặp lại y hệt trong một bản replay, giống `ambient.ts`.
 * 70/200 tick nghĩa là mỗi nhà cháy khoảng 35% thời gian ban đêm, chia thành
 * các đợt xen kẽ theo lệch pha riêng của từng nhà — không phải cả làng cùng
 * nhóm bếp một lúc.
 */
const CHIMNEY_DUTY_CYCLE_TICKS = 200;
const CHIMNEY_DUTY_ACTIVE_TICKS = 70;

/** Trần mặc định khi chỗ gọi không truyền `limit`. */
const DEFAULT_CHIMNEY_LIMIT = 5;

/**
 * Những ô nên nhả khói lúc này. Trả về tập con, đã giới hạn số lượng.
 *
 * Ba lớp điều kiện, đúng thứ tự rẻ dần → đắt dần để sớm bỏ qua ô không đạt:
 * 1. `built` và vật liệu nhìn thấy là ngói — dữ liệu có thật, không phải cờ
 *    ngẫu nhiên (đề bài đòi "một điều kiện có thật trong dữ liệu").
 * 2. Đêm hoặc sáng sớm (`dayPhase(tick)`) — cũng là dữ liệu thật của thế
 *    giới, không phải giờ đồng hồ máy người chơi.
 * 3. Hai cổng băm: ô này có bao giờ là ống khói không (`CHIMNEY_GATE_RATIO`),
 *    và nếu có thì đợt cháy hiện tại của nó có đang chạy không
 *    (`CHIMNEY_DUTY_*`).
 *
 * Vượt `limit` thì tỉa theo hạng băm (`SALT.chimneyRank`), không theo thứ tự
 * quét — cùng lý do `ambient.ts` đã giải thích cho hạt môi trường: cắt theo
 * thứ tự quét sẽ luôn xóa sạch nửa dưới khung nhìn trước, và đường ranh giới
 * chạy theo camera.
 */
export function chimneys(
  batch: TileBatch,
  tick: number,
  limit: number = DEFAULT_CHIMNEY_LIMIT,
): { x: number; y: number }[] {
  const w = Math.floor(batch.w);
  const h = Math.floor(batch.h);
  const cap = Number.isFinite(limit) ? Math.floor(limit) : 0;
  if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0 || cap <= 0) return [];

  const t = Number.isFinite(tick) ? Math.floor(tick) : 0;
  const phase = dayPhase(t);
  if (phase !== "night" && phase !== "dawn") return [];

  const xs: number[] = [];
  const ys: number[] = [];
  const ranks: number[] = [];

  for (let gy = 0; gy < h; gy++) {
    for (let gx = 0; gx < w; gx++) {
      const i = gy * w + gx;
      if ((batch.built[i] ?? 0) !== 1) continue;

      const material = batch.material[i] ?? "air";
      // Ghost lớp dưới, cùng quy tắc `terrain.ts §18.1`/`ambient.ts`: khi lát
      // đang xem là không khí, vật liệu người chơi *thấy* là mặt đất bên
      // dưới — mái nhà nhìn từ trên xuống luôn hiện qua đường này.
      const visible = material !== "air" ? material : (batch.surface[i] ?? "air");
      if (!ROOF_MATERIALS.has(visible)) continue;

      const wx = batch.x + gx;
      const wy = batch.y + gy;
      if (hash2(wx, wy, SALT.chimneyGate) >= CHIMNEY_GATE_RATIO) continue;

      const offset = Math.floor(hash2(wx, wy, SALT.chimneyDuty) * CHIMNEY_DUTY_CYCLE_TICKS);
      if (safeMod(t + offset, CHIMNEY_DUTY_CYCLE_TICKS) >= CHIMNEY_DUTY_ACTIVE_TICKS) continue;

      xs.push(wx);
      ys.push(wy);
      ranks.push(hash2(wx, wy, SALT.chimneyRank));
    }
  }

  if (xs.length <= cap) return xs.map((x, i) => ({ x, y: ys[i] as number }));

  const order = xs.map((_, i) => ({ i, rank: ranks[i] ?? 0 }));
  order.sort((a, b) => a.rank - b.rank || a.i - b.i);
  const out: { x: number; y: number }[] = [];
  for (let k = 0; k < cap; k++) {
    const o = order[k];
    if (o) out.push({ x: xs[o.i] as number, y: ys[o.i] as number });
  }
  return out;
}

/**
 * Vòng đời một hạt khói, tính bằng mili giây thực.
 *
 * Xuất ra vì `world.ts` cần con số này để dựng dòng khói liên tục: sinh một
 * hạt mới mỗi khoảng bằng một phần của `SMOKE_LIFE_MS` thì luôn có 3–5 hạt
 * sống chồng lên nhau, đúng như đề bài tả — mà không cần giữ một danh sách hạt
 * đang sống ở đâu cả (xem lời giải ở `world.ts`).
 */
export const SMOKE_LIFE_MS = 2_200;

const SMOKE_START_ALPHA = 0.4;
/** Hạt bay lên tối đa bằng ngần này ô trong suốt vòng đời — "bay lên", không "bay mất". */
const SMOKE_RISE = 0.9;
/** Trôi ngang tối đa, ô — nhỏ hơn hẳn `SMOKE_RISE` vì khói bốc lên là chính, tạt ngang chỉ là gió nhẹ. */
const SMOKE_DRIFT = 0.35;
const SMOKE_R0 = 0.1;
const SMOKE_R1 = 0.3;

/**
 * Một hạt khói tại thời điểm `ageMs` sau khi sinh: vị trí lệch, bán kính, độ mờ.
 *
 * `seed` là một số bất kỳ chỗ gọi tự chọn để nhiều hạt cùng một ống khói trôi
 * theo hướng khác nhau — không cần là toạ độ, chỉ cần khác nhau giữa các hạt
 * (`world.ts` trộn toạ độ ống khói với chỉ số hạt). Vòng đời khép kín tại
 * `SMOKE_LIFE_MS`: quá đó, hoặc `ageMs` âm (hạt chưa sinh), trả `null` để chỗ
 * gọi biết đường không vẽ gì cả — không có một hạt "vẽ ra một khung hình rỗng
 * lặng lẽ".
 *
 * `dx`/`dy`/`r` tính theo **đơn vị ô** (một ô = 1), không theo pixel: cùng lý
 * do `ambient.ts` để `scale` là bội số chứ không phải pixel tuyệt đối — phía
 * vẽ nhân với `tileSize` hiện tại, nên phóng to/nhỏ không cần công thức khác.
 */
export function smokePuff(
  seed: number,
  ageMs: number,
): { dx: number; dy: number; r: number; alpha: number } | null {
  if (!Number.isFinite(seed) || !Number.isFinite(ageMs)) return null;
  if (ageMs < 0 || ageMs > SMOKE_LIFE_MS) return null;

  const t = ageMs / SMOKE_LIFE_MS;
  // Hướng trôi ngang: một hạt băm ra `[-1, 1)` từ chính `seed`, không phải từ
  // toạ độ — giữ đúng hợp đồng "seed là số bất kỳ", không đòi nó phải là toạ độ.
  const dir = hash2(Math.trunc(seed), 0, SALT.smokeDrift) * 2 - 1;

  return {
    dx: dir * SMOKE_DRIFT * t,
    dy: -SMOKE_RISE * t,
    r: SMOKE_R0 + (SMOKE_R1 - SMOKE_R0) * t,
    // 0.4 → 0, tuyến tính: đề bài chốt đúng hai đầu này, và một đường cong
    // khác (vd. bậc hai) sẽ khiến hạt "biến mất đột ngột" gần cuối đời thay vì
    // mờ dần đều.
    alpha: SMOKE_START_ALPHA * (1 - t),
  };
}
