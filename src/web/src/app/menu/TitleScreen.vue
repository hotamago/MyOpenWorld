<script setup lang="ts">
/**
 * Màn hình mở đầu.
 *
 * Không tự quyết định gì — nó chỉ phát ba việc người chơi có thể muốn làm ở
 * đây (`play`, `settings`, `codex`) và để nơi nối nó (một `.vue` khác, ngoài
 * thư mục này) quyết định chuyển màn hình bằng `nextScreen` ở `menu.ts`.
 *
 * ## Vì sao có ô nhập seed ở chính màn hình này
 *
 * Thế giới xác định theo seed: cùng một hạt giống luôn sinh ra đúng một thế
 * giới. Đặt ô chọn nó ngay ở cửa vào, thay vì chôn trong một panel thiết lập,
 * là để "chọn thế giới nào" là một quyết định người chơi nhìn thấy trước khi
 * bước vào — không phải một tham số ẩn chỉ dân kỹ thuật mới biết tìm.
 *
 * ## Vì sao chuyển động nền chỉ là CSS
 *
 * Một quầng sáng loang rất chậm phía sau tên trò chơi là đủ để màn hình
 * không đứng hình chết, và một `@keyframes` không tốn draw call Pixi nào —
 * canvas thật của trò chơi còn chưa cần dựng ở màn hình này. `@media
 * (prefers-reduced-motion: reduce)` tắt hẳn animation cho người nhạy cảm với
 * chuyển động, độc lập với `Settings.reduceMotion` (màn hình này chạy trước
 * khi thiết lập được nạp vào bất kỳ component nào khác).
 */
import { ref } from "vue";
import { t } from "@/i18n";
import { tm } from "./strings";

const emit = defineEmits<{
  play: [seed: number];
  settings: [];
  codex: [];
}>();

/**
 * Hạt giống thế giới đang chọn.
 *
 * Sinh sẵn một số thay vì để ô trống: "chọn seed" là một việc tùy chọn, và
 * bắt người chơi phải tự nghĩ ra một con số trước khi bấm được nút chính sẽ
 * biến một việc tùy chọn thành một việc bắt buộc.
 */
const seed = ref(Math.floor(Math.random() * 1_000_000));

function play(): void {
  emit("play", seed.value);
}

/** Enter trong ô seed cũng vào thế giới — không bắt buộc phải rời bàn phím. */
function onSeedKeydown(e: KeyboardEvent): void {
  if (e.key === "Enter") play();
}
</script>

<template>
  <div class="title">
    <div class="glow" aria-hidden="true"></div>

    <div class="content">
      <h1>{{ t("app.title") }}</h1>
      <p class="tagline">{{ tm("title.tagline") }}</p>

      <div class="seedrow">
        <label for="mow-seed">{{ tm("title.seedLabel") }}</label>
        <input
          id="mow-seed"
          v-model.number="seed"
          type="number"
          step="1"
          min="0"
          @keydown="onSeedKeydown"
        />
        <span class="hint">{{ tm("title.seedHint") }}</span>
      </div>

      <div class="actions">
        <button type="button" class="primary" @click="play">{{ tm("title.play") }}</button>
        <button type="button" @click="emit('settings')">{{ tm("menu.settings") }}</button>
        <button type="button" @click="emit('codex')">{{ tm("menu.codex") }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Tông hợp với App.vue: nền tối #070910, chữ #e6e9ef, nhấn #f0c674. */
/* Phủ **cả** viewport, kể cả thanh HUD phía trên: một màn hình mở đầu để lộ
   thanh trạng thái của một thế giới chưa tồn tại là một màn hình nói dối. */
.title {
  position: fixed;
  inset: 0;
  z-index: 30;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: #070910;
  color: #e6e9ef;
  font: 14px/1.6 ui-sans-serif, system-ui, sans-serif;
}

/* Quầng sáng rất mờ, loang chậm phía sau tên trò chơi. `blur` thay cho một
   texture: một khối gradient duy nhất, không tốn thêm draw call nào. */
.glow {
  position: absolute;
  inset: -20%;
  background: radial-gradient(circle at 50% 42%, rgb(240 198 116 / 12%) 0%, rgb(240 198 116 / 0%) 42%),
    radial-gradient(circle at 50% 55%, rgb(75 131 196 / 10%) 0%, rgb(75 131 196 / 0%) 55%);
  animation: breathe 14s ease-in-out infinite;
  pointer-events: none;
}
@keyframes breathe {
  0%,
  100% {
    transform: scale(1) translateY(0);
    opacity: 0.85;
  }
  50% {
    transform: scale(1.06) translateY(-1.5%);
    opacity: 1;
  }
}
@media (prefers-reduced-motion: reduce) {
  .glow {
    animation: none;
  }
}

.content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.4rem;
  max-width: 30rem;
  padding: 2rem;
  text-align: center;
}

h1 {
  margin: 0;
  font: 700 2.6rem/1.2 ui-sans-serif, system-ui, sans-serif;
  letter-spacing: 0.03em;
  color: #f2f4f8;
  text-shadow: 0 0 32px rgb(240 198 116 / 18%);
}

.tagline {
  margin: 0;
  color: #7f8ea3;
  font-size: 0.95rem;
  line-height: 1.6;
}

.seedrow {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.3rem;
}
.seedrow label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #7f8ea3;
}
.seedrow input {
  width: 12rem;
  padding: 0.4rem 0.6rem;
  border: 1px solid #232833;
  border-radius: 4px;
  background: #10131a;
  color: #e6e9ef;
  font: 13px ui-monospace, monospace;
  text-align: center;
}
.seedrow .hint {
  color: #6b7788;
  font-size: 11px;
}

.actions {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  width: 100%;
  max-width: 18rem;
}
button {
  padding: 0.55rem 1rem;
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

/* Bàn phím phải thấy được đang ở đâu — viền sáng, không dựa vào màu nền đổi
   một chút mà mắt dễ bỏ qua. */
button:focus-visible,
input:focus-visible {
  outline: 2px solid #f0c674;
  outline-offset: 2px;
}
</style>
