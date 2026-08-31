/**
 * Tọa độ ở biên JavaScript — `idea.md §4.3`, `§22.10`.
 *
 * Bất biến phải giữ:
 *
 * > Tọa độ 64-bit không bị ép qua JavaScript `Number` mất chính xác.
 *
 * `Number` của JS là `f64`, biểu diễn chính xác số nguyên tới `2^53`. Thế giới
 * này dùng `i64`, tức tới `2^63`. Khoảng giữa hai con số đó là nơi mọi thứ hỏng
 * một cách im lặng: `9007199254740993` đọc vào thành `9007199254740992`, và một
 * thực thể ở ô đó bỗng đứng ở ô bên cạnh.
 *
 * Điều tệ nhất về lỗi này là nó **không xảy ra ở gần gốc tọa độ**. Toàn bộ quá
 * trình phát triển diễn ra quanh `(0, 0)` nơi mọi thứ hoàn hảo. Lỗi chỉ lộ ra
 * khi ai đó đi thật xa, tức là sau khi phát hành.
 *
 * Hai quy tắc, và cả hai đều được kiểm bằng test:
 *
 * 1. **Trên đường truyền và trong lưu trữ: `bigint`.** Không bao giờ `number`.
 * 2. **Khi vẽ: tọa độ tương đối camera, dạng `number`.** Hiệu giữa hai điểm gần
 *    nhau thì nhỏ, nên `number` an toàn — và WebGL chỉ nhận `f32` nên không có
 *    lựa chọn nào khác.
 *
 * Cầu nối giữa hai thế giới đó là **floating origin**: một điểm gốc `bigint`
 * được dời đi khi camera đi xa, để tọa độ cục bộ luôn nhỏ.
 */

/** Giới hạn số nguyên mà `number` còn biểu diễn chính xác. */
export const SAFE_INT = BigInt(Number.MAX_SAFE_INTEGER);

/**
 * Bán kính tối đa của tọa độ cục bộ trước khi phải dời gốc.
 *
 * `2^22` ô. Chọn con số này vì `f32` — kiểu mà WebGL dùng cho vertex — có 24
 * bit định trị, nên tới `2^22` vẫn còn hai bit dự phòng cho phần thập phân của
 * vị trí trong ô. Lớn hơn nữa thì sprite bắt đầu giật khi camera di chuyển.
 */
export const REBASE_RADIUS = 1 << 22;

/** Một điểm trong thế giới. `x` và `y` là `bigint` — không thương lượng. */
export interface WorldPoint {
  x: bigint;
  y: bigint;
  /** Tầng cao độ. Nhỏ, nên `number` là đủ. */
  z: number;
}

/** Tọa độ tương đối camera, dùng để vẽ. */
export interface LocalPoint {
  x: number;
  y: number;
  z: number;
}

/** Lỗi khi một tọa độ vượt khỏi khoảng an toàn. */
export class CoordRangeError extends Error {
  constructor(
    public readonly value: bigint,
    context: string,
  ) {
    super(
      `tọa độ ${value} vượt khoảng an toàn của Number trong ${context}. ` +
        `Dùng bigint ở biên và tọa độ camera-local khi vẽ (§22.10).`,
    );
    this.name = "CoordRangeError";
  }
}

/**
 * Đọc một tọa độ từ JSON.
 *
 * Nhận chuỗi hoặc `bigint`. **Không nhận `number`** — và đó là điểm mấu chốt:
 * nếu hàm này nhận `number`, thì mất chính xác đã xảy ra ở bước `JSON.parse`,
 * trước khi ta có cơ hội can thiệp. Server phải gửi tọa độ dưới dạng chuỗi.
 */
export function parseCoord(v: unknown, context = "payload"): bigint {
  if (typeof v === "bigint") return v;
  if (typeof v === "string") return BigInt(v);
  if (typeof v === "number") {
    throw new TypeError(
      `${context}: tọa độ tới dưới dạng number. Mất chính xác đã xảy ra ở ` +
        `JSON.parse rồi — server phải gửi tọa độ dưới dạng chuỗi (§22.10).`,
    );
  }
  throw new TypeError(`${context}: không đọc được tọa độ từ ${typeof v}`);
}

/** Ghi một tọa độ ra JSON. Luôn là chuỗi. */
export function formatCoord(v: bigint): string {
  return v.toString();
}

/** Đọc một điểm từ payload. */
export function parsePoint(v: unknown, context = "point"): WorldPoint {
  if (typeof v !== "object" || v === null) {
    throw new TypeError(`${context}: không phải object`);
  }
  const o = v as Record<string, unknown>;
  return {
    x: parseCoord(o.x, `${context}.x`),
    y: parseCoord(o.y, `${context}.y`),
    z: typeof o.z === "number" ? o.z : Number(parseCoord(o.z ?? 0, `${context}.z`)),
  };
}

/**
 * Gốc tọa độ trôi (floating origin) — `§18.4`.
 *
 * Giữ một điểm gốc `bigint`; mọi thứ được vẽ ở tọa độ tương đối với nó. Khi
 * camera đi quá xa gốc, gốc được **dời** tới vị trí camera và toàn bộ scene
 * được vẽ lại. Người xem không thấy gì — nhưng nếu không làm, mọi thứ sẽ bắt
 * đầu giật khi đi đủ xa.
 */
export class FloatingOrigin {
  #origin: WorldPoint;
  #generation = 0;

  constructor(origin: WorldPoint = { x: 0n, y: 0n, z: 0 }) {
    this.#origin = origin;
  }

  /** Gốc hiện tại. */
  get origin(): WorldPoint {
    return this.#origin;
  }

  /**
   * Số lần gốc đã dời.
   *
   * Renderer so số này với lần vẽ trước; khác nghĩa là **mọi vị trí đã lưu
   * trong bộ đệm đều sai** và phải tính lại. Không có nó, một sprite được đặt
   * trước lần dời gốc sẽ nhảy đi hàng triệu pixel.
   */
  get generation(): number {
    return this.#generation;
  }

  /**
   * Đổi một điểm thế giới sang tọa độ cục bộ.
   *
   * Ném [`CoordRangeError`] nếu điểm nằm quá xa gốc — đó là lỗi lập trình
   * (quên gọi [`FloatingOrigin.recenter`]), không phải trạng thái hợp lệ.
   */
  toLocal(p: WorldPoint): LocalPoint {
    const dx = p.x - this.#origin.x;
    const dy = p.y - this.#origin.y;
    const gioi_han = BigInt(REBASE_RADIUS) * 2n;
    if (dx > gioi_han || dx < -gioi_han) throw new CoordRangeError(dx, "toLocal.x");
    if (dy > gioi_han || dy < -gioi_han) throw new CoordRangeError(dy, "toLocal.y");
    return { x: Number(dx), y: Number(dy), z: p.z - this.#origin.z };
  }

  /** Đổi ngược từ tọa độ cục bộ về tọa độ thế giới. */
  toWorld(p: LocalPoint): WorldPoint {
    return {
      x: this.#origin.x + BigInt(Math.trunc(p.x)),
      y: this.#origin.y + BigInt(Math.trunc(p.y)),
      z: this.#origin.z + p.z,
    };
  }

  /**
   * Kiểm tra camera có cần dời gốc không, và dời nếu cần.
   *
   * Trả `true` nếu đã dời — chỗ gọi phải vẽ lại toàn bộ.
   */
  recenterIfNeeded(camera: WorldPoint): boolean {
    const dx = camera.x - this.#origin.x;
    const dy = camera.y - this.#origin.y;
    const r = BigInt(REBASE_RADIUS);
    if (dx > r || dx < -r || dy > r || dy < -r || camera.z !== this.#origin.z) {
      this.#origin = { x: camera.x, y: camera.y, z: camera.z };
      this.#generation += 1;
      return true;
    }
    return false;
  }

  /** Dời gốc tường minh. */
  recenter(to: WorldPoint): void {
    this.#origin = to;
    this.#generation += 1;
  }
}

/**
 * Chunk chứa một tọa độ.
 *
 * `bigint` không có `div_euclid`, và `/` của nó cắt về 0 — cùng cái bẫy như ở
 * Rust. Với `x = -1n` và `size = 32`, phép chia thường cho chunk `0`, nên ô
 * `-1` và ô `0` rơi vào cùng chunk còn ô `-32` thì không. Lưới lệch đúng một ô
 * quanh gốc, và không bài test nào chạy quanh `(0,0)` phát hiện được.
 */
export function chunkOf(v: bigint, size: number): bigint {
  const s = BigInt(size);
  const q = v / s;
  return v % s < 0n ? q - 1n : q;
}

/** Vị trí trong chunk, luôn trong `[0, size)`. */
export function localInChunk(v: bigint, size: number): number {
  const s = BigInt(size);
  const r = v % s;
  return Number(r < 0n ? r + s : r);
}

/** Khóa chunk dùng làm khóa `Map`. */
export function chunkKey(cx: bigint, cy: bigint, cz: number): string {
  return `${cx},${cy},${cz}`;
}
