<script setup lang="ts">
/**
 * Menu tạm dừng: lớp phủ mờ nền, hiện khi `nextScreen` đưa màn hình sang
 * `"paused"` (tức người chơi vừa bấm `Esc` giữa thế giới).
 *
 * ## Vì sao tóm tắt thế giới nhận qua props, không tự hỏi API
 *
 * Component này không import `@/api/game` — nó chỉ hiện bốn con số cha đã có
 * sẵn (`seed`, `tick`, `stateHash`, `population`). Tự hỏi lại API ở đây sẽ
 * tạo một nguồn dữ liệu thứ hai cho đúng thứ `App.vue` đã đang hỏi mỗi vòng
 * `refresh()` — hai nguồn có thể trôi khỏi nhau một nhịp, và một menu tạm
 * dừng hiện `tick` cũ hơn HUD phía sau nó là một chi tiết nhỏ nhưng phá vỡ
 * lòng tin vào "trạng thái hiện tại".
 *
 * ## Vì sao `Esc` tự đóng ở đây, không đợi cha
 *
 * Bảng chuyển trong `menu.ts` nói rõ: `esc` ở `paused` về `world`, giống hệt
 * `resume`. Bắt phím ngay trong component này (thay vì để cha lắng nghe rồi
 * gọi xuống) giữ cho quy tắc "phím nào làm gì" nằm cạnh chỗ nó áp dụng, và
 * component tự dọn listener lúc `unmounted` — không rò rỉ khi bị gỡ khỏi cây.
 */
import { onMounted, onUnmounted, ref } from "vue";
import { tm } from "./strings";

defineProps<{
  seed: number;
  tick: number;
  stateHash: string;
  population: number;
}>();

const emit = defineEmits<{
  resume: [];
  settings: [];
  codex: [];
  quitToTitle: [];
}>();

const resumeBtn = ref<HTMLButtonElement | null>(null);

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") emit("resume");
}

onMounted(() => {
  globalThis.addEventListener("keydown", onKeydown);
  // Đưa tiêu điểm vào hành động mặc định ngay khi mở: một menu tạm dừng là
  // một hộp thoại modal, và người chơi vừa rời chuột (họ bấm `Esc`) nên
  // "phím tiếp theo làm gì" phải rõ ràng mà không cần `Tab` trước.
  resumeBtn.value?.focus();
});
onUnmounted(() => {
  globalThis.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="scrim" @click.self="emit('resume')">
    <section class="panel" role="dialog" aria-modal="true" :aria-label="tm('pause.title')">
      <h1>{{ tm("pause.title") }}</h1>

      <dl class="summary">
        <div>
          <dt>{{ tm("pause.seed") }}</dt>
          <dd>{{ seed }}</dd>
        </div>
        <div>
          <dt>{{ tm("pause.tick") }}</dt>
          <dd>{{ tick }}</dd>
        </div>
        <div>
          <dt>{{ tm("pause.stateHash") }}</dt>
          <dd class="mono" :title="stateHash">#{{ stateHash.slice(0, 8) }}</dd>
        </div>
        <div>
          <dt>{{ tm("pause.population") }}</dt>
          <dd>{{ population }}</dd>
        </div>
      </dl>

      <nav class="actions">
        <button ref="resumeBtn" type="button" class="primary" @click="emit('resume')">
          {{ tm("menu.resume") }}
        </button>
        <button type="button" @click="emit('settings')">{{ tm("menu.settings") }}</button>
        <button type="button" @click="emit('codex')">{{ tm("menu.codex") }}</button>
        <button type="button" class="quit" @click="emit('quitToTitle')">
          {{ tm("menu.quitToTitle") }}
        </button>
      </nav>
    </section>
  </div>
</template>

<style scoped>
/* Tông hợp với App.vue: nền tối #0c0f15, chữ #e6e9ef, viền #232833, nhấn
   #f0c674. Lớp phủ mờ hẳn thế giới phía sau — người chơi phải cảm được là
   thời gian đã dừng, không chỉ đọc được nó qua chữ. */
.scrim {
  position: fixed;
  inset: 0;
  z-index: 35;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgb(4 5 9 / 72%);
  backdrop-filter: blur(2px);
}

.panel {
  width: min(22rem, 90vw);
  padding: 1.4rem 1.5rem;
  border: 1px solid #232833;
  border-radius: 8px;
  background: #0c0f15;
  color: #e6e9ef;
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif;
  box-shadow: 0 12px 48px rgb(0 0 0 / 55%);
}

h1 {
  margin: 0 0 0.9rem;
  font-size: 1.15rem;
  letter-spacing: 0.03em;
  color: #f2f4f8;
  text-align: center;
}

.summary {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem 0.8rem;
  margin: 0 0 1.1rem;
  padding: 0.7rem 0.8rem;
  border: 1px solid #1b2029;
  border-radius: 5px;
  background: #10131a;
}
.summary div {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}
dt {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #7f8ea3;
}
dd {
  margin: 0;
  font: 13px ui-monospace, monospace;
  color: #d8dee9;
}
dd.mono {
  color: #9ec1ff;
}

.actions {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
button {
  padding: 0.5rem 0.9rem;
  border: 1px solid #3a4553;
  border-radius: 4px;
  background: #1a1f28;
  color: #d8dee9;
  font: 13px ui-sans-serif, system-ui, sans-serif;
  cursor: pointer;
}
button:hover {
  background: #232a35;
}
button.primary {
  border-color: #6a5426;
  background: #2a2415;
  color: #f0c674;
  font-weight: 600;
}
button.primary:hover {
  background: #362c19;
}
button.quit {
  color: #93a1b5;
}

button:focus-visible {
  outline: 2px solid #f0c674;
  outline-offset: 2px;
}
</style>
