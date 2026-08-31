/**
 * Chuỗi hiển thị riêng cho hệ thống menu.
 *
 * ## Vì sao không phải là `i18n/index.ts`
 *
 * Mọi chữ trên giao diện phải tra qua `t()` (`i18n/index.ts`), nhưng nhiệm vụ
 * dựng menu bị cấm sửa file đó — nhiều người đang chỉnh nó cùng lúc, và một
 * catalog chung bị hai nhánh sửa song song là chỗ hay sinh xung đột merge vô
 * nghĩa (thêm khóa ở hai chỗ khác nhau trong cùng một object).
 *
 * File này **chép nguyên khuôn** của `i18n/index.ts`: một catalog `vi` gốc, một
 * catalog `en` được kiểm bằng kiểu là phải phủ hết khóa của `vi` (thiếu một
 * khóa là `tsc` đỏ, không phải một ô trống lúc chạy), và một hàm tra `tm()`
 * (đặt tên khác `t()` để không đụng khi cả hai cùng được import vào một
 * component). Khi nhánh `i18n/index.ts` rảnh, người ghép chỉ cần chuyển từng
 * khóa `menu.*`/`title.*`/`pause.*`/`settings.*`/`codex.*` dưới đây sang hai
 * catalog `vi`/`en` gốc rồi xóa file này — không có cấu trúc nào cần dịch lại.
 *
 * ## Vì sao dùng chung `locale()` với `i18n/index.ts`
 *
 * `tm()` đọc ngôn ngữ hiện tại qua `locale()` nhập từ `@/i18n` — **đọc**, không
 * sửa file đó. Nếu file này tự giữ một biến ngôn ngữ riêng, đổi ngôn ngữ ở
 * `SettingsPanel` sẽ phải đồng bộ hai nơi, và một trong hai thế nào cũng có
 * lúc bị quên gọi. Dùng chung nguồn sự thật thì không có "quên đồng bộ" nào
 * để mà quên.
 */
import { locale } from "@/i18n";

const vi = {
  "menu.settings": "Thiết lập",
  "menu.codex": "Thư viện tri thức",
  "menu.resume": "Tiếp tục",
  "menu.quitToTitle": "Về màn hình đầu",
  "menu.close": "Đóng",
  "menu.restoreDefaults": "Khôi phục mặc định",

  "title.tagline": "Một thế giới tự vận hành theo nhịp riêng của nó — Người quan sát, và khi cần, Người khắc.",
  "title.seedLabel": "Hạt giống thế giới",
  "title.seedHint": "cùng một hạt giống là cùng một thế giới",
  "title.play": "Bước vào thế giới",

  "pause.title": "Tạm dừng",
  "pause.seed": "hạt giống",
  "pause.tick": "nhịp",
  "pause.stateHash": "dấu thế giới",
  "pause.population": "dân số",

  "settings.title": "Thiết lập",
  "settings.locale": "Ngôn ngữ",
  "settings.locale.vi": "Tiếng Việt",
  "settings.locale.en": "English",
  "settings.speedIndex": "Nấc tốc độ mặc định",
  "settings.showLabels": "Hiện nhãn tên trên bản đồ",
  "settings.showGrid": "Hiện lưới ô",
  "settings.reduceMotion": "Giảm chuyển động",
  "settings.uiScale": "Cỡ chữ giao diện",

  "codex.title": "Thư viện tri thức",
  "codex.section.time.title": "Nhịp và tốc độ thời gian",
  "codex.section.time.body":
    "Thế giới không chờ Người. Nó tự bước theo từng nhịp — một đơn vị thời gian nhỏ nhất, cố định, không đổi dù Người có đang nhìn hay không. Người chỉ chọn nó bước nhanh hay chậm bao nhiêu qua các nấc tốc độ; Người không chọn được việc nó có bước hay không, vì một thế giới ngừng bước là một thế giới đã chết.",
  "codex.section.layers.title": "Lát cắt z",
  "codex.section.layers.body":
    "Đất không phẳng, và những gì Người thấy chỉ là một lát mỏng của nó. Mỗi lát z là một tầng cao độ — hầm dưới lòng đất, mặt đất, tán cây trên cao — xếp chồng lên nhau tại cùng một tọa độ x, y. Đổi lát z không đổi thế giới, chỉ đổi chỗ mắt Người đang nhìn tới.",
  "codex.section.materials.title": "Vật liệu và content pack",
  "codex.section.materials.body":
    "Mỗi ô đất mang một vật liệu — đá, nước, đất mùn, gỗ dựng — và mọi vật liệu đều tới từ một content pack, không phải từ luật lõi của trò chơi. Đây là ranh giới cố ý: nội dung có thể thay, luật vận hành thì không. Một content pack mới có thể thêm vật liệu Người chưa từng thấy; giao diện sẽ hiện đúng tên nó, không phải một ô trống.",
  "codex.section.causality.title": "Chuỗi nhân quả",
  "codex.section.causality.body":
    "Không có gì xảy ra mà không có lý do được ghi lại. Mỗi sự kiện mang theo một đường dẫn ngược về sự kiện đã sinh ra nó — bấm vào một sự kiện là truy được cả một chuỗi nhân quả, tới tận gốc hoặc tới chỗ chuỗi bị đứt. Nếu chuỗi đứt, giao diện sẽ nói ra điều đó, không bịa thêm một mắt xích để lấp chỗ trống.",
  "codex.section.will.title": "Ý chỉ và cách xem trước",
  "codex.section.will.body":
    "Người không sửa thế giới trực tiếp. Người soạn một Ý chỉ — một thay đổi được đề xuất — và nhìn trước hậu quả của nó trước khi nó thành thật. Chỉ khi Người ưng ý, Ý chỉ mới được khắc vào thế giới; nếu thế giới đã đổi trong lúc Người còn đang nhìn, Người sẽ được nhắc nhìn lại, vì một hậu quả xem trước trên một thế giới đã cũ không còn đáng tin.",
  "codex.section.schedule.title": "Lịch sinh hoạt của cư dân",
  "codex.section.schedule.body":
    "Mỗi cư dân có một lịch sống riêng — ngủ, ăn, làm việc, trò chuyện — trôi theo giờ trong ngày chứ không đứng yên chờ Người ra lệnh. Ý định của họ đổi khi hoàn cảnh đổi: đói thì tìm ăn, mệt thì tìm chỗ ngủ. Người có thể quan sát và can thiệp, nhưng lịch sống này vẫn tiếp diễn cả khi Người không nhìn tới.",
} as const;

/** Khóa hợp lệ của catalog menu. Một khóa lạ là lỗi biên dịch. */
export type MenuMessageKey = keyof typeof vi;

/** Mọi ngôn ngữ phải phủ hết khóa của bản gốc `vi`. */
type MenuCatalog = Record<MenuMessageKey, string>;

const en: MenuCatalog = {
  "menu.settings": "Settings",
  "menu.codex": "Codex",
  "menu.resume": "Resume",
  "menu.quitToTitle": "Return to the title screen",
  "menu.close": "Close",
  "menu.restoreDefaults": "Restore defaults",

  "title.tagline": "A world that runs by its own rhythm — You observe, and when it is needed, You inscribe.",
  "title.seedLabel": "World seed",
  "title.seedHint": "the same seed is the same world",
  "title.play": "Step into the world",

  "pause.title": "Paused",
  "pause.seed": "seed",
  "pause.tick": "tick",
  "pause.stateHash": "world mark",
  "pause.population": "population",

  "settings.title": "Settings",
  "settings.locale": "Language",
  "settings.locale.vi": "Tiếng Việt",
  "settings.locale.en": "English",
  "settings.speedIndex": "Default time speed",
  "settings.showLabels": "Show name labels on the map",
  "settings.showGrid": "Show the tile grid",
  "settings.reduceMotion": "Reduce motion",
  "settings.uiScale": "Interface text size",

  "codex.title": "Codex",
  "codex.section.time.title": "Tick and time speed",
  "codex.section.time.body":
    "The world does not wait for You. It advances by its own tick — a fixed, smallest unit of time that does not change whether or not You are watching. You may choose how fast it steps through the speed levels; You do not choose whether it steps at all, for a world that stops stepping is a world that has died.",
  "codex.section.layers.title": "Z-layers",
  "codex.section.layers.body":
    "The ground is not flat, and what You see is only one thin slice of it. Each z-layer is a layer of elevation — underground caverns, the surface, the canopy above — stacked at the same x, y coordinate. Changing the z-layer does not change the world, only where Your eyes are looking.",
  "codex.section.materials.title": "Materials and content packs",
  "codex.section.materials.body":
    "Every tile carries a material — stone, water, loam, worked wood — and every material comes from a content pack, never from the game's core rules. This boundary is deliberate: content may change, the rules that govern it may not. A new content pack may add a material You have never seen; the interface will show its true name, not an empty box.",
  "codex.section.causality.title": "Causal chain",
  "codex.section.causality.body":
    "Nothing happens without a recorded reason. Every event carries a path back to the event that caused it — clicking an event traces a whole causal chain, back to its root or to the point where the chain breaks. If it breaks, the interface will say so, not invent a link to fill the gap.",
  "codex.section.will.title": "Will and foresight",
  "codex.section.will.body":
    "You do not change the world directly. You compose a Will — a proposed change — and see its consequences before it becomes real. Only when You are satisfied is the Will inscribed into the world; if the world has moved while You were still looking, You will be asked to look again, for a foreseen consequence on a world already gone stale can no longer be trusted.",
  "codex.section.schedule.title": "Residents' daily schedule",
  "codex.section.schedule.body":
    "Each resident keeps their own daily schedule — sleeping, eating, working, talking — moving with the hours of the day rather than standing still awaiting Your command. Their intentions change as circumstances change: hunger sends them to eat, fatigue sends them to sleep. You may watch and intervene, but this life continues even when You are not looking.",
};

/**
 * Cả hai catalog, xuất ra chỉ để bài kiểm đối chiếu khóa `vi`/`en` bằng
 * runtime, làm lưới an toàn thứ hai cạnh cái kiểm bằng kiểu ở `MenuCatalog`.
 * Không dùng để tra chữ trong component — chỗ đó luôn đi qua `tm()`.
 */
export const MENU_CATALOGS = { vi, en } as const;

/** Tra một chuỗi hiển thị của menu, theo ngôn ngữ hiện tại của `@/i18n`. */
export function tm(key: MenuMessageKey): string {
  return MENU_CATALOGS[locale()][key];
}
