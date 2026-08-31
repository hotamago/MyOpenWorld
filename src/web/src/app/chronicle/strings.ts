/**
 * Chuỗi hiển thị riêng cho Biên niên sử.
 *
 * ## Vì sao không phải là `i18n/index.ts`
 *
 * Mọi chữ trên giao diện phải tra qua `t()` (`i18n/index.ts`), nhưng nhiệm vụ
 * này bị cấm sửa file đó — nhiều người đang chỉnh nó cùng lúc, và một catalog
 * chung bị hai nhánh sửa song song là chỗ hay sinh xung đột merge vô nghĩa.
 * File này chép nguyên khuôn của nó (đúng cách `app/menu/strings.ts` đã làm
 * trước): một catalog `vi` gốc, một catalog `en` được kiểm bằng kiểu là phải
 * phủ hết khóa của `vi`, và một hàm tra chuỗi. Khi nhánh `i18n/index.ts` rảnh,
 * người ghép chỉ cần chuyển từng khóa `chronicle.*` dưới đây sang hai catalog
 * gốc rồi xóa file này — không có cấu trúc nào cần dịch lại.
 *
 * ## Vì sao dùng chung `locale()` với `i18n/index.ts`
 *
 * `tc()` đọc ngôn ngữ hiện tại qua `locale()` nhập từ `@/i18n` — **đọc**,
 * không sửa file đó. Nếu module này tự giữ một biến ngôn ngữ riêng, đổi ngôn
 * ngữ ở `SettingsPanel` sẽ phải đồng bộ hai nơi, và một trong hai thế nào
 * cũng có lúc bị quên gọi.
 *
 * ## Vì sao có cả `t()` (kiểm bằng kiểu) lẫn `tc()` (kiểm lúc chạy)
 *
 * Hầu hết khóa ở đây được viết thẳng trong mã (`t("chronicle.title")`) và
 * `tsc` kiểm được ngay — dùng `MessageKey`. Nhưng `Chapter.key`
 * (`chronicle.ts`) chỉ là `string` trần: nó thuần túy, không biết tới catalog
 * này, và có thể mang một khóa `.unknown` được ghép lúc chạy tùy dữ liệu. Với
 * chuỗi đó, kiểm bằng kiểu là bất khả thi — giống hệt lý do `tRuntime` tồn
 * tại bên `i18n/index.ts`. `tc()` còn làm thêm việc thay `{slot}` bằng dữ
 * liệu thật (`Chapter.slots`), và nếu gặp khóa lạ thì trả về chính khóa đó
 * thay vì ném lỗi hay hiện ô trống — một khóa lạ hiện nguyên hình còn sửa
 * được, một ô trống thì không nói cho ai biết là thiếu gì.
 */
import { locale } from "@/i18n";

const vi = {
  "chronicle.title": "Biên niên sử",
  "chronicle.filter.all": "Trọn biên niên",
  "chronicle.filter.notable": "Chỉ biến cố",
  "chronicle.empty": "Sử xanh chưa chép một dòng nào",
  "chronicle.day": "Ngày {day}",

  "chronicle.journey": "{who} rời bước, qua {count} chặng chân",
  "chronicle.journey.unknown": "Một bóng người rời bước, qua {count} chặng chân",

  "chronicle.intent": "{who} toan tính: {intent}",
  "chronicle.intent.unknown": "Một ý toan dấy lên, không rõ chủ: {intent}",

  "chronicle.itemTaken": "{who} thu lấy một vật vào tay",
  "chronicle.itemTaken.unknown": "Một vật đã đổi chủ, không rõ ai thu lấy",

  "chronicle.itemEaten": "{who} dùng bữa, no thêm {nutrition}",
  "chronicle.itemEaten.unknown": "Có kẻ vừa dùng bữa, no thêm {nutrition}",

  "chronicle.speech": '{who} cất lời: "{text}"',
  "chronicle.speech.unknown": 'Một lời vang lên, không rõ ai nói: "{text}"',

  "chronicle.actCommitted": "{who} trọn một việc: {act}",
  "chronicle.actCommitted.unknown": "Một việc vừa trọn, không rõ tay ai: {act}",

  "chronicle.spawned": "Một sinh mệnh mới bước vào cõi này: {kind}",
  "chronicle.needSet": "Cơn {need} trong một ai đó đổi thành {value}",
  "chronicle.intervened": "Bàn tay thần chạm vào {key}",

  "chronicle.other": "{who} — một sự lạ: {kind}",
  "chronicle.other.unknown": "Một sự lạ, không rõ căn do: {kind}",
} as const;

/** Khóa hợp lệ cho lời gọi viết thẳng trong mã. */
export type ChronicleMessageKey = keyof typeof vi;

/** Mọi ngôn ngữ phải phủ hết khóa của bản gốc. */
type ChronicleCatalog = Record<ChronicleMessageKey, string>;

const en: ChronicleCatalog = {
  "chronicle.title": "The Chronicle",
  "chronicle.filter.all": "The Full Record",
  "chronicle.filter.notable": "Portents Only",
  "chronicle.empty": "No line is yet written in this chronicle",
  "chronicle.day": "Day {day}",

  "chronicle.journey": "{who} set forth, {count} paces onward",
  "chronicle.journey.unknown": "A shape set forth, {count} paces onward",

  "chronicle.intent": "{who} resolved upon: {intent}",
  "chronicle.intent.unknown": "A resolve arose, its author unknown: {intent}",

  "chronicle.itemTaken": "{who} took something into hand",
  "chronicle.itemTaken.unknown": "A thing changed hands, by whom none can say",

  "chronicle.itemEaten": "{who} ate, and was {nutrition} the fuller",
  "chronicle.itemEaten.unknown": "Someone ate, and was {nutrition} the fuller",

  "chronicle.speech": '{who} spoke: "{text}"',
  "chronicle.speech.unknown": 'A voice was heard, unattributed: "{text}"',

  "chronicle.actCommitted": "{who} accomplished a deed: {act}",
  "chronicle.actCommitted.unknown": "A deed was accomplished, by no known hand: {act}",

  "chronicle.spawned": "A new soul entered this world: {kind}",
  "chronicle.needSet": "Someone's {need} turned to {value}",
  "chronicle.intervened": "The hand of a god touched {key}",

  "chronicle.other": "{who} — a strange happening: {kind}",
  "chronicle.other.unknown": "A strange happening, of unknown cause: {kind}",
};

/** Xuất cả hai catalog để bài kiểm đối chiếu tập khóa, không phải để đọc trực
 * tiếp từ nơi khác — nơi khác phải qua `t()`/`tc()` để theo đúng `locale()`. */
export const CHRONICLE_CATALOGS = { vi, en } as const;

/** Tra một khóa viết thẳng trong mã, không có `{slot}` cần điền. */
export function t(key: ChronicleMessageKey): string {
  return CHRONICLE_CATALOGS[locale()][key];
}

/**
 * Tra một khóa chỉ biết được lúc chạy (`Chapter.key`), điền `slots` nếu có.
 *
 * Khóa lạ trả về **chính khóa**, không phải chuỗi rỗng — cùng lý do
 * `tRuntime` (`i18n/index.ts`) chọn vậy: một khóa lạ hiện ra nguyên hình là
 * một lời nhắc thiếu bản dịch, một ô trống thì không nói cho ai biết điều gì.
 * Thiếu một slot mà mẫu chữ cần thì giữ nguyên `{tên_slot}` thay vì in chữ
 * `undefined` — cùng lý do, chỉ khác chỗ.
 */
export function tc(key: string, slots?: Record<string, string | number>): string {
  const cat: Record<string, string> = CHRONICLE_CATALOGS[locale()];
  const template = cat[key];
  if (template === undefined) return key;
  if (!slots) return template;
  return template.replace(/\{(\w+)\}/g, (whole: string, name: string) => {
    const v = slots[name];
    return v === undefined ? whole : String(v);
  });
}
