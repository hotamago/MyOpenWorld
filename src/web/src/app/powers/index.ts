/**
 * Chỗ nối gọn cho bảng quyền năng của True God.
 *
 * Nơi ghép vào `App.vue` chỉ cần một dòng `import { ... } from "@/app/powers"`
 * thay vì rải bốn đường dẫn file riêng lẻ (cùng khuôn với `app/menu/index.ts`).
 * Xuất lại nguyên vẹn — không đổi tên, không bọc thêm — nên đây không phải chỗ
 * để thêm logic.
 */
export {
  fieldsFor,
  POWERS,
  POWER_GROUPS,
  readiness,
  type Power,
  type PowerEffect,
  type PowerGroup,
  type PowerNeeds,
  type PowerParam,
} from "./powers";

export { POWER_CATALOGS, tp, tpRaw, type PowerMessageKey } from "./strings";

export { default as PowerDock } from "./PowerDock.vue";
