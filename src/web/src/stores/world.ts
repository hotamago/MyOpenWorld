/**
 * Store thế giới (Pinia) — `PA-05`.
 *
 * Store giữ **read model**, không giữ luật. Mọi thứ ở đây tới từ server và
 * không được suy diễn thêm: nếu client tự tính "chắc là nhân vật này đang đói"
 * thì nó đang mô phỏng lại thế giới, và hai bản mô phỏng sẽ trôi khỏi nhau.
 */

import { defineStore } from "pinia";
import { ref, shallowRef } from "vue";
import type { EpistemicMode } from "@/api/protocol";

/** Tóm tắt một thực thể như client biết. */
export interface EntityView {
  id: string;
  x: string;
  y: string;
  z: number;
  attrs: Record<string, string | number | boolean>;
}

export const useWorldStore = defineStore("world", () => {
  /** Tick địa phương, dạng chuỗi vì nó là `u64`. */
  const tick = ref("0");
  const divineTick = ref("0");
  /** Hash state, để đối chiếu với repro bundle. */
  const stateHash = ref("");
  const paused = ref(true);
  const speed = ref(1);

  /**
   * Chế độ nhận thức. Đổi nó gửi một `ViewSubscription` mới; **việc lọc xảy ra
   * ở server** (`§18.9`, `PC-15`). Client không bao giờ nhận dữ liệu mà nó
   * không được thấy rồi ẩn đi — ẩn ở client nghĩa là dữ liệu đã ở trong bộ nhớ
   * trình duyệt, và ai cũng mở được devtools.
   */
  const mode = ref<EpistemicMode>("observer");
  const asEntity = ref<string | null>(null);

  /** `shallowRef` vì Map lớn; Vue không cần theo dõi từng thực thể. */
  const entities = shallowRef(new Map<string, EntityView>());
  const selected = ref<string | null>(null);

  /** Overlay đang bật. Chỉ một, vì chúng là nhóm loại trừ (`§18.5`). */
  const activeOverlay = ref<string | null>(null);

  function upsertEntity(e: EntityView): void {
    const m = new Map(entities.value);
    m.set(e.id, e);
    entities.value = m;
  }

  function removeEntity(id: string): void {
    const m = new Map(entities.value);
    m.delete(id);
    entities.value = m;
    if (selected.value === id) selected.value = null;
  }

  return {
    tick, divineTick, stateHash, paused, speed,
    mode, asEntity,
    entities, selected, activeOverlay,
    upsertEntity, removeEntity,
  };
});
