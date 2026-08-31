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
  "panel.controls.move": "bấm để đi",
  "panel.controls.stop": "chuột phải để dừng",
  "panel.controls.walking": "đang đi",
  "panel.controls.unreachable": "không có đường tới đó",
  "panel.controls.partial": "chỉ tới được gần đó",
  "panel.controls.take": "nhặt",
  "panel.controls.talk": "nói",
  "panel.controls.layer": "đổi lát z",
  "panel.controls.zoom": "phóng to / thu nhỏ",
  "panel.controls.nothingHere": "không có gì dưới chân",
  "panel.controls.nobodyNear": "không có ai bên cạnh",

  "panel.present": "Có mặt",
  "panel.who": "Cư dân",
  "panel.who.role": "vai",
  "panel.who.doing": "đang định",
  "panel.who.hunger": "đói",
  "panel.events": "Sự kiện",
  "panel.events.empty": "chưa có gì xảy ra",
  "panel.events.hint": "bấm một sự kiện để truy ngược nguyên nhân",
  "panel.cause": "Vì sao",
  "panel.cause.root": "không còn nguyên nhân nào trước đó",
  "panel.cause.close": "đóng",
  "panel.minimap": "Bản đồ thu nhỏ",
  "god.title": "Ý chỉ",
  "god.shape": "Khắc địa hình",
  "god.shape.hint": "chọn vật liệu rồi bấm lên bản đồ",
  "god.shape.off": "thôi khắc",
  "god.pick": "chọn một sinh mệnh ở danh sách bên dưới",
  "god.target": "đối tượng",
  "god.act.feed": "Ban no đủ",
  "god.act.starve": "Gieo cơn đói",
  "god.act.rename": "Ban tên mới",
  "god.foresee": "Nhìn trước",
  "god.inscribe": "Khắc vào thế giới",
  "god.withdraw": "Thu hồi ý chỉ",
  "god.willChange": "sẽ đổi",
  "god.nothing": "ý chỉ này không đổi gì cả",
  "god.moved": "thế giới đã đổi từ lúc Người nhìn — hãy nhìn lại",
  "god.done": "đã khắc",
  "god.impossible": "không thể",
  "panel.time": "Thời gian",
  "panel.time.paused": "đang tạm dừng",

  "time.dawn": "rạng sáng",
  "time.day": "ban ngày",
  "time.dusk": "hoàng hôn",
  "time.night": "ban đêm",

  "overlay.title": "Lớp dữ liệu",
  "overlay.hint": "một lớp một lúc — chồng hai lớp lên nhau là hết đọc được",
  "overlay.off": "tắt",
  "overlay.elevation": "cao độ",
  "overlay.water": "nước",
  "overlay.walkable": "đi lại được",
  "overlay.crowd": "mật độ người",
  "overlay.legend.low": "thấp",
  "overlay.legend.high": "cao",

  "observe.title": "Quan sát",
  "observe.hint": "chọn một cư dân rồi bám theo họ",
  "observe.follow": "bám theo",
  "observe.unfollow": "thôi bám",
  "observe.timeline": "dòng đời",
  "observe.empty": "chưa có gì xảy ra với người này",
  "observe.following": "đang bám theo",
  "observe.here": "ngay tại đây",

  "card.arms": "gia huy",
  "card.needs": "nhu cầu",
  "card.fatigue": "mệt",
  "card.home": "nhà",
  "card.work": "chỗ làm",
  "card.unknown": "chưa rõ",

  "role.farmer": "nông dân",
  "role.smith": "thợ rèn",
  "role.hunter": "thợ săn",
  "role.elder": "già làng",
  "role.child": "trẻ con",

  "intent.sleep": "đang ngủ",
  "intent.eat": "đang ăn",
  "intent.work": "đang làm việc",
  "intent.socialize": "đang trò chuyện",
  "intent.idle": "đang rảnh",
  "intent.goto.home": "đang về nhà",
  "intent.goto.workplace": "đang tới xưởng",
  "intent.goto.well": "đang ra giếng",
  "intent.goto.square": "đang ra quảng trường",
  "intent.goto.field": "đang ra đồng",

  "hud.souls": "sinh mệnh",
  "panel.controls.inspect": "bấm để soi xét",
  "panel.controls.pan": "kéo để dời cái nhìn",
  "panel.controls.cancel": "chuột phải để thôi",
  "panel.controls.pause": "dừng / thả thời gian",
  "god.act.guide": "Chỉ đường",
  "god.act.guiding": "Đang chỉ đường…",
  "god.act.guide.hint": "bấm một ô để bảo người này tới đó",
  "god.act.take": "Khiến nhặt",
  "hud.day": "ngày",
  "hud.ripe": "ruộng chín",
  "rail.yuu": "Yuu",
  "yuu.title": "Yuu",
  "yuu.hint": "Yuu đọc đồ thị nhân quả và các con số của thế giới, rồi nói lại. Câu nào không chứng minh được đã bị cắt.",
  "yuu.ask": "Hỏi",
  "yuu.placeholder": "hỏi Yuu một câu…",
  "yuu.thinking": "Yuu đang đọc…",
  "yuu.nothing": "chưa có gì để nói",
  "yuu.stripped": "Đã cắt vì không chứng minh được",
  "yuu.proposals": "Phương án",
  "yuu.ungrounded": "không có model — đọc thẳng từ đồ thị nhân quả",
  "yuu.cite": "mở sự kiện này",
  "menu.open": "Mở menu",
  "rail.observe": "Quan sát",
  "rail.layers": "Lớp dữ liệu",
  "rail.chronicle": "Biên niên sử",
  "rail.cause": "Nhân quả",
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
  "panel.controls.move": "click to walk",
  "panel.controls.stop": "right-click to stop",
  "panel.controls.walking": "walking",
  "panel.controls.unreachable": "no path there",
  "panel.controls.partial": "can only get near it",
  "panel.controls.take": "take",
  "panel.controls.talk": "talk",
  "panel.controls.layer": "change z layer",
  "panel.controls.zoom": "zoom in / out",
  "panel.controls.nothingHere": "nothing underfoot",
  "panel.controls.nobodyNear": "nobody nearby",

  "panel.present": "Present",
  "panel.who": "Resident",
  "panel.who.role": "role",
  "panel.who.doing": "intends",
  "panel.who.hunger": "hunger",
  "panel.events": "Events",
  "panel.events.empty": "nothing has happened yet",
  "panel.events.hint": "click an event to trace its causes",
  "panel.cause": "Why",
  "panel.cause.root": "no earlier cause",
  "panel.cause.close": "close",
  "panel.minimap": "Minimap",
  "god.title": "Will",
  "god.shape": "Shape the land",
  "god.shape.hint": "pick a material, then click the map",
  "god.shape.off": "stop shaping",
  "god.pick": "choose a being from the list below",
  "god.target": "target",
  "god.act.feed": "Grant fullness",
  "god.act.starve": "Sow hunger",
  "god.act.rename": "Grant a new name",
  "god.foresee": "Foresee",
  "god.inscribe": "Inscribe into the world",
  "god.withdraw": "Withdraw the will",
  "god.willChange": "will change",
  "god.nothing": "this will changes nothing",
  "god.moved": "the world moved since you looked — look again",
  "god.done": "inscribed",
  "god.impossible": "impossible",
  "panel.time": "Time",
  "panel.time.paused": "paused",

  "time.dawn": "dawn",
  "time.day": "day",
  "time.dusk": "dusk",
  "time.night": "night",

  "overlay.title": "Data layers",
  "overlay.hint": "one layer at a time — two stacked layers read as neither",
  "overlay.off": "off",
  "overlay.elevation": "elevation",
  "overlay.water": "water",
  "overlay.walkable": "walkable",
  "overlay.crowd": "crowding",
  "overlay.legend.low": "low",
  "overlay.legend.high": "high",

  "observe.title": "Observe",
  "observe.hint": "pick a resident, then follow them",
  "observe.follow": "follow",
  "observe.unfollow": "stop following",
  "observe.timeline": "life so far",
  "observe.empty": "nothing has happened to this one yet",
  "observe.following": "following",
  "observe.here": "right here",

  "card.arms": "arms",
  "card.needs": "needs",
  "card.fatigue": "fatigue",
  "card.home": "home",
  "card.work": "workplace",
  "card.unknown": "unknown",

  "role.farmer": "farmer",
  "role.smith": "smith",
  "role.hunter": "hunter",
  "role.elder": "elder",
  "role.child": "child",

  "intent.sleep": "sleeping",
  "intent.eat": "eating",
  "intent.work": "working",
  "intent.socialize": "talking",
  "intent.idle": "idle",
  "intent.goto.home": "heading home",
  "intent.goto.workplace": "heading to the workshop",
  "intent.goto.well": "heading to the well",
  "intent.goto.square": "heading to the square",
  "intent.goto.field": "heading out to the fields",

  "hud.souls": "souls",
  "panel.controls.inspect": "click to inspect",
  "panel.controls.pan": "drag to move your gaze",
  "panel.controls.cancel": "right-click to cancel",
  "panel.controls.pause": "stop / start time",
  "god.act.guide": "Guide",
  "god.act.guiding": "Guiding…",
  "god.act.guide.hint": "click a tile to send this one there",
  "god.act.take": "Bid them take",
  "hud.day": "day",
  "hud.ripe": "ripe fields",
  "rail.yuu": "Yuu",
  "yuu.title": "Yuu",
  "yuu.hint": "Yuu reads the world's causal graph and its numbers, then says it back. Anything it could not prove has been cut.",
  "yuu.ask": "Ask",
  "yuu.placeholder": "ask Yuu something…",
  "yuu.thinking": "Yuu is reading…",
  "yuu.nothing": "nothing to say yet",
  "yuu.stripped": "Cut — could not be proven",
  "yuu.proposals": "Options",
  "yuu.ungrounded": "no model — read straight from the causal graph",
  "yuu.cite": "open this event",
  "menu.open": "Open menu",
  "rail.observe": "Observe",
  "rail.layers": "Data layers",
  "rail.chronicle": "Chronicle",
  "rail.cause": "Causality",
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
 * Tra một khóa chỉ biết được lúc chạy, có đường lui thấy được.
 *
 * [`t`] kiểm khóa bằng kiểu, và đó là điều đúng cho chữ viết trong mã. Nhưng
 * engine gửi lên những khóa mà giao diện không biết trước — `npc.intent` là
 * `"goto.field"`, và một content pack có thể thêm vai mới ngày mai. Với chúng
 * thì kiểm bằng kiểu là bất khả thi, nên chỗ này nhận `string`.
 *
 * Đường lui trả về **chính khóa**, không phải chuỗi rỗng: một khóa lạ hiện ra
 * dưới dạng `goto.market` là một lời nhắc thiếu bản dịch; một ô trống thì không
 * nói cho ai biết điều gì cả.
 */
export function tRuntime(prefix: string, key: string | null | undefined): string {
  if (!key) return "—";
  const full = `${prefix}.${key}`;
  const cat: Record<string, string> = CATALOGS[current];
  return cat[full] ?? key;
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
