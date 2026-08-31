/**
 * Chế độ quan sát: bám theo một cư dân và đọc dòng đời của riêng họ (PG-14).
 *
 * ## Vì sao module này thuần
 *
 * Không Vue, không DOM, không `fetch`. `ObservePanel.vue` chỉ là lớp vẽ mỏng
 * gọi vào đây; mọi quyết định — ai được tính là "liên quan", camera có nên
 * giật hay không — nằm ở hàm thuần để test được bằng dữ liệu bịa, không cần
 * dựng cả Vue lẫn server giả.
 *
 * ## Lỗ hổng thật trong dữ liệu: không có "chủ thể"
 *
 * `WorldEvent` (`api/game.ts`) chỉ mang `actor: string | null` và
 * `payload: unknown`. Engine phía server có khái niệm "chịu tác động lên ai"
 * (chủ thể của hành động, phân biệt với người gây ra) nhưng trường đó **không
 * được gửi qua `/api/events`** — endpoint chỉ trả `seq, tick, kind, actor,
 * payload`. Hệ quả: khi True God ban no đủ cho một người (`truegod.set_attr`),
 * sự kiện `truegod.intervened` sinh ra có `actor: null` và payload chỉ chứa
 * `{key, provenance}` — **không có cách nào từ dữ liệu client biết ai vừa bị
 * can thiệp**. Đây không phải lỗi của module này; đó là giới hạn thật của dây
 * truyền hiện tại, và giả vờ suy ra được người đó là ai sẽ là đúng thứ
 * `§22.17` cấm: một chuỗi đoán ra tệ hơn không có, vì người xem sẽ tin nó.
 *
 * Cách xoay xở: chỉ tin **cấu trúc có thật** trên dây.
 * 1. `actor === id` → chắc chắn, người này đã gây ra sự kiện.
 * 2. Định danh đi trong payload dưới đúng khuôn `{ entity: "<id>" }` — quy ước
 *    dây mà server dùng cho *mọi* `Value::Uint` (xem chú thích `entityRef`
 *    trong `api/game.ts`) — là một tham chiếu có kiểu, dù nó nằm ở khóa nào.
 *    Tín hiệu này mạnh: không object nào tình cờ có đúng hình dạng ấy.
 * 3. Định danh xuất hiện trần trụi (chuỗi con) ở đâu đó trong payload mà
 *    không theo khuôn trên là tín hiệu yếu — có thể chỉ là trùng giá trị.
 *    Xếp vào "bystander" thay vì "subject" để không tự tin quá mức.
 * Với bộ sự kiện hiện tại (đi, nói, nhặt, ăn, định làm, True God can thiệp),
 * không payload nào thực sự nhúng tham chiếu thực thể — nên `subject` và
 * `bystander` hôm nay chủ yếu là chỗ trống chờ content pack tương lai
 * (`§19.7`) điền vào, không phải suy diễn của module này.
 */

import type { WorldEvent } from "@/api/game";

/** Một mắt trong dòng đời của một cư dân. */
export interface LifeEntry {
  seq: number;
  tick: number;
  kind: string;
  /** Câu tóm tắt đã sẵn để hiện, đã bỏ tiền tố namespace cho dễ đọc. */
  text: string;
  /** Người này là chủ thể hay chỉ là người chứng kiến. */
  role: "subject" | "actor" | "bystander";
}

/** Bỏ tiền tố namespace (`core.`, `npc.`, …) khỏi `kind` để hiện gọn hơn. */
function stripNamespace(kind: string): string {
  const dot = kind.indexOf(".");
  return dot < 0 ? kind : kind.slice(dot + 1);
}

/**
 * Payload có nhắc tới `id` không, và nhắc theo kiểu nào.
 *
 * `"ref"`: nằm trong một object đúng-một-khóa `{ entity: id }` — quy ước dây
 * cho tham chiếu định danh, xem chú thích đầu file.
 * `"mention"`: chuỗi `id` xuất hiện trần trụi ở đâu đó — tín hiệu yếu.
 *
 * Chỉ quét nông (mặc định 3 tầng): payload thật hôm nay phẳng, và quét không
 * đáy trên dữ liệu không kiểm soát được (content pack tương lai) là chỗ để
 * một payload tự tham chiếu treo cả vòng lặp.
 */
function payloadMentions(payload: unknown, id: string, depth = 3): "ref" | "mention" | null {
  if (typeof payload === "string") return payload === id ? "mention" : null;
  if (depth <= 0 || payload === null || typeof payload !== "object") return null;

  if (Array.isArray(payload)) {
    for (const item of payload) {
      const hit = payloadMentions(item, id, depth - 1);
      if (hit) return hit;
    }
    return null;
  }

  const obj = payload as Record<string, unknown>;
  const keys = Object.keys(obj);
  if (keys.length === 1 && keys[0] === "entity" && obj["entity"] === id) return "ref";

  for (const k of keys) {
    const hit = payloadMentions(obj[k], id, depth - 1);
    if (hit) return hit;
  }
  return null;
}

/** Vai của `id` trong một sự kiện, hoặc `null` nếu không liên quan. */
function roleIn(id: string, e: WorldEvent): LifeEntry["role"] | null {
  if (e.actor === id) return "actor";
  const hit = payloadMentions(e.payload, id);
  if (hit === "ref") return "subject";
  if (hit === "mention") return "bystander";
  return null;
}

/**
 * Lọc dòng sự kiện chung xuống còn dòng đời của một người.
 *
 * Giữ **thứ tự thời gian giảm dần** (mới nhất trước) bất kể thứ tự đầu vào —
 * `events` tới từ `App.vue` đã dồn theo lô, không đảm bảo đã sắp toàn cục.
 * Không sửa mảng đầu vào: đây là dữ liệu chia sẻ với vòng lặp `refresh()`.
 */
export function lifeOf(id: string, events: WorldEvent[], limit?: number): LifeEntry[] {
  const out: LifeEntry[] = [];
  for (const e of events) {
    const role = roleIn(id, e);
    if (!role) continue;
    out.push({ seq: e.seq, tick: e.tick, kind: e.kind, text: stripNamespace(e.kind), role });
  }
  out.sort((a, b) => b.seq - a.seq);
  return limit === undefined ? out : out.slice(0, limit);
}

/**
 * Bán kính coi là "chưa nhích đi đâu" quanh camera, tính bằng ô.
 *
 * `1.5` bọc trọn một bước đi một ô kể cả theo đường chéo (`√2 ≈ 1.41`) — đúng
 * bước cơ bản nhất mà `core.walk` tạo ra. Không có vùng chết này, mỗi tick
 * NPC bước một ô là một lần camera giật, và đó là lý do người ta tắt bám theo.
 */
const DEFAULT_DEAD_ZONE = 1.5;

/**
 * Khoảng cách coi là "đã mất dấu", tính bằng ô.
 *
 * Giữa hai lần `refresh()` một NPC đi bộ chỉ dịch vài ô; lệch tới hai chữ số
 * ô nghĩa là mục tiêu vừa bị dời tức thời (True God, đổi lát `z`, đổi người
 * đang bám) chứ không phải bước đi bình thường — trôi tới đó sẽ mất nhiều
 * giây nhìn camera lừ đừ bay ngang bản đồ, tệ hơn một cú nhảy.
 */
const DEFAULT_SNAP_DISTANCE = 10;

/** Hệ số đóng khoảng cách còn lại mỗi bước, khi đang trôi (không giật, không dán chết). */
const CLOSE_RATE = 0.25;

/** Dịch một trục về phía `to`, làm tròn ô, không bao giờ vượt qua `to`. */
function stepToward(from: number, to: number, rate: number): number {
  if (from === to) return from;
  const rounded = Math.round(from + (to - from) * rate);
  // Làm tròn có thể trả đúng ô cũ khi khoảng cách còn nhỏ (ví dụ còn 2 ô,
  // rate 0.25 → 0.5 → làm tròn về 0). Ép tiến đúng một ô theo đúng hướng để
  // camera luôn nhích tới thay vì đứng hình sát rìa vùng chết.
  const stepped = rounded === from ? from + Math.sign(to - from) : rounded;
  return to > from ? Math.min(stepped, to) : Math.max(stepped, to);
}

/**
 * Camera có nên nhảy tới không, hay chỉ trôi theo.
 *
 * Ba vùng, theo khoảng cách Euclid giữa `cam` và `target`:
 * - trong vùng chết (`<= deadZone`) → đứng yên, `snapped: false`.
 * - vượt `snapDistance` → nhảy thẳng tới `target`, `snapped: true`.
 * - ở giữa → trôi một phần quãng đường mỗi lần gọi, `snapped: false`.
 *
 * Tọa độ trả về luôn là số nguyên ô: làm tròn ở đây, một lần, để nơi gọi vẽ
 * thẳng lên canvas mà không phải tự làm tròn lại — làm tròn ở hai chỗ khác
 * nhau là cách chắc chắn để camera rung theo sai số nửa pixel.
 */
export function followStep(
  cam: { x: number; y: number },
  target: { x: number; y: number },
  opts: { snapDistance?: number; deadZone?: number } = {},
): { x: number; y: number; snapped: boolean } {
  const deadZone = opts.deadZone ?? DEFAULT_DEAD_ZONE;
  const snapDistance = opts.snapDistance ?? DEFAULT_SNAP_DISTANCE;
  const dist = Math.hypot(target.x - cam.x, target.y - cam.y);

  if (dist <= deadZone) {
    return { x: Math.round(cam.x), y: Math.round(cam.y), snapped: false };
  }
  if (dist > snapDistance) {
    return { x: Math.round(target.x), y: Math.round(target.y), snapped: true };
  }
  return {
    x: stepToward(Math.round(cam.x), Math.round(target.x), CLOSE_RATE),
    y: stepToward(Math.round(cam.y), Math.round(target.y), CLOSE_RATE),
    snapped: false,
  };
}
