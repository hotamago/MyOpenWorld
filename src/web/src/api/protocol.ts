/**
 * Giao thức đồng bộ frontend (`plan.md §P6.8`, `idea.md §8.4`).
 *
 * ## Vì sao `ViewSubscription` và `SetSimulationFocus` là hai thứ khác nhau
 *
 * Đây là phân biệt mà `PA-07` tồn tại để bảo vệ, và nó rất dễ bị gộp lại vì hai
 * thứ trông giống nhau: cả hai đều là "người chơi đang nhìn vào đâu".
 *
 * Nhưng chúng có **hệ quả khác hẳn nhau**:
 *
 * | | `ViewSubscription` | `SetSimulationFocus` |
 * |---|---|---|
 * | Nghĩa | "gửi tôi dữ liệu vùng này để vẽ" | "mô phỏng vùng này ở mức chi tiết cao" |
 * | Đổi thế giới? | **không** | **có** |
 * | Ghi event? | **không bao giờ** | **có** |
 * | Ai gửi | renderer, mỗi lần pan/zoom | người chơi, khi chọn nơi để theo dõi |
 *
 * Gộp chúng lại thì **kéo bản đồ sẽ ghi vào nhật ký sự kiện**. Hậu quả không
 * phải là nhật ký hơi dài hơn — mà là:
 *
 * - Replay của cùng một thế giới phụ thuộc vào việc người xem đã kéo chuột thế
 *   nào. Hai lần chạy từ cùng seed cho hai kết quả khác nhau.
 * - `state_hash` đổi khi không có gì trong thế giới đổi, nên determinism
 *   harness đỏ ở mọi lần chạy.
 * - Nhật ký đầy sự kiện camera, và chuỗi nhân quả của một vụ trộm bị chôn giữa
 *   mười nghìn lần cuộn chuột.
 *
 * Quy tắc một câu: **nhìn không phải là một hành động trong thế giới.**
 */

import { formatCoord, parsePoint, type WorldPoint } from "@/worker/coord";

/** Mức chi tiết mô phỏng. */
export type Lod = "active" | "near" | "far";

/** Chế độ nhận thức — lọc ở **server**, không ẩn ở client (`§18.9`, `PC-15`). */
export type EpistemicMode = "embodied" | "observer" | "true_god";

// ─────────────────────────────────────────────────────────────────────────────
// Client → server
// ─────────────────────────────────────────────────────────────────────────────

/**
 * "Gửi tôi dữ liệu của vùng này."
 *
 * Thuần đọc. Đổi nó bao nhiêu lần cũng được, với tần suất nào cũng được, và
 * thế giới không hề biết.
 */
export interface ViewSubscription {
  type: "view_subscription";
  /** Góc trên trái của vùng đang nhìn. */
  min: WorldPoint;
  /** Góc dưới phải. */
  max: WorldPoint;
  /** Mức zoom, để server chọn độ chi tiết của dữ liệu gửi về. */
  zoom: number;
  /** Overlay nào đang bật — server chỉ gửi kênh dữ liệu cần thiết. */
  overlays: string[];
  /** Nhìn thế giới qua mắt ai. */
  mode: EpistemicMode;
  /** Với chế độ hóa thân, nhìn qua mắt thực thể nào. */
  as_entity?: string;
}

/**
 * "Mô phỏng vùng này ở mức chi tiết cao."
 *
 * **Đây là một command.** Nó đổi cách thế giới được mô phỏng, nên nó đi qua
 * transaction handler và để lại một sự kiện trong nhật ký — đúng như mọi thay
 * đổi state khác (`§22.1`).
 */
export interface SetSimulationFocus {
  type: "set_simulation_focus";
  /** Tâm vùng cần mô phỏng chi tiết. */
  center: WorldPoint;
  /** Bán kính, tính bằng chunk. */
  radius_chunks: number;
  /** Mức chi tiết yêu cầu. */
  lod: Lod;
}

/** Điều khiển thời gian (`§18.8`). */
export interface TimeControl {
  type: "time_control";
  action: "pause" | "resume" | "step" | "run_until";
  /** Với `step`: bao nhiêu tick. */
  ticks?: number;
  /** Với `resume`: tốc độ (1, 4, 16...). */
  speed?: number;
  /** Với `run_until`: vị từ dừng. */
  predicate?: string;
}

/** Mọi thông điệp client gửi lên. */
export type ClientMessage = ViewSubscription | SetSimulationFocus | TimeControl;

/**
 * Thông điệp nào là **command** — tức là đổi thế giới và ghi event.
 *
 * Hàm này là chỗ duy nhất trả lời câu hỏi đó, để không ai phải nhớ. Nếu một
 * thông điệp mới được thêm mà quên khai báo ở đây, [`assertReadOnly`] sẽ bắt
 * được lúc chạy test.
 */
export function isCommand(m: ClientMessage): boolean {
  return m.type === "set_simulation_focus" || m.type === "time_control";
}

/**
 * Khẳng định một thông điệp là thuần đọc.
 *
 * Renderer gọi hàm này trước khi gửi bất cứ thứ gì trong vòng lặp vẽ. Nếu một
 * ngày nào đó ai đó gửi một command từ trong `requestAnimationFrame`, lỗi sẽ nổ
 * ngay tại đó chứ không phải sáu tháng sau khi ai đó thắc mắc vì sao nhật ký có
 * ba triệu sự kiện.
 */
export function assertReadOnly(m: ClientMessage): void {
  if (isCommand(m)) {
    throw new Error(
      `\`${m.type}\` là một command — nó đổi thế giới và ghi event. ` +
        `Không được gửi nó từ vòng lặp vẽ. Nhìn không phải là một hành động ` +
        `trong thế giới (§P6.8).`,
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Server → client
// ─────────────────────────────────────────────────────────────────────────────

/** Một chunk địa hình đã mã hóa. */
export interface ChunkPatch {
  type: "chunk";
  cx: string;
  cy: string;
  cz: number;
  /** Chỉ số vật liệu cho từng ô, hàng-trước. */
  materials: Uint16Array;
  /** Phiên bản, để client bỏ qua bản cũ tới muộn. */
  revision: number;
}

/** Delta của thực thể. */
export interface EntityPatch {
  type: "entity";
  id: string;
  /** `null` nghĩa là thực thể đã biến mất khỏi tầm nhìn — **không** phải đã chết. */
  at: WorldPoint | null;
  attrs?: Record<string, string | number | boolean>;
}

/** Một khung dữ liệu overlay (`§18.6`, `PA-12`). */
export interface OverlayPatch {
  type: "overlay";
  channel: string;
  cx: string;
  cy: string;
  /** Một byte mỗi ô. Thang thật nằm ở legend, không ở đây. */
  data: Uint8Array;
  /** Đơn vị thật, để legend hiển thị. Bắt buộc — `§18.6` cấm overlay không đơn vị. */
  unit: string;
  /** Giá trị thật ứng với `0` và `255`. */
  domain: [number, number];
}

/** Nhịp đồng hồ. */
export interface TickInfo {
  type: "tick";
  tick: string;
  divine_tick: string;
  /** Hash state, để client so với repro bundle. */
  state_hash: string;
}

/** Mọi thông điệp server gửi xuống. */
export type ServerMessage = ChunkPatch | EntityPatch | OverlayPatch | TickInfo;

// ─────────────────────────────────────────────────────────────────────────────
// Mã hóa
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Mã hóa một thông điệp client thành JSON.
 *
 * Tọa độ ra dạng **chuỗi**, luôn luôn (`§22.10`). Nếu để `JSON.stringify` tự
 * xử lý `bigint`, nó sẽ ném lỗi — điều đó tốt hơn là im lặng làm tròn, nhưng
 * vẫn không đủ: ta muốn một dạng mã hóa rõ ràng và đối xứng với phía đọc.
 */
export function encodeClient(m: ClientMessage): string {
  const p = (v: WorldPoint) => ({
    x: formatCoord(v.x),
    y: formatCoord(v.y),
    z: v.z,
  });
  switch (m.type) {
    case "view_subscription":
      return JSON.stringify({ ...m, min: p(m.min), max: p(m.max) });
    case "set_simulation_focus":
      return JSON.stringify({ ...m, center: p(m.center) });
    case "time_control":
      return JSON.stringify(m);
  }
}

/** Đọc một thông điệp server. */
export function decodeServer(raw: string): ServerMessage {
  const o = JSON.parse(raw) as Record<string, unknown>;
  switch (o.type) {
    case "entity":
      return {
        type: "entity",
        id: String(o.id),
        at: o.at === null ? null : parsePoint(o.at, "entity.at"),
        ...(o.attrs ? { attrs: o.attrs as Record<string, string | number | boolean> } : {}),
      };
    case "tick":
      return {
        type: "tick",
        tick: String(o.tick),
        divine_tick: String(o.divine_tick),
        state_hash: String(o.state_hash),
      };
    case "overlay": {
      if (typeof o.unit !== "string" || o.unit.length === 0) {
        // `§18.6`: legend bắt buộc kèm đơn vị thật. Một overlay không đơn vị là
        // một bức tranh đẹp mà không ai đọc được, và tệ hơn, người xem sẽ tự
        // bịa ra một thang trong đầu.
        throw new Error("overlay thiếu `unit` — §18.6 cấm overlay không có đơn vị thật");
      }
      return {
        type: "overlay",
        channel: String(o.channel),
        cx: String(o.cx),
        cy: String(o.cy),
        data: new Uint8Array(o.data as number[]),
        unit: o.unit,
        domain: o.domain as [number, number],
      };
    }
    case "chunk":
      return {
        type: "chunk",
        cx: String(o.cx),
        cy: String(o.cy),
        cz: Number(o.cz),
        materials: new Uint16Array(o.materials as number[]),
        revision: Number(o.revision),
      };
    default:
      throw new Error(`thông điệp lạ: ${String(o.type)}`);
  }
}
