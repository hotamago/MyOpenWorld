<script setup lang="ts">
/**
 * Panel chọn lớp dữ liệu phủ lên bản đồ (`PG-07`, `idea.md §18.5`).
 *
 * Người chơi là một vị thần quan sát, nhưng `terrain.ts` chỉ trả lời được
 * "ô này chất gì" — bốn câu còn lại (cao độ, nước, đi lại được, mật độ người)
 * cần một lớp riêng, bật/tắt được, không trộn vào màu vật liệu.
 *
 * ## Vì sao component này không tự vẽ lên canvas
 *
 * Nó chỉ tính ra một `Uint8ClampedArray` rồi `emit` ra ngoài. Đưa buffer đó
 * lên Pixi là việc của `world.ts`/`App.vue` — chúng biết cách dựng texture,
 * còn panel này thì không cần biết, và nhờ vậy phần tính toán ở `field.ts`
 * (thứ thật sự có thể sai) vẫn kiểm được bằng `vitest` thuần.
 *
 * ## Vì sao thang tối cố định, không nhận `scheme` qua prop
 *
 * Giao diện hiện tại không có công tắc sáng/tối (`App.vue` là nền tối cố
 * định `#0c0f15`) — thêm một prop `scheme` bây giờ là thêm một tham số không
 * ai truyền khác `"dark"`, và một tham số không đổi không phải là cấu hình.
 * Khi nào `App.vue` có chế độ sáng thật, đây là chỗ duy nhất cần sửa.
 */
import { computed, watchEffect } from "vue";
import type { Entity, TileBatch } from "@/api/game";
import { computeField, LAYERS, paintField, type LayerId } from "@/render/overlays/field";
import { formatValue } from "@/render/overlays/datatexture";
import { SCALES, type Scheme } from "@/render/accessibility";
import { t, type MessageKey } from "@/i18n";

/** Thang cố định — lý do ở đầu file. */
const SCHEME: Scheme = "dark";
/** Độ mờ chung của lớp phủ: đủ rõ để đọc, đủ mờ để vẫn thấy địa hình bên dưới. */
const ALPHA = 0.72;

const props = defineProps<{
  batch: TileBatch | null;
  entities: Entity[];
}>();

const emit = defineEmits<{
  paint: [buf: Uint8ClampedArray | null];
}>();

/** Lớp đang chọn; `null` là tắt. */
const layer = defineModel<LayerId | null>({ default: null });

/**
 * Khóa i18n cho mỗi lớp, khai tường minh thay vì ghép chuỗi `overlay.${id}`.
 *
 * `t()` nhận `MessageKey` — một union kiểu chữ, không phải `string` trần
 * (`i18n/index.ts` cố ý làm vậy để thiếu bản dịch là lỗi biên dịch). Ghép
 * chuỗi để tra khóa sẽ cho ra kiểu `string`, xóa mất chính cái kiểm đó.
 */
const LABEL_KEY: Record<LayerId, MessageKey> = {
  elevation: "overlay.elevation",
  water: "overlay.water",
  walkable: "overlay.walkable",
  crowd: "overlay.crowd",
};

/** Trường hiện hành: `null` khi tắt hoặc chưa có lô ô nào tới. */
const field = computed(() => {
  if (layer.value === null || !props.batch) return null;
  return computeField(layer.value, props.batch, props.entities);
});

/**
 * Báo cho cha mỗi khi lớp hoặc dữ liệu đổi.
 *
 * `watchEffect`, không phải theo dõi rồi so sánh: cha cần vẽ lại canvas mỗi
 * lần trường đổi, kể cả khi hai buffer liên tiếp trùng byte-for-byte (ví dụ
 * mật độ người không đổi giữa hai lần hỏi lại) — việc so `Uint8ClampedArray`
 * cũ/mới tốn công đúng bằng việc vẽ lại, nên không đáng làm.
 */
watchEffect(() => {
  emit("paint", field.value ? paintField(field.value, SCHEME, ALPHA) : null);
});

/** Đơn vị hiện kèm số, có khoảng trắng dẫn đầu; rỗng thì không thêm gì. */
const unitSuffix = computed(() => (field.value?.unit ? ` ${field.value.unit}` : ""));

/**
 * Dải màu CSS cho chú giải, dựng từ đúng các mốc của `SCALES` — không bịa
 * thang riêng. `linear-gradient` nội suy tuyến tính giữa các mốc giống hệt
 * cách `paintField` nội suy, nên dải chú giải khớp đúng màu vẽ trên bản đồ.
 */
const gradientCss = computed(() => `linear-gradient(90deg, ${SCALES[SCHEME].join(", ")})`);

/** Bật lớp `id`; bấm lại lớp đang bật thì tắt. */
function pick(id: LayerId): void {
  layer.value = layer.value === id ? null : id;
}
</script>

<template>
  <section class="dataoverlay">
    <h2>{{ t("overlay.title") }}</h2>
    <p class="hint">{{ t("overlay.hint") }}</p>

    <div class="row">
      <button
        type="button"
        class="chip"
        :class="{ on: layer === null }"
        @click="layer = null"
      >
        {{ t("overlay.off") }}
      </button>
      <button
        v-for="id in LAYERS"
        :key="id"
        type="button"
        class="chip"
        :class="{ on: layer === id }"
        @click="pick(id)"
      >
        {{ t(LABEL_KEY[id]) }}
      </button>
    </div>

    <div v-if="field" class="legend">
      <div class="ramp" :style="{ background: gradientCss }"></div>
      <div class="marks">
        <span>{{ t("overlay.legend.low") }} · {{ formatValue(field.min) }}{{ unitSuffix }}</span>
        <span>{{ t("overlay.legend.high") }} · {{ formatValue(field.max) }}{{ unitSuffix }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Tông hợp với `App.vue`: nền tối `#0c0f15`, chữ `#e6e9ef`, nhãn mờ `#7f8ea3`. */
.dataoverlay {
  padding: 0.7rem 0.8rem;
  border-bottom: 1px solid #1b2029;
  background: #0c0f15;
  color: #e6e9ef;
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif;
}
h2 {
  margin: 0 0 0.3rem;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #7f8ea3;
  font-weight: 600;
}
.hint {
  margin: 0 0 0.5rem;
  color: #7f8ea3;
  font-size: 11px;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.chip {
  padding: 0.25rem 0.55rem;
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
  background: #26456e;
  border-color: #4b83c4;
  color: #d8e8ff;
}
.legend {
  margin-top: 0.6rem;
}
.ramp {
  height: 10px;
  border-radius: 3px;
  border: 1px solid #232833;
}
.marks {
  display: flex;
  justify-content: space-between;
  margin-top: 0.3rem;
  font: 11px ui-monospace, monospace;
  color: #7f8ea3;
}
</style>
