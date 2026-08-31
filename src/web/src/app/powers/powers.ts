/**
 * Danh mục quyền năng của True God, hiển thị trên `PowerDock`.
 *
 * ## Vì sao module này thuần
 *
 * Không Vue, không `fetch`, không import từ `App.vue`/`i18n/index.ts`/`api/*`/
 * `render/*` — những chỗ đó đang được sửa song song bởi người khác lúc nhiệm
 * vụ này ra đời. Đứng một mình nghĩa là module này không thể vỡ vì một refactor
 * ở chỗ khác, và test được bằng dữ liệu bịa tay, không cần dựng Vue lẫn server
 * giả (cùng tinh thần với `../observe.ts`).
 *
 * ## "Quyền năng" không phải "lệnh"
 *
 * Một quyền năng là một **ý định chơi được** ("ban no đủ cho người này"); lệnh
 * là thứ dây thật hiểu (`truegod.set_attr` với `key: "need.hunger"`). Tách hai
 * khái niệm để danh mục đọc như luật chơi chứ không như một bảng tra API, và
 * để `fieldsFor` là **chỗ duy nhất** biết dịch từ ý định sang dây — thêm hay
 * sửa một quyền năng không bao giờ phải đụng vào UI.
 *
 * ## Sự thật khó chịu: "dời tổ ấm" không phải một lệnh
 *
 * `truegod.set_attr` đặt đúng **một** khóa mỗi lần gọi. Dời nhà một cư dân đổi
 * cả `npc.home.x` lẫn `npc.home.y` — hai khóa, nên đúng ra phải là hai lệnh.
 * Giả vờ một quyền năng "dời tổ ấm" tự lo cả hai trục bằng một lần khắc là bịa
 * ra một khả năng mà dây không có. Danh mục dưới đây tách thành hai quyền năng
 * trục riêng (`mind.uproot_x`, `mind.uproot_y`) — Người khắc, và nhìn trước,
 * từng trục một, đúng như dây thật cho phép. Cùng lý do cho "đổi chỗ làm"
 * (`mind.reassign_x`/`mind.reassign_y`).
 */

/** Nhóm để xếp quyền năng trên `PowerDock` — mỗi nhóm một nhãn nhỏ. */
export type PowerGroup = "land" | "body" | "mind" | "time" | "sight";

/** Thứ mà một quyền năng cần trỏ vào trước khi thi hành được. */
export type PowerNeeds = "none" | "being" | "tile" | "being_and_tile";

/**
 * Cách một quyền năng chạm vào thế giới — khớp đúng bốn nhóm hàm có thật ở
 * `api/game.ts` (đọc `src/web/src/api/game.ts`). Không có cách thứ năm, vì
 * không có gì để gọi thêm.
 *
 * `fieldsFor` trả về đúng hình dạng mà mỗi cách gọi cần, để chỗ tích hợp (nơi
 * thật sự gọi `api.*`, không phải module này) không phải đoán:
 * - `command`: gửi thẳng `api.command(effect.kind, fields)`.
 * - `preview`: `api.preview(effect.kind, fields)` rồi `api.commit(effect.kind,
 *   fields, base_hash)` khi Người ưng ý — `fields` giống hệt cho cả hai lời gọi.
 * - `build`: gọi `api.build(fields.x, fields.y, fields.material)`.
 * - `guide`: gọi `api.guide(fields.who, fields.x, fields.y)`.
 * - `view`: không lệnh nào được gửi tới server; `fields` chỉ mang tham số đã
 *   chọn (ví dụ lớp dữ liệu, lát z) để phía tích hợp tự quyết định vẽ gì.
 */
export type PowerEffect =
  | { via: "command"; kind: string }
  | { via: "preview"; kind: string }
  | { via: "build" }
  | { via: "guide" }
  | { via: "view" };

/** Một tham số người chơi chọn được trước khi thi hành quyền năng. */
export interface PowerParam {
  key: string;
  kind: "int" | "choice" | "text";
  /** Với `int`. */
  min?: number;
  max?: number;
  step?: number;
  def?: number;
  /** Với `choice`. */
  options?: string[];
}

export interface Power {
  id: string;
  /** Nhóm để xếp trên thanh: `land` | `body` | `mind` | `time` | `sight`. */
  group: PowerGroup;
  needs: PowerNeeds;
  effect: PowerEffect;
  /** Một emoji hoặc một ký tự làm biểu tượng — không tải ảnh ngoài. */
  glyph: string;
  /** Tham số người chơi chọn được, nếu có. */
  params?: PowerParam[];
}

/** Vai trò hợp lệ của cư dân — khớp `npc.role` mà engine hiểu (`slice.rs`). */
const ROLES = ["farmer", "smith", "hunter", "elder", "child"] as const;

/** Ý định hợp lệ cho `npc.intend` — khớp đúng tập khóa mà engine hiểu. */
const INTENTS = [
  "eat",
  "sleep",
  "work",
  "socialize",
  "idle",
  "goto.home",
  "goto.workplace",
  "goto.well",
  "goto.square",
  "goto.field",
] as const;

/**
 * Lớp dữ liệu chọn được cho "Thiên nhãn". Đây là quyền năng `via: "view"` —
 * không gọi lệnh nào — nên danh sách này không cần khớp một enum phía server;
 * nó chỉ cần khớp thứ phía tích hợp (giữ ở `App.vue`) biết vẽ.
 */
const OVERLAYS = ["elevation", "water", "walkable", "crowd"] as const;

/** Nhãn thứ tự các nhóm xuất hiện trên `PowerDock`. */
export const POWER_GROUPS: readonly PowerGroup[] = ["sight", "time", "land", "body", "mind"];

export const POWERS: readonly Power[] = [
  // ── Tầm nhìn ────────────────────────────────────────────────────────────
  {
    id: "sight.reveal",
    group: "sight",
    needs: "none",
    effect: { via: "view" },
    glyph: "👁",
    params: [{ key: "overlay", kind: "choice", options: [...OVERLAYS] }],
  },
  {
    id: "sight.pierce",
    group: "sight",
    needs: "none",
    effect: { via: "view" },
    glyph: "⛏",
    params: [{ key: "z", kind: "int", min: -8, max: 8, step: 1, def: 0 }],
  },

  // ── Thời gian ───────────────────────────────────────────────────────────
  {
    id: "time.still",
    group: "time",
    needs: "none",
    effect: { via: "view" },
    glyph: "⏳",
  },

  // ── Đất đai ─────────────────────────────────────────────────────────────
  {
    id: "land.carve",
    group: "land",
    needs: "tile",
    effect: { via: "build" },
    glyph: "⛰",
    params: [{ key: "material", kind: "text" }],
  },
  {
    id: "land.till",
    group: "land",
    needs: "tile",
    effect: { via: "build" },
    glyph: "🌾",
  },
  {
    id: "land.pave",
    group: "land",
    needs: "tile",
    effect: { via: "build" },
    glyph: "🪨",
  },

  // ── Thân xác ────────────────────────────────────────────────────────────
  {
    id: "body.feed",
    group: "body",
    needs: "being",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "🍞",
  },
  {
    id: "body.starve",
    group: "body",
    needs: "being",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "🥀",
  },
  {
    id: "body.rename",
    group: "body",
    needs: "being",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "🖋",
    params: [{ key: "name", kind: "text" }],
  },
  {
    id: "body.recast",
    group: "body",
    needs: "being",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "🎭",
    params: [{ key: "role", kind: "choice", options: [...ROLES] }],
  },
  {
    id: "body.guide",
    group: "body",
    needs: "being_and_tile",
    effect: { via: "guide" },
    glyph: "🧭",
  },
  {
    // `core.take` đòi cả `who` (ai nhặt) lẫn `what` (nhặt gì) — hai định danh
    // thực thể. Props của `PowerDock` chỉ cho chọn một sinh mệnh và một ô, nên
    // không có cách nào tự suy ra "vật phẩm bên cạnh" từ hai thứ đó (cần danh
    // sách thực thể quanh ô, mà `Power`/`PowerDock` không giữ trạng thái thế
    // giới). Đánh đổi trung thực: `item` là tham số văn bản, người chơi (hoặc
    // phía tích hợp, nếu có picker vật phẩm gần đó) gõ đúng định danh.
    id: "body.take",
    group: "body",
    needs: "being",
    effect: { via: "command", kind: "core.take" },
    glyph: "✋",
    params: [{ key: "item", kind: "text" }],
  },

  // ── Tâm trí ─────────────────────────────────────────────────────────────
  {
    id: "mind.dream",
    group: "mind",
    needs: "being",
    effect: { via: "command", kind: "npc.intend" },
    glyph: "🌙",
    params: [{ key: "intent", kind: "choice", options: [...INTENTS] }],
  },
  {
    id: "mind.proclaim",
    group: "mind",
    needs: "being",
    effect: { via: "command", kind: "core.speak" },
    glyph: "📜",
    params: [{ key: "text", kind: "text" }],
  },
  {
    id: "mind.uproot_x",
    group: "mind",
    needs: "being_and_tile",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "🏠",
  },
  {
    id: "mind.uproot_y",
    group: "mind",
    needs: "being_and_tile",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "🏠",
  },
  {
    id: "mind.reassign_x",
    group: "mind",
    needs: "being_and_tile",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "⚒",
  },
  {
    id: "mind.reassign_y",
    group: "mind",
    needs: "being_and_tile",
    effect: { via: "preview", kind: "truegod.set_attr" },
    glyph: "⚒",
  },
];

/** Quyền năng này thi hành được với thứ đang chọn không, và vì sao không. */
export function readiness(
  p: Power,
  sel: { being: boolean; tile: boolean },
): { ready: true } | { ready: false; reason: "need_being" | "need_tile" } {
  switch (p.needs) {
    case "none":
      return { ready: true };
    case "being":
      return sel.being ? { ready: true } : { ready: false, reason: "need_being" };
    case "tile":
      return sel.tile ? { ready: true } : { ready: false, reason: "need_tile" };
    case "being_and_tile":
      if (!sel.being) return { ready: false, reason: "need_being" };
      if (!sel.tile) return { ready: false, reason: "need_tile" };
      return { ready: true };
  }
}

/**
 * Bọc một định danh cho đúng khuôn dây `{entity: N}` (`§22.10`; xem chú thích
 * `entityRef` ở `api/game.ts` — cùng lý do, cùng cách làm). Viết lại tại chỗ
 * thay vì import từ `api/game.ts`: module này không phụ thuộc một file đang bị
 * sửa song song. Chữ ký giống hệt `entityRef`, nên khi gộp lại chỉ là xóa một
 * hàm trùng, không đổi hành vi.
 */
function wrapEntity(id: string): { entity: number } {
  return { entity: Number(id) };
}

function findParam(p: Power, key: string): PowerParam | undefined {
  return p.params?.find((param) => param.key === key);
}

/** Đọc một tham số `text`: chuỗi khác rỗng sau khi cắt khoảng trắng, hoặc `null`. */
function readText(key: string, params: Record<string, string | number> | undefined): string | null {
  const raw = params?.[key];
  if (typeof raw !== "string") return null;
  const trimmed = raw.trim();
  return trimmed === "" ? null : trimmed;
}

/** Đọc một tham số `choice`: phải khớp đúng một trong `options` khai ở `Power`. */
function readChoice(
  p: Power,
  key: string,
  params: Record<string, string | number> | undefined,
): string | null {
  const raw = params?.[key];
  if (typeof raw !== "string") return null;
  const options = findParam(p, key)?.options;
  if (options && !options.includes(raw)) return null;
  return raw;
}

/** Đọc một tham số `int`, kẹp về `[min, max]` khai ở `Power`, rơi về `def` khi thiếu. */
function readInt(p: Power, key: string, params: Record<string, string | number> | undefined): number {
  const def = findParam(p, key);
  const raw = params?.[key];
  let v = typeof raw === "number" ? raw : (def?.def ?? 0);
  if (def?.min !== undefined) v = Math.max(def.min, v);
  if (def?.max !== undefined) v = Math.min(def.max, v);
  return v;
}

/**
 * Dựng `fields` để gửi đi. Trả `null` khi thiếu thứ bắt buộc — chỗ gọi phải
 * kiểm `readiness` trước, và trả `null` là lưới an toàn thứ hai: thiếu đúng
 * thứ `needs` đòi vẫn phải chặn ở đây, không chặn ở dây (server sẽ trả một lỗi
 * kiểu khó truy hơn nhiều, kiểu `"missing field who"`).
 */
export function fieldsFor(
  p: Power,
  ctx: { beingId?: string; tile?: { x: number; y: number }; params?: Record<string, string | number> },
): Record<string, unknown> | null {
  if ((p.needs === "being" || p.needs === "being_and_tile") && !ctx.beingId) return null;
  if ((p.needs === "tile" || p.needs === "being_and_tile") && !ctx.tile) return null;

  const being = ctx.beingId;
  const tile = ctx.tile;

  switch (p.id) {
    case "sight.reveal":
      return { overlay: readChoice(p, "overlay", ctx.params) ?? OVERLAYS[0] };
    case "sight.pierce":
      return { z: readInt(p, "z", ctx.params) };
    case "time.still":
      return {};

    case "land.carve": {
      const material = readText("material", ctx.params);
      return tile && material !== null ? { x: tile.x, y: tile.y, material } : null;
    }
    case "land.till":
      return tile ? { x: tile.x, y: tile.y, material: "farmland" } : null;
    case "land.pave":
      return tile ? { x: tile.x, y: tile.y, material: "path_gravel" } : null;

    case "body.feed":
      return being ? { entity: wrapEntity(being), key: "need.hunger", value: 0 } : null;
    case "body.starve":
      return being ? { entity: wrapEntity(being), key: "need.hunger", value: 9000 } : null;
    case "body.rename": {
      const name = readText("name", ctx.params);
      return being && name !== null ? { entity: wrapEntity(being), key: "core.name", value: name } : null;
    }
    case "body.recast": {
      const role = readChoice(p, "role", ctx.params);
      return being && role !== null ? { entity: wrapEntity(being), key: "npc.role", value: role } : null;
    }
    case "body.guide":
      return being && tile ? { who: being, x: tile.x, y: tile.y } : null;
    case "body.take": {
      const item = readText("item", ctx.params);
      return being && item !== null ? { who: wrapEntity(being), what: wrapEntity(item) } : null;
    }

    case "mind.dream": {
      const intent = readChoice(p, "intent", ctx.params);
      return being && intent !== null ? { who: wrapEntity(being), intent } : null;
    }
    case "mind.proclaim": {
      const text = readText("text", ctx.params);
      return being && text !== null ? { who: wrapEntity(being), text } : null;
    }
    case "mind.uproot_x":
      return being && tile ? { entity: wrapEntity(being), key: "npc.home.x", value: tile.x } : null;
    case "mind.uproot_y":
      return being && tile ? { entity: wrapEntity(being), key: "npc.home.y", value: tile.y } : null;
    case "mind.reassign_x":
      return being && tile ? { entity: wrapEntity(being), key: "npc.work.x", value: tile.x } : null;
    case "mind.reassign_y":
      return being && tile ? { entity: wrapEntity(being), key: "npc.work.y", value: tile.y } : null;

    default:
      return null;
  }
}
