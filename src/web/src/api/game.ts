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
  /**
   * Ô mà cái nhìn của vị thần đang đặt vào, dạng `[x, y]`.
   *
   * **Không** phải một thực thể. Người chơi là một true god: không thân xác,
   * không tọa độ trong thế giới. Trước đây chỗ này là `avatar: string`, và câu
   * hỏi đầu tiên người chơi hỏi khi nhìn màn hình là *"tại sao mặc định true
   * god lại có cơ thể?"*.
   */
  eye: [number, number];
  z: number;
  view_radius: number;
  event_cursor: number;
  /** Tốc độ thời gian, phần nghìn. `1000` là ×1, `0` là tạm dừng. */
  speed_milli: number;
  max_speed_milli: number;
  /** Tổng số bước còn lại của mọi kế hoạch đi đang chạy. */
  steps_remaining: number;
}

/**
 * Một lô ô, trả về dạng **cột song song** thay vì mảng object.
 *
 * `{material: [...], biome: [...]}` thay vì `[{material, biome}, ...]`: một
 * vùng 87×41 là hơn 3500 ô, và dạng object lặp tên khóa 3500 lần.
 *
 * Đây là kiểu **sau khi giải mã** — hình dạng mà `render/terrain.ts`,
 * `render/minimap.ts`, `render/overlays/field.ts` và `App.vue` đã quen dùng.
 * Trên dây, `material`/`surface`/`biome` giờ đi qua một bảng chỉ mục (xem
 * [`decodeTiles`]); `api.tiles` giải mã trước khi trả về, nên bốn chỗ gọi kể
 * trên không phải đổi gì.
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

/**
 * Hình dạng trên dây của `/api/tiles` ở định dạng **mới**: `material`,
 * `surface`, `biome` là chỉ mục nhỏ trỏ vào bảng `names` dùng chung, thay vì
 * ba mảng chuỗi lặp lại. Xem tài liệu `tiles` ở `mow-server/src/api.rs` cho lý
 * do, và [`decodeTiles`] cho cách giải mã.
 */
interface TileBatchWire {
  x: number;
  y: number;
  w: number;
  h: number;
  z: number;
  /** Bảng tra: chỉ mục -> chuỗi. Vài chục phần tử, không phải vài nghìn. */
  names: string[];
  material: number[];
  surface: number[];
  biome: number[];
  drop: number[];
  built: number[];
  height: number[];
  river: number[];
}

/**
 * Giải một mảng chỉ mục thành mảng chuỗi qua bảng `names`.
 *
 * Vòng lặp trần, không `map`: có ba trường (`material`, `surface`, `biome`)
 * nhân với hàng nghìn ô mỗi khung nhìn, và mục tiêu chính của việc đổi định
 * dạng này là giảm việc CPU phải làm mỗi khung — dựng thêm mảng trung gian
 * qua `map` lồng nhau đi ngược lại đúng mục tiêu đó.
 */
function resolveNames(indices: number[], names: string[]): string[] {
  const out = new Array<string>(indices.length);
  for (let i = 0; i < indices.length; i++) {
    const idx = indices[i];
    // Chỉ mục ngoài phạm vi bảng phải hiện ra được, không được là `undefined`:
    // một `undefined` lọt vào tầng vẽ sẽ thành một màu tím không giải thích
    // được, và người ta sẽ đổ lỗi cho renderer thay vì cho bảng chỉ mục sai.
    // (`idx` tự nó cũng có thể `undefined` nếu `indices` thưa hơn khai báo —
    // cùng một lý do, cùng một cách xử lý.)
    out[i] = (idx === undefined ? undefined : names[idx]) ?? "?";
  }
  return out;
}

/**
 * Giải mã một `TileBatch` từ dây.
 *
 * ## Dây thay đổi, kiểu ở client thì không
 *
 * Server giờ gửi một bảng chỉ mục `names` cộng ba mảng số thay vì ba mảng
 * chuỗi lặp lại (xem tài liệu hàm `tiles` ở `api.rs`). Hàm này giải mã ngược
 * lại **ngay tại đây**, để bốn chỗ dùng `TileBatch` — `render/terrain.ts`,
 * `render/minimap.ts`, `render/overlays/field.ts`, `App.vue` — không phải đổi
 * một dòng nào: chúng vẫn nhận đúng `material`/`surface`/`biome: string[]`
 * như trước.
 *
 * Nghe có vẻ mất hết cái lợi của việc đổi định dạng — không phải. Cái lợi nằm
 * ở **băng thông trên dây** và **thời gian `JSON.parse`**: với một khung nhìn
 * ~4000 ô, JSON kiểu cũ buộc trình duyệt dựng lại hàng nghìn bản sao của vài
 * chục chuỗi, còn JSON kiểu mới chỉ cần gửi và parse vài chục chuỗi cộng vài
 * nghìn số nguyên nhỏ. Cả hai cái lợi đó đã thu được **trước khi** hàm này
 * chạy; bước giải mã ở đây chỉ là một lượt tra bảng rẻ, không phải chỗ tiền
 * tiết kiệm bị tiêu hết.
 *
 * Bước kế tiếp — khi `render/*` rảnh tay để đổi theo — là bỏ hẳn bước giải mã
 * này và để chỉ mục chạy thẳng tới tầng vẽ (renderer tự tra `names` lúc vẽ),
 * tiết kiệm luôn phần dựng lại ba mảng chuỗi ở đây.
 *
 * ## Vẫn đọc được định dạng cũ
 *
 * Trong lúc phát triển, client và server không phải lúc nào cũng khởi động
 * lại cùng nhau — một server cũ (ba mảng chuỗi thẳng, không có `names`) có
 * thể đang chạy trong khi client mới đã nạp. Không kiểm thì kết quả là một
 * màn hình trắng không nói cho ai biết vì sao. Nhận diện hai định dạng bằng
 * sự có mặt của `names`: định dạng mới luôn có nó, định dạng cũ thì không.
 */
export function decodeTiles(raw: unknown): TileBatch {
  const r = raw as Record<string, unknown>;
  if (!Array.isArray(r.names)) {
    // Định dạng cũ: `material`/`surface`/`biome` đã là chuỗi sẵn, không có gì
    // để giải mã.
    return r as unknown as TileBatch;
  }
  const batch = r as unknown as TileBatchWire;
  return {
    x: batch.x,
    y: batch.y,
    w: batch.w,
    h: batch.h,
    z: batch.z,
    material: resolveNames(batch.material, batch.names),
    surface: resolveNames(batch.surface, batch.names),
    biome: resolveNames(batch.biome, batch.names),
    drop: batch.drop,
    built: batch.built,
    height: batch.height,
    river: batch.river,
  };
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

  tiles: async (x: number, y: number, w: number, h: number, z: number): Promise<TileBatch> => {
    // Dây trả về chỉ mục, không phải chuỗi (xem `decodeTiles`); giải mã ở
    // đây để mọi chỗ gọi `api.tiles` không phải biết chuyện đó.
    const raw = await getJson<unknown>(`/api/tiles?x=${x}&y=${y}&w=${w}&h=${h}&z=${z}`);
    return decodeTiles(raw);
  },

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
   * Ra lệnh cho **một cư dân** đi tới một ô.
   *
   * `who` là bắt buộc, và đó là điểm khác cốt lõi so với bản trước: vị thần
   * không có chân, nên "đi tới đó" luôn là một mệnh lệnh gửi cho ai đó. Server
   * tính đường và giữ kế hoạch; mỗi bước vẫn là một `core.walk` riêng nên luật
   * thế giới vẫn chặn được từng bước.
   */
  guide: (who: string, x: number, y: number) =>
    postJson<{
      steps: number;
      outcome: string;
      walkable: boolean;
      /** Đường đi đã lên kế hoạch, để vẽ ra cho người chơi thấy. */
      path: [number, number][];
    }>("/api/goto", { who, x, y }),

  /**
   * Dời cái nhìn của vị thần tới một ô.
   *
   * Không ghi sự kiện nào: `§P6.8` xếp camera vào **truy vấn khung nhìn**, và
   * một nhật ký đầy "thần đã nhìn sang trái" là một nhật ký không còn đọc được.
   */
  look: (x: number, y: number) => postJson<{ eye: [number, number] }>("/api/look", { x, y }),

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

  /** Thu hồi mệnh lệnh đi của một cư dân. */
  halt: (who: string) =>
    postJson<{ steps: number; outcome: string }>("/api/goto", {
      who,
      x: 0,
      y: 0,
      cancel: true,
    }),

  /**
   * Khởi nguyên một thế giới mới từ seed.
   *
   * Seed đi ra dạng **chuỗi**: `§22.10` cấm cho `u64` đi qua `Number` của
   * JavaScript, và một seed bị làm tròn vẫn là một seed **hợp lệ** — chỉ là của
   * một thế giới khác, nên lỗi này không bao giờ tự lộ ra.
   */
  genesis: (seed: string) =>
    postJson<{ seed: string; state_hash: string; content_error?: string }>("/api/genesis", {
      seed,
    }),

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
