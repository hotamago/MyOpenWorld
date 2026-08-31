<script setup lang="ts">
/**
 * Thư viện tri thức: giải thích cơ chế trò chơi cho Người, không phải một
 * bảng đổi thiết lập.
 *
 * ## Vì sao nội dung là một mảng dữ liệu, không phải sáu khối lặp lại trong
 * `<template>`
 *
 * Mục lục bên trái và nội dung bên phải phải luôn khớp nhau — thêm một mục
 * mới chỉ nên là thêm một phần tử vào `SECTIONS`, không phải sửa hai chỗ
 * (danh sách nút bấm, và khối nội dung tương ứng) mà quên mất một trong hai
 * là chuyện dễ xảy ra khi sửa tay.
 */
import { computed, onMounted, onUnmounted, ref } from "vue";
import { tm, type MenuMessageKey } from "./strings";

interface Section {
  id: string;
  titleKey: MenuMessageKey;
  bodyKey: MenuMessageKey;
}

/**
 * Sáu mục bắt buộc: nhịp và tốc độ thời gian, lát cắt z, vật liệu và content
 * pack, chuỗi nhân quả, Ý chỉ và cách xem trước, lịch sinh hoạt của cư dân.
 * Thứ tự ở đây là thứ tự trong mục lục.
 */
const SECTIONS: Section[] = [
  { id: "time", titleKey: "codex.section.time.title", bodyKey: "codex.section.time.body" },
  { id: "layers", titleKey: "codex.section.layers.title", bodyKey: "codex.section.layers.body" },
  {
    id: "materials",
    titleKey: "codex.section.materials.title",
    bodyKey: "codex.section.materials.body",
  },
  {
    id: "causality",
    titleKey: "codex.section.causality.title",
    bodyKey: "codex.section.causality.body",
  },
  { id: "will", titleKey: "codex.section.will.title", bodyKey: "codex.section.will.body" },
  {
    id: "schedule",
    titleKey: "codex.section.schedule.title",
    bodyKey: "codex.section.schedule.body",
  },
];

const emit = defineEmits<{ close: [] }>();

const activeId = ref(SECTIONS[0]!.id);

const active = computed(() => SECTIONS.find((s) => s.id === activeId.value) ?? SECTIONS[0]!);

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") emit("close");
}

onMounted(() => globalThis.addEventListener("keydown", onKeydown));
onUnmounted(() => globalThis.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="scrim" @click.self="emit('close')">
    <section class="panel" role="dialog" aria-modal="true" :aria-label="tm('codex.title')">
      <header>
        <h1>{{ tm("codex.title") }}</h1>
        <button type="button" class="iconbtn" :aria-label="tm('menu.close')" @click="emit('close')">
          ×
        </button>
      </header>

      <div class="body">
        <nav class="toc">
          <button
            v-for="s in SECTIONS"
            :key="s.id"
            type="button"
            class="tocitem"
            :class="{ on: s.id === activeId }"
            :aria-current="s.id === activeId ? 'true' : undefined"
            @click="activeId = s.id"
          >
            {{ tm(s.titleKey) }}
          </button>
        </nav>

        <article class="entry">
          <h2>{{ tm(active.titleKey) }}</h2>
          <p>{{ tm(active.bodyKey) }}</p>
        </article>
      </div>
    </section>
  </div>
</template>

<style scoped>
/* Tông hợp với App.vue: nền tối #070910 / #0c0f15, chữ #e6e9ef, viền #232833,
   nhãn mờ #7f8ea3, nhấn #f0c674. */
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
  width: min(42rem, 92vw);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  border: 1px solid #232833;
  border-radius: 8px;
  background: #0c0f15;
  color: #e6e9ef;
  font: 13px/1.6 ui-sans-serif, system-ui, sans-serif;
  box-shadow: 0 12px 48px rgb(0 0 0 / 55%);
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.2rem;
  border-bottom: 1px solid #1b2029;
  flex: none;
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

.body {
  display: flex;
  min-height: 0;
  flex: 1;
}

.toc {
  flex: none;
  width: 13rem;
  display: flex;
  flex-direction: column;
  padding: 0.6rem;
  gap: 0.15rem;
  border-right: 1px solid #1b2029;
  background: #070910;
  overflow-y: auto;
}
.tocitem {
  padding: 0.5rem 0.6rem;
  border: 1px solid transparent;
  border-radius: 4px;
  background: transparent;
  color: #93a1b5;
  font: 12px ui-sans-serif, system-ui, sans-serif;
  text-align: left;
  cursor: pointer;
}
.tocitem:hover {
  color: #d8dee9;
  background: #10131a;
}
.tocitem.on {
  border-color: #4b83c4;
  background: #16233a;
  color: #d8e8ff;
}

.entry {
  flex: 1;
  min-width: 0;
  padding: 1.2rem 1.4rem;
  overflow-y: auto;
}
.entry h2 {
  margin: 0 0 0.7rem;
  font-size: 1rem;
  color: #f0c674;
  letter-spacing: 0.02em;
}
.entry p {
  margin: 0;
  color: #d8dee9;
}

button:focus-visible {
  outline: 2px solid #f0c674;
  outline-offset: 2px;
}
</style>
