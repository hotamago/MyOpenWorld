/**
 * Chỗ nối gọn cho Biên niên sử.
 *
 * Nơi ghép panel vào `App.vue` chỉ cần một dòng `import { ChroniclePanel } from
 * "@/app/chronicle"` thay vì rải bốn đường dẫn file riêng lẻ. Xuất lại nguyên
 * vẹn — không đổi tên, không bọc thêm — nên đây không phải chỗ để thêm logic.
 */
export { pulse, weave, type Chapter } from "./chronicle";

export { CHRONICLE_CATALOGS, t as tChronicle, tc, type ChronicleMessageKey } from "./strings";

export { default as ChroniclePanel } from "./ChroniclePanel.vue";
