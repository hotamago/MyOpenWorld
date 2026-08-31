/**
 * Đa ngôn ngữ cho giao diện.
 *
 * ## Vì sao tự viết thay vì lấy `vue-i18n`
 *
 * Thứ cần ở đây là tra một khóa ra một chuỗi, cộng số nhiều đơn giản. `vue-i18n`
 * cho thêm định dạng ngày/số theo miền, trộn message, lazy load theo route —
 * toàn thứ dự án này chưa dùng tới, và mỗi thứ là một cách để chuỗi hiển thị
 * khác chuỗi trong file.
 *
 * Đổi lại, module này giữ đúng hai thứ khó mà một `Record<string,string>` trần
 * không có:
 *
 * 1. **Khóa được kiểm bằng kiểu.** `t("nope")` là lỗi biên dịch, không phải một
 *    ô trống trên màn hình lúc chạy.
 * 2. **Thiếu bản dịch là lỗi biên dịch.** Mỗi ngôn ngữ phải phủ hết khóa của
 *    `vi`; thiếu một khóa thì `tsc` đỏ chứ không lặng lẽ rơi về tiếng Việt.
 *
 * ## Vì sao tiếng Việt là ngôn ngữ gốc
 *
 * Vì tài liệu thiết kế viết bằng tiếng Việt, nên bản tiếng Việt là bản có nghĩa
 * chính xác nhất. Định danh trong mã thì ngược lại — chúng là tiếng Anh, vì mã
 * được đọc bởi công cụ và bởi người không đọc tiếng Việt.
 */

const vi = {
  "app.title": "My Open World",
  "app.connecting": "đang kết nối…",
  "app.running": "đang chạy",
  "app.failed": "không kết nối được",

  "hud.tick": "nhịp",
  "hud.layer": "lát z",
  "hud.hunger": "đói",
  "hud.daytime": "giờ",

  "panel.tile": "Ô đang trỏ",
  "panel.tile.hint": "đưa chuột lên bản đồ",
  "panel.tile.position": "vị trí",
  "panel.tile.material": "vật liệu",
  "panel.tile.biome": "quần xã",
  "panel.tile.elevation": "cao độ",
  "panel.tile.depth": "sâu dưới chân",

  "panel.controls": "Điều khiển",
  "panel.controls.move": "đi",
  "panel.controls.take": "nhặt",
  "panel.controls.talk": "nói",
  "panel.controls.layer": "đổi lát z",
  "panel.controls.zoom": "phóng to / thu nhỏ",
  "panel.controls.nothingHere": "không có gì dưới chân",
  "panel.controls.nobodyNear": "không có ai bên cạnh",

  "panel.present": "Có mặt",
  "panel.events": "Sự kiện",
  "panel.events.empty": "chưa có gì xảy ra",
  "panel.minimap": "Bản đồ thu nhỏ",

  "time.dawn": "rạng sáng",
  "time.day": "ban ngày",
  "time.dusk": "hoàng hôn",
  "time.night": "ban đêm",
} as const;

/** Khóa hợp lệ. Một khóa lạ là lỗi biên dịch, không phải ô trống lúc chạy. */
export type MessageKey = keyof typeof vi;

/** Mọi ngôn ngữ phải phủ hết khóa của bản gốc. */
type Catalog = Record<MessageKey, string>;

const en: Catalog = {
  "app.title": "My Open World",
  "app.connecting": "connecting…",
  "app.running": "running",
  "app.failed": "cannot connect",

  "hud.tick": "tick",
  "hud.layer": "layer z",
  "hud.hunger": "hunger",
  "hud.daytime": "time",

  "panel.tile": "Hovered tile",
  "panel.tile.hint": "move the cursor over the map",
  "panel.tile.position": "position",
  "panel.tile.material": "material",
  "panel.tile.biome": "biome",
  "panel.tile.elevation": "elevation",
  "panel.tile.depth": "depth below",

  "panel.controls": "Controls",
  "panel.controls.move": "move",
  "panel.controls.take": "take",
  "panel.controls.talk": "talk",
  "panel.controls.layer": "change z layer",
  "panel.controls.zoom": "zoom in / out",
  "panel.controls.nothingHere": "nothing underfoot",
  "panel.controls.nobodyNear": "nobody nearby",

  "panel.present": "Present",
  "panel.events": "Events",
  "panel.events.empty": "nothing has happened yet",
  "panel.minimap": "Minimap",

  "time.dawn": "dawn",
  "time.day": "day",
  "time.dusk": "dusk",
  "time.night": "night",
};

const CATALOGS = { vi, en } as const;

/** Mã ngôn ngữ được hỗ trợ. */
export type Locale = keyof typeof CATALOGS;

/**
 * Ngôn ngữ đang dùng.
 *
 * Lấy từ trình duyệt, rơi về `vi`. Không lưu vào `localStorage` ở bước này:
 * một lựa chọn đã lưu mà không có chỗ đổi lại là một cái bẫy.
 */
let current: Locale = detect();

function detect(): Locale {
  const nav = globalThis.navigator?.language ?? "vi";
  return nav.startsWith("en") ? "en" : "vi";
}

/** Đổi ngôn ngữ. */
export function setLocale(l: Locale): void {
  current = l;
}

/** Ngôn ngữ hiện tại. */
export function locale(): Locale {
  return current;
}

/** Tra một chuỗi hiển thị. */
export function t(key: MessageKey): string {
  return CATALOGS[current][key];
}

/**
 * Chọn tên theo ngôn ngữ từ một bản ghi đa ngữ của content pack.
 *
 * Block và item mang tên dạng `{ en, vi }`. Thiếu ngôn ngữ hiện tại thì rơi về
 * tiếng Anh rồi tới chính `id` — **không** trả chuỗi rỗng, vì một ô trống trên
 * màn hình không nói cho ai biết là thiếu bản dịch.
 */
export function localized(names: Partial<Record<Locale, string>> | undefined, id: string): string {
  return names?.[current] ?? names?.en ?? id;
}
