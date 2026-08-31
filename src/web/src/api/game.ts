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
  /** Tốc độ thời gian, phần nghìn. `1000` là ×1, `0` là tạm dừng. */
  speed_milli: number;
  max_speed_milli: number;
  /** Số bước còn lại trong kế hoạch đi của avatar. */
  steps_remaining: number;
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
  /** `1` nếu ô đã bị ai đó xây/sửa, `0` nếu là địa hình sinh ra. */
  built: number[];
  biome: string[];
  height: number[];
  river: number[];
}

/** Một vật liệu như content pack khai. */
export interface BlockInfo {
  id: string;
  name: { en?: string; vi?: string };
  /** Chuỗi hex `#rrggbb`. */
  color: string;
  liquid: boolean;
  walkable: boolean;
  hardness: number;
  tags: string[];
}

export interface Entity {
  id: string;
  name: string;
  x: number;
  y: number;
  kind: "being" | "item";
  is_avatar: boolean;
  hunger: number | null;
  /** Vai trong làng, nếu là cư dân. */
  role: string | null;
  /** Việc nó đang định làm, và vì sao panel trả lời được câu đó. */
  intent: string | null;
}

export interface WorldEvent {
  seq: number;
  tick: number;
  kind: string;
  actor: string | null;
  payload: unknown;
}

/** Một thực thể sẽ đổi, trong diff xem trước. */
export interface DiffChange {
  id: string;
  name: string;
  from: [number, number] | null;
  to: [number, number] | null;
  moved: boolean;
  attrs: string[];
}

/** Kết quả nhìn trước một ý chỉ. */
export interface Foresight {
  command: string;
  /** Hash thế giới lúc nhìn. Lúc khắc phải mang lại đúng giá trị này. */
  base_hash: string;
  after_hash: string;
  changes_anything: boolean;
  error: string | null;
  events: { kind: string; summary: string }[];
  changes: DiffChange[];
}

/** Một mắt xích trong chuỗi nhân quả. */
export interface CauseLink {
  seq: number;
  tick: number;
  kind: string;
  actor: string | null;
  summary: string;
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

  /** Bảng vật liệu của content pack đang nạp (`§19.7`). */
  blocks: () => getJson<{ blocks: BlockInfo[]; loaded: boolean }>("/api/blocks"),

  events: (after: number) =>
    getJson<{ cursor: number; events: WorldEvent[] }>(`/api/events?after=${after}`),

  /**
   * Đổi tốc độ thời gian. Cũng là **query**: tốc độ không đổi kết quả mô phỏng.
   * Cùng seed chạy ×0.001 hay ×100 vẫn cho cùng `state_hash` ở cùng số tick.
   */
  setSpeed: (speed_milli: number) =>
    postJson<{ speed_milli: number }>("/api/speed", { speed_milli }),

  /**
   * Bấm chuột để đi. Server tính đường và giữ kế hoạch; mỗi bước vẫn là một
   * `core.walk` riêng nên luật thế giới vẫn chặn được từng bước.
   */
  goto: (x: number, y: number) =>
    postJson<{
      steps: number;
      outcome: string;
      walkable: boolean;
      /** Đường đi đã lên kế hoạch, để vẽ ra cho người chơi thấy. */
      path: [number, number][];
    }>("/api/goto", { x, y }),

  /**
   * Truy ngược chuỗi nhân quả của một sự kiện (`§18.10`).
   *
   * Cạnh nhân quả được ghi **lúc tạo** sự kiện, không suy ngược sau — một chuỗi
   * đoán ra thì tệ hơn không có, vì người xem sẽ tin nó.
   */
  causes: (seq: number) => getJson<{ chain: CauseLink[] }>(`/api/cause?seq=${seq}`),

  /** Nhìn trước một ý chỉ. **Không** đổi thế giới. */
  preview: (kind: string, fields: Record<string, unknown>) =>
    postJson<Foresight>("/api/preview", { kind, fields }),

  /**
   * Khắc một ý chỉ vào thế giới.
   *
   * `base_hash` là hash lúc nhìn trước. Server từ chối nếu thế giới đã đổi —
   * thứ được khắc luôn đúng bằng thứ đã nhìn.
   */
  commit: (kind: string, fields: Record<string, unknown>, base_hash: string) =>
    postJson<{
      ok: boolean;
      after_hash?: string;
      changes?: number;
      reason?: string;
      message?: string;
      state_hash?: string;
    }>("/api/commit", { kind, fields, base_hash }),

  /** Khắc địa hình: đổi vật liệu một ô. Quyền năng True God lên vật chất. */
  build: (x: number, y: number, material: string) =>
    postJson<{ ok: boolean; error?: string; built_cells?: number }>("/api/build", {
      x,
      y,
      material,
    }),

  /** Chuột phải: dừng tại chỗ. */
  stop: () =>
    postJson<{ steps: number; outcome: string }>("/api/goto", { x: 0, y: 0, cancel: true }),

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

/**
 * Các nấc tốc độ, phần nghìn.
 *
 * Nấc rời rạc chứ không phải thanh trượt liên tục theo log: người chơi muốn
 * "×1" và "×10", không muốn "×7.4". Nấc cũng làm bàn phím và ảnh chụp màn hình
 * so sánh được với nhau.
 */
export const SPEED_STEPS = [
  { milli: 0, label: "⏸" },
  { milli: 1, label: "x0.001" },
  { milli: 10, label: "x0.01" },
  { milli: 100, label: "x0.1" },
  { milli: 500, label: "x0.5" },
  { milli: 1_000, label: "x1" },
  { milli: 2_000, label: "x2" },
  { milli: 5_000, label: "x5" },
  { milli: 10_000, label: "x10" },
  { milli: 25_000, label: "x25" },
  { milli: 50_000, label: "x50" },
  { milli: 100_000, label: "x100" },
] as const;
