<script setup lang="ts">
/**
 * Thanh quyền năng của True God, neo ở đáy màn hình.
 *
 * ## Phạm vi cố ý hẹp
 *
 * `PowerDock` chỉ làm một việc: để Người **chọn quyền năng nào**, và báo cho
 * cha biết bằng `pick(power)`. Nó **không** thu thập tham số (`params`), không
 * gọi `fieldsFor`, không đụng `api.*`. Lý do: `selectedBeing`/`selectedTile`
 * là props chỉ-đọc từ ngoài truyền vào (nơi giữ trạng thái chọn thật là
 * `App.vue`), và bản thân quyền năng còn cần một bước xin tham số (tên mới,
 * vật liệu, ý định...) mà giao diện đó thuộc về màn "Ý chỉ" đã có sẵn — dựng
 * lại nó ở đây là vẽ hai nơi cho cùng một việc. `App.vue` (đang sửa song song,
 * không thuộc phạm vi nhiệm vụ này) nhận `Power` đầy đủ từ sự kiện `pick`, tự
 * quyết định mở gì tiếp theo.
 *
 * ## Vì sao nút bị mờ vẫn phải focus được
 *
 * Một quyền năng thiếu điều kiện (chưa chọn sinh mệnh/ô) dùng `aria-disabled`
 * cộng CSS mờ đi — **không** dùng thuộc tính `disabled` gốc của trình duyệt.
 * `disabled` gốc kéo nút ra khỏi thứ tự Tab, nên người dùng bàn phím không bao
 * giờ biết quyền năng đó tồn tại hay vì sao nó chưa dùng được (`title` sẽ
 * không bao giờ hiện ra). `onPick` mới là nơi thật sự chặn: nút vẫn nhận focus
 * và `Enter`/click gọi được handler, handler tự kiểm `readiness` trước khi
 * phát `pick`.
 */
import { computed } from "vue";
import type { Power, PowerGroup } from "./powers";
import { POWERS, POWER_GROUPS, readiness } from "./powers";
import { tp, tpRaw } from "./strings";

const props = defineProps<{
  selectedBeing: string | null;
  selectedTile: { x: number; y: number } | null;
  active: string | null;
}>();

const emit = defineEmits<{
  pick: [power: Power];
  cancel: [];
}>();

/** Cái mà `readiness` cần — dẫn thẳng từ hai props chỉ-đọc, không giữ bản sao. */
const sel = computed(() => ({
  being: props.selectedBeing !== null,
  tile: props.selectedTile !== null,
}));

/** Quyền năng đã gom theo nhóm, theo đúng thứ tự `POWER_GROUPS`. Bỏ nhóm rỗng. */
const groupedPowers = computed(() =>
  POWER_GROUPS.map((group) => ({ group, powers: POWERS.filter((p) => p.group === group) })).filter(
    (entry) => entry.powers.length > 0,
  ),
);

/**
 * Nhãn của một nhóm. Viết bằng `switch` thay vì một bảng tra `Record` để nhãn
 * vẫn đi qua `tp()` (khóa kiểm bằng kiểu lúc biên dịch) — `PowerGroup` chỉ có
 * năm giá trị, nên liệt kê tay ở đây rẻ hơn dựng thêm một cấu trúc tra cứu.
 */
function groupLabel(group: PowerGroup): string {
  switch (group) {
    case "sight":
      return tp("group.sight");
    case "time":
      return tp("group.time");
    case "land":
      return tp("group.land");
    case "body":
      return tp("group.body");
    case "mind":
      return tp("group.mind");
  }
}

function readinessOf(power: Power): ReturnType<typeof readiness> {
  return readiness(power, sel.value);
}

function labelFor(power: Power): string {
  return tpRaw(`power.${power.id}.label`);
}

/** `title` của nút: lý do chưa dùng được nếu chưa sẵn sàng, gợi ý quyền năng nếu đã sẵn sàng. */
function titleFor(power: Power): string {
  const r = readinessOf(power);
  if (!r.ready) return tp(r.reason === "need_being" ? "reason.need_being" : "reason.need_tile");
  return tpRaw(`power.${power.id}.hint`);
}

/** Chặn thi hành khi chưa sẵn sàng — lưới an toàn cạnh việc mờ nút bằng CSS. */
function onPick(power: Power): void {
  if (!readinessOf(power).ready) return;
  emit("pick", power);
}

function onCancel(): void {
  emit("cancel");
}
</script>

<template>
  <div class="power-dock" role="toolbar" :aria-label="tp('dock.title')" @keydown.escape="onCancel">
    <div class="dock-header">
      <span class="dock-title">{{ tp("dock.title") }}</span>
      <button v-if="active !== null" type="button" class="cancel-btn" @click="onCancel">
        {{ tp("dock.cancel") }}
      </button>
    </div>

    <div class="groups">
      <section v-for="entry in groupedPowers" :key="entry.group" class="group">
        <h3 class="group-label">{{ groupLabel(entry.group) }}</h3>
        <div class="buttons">
          <button
            v-for="power in entry.powers"
            :key="power.id"
            type="button"
            class="power-btn"
            :class="{ 'is-active': active === power.id, 'is-disabled': !readinessOf(power).ready }"
            :aria-disabled="!readinessOf(power).ready"
            :aria-pressed="active === power.id"
            :title="titleFor(power)"
            @click="onPick(power)"
          >
            <span class="glyph" aria-hidden="true">{{ power.glyph }}</span>
            <span class="label">{{ labelFor(power) }}</span>
          </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
/* Cùng tông toàn ứng dụng: nền `#0c0f15`, chữ `#e6e9ef`, nhãn mờ `#7f8ea3`,
   nhấn vàng `#f0c674`, viền `#232833` (đúng bảng `ObservePanel.vue` dùng). */
.power-dock {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 20;
  background: #0c0f15;
  color: #e6e9ef;
  border-top: 1px solid #232833;
  font: 13px/1.4 ui-sans-serif, system-ui, sans-serif;
  padding: 0.4rem 0.7rem 0.5rem;
}
.dock-header {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 0.3rem;
}
.dock-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #7f8ea3;
  font-weight: 600;
}
.cancel-btn {
  margin-left: auto;
  padding: 2px 8px;
  border: 1px solid #6a5426;
  border-radius: 3px;
  background: #2a2415;
  color: #f0c674;
  font: inherit;
  font-size: 11px;
  cursor: pointer;
}
.cancel-btn:hover {
  background: #352c19;
}
.groups {
  display: flex;
  gap: 1rem;
  overflow-x: auto;
  padding-bottom: 2px;
}
.group {
  flex: none;
}
.group-label {
  margin: 0 0 0.25rem;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: #7f8ea3;
  font-weight: 600;
}
.buttons {
  display: flex;
  gap: 0.35rem;
}
.power-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  min-width: 3.4rem;
  padding: 0.3rem 0.4rem;
  border: 1px solid #232833;
  border-radius: 4px;
  background: #141922;
  color: #e6e9ef;
  font: inherit;
  cursor: pointer;
}
.power-btn:hover {
  background: #1b212c;
}
.power-btn:focus-visible {
  outline: 2px solid #f0c674;
  outline-offset: 1px;
}
.power-btn .glyph {
  font-size: 16px;
  line-height: 1;
}
.power-btn .label {
  font-size: 10px;
  color: #7f8ea3;
  white-space: nowrap;
}
/* Đang chọn: viền vàng — đúng màu nhấn dùng chung toàn ứng dụng. */
.power-btn.is-active {
  border-color: #f0c674;
  background: #201a0d;
}
.power-btn.is-active .label {
  color: #f0c674;
}
/* Chưa đủ điều kiện: mờ đi bằng CSS, không dùng `disabled` gốc — xem chú
   thích ở đầu `<script setup>` để biết vì sao nút vẫn phải focus được. */
.power-btn.is-disabled {
  opacity: 0.42;
  cursor: not-allowed;
}
</style>
