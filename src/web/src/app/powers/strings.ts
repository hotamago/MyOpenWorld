/**
 * Chữ hiển thị riêng cho bảng quyền năng (`PowerDock`).
 *
 * ## Vì sao không phải là `i18n/index.ts`
 *
 * Cùng lý do với `app/menu/strings.ts` (đọc file đó trước — đây là cùng một
 * khuôn): `i18n/index.ts` đang bị sửa song song lúc nhiệm vụ này ra đời, nên
 * nó không được đụng vào. File này chép nguyên khuôn của `i18n/index.ts` — một
 * `vi` gốc, một `en` được kiểm bằng kiểu là phải phủ hết khóa của `vi` (thiếu
 * một khóa là `tsc` đỏ, không phải một ô trống lúc chạy) — và một hàm tra
 * riêng `tp()` (đặt tên khác `t()`/`tm()` để cả ba cùng import được vào một
 * component mà không đụng nhau).
 *
 * Khi nhánh `i18n/index.ts` rảnh tay: chuyển từng khóa `group.*`/`dock.*`/
 * `reason.*`/`power.*`/`param.*`/`value.*` dưới đây sang hai catalog `vi`/`en`
 * gốc rồi xóa file này — không khóa nào cần dịch lại, và `PowerDock.vue` đổi
 * `tp()`/`tpRaw()` thành `t()`/`tRuntime()`.
 *
 * ## Vì sao dùng chung `locale()` với `i18n/index.ts`
 *
 * Giống `tm()` ở menu: `tp()` đọc ngôn ngữ hiện tại qua `locale()` nhập từ
 * `@/i18n` — **đọc**, không sửa file đó. Nếu module này tự giữ một biến ngôn
 * ngữ riêng, đổi ngôn ngữ ở panel cài đặt sẽ không đổi ngôn ngữ của
 * `PowerDock`, và đó là một lỗi thật, không phải một chi tiết vặt.
 *
 * ## Giọng văn: vì sao khác phần còn lại
 *
 * Người chơi vừa chê game "chả có cái gì ấy, chả giống một true god tí nào".
 * `panel.*`/`hud.*` ở `i18n/index.ts` nói bằng giọng chức năng — đúng cho một
 * bảng số liệu. Các khóa `power.*.hint` dưới đây cố tình trang nghiêm và hơi
 * cổ, gọi người chơi là "Người" — khớp giọng đã có sẵn ở
 * `title.tagline`/`codex.*` của `app/menu/strings.ts` — vì đây là bảng quyền
 * năng của một vị thần, không phải một bảng số liệu.
 */
import { locale } from "@/i18n";

const vi = {
  "group.sight": "Tầm nhìn",
  "group.time": "Thời gian",
  "group.land": "Đất đai",
  "group.body": "Thân xác",
  "group.mind": "Tâm trí",

  "dock.title": "Quyền năng",
  "dock.cancel": "Thu quyền năng",

  "reason.need_being": "cần chọn một sinh mệnh trước",
  "reason.need_tile": "cần chọn một ô đất trước",

  "power.sight.reveal.label": "Thiên nhãn",
  "power.sight.reveal.hint":
    "Người mở mắt trời, thấy xuyên qua lớp áo của đất — chọn một lớp dữ liệu để soi.",
  "power.sight.pierce.label": "Thấu địa tầng",
  "power.sight.pierce.hint":
    "Người bóc từng lớp đất đá, nhìn xuống tầng sâu hơn hoặc vượt lên tầng cao.",

  "power.time.still.label": "Ngưng đọng",
  "power.time.still.hint": "Người ngưng dòng chảy của thời gian, hoặc buông cho nó trôi tiếp.",

  "power.land.carve.label": "Khắc đất",
  "power.land.carve.hint": "Người khắc một vật liệu tùy ý lên ô đất đang trỏ tới.",
  "power.land.till.label": "Khơi ruộng",
  "power.land.till.hint": "Người biến ô đất thành ruộng màu, cho mùa màng bén rễ.",
  "power.land.pave.label": "Mở đường",
  "power.land.pave.hint": "Người trải sỏi thành lối đi, cho bước chân người trần có đường mà theo.",

  "power.body.feed.label": "Ban no đủ",
  "power.body.feed.hint": "Người xóa hết cơn đói khỏi một sinh mệnh, ban cho họ sự no đủ tức thì.",
  "power.body.starve.label": "Gieo cơn đói",
  "power.body.starve.hint":
    "Người gieo một cơn đói lớn vào một sinh mệnh — một thử thách, hoặc một lời cảnh báo.",
  "power.body.rename.label": "Ban tên mới",
  "power.body.rename.hint": "Người đặt lại tên cho một sinh mệnh, như thể họ vừa được sinh ra lần nữa.",
  "power.body.recast.label": "Đổi phận",
  "power.body.recast.hint": "Người đổi vai trò của một sinh mệnh trong làng, theo ý Người chọn.",
  "power.body.guide.label": "Chỉ đường",
  "power.body.guide.hint": "Người chỉ cho một sinh mệnh một ô để bước tới.",
  "power.body.take.label": "Khiến nhặt",
  "power.body.take.hint": "Người khiến một sinh mệnh nhặt lấy một vật đang nằm cạnh họ.",

  "power.mind.dream.label": "Báo mộng",
  "power.mind.dream.hint":
    "Người gieo vào tâm trí một sinh mệnh một ý định mới, như một giấc mộng chỉ đường.",
  "power.mind.proclaim.label": "Truyền lời",
  "power.mind.proclaim.hint": "Người mượn miệng một sinh mệnh để lời của Người vang lên giữa làng.",
  "power.mind.uproot_x.label": "Dời tổ ấm — hoành độ",
  "power.mind.uproot_x.hint":
    "Người dời tọa độ đông–tây của mái nhà một sinh mệnh. Còn một quyền năng nữa để dời trục kia.",
  "power.mind.uproot_y.label": "Dời tổ ấm — tung độ",
  "power.mind.uproot_y.hint": "Người dời tọa độ bắc–nam của mái nhà một sinh mệnh.",
  "power.mind.reassign_x.label": "Đổi chỗ làm — hoành độ",
  "power.mind.reassign_x.hint": "Người dời tọa độ đông–tây của nơi một sinh mệnh làm việc.",
  "power.mind.reassign_y.label": "Đổi chỗ làm — tung độ",
  "power.mind.reassign_y.hint": "Người dời tọa độ bắc–nam của nơi một sinh mệnh làm việc.",

  "param.land.carve.material": "Vật liệu",
  "param.body.rename.name": "Tên mới",
  "param.body.recast.role": "Phận mới",
  "param.body.take.item": "Định danh vật phẩm",
  "param.mind.dream.intent": "Ý định",
  "param.mind.proclaim.text": "Lời truyền",
  "param.sight.reveal.overlay": "Lớp dữ liệu",
  "param.sight.pierce.z": "Lát z",

  "value.role.farmer": "nông dân",
  "value.role.smith": "thợ rèn",
  "value.role.hunter": "thợ săn",
  "value.role.elder": "già làng",
  "value.role.child": "trẻ con",

  "value.intent.eat": "ăn",
  "value.intent.sleep": "ngủ",
  "value.intent.work": "làm việc",
  "value.intent.socialize": "trò chuyện",
  "value.intent.idle": "rảnh rỗi",
  "value.intent.goto.home": "về nhà",
  "value.intent.goto.workplace": "tới xưởng",
  "value.intent.goto.well": "ra giếng",
  "value.intent.goto.square": "ra quảng trường",
  "value.intent.goto.field": "ra đồng",

  "value.overlay.elevation": "cao độ",
  "value.overlay.water": "nước",
  "value.overlay.walkable": "đi lại được",
  "value.overlay.crowd": "mật độ người",
} as const;

/** Khóa hợp lệ của catalog quyền năng. Một khóa lạ là lỗi biên dịch. */
export type PowerMessageKey = keyof typeof vi;

/** Mọi ngôn ngữ phải phủ hết khóa của bản gốc `vi`. */
type PowerCatalog = Record<PowerMessageKey, string>;

const en: PowerCatalog = {
  "group.sight": "Sight",
  "group.time": "Time",
  "group.land": "Land",
  "group.body": "Body",
  "group.mind": "Mind",

  "dock.title": "Powers",
  "dock.cancel": "Withdraw the power",

  "reason.need_being": "choose a being first",
  "reason.need_tile": "choose a tile first",

  "power.sight.reveal.label": "Third Eye",
  "power.sight.reveal.hint":
    "You open a heavenly eye, seeing through the skin of the earth — choose a data layer to reveal.",
  "power.sight.pierce.label": "Pierce the Strata",
  "power.sight.pierce.hint":
    "You peel back layers of earth and stone, looking down to a deeper layer or up to a higher one.",

  "power.time.still.label": "Stillness",
  "power.time.still.hint": "You still the flow of time, or let it flow again.",

  "power.land.carve.label": "Carve the Land",
  "power.land.carve.hint": "You carve any material onto the tile You are pointing at.",
  "power.land.till.label": "Till the Field",
  "power.land.till.hint": "You turn a tile into farmland, so a harvest may take root.",
  "power.land.pave.label": "Lay a Path",
  "power.land.pave.hint": "You lay gravel into a path, so mortal feet have a way to follow.",

  "power.body.feed.label": "Grant Fullness",
  "power.body.feed.hint": "You erase all hunger from a being, granting them fullness at once.",
  "power.body.starve.label": "Sow Hunger",
  "power.body.starve.hint": "You sow a great hunger into a being — a trial, or a warning.",
  "power.body.rename.label": "Grant a New Name",
  "power.body.rename.hint": "You rename a being, as though they were born again.",
  "power.body.recast.label": "Recast Their Lot",
  "power.body.recast.hint": "You change a being's role in the village, to whatever lot You choose.",
  "power.body.guide.label": "Guide",
  "power.body.guide.hint": "You point a being to a tile for them to walk to.",
  "power.body.take.label": "Bid Them Take",
  "power.body.take.hint": "You bid a being take up an item lying beside them.",

  "power.mind.dream.label": "Send a Dream",
  "power.mind.dream.hint": "You plant a new intent in a being's mind, like a dream pointing the way.",
  "power.mind.proclaim.label": "Proclaim",
  "power.mind.proclaim.hint": "You borrow a being's voice, so Your word rings out through the village.",
  "power.mind.uproot_x.label": "Move the Hearth — Eastings",
  "power.mind.uproot_x.hint":
    "You move the east–west coordinate of a being's home. A second power moves the other axis.",
  "power.mind.uproot_y.label": "Move the Hearth — Northings",
  "power.mind.uproot_y.hint": "You move the north–south coordinate of a being's home.",
  "power.mind.reassign_x.label": "Reassign Their Work — Eastings",
  "power.mind.reassign_x.hint": "You move the east–west coordinate of a being's workplace.",
  "power.mind.reassign_y.label": "Reassign Their Work — Northings",
  "power.mind.reassign_y.hint": "You move the north–south coordinate of a being's workplace.",

  "param.land.carve.material": "Material",
  "param.body.rename.name": "New name",
  "param.body.recast.role": "New lot",
  "param.body.take.item": "Item id",
  "param.mind.dream.intent": "Intent",
  "param.mind.proclaim.text": "Words to proclaim",
  "param.sight.reveal.overlay": "Data layer",
  "param.sight.pierce.z": "Z layer",

  "value.role.farmer": "farmer",
  "value.role.smith": "smith",
  "value.role.hunter": "hunter",
  "value.role.elder": "elder",
  "value.role.child": "child",

  "value.intent.eat": "eat",
  "value.intent.sleep": "sleep",
  "value.intent.work": "work",
  "value.intent.socialize": "socialize",
  "value.intent.idle": "idle",
  "value.intent.goto.home": "head home",
  "value.intent.goto.workplace": "head to the workshop",
  "value.intent.goto.well": "head to the well",
  "value.intent.goto.square": "head to the square",
  "value.intent.goto.field": "head to the fields",

  "value.overlay.elevation": "elevation",
  "value.overlay.water": "water",
  "value.overlay.walkable": "walkable",
  "value.overlay.crowd": "crowding",
};

/**
 * Cả hai catalog, xuất ra để bài kiểm đối chiếu khóa `vi`/`en` bằng runtime
 * (lưới an toàn thứ hai cạnh kiểm bằng kiểu ở `PowerCatalog`) và để kiểm "mọi
 * `Power.id` đều có khóa chữ". Không dùng để tra chữ trong component — chỗ đó
 * luôn đi qua `tp()`/`tpRaw()`.
 */
export const POWER_CATALOGS = { vi, en } as const;

/** Tra một chuỗi hiển thị đã biết khóa lúc biên dịch. */
export function tp(key: PowerMessageKey): string {
  return POWER_CATALOGS[locale()][key];
}

/**
 * Tra một khóa ghép từ `Power.id` lúc chạy: `power.<id>.label`,
 * `power.<id>.hint`, `param.<id>.<paramKey>`, `value.<paramKey>.<option>`.
 *
 * Cùng lý do tồn tại với `tRuntime` ở `i18n/index.ts`: `Power.id` là `string`
 * trên kiểu, không phải union literal, nên chỗ gọi (`PowerDock.vue`) không thể
 * kiểm hết bằng kiểu. Khác `tRuntime` — nơi khóa lạ hợp lệ vì tới từ content
 * pack tương lai — thiếu khóa ở đây luôn là lỗi thật của chính
 * `powers.ts`/`strings.ts`, vì mọi `id` đều do hai file này sinh ra. Đường lui
 * trả về chính khóa để lộ ngay lỗi đó, không im lặng hiện một ô trống.
 */
export function tpRaw(key: string): string {
  const cat: Record<string, string> = POWER_CATALOGS[locale()];
  return cat[key] ?? key;
}
