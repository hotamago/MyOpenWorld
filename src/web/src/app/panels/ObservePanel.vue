<script setup lang="ts">
/**
 * Panel Quan sát (PG-14): bám theo một cư dân và đọc dòng đời riêng của họ.
 *
 * Component này không tự giữ trạng thái camera — nó chỉ phát `toggleFollow`
 * và để cha (nơi vòng lặp `refresh()` sống, trong `App.vue`) quyết định gọi
 * `followStep` mỗi lần vẽ lại. Lý do: camera là một khái niệm của khung nhìn,
 * còn panel này chỉ là một mặt hiển thị dữ liệu — trộn hai việc vào một
 * component thì tách chúng ra sau này (ví dụ khi có nhiều camera, hoặc chế độ
 * xem tách khỏi bám theo) sẽ phải viết lại từ đầu.
 *
 * ## Vì sao thiếu `card.fatigue` / `card.home` / `card.work`
 *
 * Bốn khóa `card.*` đã có sẵn trong `i18n`, nhưng `Entity` (`api/game.ts`)
 * hiện chỉ mang `role`, `intent`, `hunger` — không có mệt mỏi, nhà, hay chỗ
 * làm. Hiện các nhãn đó ở đây mà không có dữ liệu đứng sau sẽ là bịa ra một
 * con số không tồn tại, đúng thứ dự án này nhất quyết tránh (`§18.13`,
 * `§22.17`). Panel chỉ dùng `card.needs` (bọc `hunger`, trường thật) và
 * `card.unknown` (nhãn khi giá trị thật là `null`) — không dùng ba khóa kia.
 * Khi `Entity` được bổ sung dữ liệu đó, phần hiện chúng đi kèm luôn.
 */
import { computed } from "vue";
import type { Entity, WorldEvent } from "@/api/game";
import { t } from "@/i18n";
import { lifeOf } from "../observe";

const props = defineProps<{
  target: Entity | null;
  events: WorldEvent[];
  following: boolean;
}>();

const emit = defineEmits<{
  toggleFollow: [];
  trace: [seq: number];
}>();

/**
 * Dòng đời của mục tiêu đang chọn.
 *
 * Không giới hạn thêm ở đây: `events` do cha truyền vào đã bị nhật ký chung
 * (server + `App.vue`) giới hạn 60 mắt gần nhất, nên dòng đời của một người —
 * vốn là tập con của đó — không bao giờ dài hơn thế.
 */
const life = computed(() => (props.target ? lifeOf(props.target.id, props.events) : []));
</script>

<template>
  <div class="observe">
    <h2>{{ t("observe.title") }}</h2>

    <template v-if="target">
      <div class="who">
        <strong class="name">{{ target.name }}</strong>
        <button
          type="button"
          class="follow"
          :class="{ on: following }"
          @click="emit('toggleFollow')"
        >
          {{ following ? t("observe.unfollow") : t("observe.follow") }}
        </button>
      </div>
      <p v-if="following" class="dim tag">{{ t("observe.following") }}</p>

      <table>
        <tbody>
          <tr>
            <td>{{ t("panel.who.role") }}</td>
            <td>{{ target.role ?? t("card.unknown") }}</td>
          </tr>
          <tr>
            <td>{{ t("panel.who.doing") }}</td>
            <td>{{ target.intent ?? t("card.unknown") }}</td>
          </tr>
        </tbody>
      </table>

      <section v-if="target.hunger !== null" class="needs">
        <h3>{{ t("card.needs") }}</h3>
        <p>{{ t("panel.who.hunger") }}: {{ target.hunger }}</p>
      </section>

      <h3>{{ t("observe.timeline") }}</h3>
      <ul v-if="life.length" class="life">
        <li
          v-for="entry in life"
          :key="entry.seq"
          class="entry"
          :class="entry.role"
          @click="emit('trace', entry.seq)"
        >
          <span class="dim tick">t{{ entry.tick }}</span>
          <span class="text">{{ entry.text }}</span>
        </li>
      </ul>
      <p v-else class="dim">{{ t("observe.empty") }}</p>
    </template>

    <p v-else class="dim">{{ t("observe.hint") }}</p>
  </div>
</template>

<style scoped>
/* Cùng tông với `App.vue`: nền tối `#0c0f15`, chữ `#e6e9ef`, nhãn mờ
   `#7f8ea3`, nhấn vàng `#f0c674` — panel này phải trông như nó vốn thuộc
   khung nhìn đó, không phải một mảnh ghép ngoài vào. */
.observe {
  background: #0c0f15;
  color: #e6e9ef;
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif;
  padding: 0.7rem 0.8rem;
}
h2 {
  margin: 0 0 0.45rem;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #7f8ea3;
  font-weight: 600;
}
h3 {
  margin: 0.6rem 0 0.3rem;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #7f8ea3;
  font-weight: 600;
}
.who {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.follow {
  flex: none;
  padding: 3px 8px;
  border: 1px solid #3a4553;
  border-radius: 3px;
  background: #1a1f28;
  color: #d8dee9;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.follow:hover {
  background: #232a35;
}
/* Đang bám thì nút phải nói ngay là "đang" chứ không chỉ chờ người chơi nhớ —
   viền và chữ ngả sang màu nhấn, cùng vai với nút "Khắc" ở panel Ý chỉ. */
.follow.on {
  border-color: #6a5426;
  background: #2a2415;
  color: #f0c674;
}
.tag {
  margin: 0.3rem 0 0;
  font-size: 11px;
}
.dim {
  color: #7f8ea3;
}
table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 0.5rem;
}
td {
  padding: 1px 0;
}
td:first-child {
  color: #7f8ea3;
  width: 6.5rem;
}
.needs {
  padding-top: 0.2rem;
}
.needs p {
  margin: 0;
}
.life {
  list-style: none;
  margin: 0;
  padding: 0;
}
.life .entry {
  padding: 2px 0;
  cursor: pointer;
  display: flex;
  gap: 0.4rem;
}
.life .entry:hover .text {
  color: #cfd8e3;
}
.life .tick {
  flex: none;
  font: 11px ui-monospace, monospace;
}
/* Vai trong đời của người đang xem đổi màu chữ tường thuật: chủ thể (bị tác
   động) nổi bật nhất — đó thường là mấu chốt câu chuyện của họ; người chứng
   kiến mờ nhất vì tín hiệu gắn họ với sự kiện đó là yếu nhất (`observe.ts`). */
.life .entry.subject .text {
  color: #f0c674;
}
.life .entry.actor .text {
  color: #e6e9ef;
}
.life .entry.bystander .text {
  color: #7f8ea3;
}
</style>
