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
  entityRef,
  SPEED_STEPS,
  type CauseLink,
  type Entity,
  type Foresight,
  type WorldEvent,
  type WorldMeta,
} from "@/api/game";
import { WorldView } from "@/render/world";
import { BlockPalette, paletteFrom } from "@/render/blocks";
import { dayPhase } from "@/render/terrain";
import { minimapMarker, minimapToWorld, paintMinimap } from "@/render/minimap";
import { t, tRuntime } from "@/i18n";
import DataOverlay from "./panels/DataOverlay.vue";
import ObservePanel from "./panels/ObservePanel.vue";
import { followStep } from "./observe";
import type { LayerId } from "@/render/overlays/field";

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
/** Chỉ số nấc tốc độ đang chọn. `5` là ×1. */
const speedIndex = ref(5);
/** Việc định làm khi tới nơi: nhặt vật phẩm, hoặc nói với ai đó. */
const intent = ref<{ id: string; kind: "being" | "item" } | null>(null);
const walkNote = ref("");

// ── Console True God ────────────────────────────────────────────────────────
const godOpen = ref(false);
const godTarget = ref<Entity | null>(null);
const foresight = ref<Foresight | null>(null);
const godNote = ref("");
/** Ý chỉ đang soạn, dạng `(kind, fields)`. */
const pendingWill = ref<{ kind: string; fields: Record<string, unknown> } | null>(null);
/** Vật liệu đang cầm để khắc. `null` là chuột trở lại chế độ đi. */
const brush = ref<string | null>(null);
/** Vật liệu người xây được — lọc theo thẻ `built` của content pack. */
const buildable = computed(() =>
  palette.value.ids().filter((id) => palette.value.get(id)?.tags.includes("built")),
);

// ── Bản đồ thu nhỏ ──────────────────────────────────────────────────────────
/** 146 px: đúng một nửa 292 px của panel, nên phóng 2× là nearest thuần. */
const MINIMAP_SIZE = 146;
/** Vùng vuông quanh avatar, độc lập với khung nhìn chính. */
const MINIMAP_TILES = 128;
const minimapEl = ref<HTMLCanvasElement | null>(null);
const minimapMark = ref<{ x: number; y: number } | null>(null);
let minimapBatch: Awaited<ReturnType<typeof api.tiles>> | null = null;

// ── Lớp dữ liệu (`PG-07`) ───────────────────────────────────────────────────
/** Lớp đang phủ lên bản đồ. `null` là tắt — mặc định, vì địa hình là gốc. */
const overlayLayer = ref<LayerId | null>(null);
/** Lô ô mới nhất, để panel lớp dữ liệu tính trường mà không phải hỏi lại API. */
const lastBatch = shallowRef<Awaited<ReturnType<typeof api.tiles>> | null>(null);

// ── Chế độ quan sát (`PG-14`) ───────────────────────────────────────────────
/** Đang bám theo đối tượng đã chọn hay không. */
const following = ref(false);
/**
 * Ô mà khung nhìn đang lấy làm tâm.
 *
 * Giữ riêng khỏi vị trí avatar vì hai thứ tách nhau khi đang bám theo người
 * khác: người chơi là một vị thần, và một vị thần nhìn được chỗ mình không
 * đứng.
 */
const camera = ref<{ x: number; y: number } | null>(null);

// ── Chuỗi nhân quả (`§18.10`) ───────────────────────────────────────────────
const causeChain = ref<CauseLink[] | null>(null);
/** Nhãn tên nổi trên canvas — HTML, không phải `PIXI.Text` (`§P6.9.2`). */
const labels = ref<{ id: string; text: string; left: number; top: number; self: boolean }[]>([]);

let cursor = 0;
let stopPolling: (() => void) | null = null;

const avatar = computed(() => entities.value.find((e) => e.is_avatar) ?? null);

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
  // Giữ đối tượng đang chọn đồng bộ với dữ liệu mới: không có dòng này thì
  // panel cư dân đóng băng ở trạng thái lúc bấm chọn, và người chơi tưởng NPC
  // đứng im trong khi nó đang đi.
  if (godTarget.value) {
    godTarget.value = list.find((e) => e.id === godTarget.value?.id) ?? null;
  }

  const me = list.find((e) => e.is_avatar);
  // Bám theo ai thì lấy người đó làm tâm; không thì về avatar. `followStep` có
  // vùng chết, nên mục tiêu nhích một ô không làm cả bản đồ giật một cái.
  const anchor = following.value && godTarget.value ? godTarget.value : me;
  const want = { x: anchor?.x ?? 0, y: anchor?.y ?? 0 };
  const step = followStep(camera.value ?? want, want);
  camera.value = { x: step.x, y: step.y };
  const cx = step.x;
  const cy = step.y;
  const { w, h } = v.viewportTiles();
  const batch = await api.tiles(cx - (w >> 1), cy - (h >> 1), w, h, m.z);
  lastBatch.value = batch;
  v.setTerrain(batch, m.tick);
  v.setCenter(cx, cy);
  v.setEntities(list);
  refreshLabels(list);
  if (m.steps_remaining === 0) v.setPath([]);

  const e = await api.events(cursor);
  cursor = e.cursor;
  if (e.events.length) events.value = [...e.events, ...events.value].slice(0, 60);

  status.value = t("app.running");
  await resolveIntent();
}

/** Tới nơi rồi thì làm việc đã định. */
async function resolveIntent(): Promise<void> {
  const want = intent.value;
  const a = avatar.value;
  const m = meta.value;
  if (!want || !a || !m || m.steps_remaining > 0) return;

  const target = entities.value.find((x) => x.id === want.id);
  if (!target) {
    intent.value = null;
    return;
  }
  const adjacent = Math.abs(target.x - a.x) <= 1 && Math.abs(target.y - a.y) <= 1;
  if (!adjacent) {
    // Không tới được — bỏ ý định thay vì thử mãi.
    intent.value = null;
    return;
  }
  intent.value = null;
  if (want.kind === "item") {
    await send("core.take", { who: entityRef(a.id), what: entityRef(target.id) });
  } else {
    await send("core.speak", {
      who: entityRef(a.id),
      to: entityRef(target.id),
      text: "Chào anh bạn.",
    });
  }
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

/** Soạn một ý chỉ rồi nhìn trước ngay — không có bước "bấm xem trước" thừa. */
async function foresee(kind: string, fields: Record<string, unknown>): Promise<void> {
  pendingWill.value = { kind, fields };
  godNote.value = "";
  try {
    const f = await api.preview(kind, fields);
    foresight.value = f;
    view.value?.setDiff(f.changes);
    if (f.error) godNote.value = `${t("god.impossible")}: ${f.error}`;
    else if (!f.changes_anything) godNote.value = t("god.nothing");
  } catch (e) {
    godNote.value = e instanceof Error ? e.message : String(e);
  }
}

/** Khắc ý chỉ đang nhìn trước vào thế giới. */
async function inscribe(): Promise<void> {
  const f = foresight.value;
  const w = pendingWill.value;
  if (!f || !w) return;
  try {
    const r = await api.commit(w.kind, w.fields, f.base_hash);
    if (r.ok) {
      godNote.value = `${t("god.done")} — ${r.changes ?? 0} ${t("god.willChange")}`;
      withdraw();
      await refresh();
    } else if (r.reason === "world_moved") {
      // Không im lặng thất bại: nhìn lại ngay, để người chơi thấy hậu quả mới.
      godNote.value = t("god.moved");
      await foresee(w.kind, w.fields);
    } else {
      godNote.value = r.message ?? t("god.impossible");
    }
  } catch (e) {
    godNote.value = e instanceof Error ? e.message : String(e);
  }
}

/** Bỏ ý chỉ đang soạn và xóa diff khỏi bản đồ. */
function withdraw(): void {
  foresight.value = null;
  pendingWill.value = null;
  view.value?.setDiff([]);
}

function godAct(kind: "feed" | "starve"): void {
  const target = godTarget.value;
  if (!target) return;
  const value = kind === "feed" ? 0 : 9_000;
  void foresee("truegod.set_attr", {
    entity: entityRef(target.id),
    key: "need.hunger",
    value,
  });
}

function selectTarget(e: Entity): void {
  // Chọn người khác thì thôi bám: bám theo một người rồi bấm sang người kia mà
  // camera vẫn dính vào người cũ là thứ không ai đoán được.
  if (godTarget.value?.id !== e.id) following.value = false;
  godTarget.value = e;
  withdraw();
}

/** Bật/tắt bám theo. Tắt thì camera trôi về avatar ở lần vẽ sau. */
function toggleFollow(): void {
  following.value = !following.value;
}

/** Vẽ lại bản đồ thu nhỏ. Nhịp chậm hơn khung nhìn chính vì nó đổi chậm. */
async function refreshMinimap(): Promise<void> {
  const a = avatar.value;
  const m = meta.value;
  const cv = minimapEl.value;
  if (!a || !m || !cv) return;
  const half = MINIMAP_TILES >> 1;
  minimapBatch = await api.tiles(a.x - half, a.y - half, MINIMAP_TILES, MINIMAP_TILES, m.z);
  const ctx = cv.getContext("2d");
  if (!ctx) return;
  const rgba = paintMinimap(minimapBatch, palette.value, MINIMAP_SIZE);
  const img = ctx.createImageData(MINIMAP_SIZE, MINIMAP_SIZE);
  img.data.set(rgba);
  ctx.putImageData(img, 0, 0);
  minimapMark.value = minimapMarker(minimapBatch, MINIMAP_SIZE, a.x, a.y);
}

/** Bấm vào bản đồ thu nhỏ: đi tới đó. Chuột vẫn là thao tác chính. */
async function onMinimapClick(e: MouseEvent): Promise<void> {
  const cv = minimapEl.value;
  if (!cv || !minimapBatch) return;
  const r = cv.getBoundingClientRect();
  const px = ((e.clientX - r.left) / r.width) * MINIMAP_SIZE;
  const py = ((e.clientY - r.top) / r.height) * MINIMAP_SIZE;
  const w = minimapToWorld(minimapBatch, MINIMAP_SIZE, px, py);
  const res = await api.goto(w.x, w.y);
  view.value?.setPath(res.path);
  await refresh();
}

/**
 * Truy ngược nguyên nhân của một sự kiện.
 *
 * Cạnh nhân quả được engine ghi **lúc tạo** sự kiện. Không có nó thì câu hỏi
 * "vì sao chuyện này xảy ra" chỉ trả lời được bằng phỏng đoán — và một chuỗi
 * phỏng đoán thì tệ hơn không có, vì người xem sẽ tin nó.
 */
async function traceCause(seq: number): Promise<void> {
  try {
    causeChain.value = (await api.causes(seq)).chain;
  } catch (e) {
    errorText.value = e instanceof Error ? e.message : String(e);
  }
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

async function applySpeed(i: number): Promise<void> {
  speedIndex.value = i;
  const step = SPEED_STEPS[i];
  if (!step) return;
  try {
    await api.setSpeed(step.milli);
  } catch (e) {
    errorText.value = e instanceof Error ? e.message : String(e);
  }
}

async function changeLayer(d: number): Promise<void> {
  const m = meta.value;
  if (!m) return;
  await api.setLayer(m.z + d);
  await refresh();
}

function onKeyDown(e: KeyboardEvent): void {
  // Bàn phím chỉ còn giữ hai thứ không có chỗ tự nhiên trên bản đồ: đổi lát `z`
  // và thoát. Mọi hành động trong thế giới đi qua chuột — `§P6.9.5` nói mọi
  // hành động đổi thế giới phải là một command có schema, và một cú bấm vào
  // đúng ô là cách nói "chỗ này" ít mơ hồ nhất.
  if (e.key === "PageUp") void changeLayer(1);
  else if (e.key === "PageDown") void changeLayer(-1);
  else if (e.key === "Escape") void api.stop().then(() => refresh());
  else if (e.key === "g" || e.key === "G") godOpen.value = !godOpen.value;
}

/**
 * Bấm trái: đi tới ô đó. Bấm vào một thực thể thì đi tới rồi làm việc với nó.
 *
 * Ý định được nhớ ở client vì chỉ client biết cú bấm nhắm vào cái gì; nhưng
 * **việc thực hiện** vẫn là một command qua server khi tới nơi.
 *
 * Dùng `click` chứ không phải `pointerdown`: `click` là "nhấn và thả trên cùng
 * một phần tử", nên kéo bản đồ rồi thả không bị hiểu nhầm thành một lệnh đi.
 * Bản đầu dùng `pointerdown` và nó **im lặng không chạy** dưới điều khiển tự
 * động — thứ chỉ lộ ra khi tự vào chơi, không lộ ra ở bất kỳ bài test nào.
 */
async function onClick(e: MouseEvent): Promise<void> {
  const v = view.value;
  const c = canvasEl.value;
  if (!v || !c) return;
  const r = c.getBoundingClientRect();
  const at = v.tileAt(e.clientX - r.left, e.clientY - r.top);
  if (!at) return;

  // Đang cầm vật liệu thì cú bấm là một nhát khắc, không phải một bước đi.
  if (brush.value) {
    const r = await api.build(at.x, at.y, brush.value);
    if (!r.ok) godNote.value = r.error ?? t("god.impossible");
    await refresh();
    return;
  }

  const target = entities.value.find((x) => x.x === at.x && x.y === at.y && !x.is_avatar);
  intent.value = target ? { id: target.id, kind: target.kind } : null;

  try {
    const res = await api.goto(at.x, at.y);
    view.value?.setPath(res.path);
    walkNote.value =
      res.outcome === "unreachable"
        ? t("panel.controls.unreachable")
        : res.outcome === "partial"
          ? t("panel.controls.partial")
          : "";
    await refresh();
  } catch (err) {
    errorText.value = err instanceof Error ? err.message : String(err);
  }
}

function onContextMenu(e: MouseEvent): void {
  e.preventDefault();
  intent.value = null;
  void api.stop().then(() => refresh());
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

    // Bảng vật liệu tới từ content pack đang nạp (`§19.7`). Hỏng thì giữ bảng
    // dự phòng — một thế giới vẽ bằng màu dự phòng vẫn tốt hơn một màn hình
    // trắng, và vật liệu không có định nghĩa hiện màu tím nên nhìn là biết.
    try {
      const bs = await api.blocks();
      if (bs.loaded && bs.blocks.length) {
        palette.value = paletteFrom(bs.blocks);
        v.setPalette(palette.value);
      }
    } catch {
      // giữ bảng dự phòng
    }
    await refresh();
    // Nhịp hỏi lại chậm hơn nhịp tick của server. Đây là chỗ WebSocket sẽ thay
    // vào khi `§P6.8` được nối đầy đủ.
    const timer = globalThis.setInterval(() => void refresh().catch(() => {}), 400);
    await refreshMinimap().catch(() => {});
    // Bản đồ thu nhỏ đổi chậm hơn nhiều so với khung nhìn chính, và mỗi lần vẽ
    // là một lô 128×128 ô — hỏi lại mỗi 400 ms là trả giá cho thứ không đổi.
    const miniTimer = globalThis.setInterval(() => void refreshMinimap().catch(() => {}), 2_500);
    stopPolling = () => {
      globalThis.clearInterval(timer);
      globalThis.clearInterval(miniTimer);
    };
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
        <canvas
          ref="canvasEl"
          @pointermove="onPointerMove"
          @wheel="onWheel"
          @click="onClick"
          @contextmenu="onContextMenu"
        ></canvas>
        <div class="vignette"></div>
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
          <h2>{{ t("panel.minimap") }}</h2>
          <div class="mini">
            <canvas
              ref="minimapEl"
              :width="MINIMAP_SIZE"
              :height="MINIMAP_SIZE"
              @click="onMinimapClick"
            ></canvas>
            <span
              v-if="minimapMark"
              class="minimark"
              :style="{
                left: `${(minimapMark.x / MINIMAP_SIZE) * 100}%`,
                top: `${(minimapMark.y / MINIMAP_SIZE) * 100}%`,
              }"
            ></span>
          </div>
        </section>

        <section>
          <DataOverlay
            v-model="overlayLayer"
            :batch="lastBatch"
            :entities="entities"
            @paint="view?.setOverlay($event)"
          />
        </section>

        <section>
          <h2>{{ t("panel.time") }}</h2>
          <div class="speed">
            <input
              type="range"
              min="0"
              :max="SPEED_STEPS.length - 1"
              step="1"
              :value="speedIndex"
              @input="applySpeed(Number(($event.target as HTMLInputElement).value))"
            />
            <b class="speedval">{{ SPEED_STEPS[speedIndex]?.label }}</b>
          </div>
          <p v-if="SPEED_STEPS[speedIndex]?.milli === 0" class="dim">
            {{ t("panel.time.paused") }}
          </p>
        </section>

        <section>
          <h2>{{ t("panel.controls") }}</h2>
          <ul class="keys">
            <li><b>{{ t("panel.controls.move") }}</b></li>
            <li>{{ t("panel.controls.stop") }}</li>
            <li><kbd>PgUp</kbd><kbd>PgDn</kbd> {{ t("panel.controls.layer") }}</li>
            <li><kbd>scroll</kbd> {{ t("panel.controls.zoom") }}</li>
          </ul>
          <p v-if="meta && meta.steps_remaining > 0" class="cmd">
            {{ t("panel.controls.walking") }} — {{ meta.steps_remaining }}
          </p>
          <p v-if="walkNote" class="cmd warn">{{ walkNote }}</p>
          <p v-if="lastCommand" class="cmd">{{ lastCommand }}</p>
        </section>

        <section class="god" :class="{ open: godOpen }">
          <h2>
            {{ t("god.title") }}
            <button class="toggle" @click="godOpen = !godOpen">{{ godOpen ? "−" : "+" }}</button>
          </h2>
          <template v-if="godOpen">
            <div class="shape">
              <div class="shapehead">
                {{ t("god.shape") }}
                <button v-if="brush" class="toggle wide" @click="brush = null">
                  {{ t("god.shape.off") }}
                </button>
              </div>
              <div class="swatches">
                <button
                  v-for="id in buildable"
                  :key="id"
                  class="swatch"
                  :class="{ on: brush === id }"
                  :style="{ background: `#${palette.color(id).toString(16).padStart(6, '0')}` }"
                  :title="palette.label(id)"
                  @click="brush = brush === id ? null : id"
                ></button>
              </div>
              <p class="dim hint">{{ t("god.shape.hint") }}</p>
            </div>

            <p v-if="!godTarget" class="dim">{{ t("god.pick") }}</p>
            <template v-else>
              <p class="godtarget">{{ t("god.target") }}: <b>{{ godTarget.name }}</b></p>
              <div class="acts">
                <button @click="godAct('feed')">{{ t("god.act.feed") }}</button>
                <button @click="godAct('starve')">{{ t("god.act.starve") }}</button>
              </div>
            </template>

            <div v-if="foresight" class="foresight">
              <ul class="list">
                <li v-for="c in foresight.changes" :key="c.id">
                  <b>{{ c.name }}</b>
                  <span v-if="c.moved" class="dim">
                    ({{ c.from?.[0] }},{{ c.from?.[1] }}) → ({{ c.to?.[0] }},{{ c.to?.[1] }})
                  </span>
                  <span v-else class="dim">{{ c.attrs.join(", ") }}</span>
                </li>
              </ul>
              <div class="acts">
                <button
                  class="primary"
                  :disabled="!foresight.changes_anything || !!foresight.error"
                  @click="inscribe"
                >
                  {{ t("god.inscribe") }}
                </button>
                <button @click="withdraw">{{ t("god.withdraw") }}</button>
              </div>
            </div>
            <p v-if="godNote" class="cmd warn">{{ godNote }}</p>
          </template>
        </section>

        <section v-if="godTarget && godTarget.kind === 'being'">
          <ObservePanel
            :target="godTarget"
            :events="events"
            :following="following"
            @toggle-follow="toggleFollow"
            @trace="traceCause"
          />
        </section>

        <section>
          <h2>{{ t("panel.present") }} ({{ entities.length }})</h2>
          <ul class="list">
            <li
              v-for="e in entities"
              :key="e.id"
              :class="{ me: e.is_avatar, sel: godTarget?.id === e.id }"
              class="pick"
              @click="selectTarget(e)"
            >
              <span class="dot" :class="e.kind"></span>
              {{ e.name }}
              <span v-if="e.role" class="dim">· {{ tRuntime("role", e.role) }}</span>
              <div v-if="e.intent" class="doing">{{ tRuntime("intent", e.intent) }}</div>
            </li>
          </ul>
        </section>

        <section v-if="causeChain">
          <h2>
            {{ t("panel.cause") }}
            <button class="toggle" @click="causeChain = null">×</button>
          </h2>
          <ol class="chain">
            <li v-for="c in causeChain" :key="c.seq">
              <span class="dim">t{{ c.tick }}</span> <b>{{ c.kind }}</b>
              <div class="dim sum">{{ c.summary }}</div>
            </li>
          </ol>
          <p v-if="causeChain.length <= 1" class="dim">{{ t("panel.cause.root") }}</p>
        </section>

        <section class="grow">
          <h2>{{ t("panel.events") }}</h2>
          <p v-if="events.length" class="dim hint">{{ t("panel.events.hint") }}</p>
          <ul class="list">
            <li v-for="s in events" :key="s.seq" class="pick" @click="traceCause(s.seq)">
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

/* Khung tối bốn góc. Một gradient CSS, không tốn draw call nào — nhưng nó là
   thứ tách "đang nhìn xuống một thế giới" khỏi "đang mở một bảng điều khiển". */
.vignette { position: absolute; inset: 0; pointer-events: none;
  background: radial-gradient(ellipse at center,
    rgb(0 0 0 / 0%) 45%, rgb(0 0 0 / 18%) 78%, rgb(0 0 0 / 42%) 100%); }
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
.speed { display: flex; align-items: center; gap: 0.6rem; }
.speed input { flex: 1; accent-color: #4b83c4; }
.speedval { min-width: 3.6rem; text-align: right; font: 12px ui-monospace, monospace;
  color: #cfd8e3; }
.mini { position: relative; line-height: 0; }
/* `pixelated` là bắt buộc: minimap gộp ô bằng `mode` để giữ ranh giới sắc nét,
   và một phép nội suy song tuyến ở tầng trình duyệt sẽ trung bình các pixel lân
   cận — tức là dựng lại đúng màu bùn mà module vừa tránh. */
.mini canvas { width: 100%; height: auto; display: block; border-radius: 3px;
  image-rendering: pixelated; cursor: crosshair; }
.minimark { position: absolute; width: 7px; height: 7px; margin: -4px 0 0 -4px;
  border: 2px solid #fff; border-radius: 50%; box-shadow: 0 0 0 1px #16324f;
  pointer-events: none; }

.god h2 { display: flex; align-items: center; }
.toggle { margin-left: auto; width: 20px; height: 18px; padding: 0; line-height: 1;
  border: 1px solid #333c4a; border-radius: 3px; background: #171b22; color: #9aa7b8;
  cursor: pointer; }
.godtarget { margin: 0 0 0.4rem; }
.acts { display: flex; gap: 0.4rem; flex-wrap: wrap; margin-top: 0.4rem; }
.acts button { padding: 3px 8px; border: 1px solid #3a4553; border-radius: 3px;
  background: #1a1f28; color: #d8dee9; font: inherit; font-size: 12px; cursor: pointer; }
.acts button:hover:not(:disabled) { background: #232a35; }
.acts button:disabled { opacity: 0.45; cursor: default; }
.acts button.primary { border-color: #6a5426; background: #2a2415; color: #f0c674; }
.foresight { margin-top: 0.5rem; padding-top: 0.5rem; border-top: 1px solid #1b2029; }
.hint { margin: 0 0 0.35rem; font-size: 11px; }
/* Chuỗi đi ngược thời gian, nên nó xuống dòng như một dòng dõi: mỗi mắt lùi
   vào một chút để mắt đọc được chiều "cái này gây ra cái trước nó". */
.chain { list-style: none; margin: 0; padding: 0; }
.chain li { padding: 2px 0 2px 10px; border-left: 2px solid #2a3441; margin-left: 3px; }
.chain .sum { font: 11px ui-monospace, monospace; }
.shape { margin-bottom: 0.6rem; padding-bottom: 0.6rem; border-bottom: 1px solid #1b2029; }
.shapehead { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.35rem; }
.toggle.wide { width: auto; padding: 0 6px; }
.swatches { display: flex; gap: 4px; flex-wrap: wrap; }
/* Ô màu vuông, viền dày khi đang cầm: người chơi phải biết cú bấm tiếp theo sẽ
   làm gì trước khi bấm — không có dấu hiệu này thì một cú bấm nhầm sẽ sửa thế
   giới trong khi họ tưởng mình chỉ đang đi. */
.swatch { width: 22px; height: 22px; padding: 0; border: 2px solid #2a3441;
  border-radius: 3px; cursor: pointer; }
.swatch.on { border-color: #f0c674; box-shadow: 0 0 0 2px rgb(240 198 116 / 25%); }
/* Việc đang làm xuống dòng riêng và lùi vào: nó đổi mỗi vài giây, và để nó
   cùng dòng với tên sẽ làm cả danh sách nhảy chiều rộng liên tục. */
.doing { padding-left: 13px; font-size: 11px; color: #8a9bb0; }
.pick { cursor: pointer; }
.pick:hover { color: #cfd8e3; }
.pick.sel { color: #f0c674; }
.cmd.warn { color: #e0a04a; }
.cmd { margin: 0.5rem 0 0; font: 11px ui-monospace, monospace; color: #9aa7b8; }
</style>
