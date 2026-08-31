/**
 * Chỗ nối gọn cho hệ thống menu.
 *
 * Nơi ghép menu vào `App.vue` chỉ cần một dòng `import { ... } from
 * "@/app/menu"` thay vì rải năm đường dẫn file riêng lẻ. Xuất lại nguyên vẹn
 * — không đổi tên, không bọc thêm — nên đây không phải chỗ để thêm logic.
 */
export {
  DEFAULT_SETTINGS,
  MAX_SPEED_INDEX,
  loadSettings,
  nextScreen,
  sanitize,
  saveSettings,
  type Screen,
  type ScreenAction,
  type Settings,
} from "./menu";

export { MENU_CATALOGS, tm, type MenuMessageKey } from "./strings";

export { default as TitleScreen } from "./TitleScreen.vue";
export { default as PauseMenu } from "./PauseMenu.vue";
export { default as SettingsPanel } from "./SettingsPanel.vue";
export { default as CodexPanel } from "./CodexPanel.vue";
