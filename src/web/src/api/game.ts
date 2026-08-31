/**
 * Client của `mow-server`.
 *
 * ## Định danh là **chuỗi**, không phải số
 *
 * `§22.10` cấm ép định danh 64-bit qua `Number` của JS: `2^53 + 1` và
 * `2^53 + 2` là cùng một `Number`, nên hai thực thể khác nhau trở thành một mà
 * không có gì báo. Server gửi chúng dưới dạng chuỗi và client giữ nguyên chuỗi
 * — không parse, không so sánh bằng số.
 *
 * ## Không có optimistic UI
 *
 * `§P6.9.5`: người chơi ra lệnh đi thì avatar dịch chuyển **sau khi engine
 * ack**, không dịch ngay rồi hoàn tác nếu lệnh bị từ chối. Nên mọi hàm ở đây
 * trả về trạng thái sau lệnh, và chỗ gọi vẽ lại từ đó.
 */

/** Địa chỉ server. Cùng origin khi server phục vụ luôn giao diện. */
const ORIGIN =
  (import.meta as { env?: Record<string, string> }).env?.["VITE_MOW_SERVER"] ??
  (globalThis.location?.port === "5173" ? "http://localhost:17777" : "");

export interface WorldMeta {
  world: number;
  seed: string;
  tick: number;
  state_hash: string;
  /** Định danh avatar, dạng chuỗi. */
  avatar: string;
  z: number;
  view_radius: number;
  event_cursor: number;
}

/**
 * Một lô ô, trả về dạng **cột song song** thay vì mảng object.
 *
 * `{material: [...], biome: [...]}` thay vì `[{material, biome}, ...]`: một
 * vùng 87×41 là hơn 3500 ô, và dạng object lặp tên khóa 3500 lần.
 */
export interface TileBatch {
  x: number;
  y: number;
  w: number;
  h: number;
  z: number;
  /** Vật liệu tại lát `z`. */
  material: string[];
  /** Vật liệu của ô rắn trên cùng của cột — dùng vẽ ghost lớp dưới. */
  surface: string[];
  /** Số mét từ lát đang xem xuống mặt đất. */
  drop: number[];
  biome: string[];
  height: number[];
  river: number[];
}

export interface Entity {
  id: string;
  name: string;
  x: number;
  y: number;
  kind: "being" | "item";
  is_avatar: boolean;
  hunger: number | null;
}

export interface WorldEvent {
  seq: number;
  tick: number;
  kind: string;
  actor: string | null;
  payload: unknown;
}

export interface CommandResult {
  ok: boolean;
  tick: number;
  state_hash?: string;
  event_cursor?: number;
  code?: string;
  error?: string;
}

async function getJson<T>(path: string): Promise<T> {
  const r = await fetch(`${ORIGIN}${path}`, { headers: { Accept: "application/json" } });
  if (!r.ok) throw new Error(`${path}: HTTP ${r.status}`);
  return (await r.json()) as T;
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const r = await fetch(`${ORIGIN}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!r.ok) throw new Error(`${path}: HTTP ${r.status}`);
  return (await r.json()) as T;
}

export const api = {
  meta: () => getJson<WorldMeta>("/api/meta"),

  tiles: (x: number, y: number, w: number, h: number, z: number) =>
    getJson<TileBatch>(`/api/tiles?x=${x}&y=${y}&w=${w}&h=${h}&z=${z}`),

  entities: () => getJson<{ entities: Entity[] }>("/api/entities"),

  events: (after: number) =>
    getJson<{ cursor: number; events: WorldEvent[] }>(`/api/events?after=${after}`),

  /** Đổi lát `z`. Là **query**, không ghi vào thế giới (`§P6.8`). */
  setLayer: (z: number) => postJson<{ z: number; state_hash: string }>("/api/view", { z }),

  /**
   * Gửi một lệnh.
   *
   * `fields` phải nói rõ kiểu: `{"entity": N}` cho định danh, số trần cho số
   * nguyên. Server **không đoán**, vì engine phân biệt `Uint` với `Int` và đoán
   * sai ở đây cho ra `wrong_type` rất khó truy.
   */
  command: (kind: string, fields: Record<string, unknown>) =>
    postJson<CommandResult>("/api/command", { kind, fields }),
};

/** Bọc một định danh cho đúng kiểu trên dây. */
export function entityRef(id: string): { entity: number } {
  // `Number` ở đây **an toàn** vì nó đi thẳng vào JSON rồi thành `u64` ở server;
  // nó không bao giờ được dùng để so sánh hay lưu ở phía JS.
  return { entity: Number(id) };
}

/** Bốn hướng đi, khớp bàn phím. */
export const DIRECTIONS: Readonly<Record<string, readonly [number, number]>> = {
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  w: [0, -1],
  s: [0, 1],
  a: [-1, 0],
  d: [1, 0],
  W: [0, -1],
  S: [0, 1],
  A: [-1, 0],
  D: [1, 0],
};
