<script setup lang="ts">
/**
 * Panel Biên niên sử: dệt nhật ký sự kiện thô (`events`) thành các dòng sử
 * đọc được, cộng một dải nhịp hoạt động theo ngày.
 *
 * ## Vì sao panel này mỏng
 *
 * Toàn bộ quyết định "gộp bao nhiêu là một chuyến đi", "loại nào là biến cố"
 * nằm ở `weave()`/`pulse()` (`chronicle.ts`, thuần, có test riêng). Panel chỉ
 * làm ba việc nó thật sự cần DOM: giữ trạng thái bộ lọc, gọi `weave`/`pulse`
 * lại mỗi khi `events`/`entities` đổi, và vẽ. Trộn logic gộp vào component
 * Vue sẽ buộc mọi bài kiểm cho nó phải dựng cả Vue lẫn DOM — chỉ để kiểm một
 * phép gộp mảng thuần túy.
 *
 * ## Vì sao dải nhịp là `div`, không phải canvas
 *
 * Yêu cầu của lát cắt này cấm canvas và thư viện biểu đồ. Một dải cột không
 * cần vẽ điểm ảnh: mỗi ngày là một `div` cao theo tỉ lệ `count / max`, xếp
 * bằng flexbox — CSS làm hết phần bố cục, và nó vẫn co giãn đúng khi panel
 * đổi bề rộng, thứ một canvas có kích thước cố định không tự làm được.
 *
 * ## Vì sao nhóm theo ngày mà không sắp lại
 *
 * `weave()` đã trả về đúng thứ tự "mới nhất trước". Nhóm ở đây chỉ cắt dãy đó
 * thành từng đoạn liên tiếp cùng `day` — không gom rồi sắp lại theo ngày,
 * vì làm vậy sẽ đảo thứ tự bên trong một ngày nếu tương lai `weave()` từng
 * trả về một chương "muộn" xen giữa hai chương "sớm hơn" cùng ngày (không xảy
 * ra hôm nay, nhưng cắt-không-sắp là bất biến đúng trong mọi trường hợp, còn
 * gom-rồi-sắp là một bất biến chỉ đúng tình cờ).
 */
import { computed, ref } from "vue";
import type { Entity, WorldEvent } from "@/api/game";
import { pulse, weave, type Chapter } from "./chronicle";
import { t, tc } from "./strings";

const props = defineProps<{
  events: WorldEvent[];
  entities: Entity[];
}>();

const emit = defineEmits<{
  trace: [seq: number];
}>();

/** *tất cả* hay *chỉ biến cố* (`weight >= 1`) — nền (`weight: 0`) ẩn đi. */
const filter = ref<"all" | "notable">("all");

/** Bảng `id -> tên`, dựng lại mỗi khi danh sách thực thể hiện có đổi. */
const names = computed(() => new Map(props.entities.map((e) => [e.id, e.name])));

const chapters = computed<Chapter[]>(() => weave(props.events, names.value));

const visible = computed<Chapter[]>(() =>
  filter.value === "all" ? chapters.value : chapters.value.filter((c) => c.weight >= 1),
);

interface DayGroup {
  day: number;
  chapters: Chapter[];
}

const groups = computed<DayGroup[]>(() => {
  const out: DayGroup[] = [];
  for (const c of visible.value) {
    const last = out.at(-1);
    if (last !== undefined && last.day === c.day) last.chapters.push(c);
    else out.push({ day: c.day, chapters: [c] });
  }
  return out;
});

/** Dải nhịp tính trên **toàn bộ** `events`, không phải `visible` — bộ lọc chỉ
 * ẩn dòng chữ, chưa từng có ý nghĩa "đã xảy ra ít hơn" ở tầng dữ liệu. */
const bars = computed(() => pulse(props.events));
const maxCount = computed(() => bars.value.reduce((m, b) => Math.max(m, b.count), 0));

function heightPercent(count: number): number {
  // Sàn 6% chứ không phải 0: một ngày có đúng một sự kiện vẫn phải hiện một
  // vệt thấy được, không phải một khoảng trống trông như không có dữ liệu.
  if (maxCount.value <= 0) return 0;
  return Math.max(6, Math.round((count / maxCount.value) * 100));
}

function barTitle(b: { day: number; count: number }): string {
  return `${tc("chronicle.day", { day: b.day })} — ${b.count}`;
}

function weightClass(w: Chapter["weight"]): string {
  return w === 2 ? "notable" : w === 1 ? "normal" : "background";
}
</script>

<template>
  <div class="chronicle">
    <h2>{{ t("chronicle.title") }}</h2>

    <!-- Trang trí, không phải nguồn dữ liệu duy nhất: mọi con số ở đây cũng
         đọc được bằng chữ trong danh sách chương bên dưới. -->
    <div class="pulse" aria-hidden="true">
      <div
        v-for="b in bars"
        :key="b.day"
        class="bar"
        :style="{ height: heightPercent(b.count) + '%' }"
        :title="barTitle(b)"
      />
    </div>

    <div class="filters" role="group">
      <button type="button" :class="{ on: filter === 'all' }" @click="filter = 'all'">
        {{ t("chronicle.filter.all") }}
      </button>
      <button type="button" :class="{ on: filter === 'notable' }" @click="filter = 'notable'">
        {{ t("chronicle.filter.notable") }}
      </button>
    </div>

    <p v-if="visible.length === 0" class="dim empty">{{ t("chronicle.empty") }}</p>

    <div v-else class="chapters">
      <section v-for="group in groups" :key="group.day" class="day-group">
        <h3 class="day-heading">{{ tc("chronicle.day", { day: group.day }) }}</h3>
        <ul>
          <li
            v-for="c in group.chapters"
            :key="c.seq"
            class="chapter"
            :class="weightClass(c.weight)"
            @click="emit('trace', c.seq)"
          >
            <span class="dim tick">t{{ c.to }}</span>
            <span class="text">{{ tc(c.key, c.slots) }}</span>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>

<style scoped>
/* Cùng tông với `App.vue`/các panel khác: nền `#0c0f15`, chữ `#e6e9ef`, nhãn
   mờ `#7f8ea3`, nhấn vàng `#f0c674`, viền `#232833` — panel này phải trông
   như nó vốn thuộc khung nhìn đó. */
.chronicle {
  background: #0c0f15;
  color: #e6e9ef;
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif;
  padding: 0.7rem 0.8rem;
}
h2 {
  margin: 0 0 0.5rem;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #7f8ea3;
  font-weight: 600;
}
.dim {
  color: #7f8ea3;
}

.pulse {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 36px;
  padding: 3px;
  background: #10141c;
  border: 1px solid #232833;
  border-radius: 4px;
}
.bar {
  flex: 1 1 auto;
  min-width: 2px;
  background: #3a4553;
  border-radius: 1px;
}

.filters {
  display: flex;
  gap: 0.4rem;
  margin: 0.5rem 0;
}
.filters button {
  flex: 1;
  padding: 3px 8px;
  border: 1px solid #3a4553;
  border-radius: 3px;
  background: #1a1f28;
  color: #d8dee9;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.filters button:hover {
  background: #232a35;
}
.filters button.on {
  border-color: #6a5426;
  background: #2a2415;
  color: #f0c674;
}

.empty {
  margin: 0.6rem 0 0;
}

.chapters {
  margin-top: 0.3rem;
}
.day-group + .day-group {
  margin-top: 0.5rem;
}
.day-heading {
  margin: 0 0 0.2rem;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #7f8ea3;
  font-weight: 600;
  border-bottom: 1px solid #232833;
  padding-bottom: 2px;
}
ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

/* Ba hạng, ba cách trông thấy khác nhau — đúng yêu cầu: biến cố nổi bật, bình
   thường không trang trí, nền mờ đi để mắt lướt qua mà không dừng lại. */
.chapter {
  display: flex;
  gap: 0.5rem;
  padding: 3px 6px;
  border-left: 3px solid transparent;
  cursor: pointer;
}
.chapter:hover .text {
  color: #f0c674;
}
.chapter.notable {
  border-left-color: #f0c674;
}
.chapter.normal {
  border-left-color: #232833;
}
.chapter.background {
  border-left-color: #232833;
  color: #7f8ea3;
}
.chapter .tick {
  flex: none;
  font: 11px ui-monospace, monospace;
}
.chapter .text {
  min-width: 0;
}
</style>
