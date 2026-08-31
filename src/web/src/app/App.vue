<script setup lang="ts">
/**
 * Màn hình chính (`§18.3`).
 *
 * Bốn panel, không hơn, cho lát cắt này: bản đồ, thanh trạng thái, ô đang trỏ,
 * nhật ký sự kiện. `§18.3` liệt mười hai panel; dựng cả mười hai lúc chưa có gì
 * chạy được là cách chắc chắn nhất để có mười hai panel rỗng — đúng chỗ dự án
 * này đã đứng trước đó.
 *
 * ## Vòng lặp: gửi, đợi ack, vẽ
 *
 * Không optimistic UI (`§P6.9.5`). Bấm phím → gửi lệnh → **đợi engine ack** →
 * hỏi lại trạng thái → vẽ. Có độ trễ một vòng, và độ trễ đó là thật: thế giới
 * chỉ đổi khi engine nói nó đã đổi.
 */
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import {
  api,
  DIRECTIONS,
  entityRef,
  type Entity,
  type WorldEvent,
  type WorldMeta,
} from "@/api/game";
import { WorldView } from "@/render/world";
import { BlockPalette } from "@/render/blocks";
import { dayPhase } from "@/render/terrain";
import { t } from "@/i18n";

const canvasEl = ref<HTMLCanvasElement | null>(null);
const view = shallowRef<WorldView | null>(null);
const palette = shallowRef(new BlockPalette());

const meta = ref<WorldMeta | null>(null);
const entities = ref<Entity[]>([]);
const events = ref<WorldEvent[]>([]);
const errorText = ref("");
const status = ref(t("app.connecting"));
const hovered = ref<{
  x: number;
  y: number;
  material: string;
  biome: string;
  height: number;
  drop: number;
} | null>(null);
const lastCommand = ref("");
/** Nhãn tên nổi trên canvas — HTML, không phải `PIXI.Text` (`§P6.9.2`). */
const labels = ref<{ id: string; text: string; left: number; top: number; self: boolean }[]>([]);

let cursor = 0;
let stopPolling: (() => void) | null = null;

const avatar = computed(() => entities.value.find((e) => e.is_avatar) ?? null);
const others = computed(() => entities.value.filter((e) => e.kind === "being" && !e.is_avatar));
const items = computed(() => entities.value.filter((e) => e.kind === "item"));

/** Vật phẩm nằm đúng dưới chân avatar. */
const underfoot = computed(() => {
  const a = avatar.value;
  if (!a) return null;
  return items.value.find((v) => v.x === a.x && v.y === a.y) ?? null;
});

/** Người đứng kề một ô — điều kiện để nói chuyện. */
const neighbour = computed(() => {
  const a = avatar.value;
  if (!a) return null;
  return others.value.find((n) => Math.abs(n.x - a.x) <= 1 && Math.abs(n.y - a.y) <= 1) ?? null;
});

const phaseLabel = computed(() => {
  const m = meta.value;
  if (!m) return "";
  return t(`time.${dayPhase(m.tick)}` as "time.day");
});

async function refresh(): Promise<void> {
  const v = view.value;
  if (!v) return;

  const m = await api.meta();
  meta.value = m;

  const list = (await api.entities()).entities;
  entities.value = list;

  const me = list.find((e) => e.is_avatar);
  const cx = me?.x ?? 0;
  const cy = me?.y ?? 0;
  const { w, h } = v.viewportTiles();
  const batch = await api.tiles(cx - (w >> 1), cy - (h >> 1), w, h, m.z);
  v.setTerrain(batch, m.tick);
  v.setCenter(cx, cy);
  v.setEntities(list);
  refreshLabels(list);

  const e = await api.events(cursor);
  cursor = e.cursor;
  if (e.events.length) events.value = [...e.events, ...events.value].slice(0, 60);

  status.value = t("app.running");
}

function refreshLabels(list: Entity[]): void {
  const v = view.value;
  if (!v) return;
  // Chỉ đặt nhãn cho sinh vật: một bãi vật phẩm sẽ phủ kín bản đồ bằng chữ.
  labels.value = list
    .filter((e) => e.kind === "being")
    .flatMap((e) => {
      const p = v.screenOf(e.x, e.y);
      return p ? [{ id: e.id, text: e.name, left: p.left, top: p.top, self: e.is_avatar }] : [];
    });
}

async function send(kind: string, fields: Record<string, unknown>): Promise<void> {
  try {
    const r = await api.command(kind, fields);
    lastCommand.value = r.ok ? `${kind} ✓` : `${kind} ✗ ${r.error ?? r.code ?? ""}`;
    await refresh();
  } catch (e) {
    errorText.value = e instanceof Error ? e.message : String(e);
  }
}

async function walk(dx: number, dy: number): Promise<void> {
  const a = avatar.value;
  if (a) await send("core.walk", { who: entityRef(a.id), dx, dy });
}

async function take(): Promise<void> {
  const a = avatar.value;
  const it = underfoot.value;
  if (a && it) await send("core.take", { who: entityRef(a.id), what: entityRef(it.id) });
}

async function talk(): Promise<void> {
  const a = avatar.value;
  const n = neighbour.value;
  if (a && n) {
    await send("core.speak", { who: entityRef(a.id), to: entityRef(n.id), text: "Chào anh bạn." });
  }
}

async function changeLayer(d: number): Promise<void> {
  const m = meta.value;
  if (!m) return;
  await api.setLayer(m.z + d);
  await refresh();
}

function onKeyDown(e: KeyboardEvent): void {
  const dir = DIRECTIONS[e.key];
  if (dir) {
    e.preventDefault();
    void walk(dir[0], dir[1]);
    return;
  }
  if (e.key === "e" || e.key === "E") void take();
  else if (e.key === "t" || e.key === "T") void talk();
  else if (e.key === "PageUp") void changeLayer(1);
  else if (e.key === "PageDown") void changeLayer(-1);
}

function onWheel(e: WheelEvent): void {
  e.preventDefault();
  if (view.value?.zoom(e.deltaY < 0 ? 1 : -1)) void refresh();
}

function onPointerMove(e: PointerEvent): void {
  const v = view.value;
  const c = canvasEl.value;
  if (!v || !c) return;
  const r = c.getBoundingClientRect();
  const at = v.tileAt(e.clientX - r.left, e.clientY - r.top);
  const batch = v.currentBatch();
  if (!at || !batch) {
    hovered.value = null;
    return;
  }
  const i = v.indexOf(at.x, at.y);
  if (i < 0) {
    hovered.value = null;
    return;
  }
  hovered.value = {
    x: at.x,
    y: at.y,
    material: batch.material[i] ?? "?",
    biome: batch.biome[i] ?? "?",
    height: batch.height[i] ?? 0,
    drop: batch.drop[i] ?? 0,
  };
}

onMounted(async () => {
  if (!canvasEl.value) return;
  try {
    const v = new WorldView(palette.value);
    await v.attach(canvasEl.value);
    view.value = v;
    await refresh();
    // Nhịp hỏi lại chậm hơn nhịp tick của server. Đây là chỗ WebSocket sẽ thay
    // vào khi `§P6.8` được nối đầy đủ.
    const timer = globalThis.setInterval(() => void refresh().catch(() => {}), 400);
    stopPolling = () => globalThis.clearInterval(timer);
    globalThis.addEventListener("keydown", onKeyDown);
  } catch (e) {
    errorText.value = e instanceof Error ? e.message : String(e);
    status.value = t("app.failed");
  }
});

onUnmounted(() => {
  stopPolling?.();
  globalThis.removeEventListener("keydown", onKeyDown);
  view.value?.destroy();
});
</script>

<template>
  <div class="mow">
    <header>
      <strong>{{ t("app.title") }}</strong>
      <span v-if="meta" class="stat">{{ t("hud.tick") }} {{ meta.tick }}</span>
      <span v-if="meta" class="stat">{{ phaseLabel }}</span>
      <span v-if="meta" class="stat" :title="meta.state_hash">#{{ meta.state_hash.slice(0, 8) }}</span>
      <span v-if="meta" class="stat">{{ t("hud.layer") }} {{ meta.z }}</span>
      <span v-if="avatar" class="stat">({{ avatar.x }}, {{ avatar.y }})</span>
      <span v-if="avatar && avatar.hunger !== null" class="stat">
        {{ t("hud.hunger") }} {{ avatar.hunger }}
      </span>
      <span class="spacer"></span>
      <span class="status">{{ status }}</span>
    </header>

    <div class="body">
      <main>
        <canvas ref="canvasEl" @pointermove="onPointerMove" @wheel="onWheel"></canvas>
        <div class="labels">
          <span
            v-for="l in labels"
            :key="l.id"
            class="label"
            :class="{ self: l.self }"
            :style="{ left: `${l.left}px`, top: `${l.top}px` }"
            >{{ l.text }}</span
          >
        </div>
        <p v-if="errorText" class="error">{{ errorText }}</p>
      </main>

      <aside>
        <section>
          <h2>{{ t("panel.tile") }}</h2>
          <table v-if="hovered">
            <tbody>
              <tr><td>{{ t("panel.tile.position") }}</td><td>{{ hovered.x }}, {{ hovered.y }}</td></tr>
              <tr><td>{{ t("panel.tile.material") }}</td><td>{{ palette.label(hovered.material) }}</td></tr>
              <tr><td>{{ t("panel.tile.biome") }}</td><td>{{ hovered.biome }}</td></tr>
              <tr><td>{{ t("panel.tile.elevation") }}</td><td>{{ hovered.height }} m</td></tr>
              <tr v-if="hovered.drop > 0"><td>{{ t("panel.tile.depth") }}</td><td>{{ hovered.drop }} m</td></tr>
            </tbody>
          </table>
          <p v-else class="dim">{{ t("panel.tile.hint") }}</p>
        </section>

        <section>
          <h2>{{ t("panel.controls") }}</h2>
          <ul class="keys">
            <li><kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd> {{ t("panel.controls.move") }}</li>
            <li>
              <kbd>E</kbd> {{ t("panel.controls.take") }}
              <b v-if="underfoot">{{ underfoot.name }}</b>
              <span v-else class="dim">({{ t("panel.controls.nothingHere") }})</span>
            </li>
            <li>
              <kbd>T</kbd> {{ t("panel.controls.talk") }}
              <b v-if="neighbour">{{ neighbour.name }}</b>
              <span v-else class="dim">({{ t("panel.controls.nobodyNear") }})</span>
            </li>
            <li><kbd>PgUp</kbd><kbd>PgDn</kbd> {{ t("panel.controls.layer") }}</li>
            <li><kbd>scroll</kbd> {{ t("panel.controls.zoom") }}</li>
          </ul>
          <p v-if="lastCommand" class="cmd">{{ lastCommand }}</p>
        </section>

        <section>
          <h2>{{ t("panel.present") }} ({{ entities.length }})</h2>
          <ul class="list">
            <li v-for="e in entities" :key="e.id" :class="{ me: e.is_avatar }">
              <span class="dot" :class="e.kind"></span>
              {{ e.name }} <span class="dim">({{ e.x }}, {{ e.y }})</span>
            </li>
          </ul>
        </section>

        <section class="grow">
          <h2>{{ t("panel.events") }}</h2>
          <ul class="list">
            <li v-for="s in events" :key="s.seq">
              <span class="dim">t{{ s.tick }}</span> {{ s.kind }}
            </li>
          </ul>
          <p v-if="!events.length" class="dim">{{ t("panel.events.empty") }}</p>
        </section>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.mow { display: flex; flex-direction: column; height: 100vh; background: #070910; color: #e6e9ef;
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif; }
header { display: flex; align-items: center; gap: 0.9rem; padding: 0.45rem 0.8rem;
  border-bottom: 1px solid #232833; background: #10131a; font: 12px/1.4 ui-monospace, monospace; }
header strong { font-family: ui-sans-serif, system-ui, sans-serif; letter-spacing: 0.02em; }
.stat { color: #93a1b5; }
.spacer { flex: 1; }
.status { color: #7f8ea3; }

.body { flex: 1; min-height: 0; display: flex; }
main { flex: 1; min-width: 0; position: relative; overflow: hidden; }
canvas { display: block; width: 100%; height: 100%; }

.labels { position: absolute; inset: 0; pointer-events: none; }
.label { position: absolute; transform: translate(-50%, -100%); white-space: nowrap;
  padding: 0 4px; border-radius: 3px; background: rgb(8 10 16 / 68%);
  font: 11px/1.5 ui-sans-serif, system-ui, sans-serif; color: #d8dee9;
  text-shadow: 0 1px 2px rgb(0 0 0 / 80%); }
.label.self { color: #bcd6ff; background: rgb(20 38 68 / 78%); }

.error { position: absolute; left: 1rem; bottom: 1rem; margin: 0; padding: 0.5rem 0.75rem;
  background: #45161a; border: 1px solid #7d2b32; border-radius: 4px; color: #ffd7d7; }

aside { width: 292px; flex: none; border-left: 1px solid #232833; background: #0c0f15;
  overflow-y: auto; display: flex; flex-direction: column; }
section { padding: 0.7rem 0.8rem; border-bottom: 1px solid #1b2029; }
section.grow { flex: 1; min-height: 0; overflow-y: auto; }
h2 { margin: 0 0 0.45rem; font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em;
  color: #7f8ea3; font-weight: 600; }
table { width: 100%; border-collapse: collapse; }
td { padding: 1px 0; }
td:first-child { color: #7f8ea3; width: 6.5rem; }
.dim { color: #6b7788; }
.list { list-style: none; margin: 0; padding: 0; }
.list li { padding: 1px 0; }
.list li.me { color: #9ec1ff; }
.dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 5px;
  background: #d08770; }
.dot.item { border-radius: 1px; background: #f0c674; transform: rotate(45deg); }
.keys { list-style: none; margin: 0; padding: 0; }
.keys li { padding: 2px 0; }
kbd { display: inline-block; min-width: 1.1em; padding: 1px 4px; margin-right: 2px;
  border: 1px solid #333c4a; border-bottom-width: 2px; border-radius: 3px;
  background: #171b22; font: 11px ui-monospace, monospace; }
.cmd { margin: 0.5rem 0 0; font: 11px ui-monospace, monospace; color: #9aa7b8; }
</style>
