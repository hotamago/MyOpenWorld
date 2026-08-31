<script setup lang="ts">
/**
 * Panel thiết lập: mỗi trường của `Settings` (`menu.ts`) có đúng một điều
 * khiển, không hơn.
 *
 * ## Vì sao `model.value = { ...model.value, [key]: value }` chứ không gán
 * thẳng vào trường
 *
 * `defineModel` phát `update:modelValue` khi **gán lại `.value`** — nó là một
 * `computed` có setter đứng sau, không phải một object thường. Sửa thẳng một
 * trường (`model.value.showGrid = true`) chỉ đổi object bên trong, không đi
 * qua setter đó, nên cha (nơi lưu `saveSettings`) sẽ không hay biết gì đã
 * đổi. Luôn tạo một object mới rồi gán cả cục vào `.value` để mỗi lần đổi một
 * trường đều là một lần `update:modelValue` thật.
 */
import { onMounted, onUnmounted } from "vue";
import { DEFAULT_SETTINGS, MAX_SPEED_INDEX, type Settings } from "./menu";
import { tm } from "./strings";

/** Nấc cỡ chữ giao diện hỗ trợ — đúng ba giá trị nêu trong tài liệu `Settings`. */
const UI_SCALES = [90, 100, 115] as const;

const model = defineModel<Settings>({ required: true });

const emit = defineEmits<{ close: [] }>();

function set<K extends keyof Settings>(key: K, value: Settings[K]): void {
  model.value = { ...model.value, [key]: value };
}

function restoreDefaults(): void {
  // Bản sao mới, không phải tham chiếu tới `DEFAULT_SETTINGS`: nếu component
  // khác giữ tham chiếu ra `model.value` rồi lỡ sửa nó, hằng số dùng chung
  // cho mọi lần "khôi phục mặc định" sau đó không được phép bị đổi theo.
  model.value = { ...DEFAULT_SETTINGS };
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") emit("close");
}

onMounted(() => globalThis.addEventListener("keydown", onKeydown));
onUnmounted(() => globalThis.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="scrim" @click.self="emit('close')">
    <section class="panel" role="dialog" aria-modal="true" :aria-label="tm('settings.title')">
      <header>
        <h1>{{ tm("settings.title") }}</h1>
        <button type="button" class="iconbtn" :aria-label="tm('menu.close')" @click="emit('close')">
          ×
        </button>
      </header>

      <div class="field">
        <span class="label">{{ tm("settings.locale") }}</span>
        <div class="row">
          <button
            type="button"
            class="chip"
            :class="{ on: model.locale === 'vi' }"
            @click="set('locale', 'vi')"
          >
            {{ tm("settings.locale.vi") }}
          </button>
          <button
            type="button"
            class="chip"
            :class="{ on: model.locale === 'en' }"
            @click="set('locale', 'en')"
          >
            {{ tm("settings.locale.en") }}
          </button>
        </div>
      </div>

      <div class="field">
        <label for="mow-speed">{{ tm("settings.speedIndex") }}</label>
        <div class="row">
          <input
            id="mow-speed"
            type="range"
            min="0"
            :max="MAX_SPEED_INDEX"
            step="1"
            :value="model.speedIndex"
            @input="set('speedIndex', Number(($event.target as HTMLInputElement).value))"
          />
          <span class="val">{{ model.speedIndex }}</span>
        </div>
      </div>

      <div class="field">
        <span class="label">{{ tm("settings.uiScale") }}</span>
        <div class="row">
          <button
            v-for="opt in UI_SCALES"
            :key="opt"
            type="button"
            class="chip"
            :class="{ on: model.uiScale === opt }"
            @click="set('uiScale', opt)"
          >
            {{ opt }}%
          </button>
        </div>
      </div>

      <label class="checkline">
        <input
          type="checkbox"
          :checked="model.showLabels"
          @change="set('showLabels', ($event.target as HTMLInputElement).checked)"
        />
        {{ tm("settings.showLabels") }}
      </label>

      <label class="checkline">
        <input
          type="checkbox"
          :checked="model.showGrid"
          @change="set('showGrid', ($event.target as HTMLInputElement).checked)"
        />
        {{ tm("settings.showGrid") }}
      </label>

      <label class="checkline">
        <input
          type="checkbox"
          :checked="model.reduceMotion"
          @change="set('reduceMotion', ($event.target as HTMLInputElement).checked)"
        />
        {{ tm("settings.reduceMotion") }}
      </label>

      <div class="actions">
        <button type="button" @click="restoreDefaults">{{ tm("menu.restoreDefaults") }}</button>
        <button type="button" class="primary" @click="emit('close')">{{ tm("menu.close") }}</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
/* Tông hợp với App.vue: nền tối #0c0f15, chữ #e6e9ef, viền #232833, nhãn mờ
   #7f8ea3, nhấn #f0c674. */
.scrim {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgb(4 5 9 / 72%);
  backdrop-filter: blur(2px);
}

.panel {
  width: min(24rem, 90vw);
  max-height: 85vh;
  overflow-y: auto;
  padding: 1.2rem 1.4rem 1.4rem;
  border: 1px solid #232833;
  border-radius: 8px;
  background: #0c0f15;
  color: #e6e9ef;
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif;
  box-shadow: 0 12px 48px rgb(0 0 0 / 55%);
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.9rem;
}
h1 {
  margin: 0;
  font-size: 1.05rem;
  letter-spacing: 0.03em;
  color: #f2f4f8;
}
.iconbtn {
  width: 24px;
  height: 22px;
  padding: 0;
  border: 1px solid #333c4a;
  border-radius: 3px;
  background: #171b22;
  color: #9aa7b8;
  line-height: 1;
  cursor: pointer;
}
.iconbtn:hover {
  background: #232a35;
}

.field {
  margin-bottom: 0.9rem;
}
.field .label,
.field label {
  display: block;
  margin-bottom: 0.35rem;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #7f8ea3;
}
.row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.row input[type="range"] {
  flex: 1;
  accent-color: #4b83c4;
}
.val {
  min-width: 1.6rem;
  text-align: right;
  font: 12px ui-monospace, monospace;
  color: #cfd8e3;
}

.chip {
  padding: 0.3rem 0.65rem;
  border: 1px solid #232833;
  border-radius: 999px;
  background: #10131a;
  color: #e6e9ef;
  font: 12px ui-sans-serif, system-ui, sans-serif;
  cursor: pointer;
}
.chip:hover {
  border-color: #333c4a;
}
.chip.on {
  border-color: #4b83c4;
  background: #26456e;
  color: #d8e8ff;
}

.checkline {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.65rem;
  color: #d8dee9;
  cursor: pointer;
}
.checkline input {
  accent-color: #f0c674;
}

.actions {
  display: flex;
  justify-content: space-between;
  gap: 0.6rem;
  margin-top: 1rem;
  padding-top: 0.9rem;
  border-top: 1px solid #1b2029;
}
.actions button {
  flex: 1;
  padding: 0.5rem 0.8rem;
  border: 1px solid #3a4553;
  border-radius: 4px;
  background: #1a1f28;
  color: #d8dee9;
  font: 13px ui-sans-serif, system-ui, sans-serif;
  cursor: pointer;
}
.actions button:hover {
  background: #232a35;
}
.actions button.primary {
  border-color: #6a5426;
  background: #2a2415;
  color: #f0c674;
  font-weight: 600;
}
.actions button.primary:hover {
  background: #362c19;
}

button:focus-visible,
input:focus-visible {
  outline: 2px solid #f0c674;
  outline-offset: 2px;
}
</style>
