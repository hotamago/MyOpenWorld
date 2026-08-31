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
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
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
import { setLocale, t, tRuntime, type MessageKey } from "@/i18n";
import {
  CodexPanel,
  PauseMenu,
  SettingsPanel,
  TitleScreen,
  loadSettings,
  nextScreen,
  saveSettings,
  type Screen,
  type Settings,
} from "./menu";
import { ChroniclePanel } from "./chronicle";
import { PowerDock, fieldsFor, readiness, tpRaw, type Power } from "./powers";
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
const walkNote = ref("");

// ── Console True God ────────────────────────────────────────────────────────
const godOpen = ref(false);
const godTarget = ref<Entity | null>(null);
const foresight = ref<Foresight | null>(null);
const godNote = ref("");
/** Ý chỉ đang soạn, dạng `(kind, fields)`. */
const pendingWill = ref<{ kind: string; fields: Record<string, unknown> } | null>(null);
/**
 * Đang soạn mệnh lệnh "chỉ đường": cú bấm tiếp theo là đích cho người đang chọn.
 *
 * Một trạng thái riêng chứ không phải "bấm vào người rồi bấm vào đất": không có
 * nó thì mọi cú bấm lên đất sau khi chọn ai đó đều thành một mệnh lệnh, và
 * người chơi không xem được bản đồ nữa mà không vô tình sai khiến ai.
 */
const guiding = ref(false);
/** Vật liệu đang cầm để khắc. `null` là chuột trở lại chế độ soi xét. */
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

// ── Màn hình và thiết lập ───────────────────────────────────────────────────
/**
 * Màn hình đang hiện.
 *
 * Bắt đầu ở `title`, không ở `world`: một trò chơi mở thẳng vào bản đồ không
 * cho người chơi cơ hội chọn thế giới của mình, và người chơi đã nói thẳng rằng
 * "menu vẫn rất sơ sài".
 */
const screen = ref<Screen>("title");
/** Màn hình đã mở `settings`/`codex`, để `Esc` quay đúng về đó. */
const cameFrom = ref<Screen>("title");
const settings = ref<Settings>(loadSettings());

/** Thế giới đã được khởi nguyên chưa — vòng lặp chỉ chạy sau khi có. */
const started = ref(false);

function go(action: "esc" | "play" | "settings" | "codex" | "resume"): void {
  if (action === "settings" || action === "codex") cameFrom.value = screen.value;
  screen.value = nextScreen(screen.value, action, cameFrom.value);
}

/** Bước vào một thế giới mới. */
async function enterWorld(seed: number): Promise<void> {
  try {
    status.value = t("app.connecting");
    await api.genesis(String(seed));
    screen.value = "world";
    started.value = true;
    camera.value = null;
    events.value = [];
    cursor = 0;
    godTarget.value = null;
    following.value = false;
    await applySpeed(settings.value.speedIndex);
    await refresh();
    await refreshMinimap().catch(() => {});
  } catch (e) {
    errorText.value = e instanceof Error ? e.message : String(e);
    status.value = t("app.failed");
  }
}

/** Rời thế giới, về màn hình đầu. Thời gian dừng lại để nó không trôi sau lưng. */
async function leaveWorld(): Promise<void> {
  await applySpeed(0);
  started.value = false;
  screen.value = "title";
}

// ── Lớp dữ liệu (`PG-07`) ───────────────────────────────────────────────────
/** Lớp đang phủ lên bản đồ. `null` là tắt — mặc định, vì địa hình là gốc. */
const overlayLayer = ref<LayerId | null>(null);
/** Lô ô mới nhất, để panel lớp dữ liệu tính trường mà không phải hỏi lại API. */
const lastBatch = shallowRef<Awaited<ReturnType<typeof api.tiles>> | null>(null);

// ── Ngăn kéo bên phải ───────────────────────────────────────────────────────
/**
 * Ngăn nào đang mở, hoặc `null` khi thế giới chiếm trọn màn hình.
 *
 * Đè **lên** canvas chứ không thu hẹp nó: một tấm bản đồ bị cắt mất một phần ba
 * mỗi khi người chơi muốn đọc một dòng lịch sử là một tấm bản đồ luôn nhỏ.
 */
type Drawer = "observe" | "layers" | "chronicle" | "cause";
const drawer = ref<Drawer | null>(null);

/** Các ngăn, theo đúng thứ tự trên thanh công cụ trái. */
const DRAWERS = [
  { id: "observe", glyph: "👁", label: "rail.observe" },
  { id: "layers", glyph: "🗺", label: "rail.layers" },
  { id: "chronicle", glyph: "📜", label: "rail.chronicle" },
  { id: "cause", glyph: "🜁", label: "rail.cause" },
] as const satisfies readonly { id: Drawer; glyph: string; label: MessageKey }[];

/**
 * Bốn nấc tốc độ đặt sẵn trên thanh trên.
 *
 * `SPEED_STEPS` có mười hai nấc — đủ cho một thanh trượt, quá nhiều cho một
 * hàng nút. Người chơi cần **nhảy** về dừng hoặc về ×1, không cần rà tìm; các
 * nấc còn lại vẫn tới được qua bảng thiết lập.
 */
const SPEED_MARKS = [
  { i: 0, label: "⏸" },
  { i: 5, label: "×1" },
  { i: 7, label: "×5" },
  { i: 9, label: "×25" },
] as const;

/** Bản đồ thu nhỏ đang mở hay đã thu lại. */
const miniOpen = ref(true);

/** Bỏ mọi thứ đang chọn và mọi ý chỉ đang soạn. */
function clearSelection(): void {
  godTarget.value = null;
  pickedTile.value = null;
  following.value = false;
  godNote.value = "";
  walkNote.value = "";
  dropPower();
}

function toggleDrawer(d: Drawer): void {
  drawer.value = drawer.value === d ? null : d;
}

// ── Quyền năng của thần ─────────────────────────────────────────────────────
/**
 * Quyền năng đang cầm trên tay, hoặc `null`.
 *
 * Một trạng thái tường minh chứ không phải "bấm cái này rồi bấm cái kia": người
 * chơi phải biết cú bấm tiếp theo sẽ làm gì **trước khi** bấm. Không có nó thì
 * mọi cú bấm lên bản đồ sau khi chọn một cư dân đều có thể là một mệnh lệnh, và
 * xem bản đồ trở thành việc nguy hiểm.
 */
const activePower = ref<Power | null>(null);
/** Tham số người chơi đang điền cho quyền năng đang cầm. */
const powerParams = ref<Record<string, string | number>>({});
/** Ô đang chọn — khác `hovered`, vốn chỉ theo con trỏ. */
const pickedTile = ref<{ x: number; y: number } | null>(null);

/** Quyền năng đang cầm đã đủ điều kiện thi hành chưa. */
const powerReady = computed(() => {
  const p = activePower.value;
  if (!p) return false;
  return readiness(p, { being: !!godTarget.value, tile: !!pickedTile.value }).ready;
});

/** Tham số còn thiếu của quyền năng đang cầm. */
const missingParams = computed(() => {
  const p = activePower.value;
  if (!p?.params) return [];
  return p.params.filter((q) => {
    const v = powerParams.value[q.key];
    return v === undefined || v === "";
  });
});

function pickPower(p: Power): void {
  // Bấm lại đúng quyền năng đang cầm là bỏ nó xuống — một nút vừa bật vừa tắt
  // là thứ tay tự tìm ra mà không cần đọc hướng dẫn.
  if (activePower.value?.id === p.id) {
    dropPower();
    return;
  }
  activePower.value = p;
  powerParams.value = {};
  for (const q of p.params ?? []) {
    if (q.kind === "int" && q.def !== undefined) powerParams.value[q.key] = q.def;
    if (q.kind === "choice" && q.options?.[0]) powerParams.value[q.key] = q.options[0];
  }
  brush.value = null;
  guiding.value = false;
  godNote.value = "";
  // Quyền năng không cần trỏ vào gì thì thi hành ngay.
  if (p.needs === "none" && !p.params?.length) void castPower();
}

function dropPower(): void {
  activePower.value = null;
  powerParams.value = {};
  withdraw();
}

/**
 * Thi hành quyền năng đang cầm.
 *
 * Năm nhánh, theo `effect.via`. Ba nhánh đầu **đổi thế giới** và đi qua đúng
 * đường ghi của mọi thứ khác; hai nhánh `view` chỉ đổi cách nhìn và không sinh
 * một sự kiện nào (`§P6.8`).
 */
async function castPower(): Promise<void> {
  const p = activePower.value;
  if (!p) return;
  const r = readiness(p, { being: !!godTarget.value, tile: !!pickedTile.value });
  if (!r.ready) {
    godNote.value = tpRaw(`reason.${r.reason}`);
    return;
  }
  if (missingParams.value.length > 0) return;

  // ── Nhánh khung nhìn: không ghi gì vào thế giới ─────────────────────────
  if (p.effect.via === "view") {
    if (p.id === "sight.reveal") cycleOverlay();
    else if (p.id === "sight.pierce") await changeLayer(-1);
    else if (p.id === "time.still") await toggleTime();
    dropPower();
    return;
  }

  // Dựng ngữ cảnh bằng cách **thêm dần** thay vì gán `undefined`: dự án bật
  // `exactOptionalPropertyTypes`, và ở chế độ đó một trường mang `undefined`
  // khác hẳn một trường vắng mặt.
  const ctx: Parameters<typeof fieldsFor>[1] = { params: powerParams.value };
  if (godTarget.value) ctx.beingId = godTarget.value.id;
  if (pickedTile.value) ctx.tile = pickedTile.value;
  const fields = fieldsFor(p, ctx);
  if (!fields) {
    godNote.value = t("god.impossible");
    return;
  }

  try {
    if (p.effect.via === "build") {
      const mat = String(fields.material ?? "");
      const at = pickedTile.value;
      if (!at || !mat) {
        godNote.value = t("god.impossible");
        return;
      }
      const res = await api.build(at.x, at.y, mat);
      godNote.value = res.ok ? t("god.done") : (res.error ?? t("god.impossible"));
      await refresh();
    } else if (p.effect.via === "guide") {
      const who = godTarget.value;
      const at = pickedTile.value;
      if (!who || !at) return;
      await guide(who.id, at.x, at.y);
    } else if (p.effect.via === "command") {
      await send(p.effect.kind, fields);
    } else {
      // `preview`: nhìn trước hậu quả, người chơi tự quyết có khắc hay không.
      // Đây là điều tách một vị thần khỏi một cái nút — ngài **thấy** trước.
      await foresee(p.effect.kind, fields);
      return;
    }
    dropPower();
  } catch (e) {
    godNote.value = e instanceof Error ? e.message : String(e);
  }
}

/** Xoay vòng qua các lớp dữ liệu. `null` là tắt. */
function cycleOverlay(): void {
  const order = [null, "elevation", "water", "walkable", "crowd"] as const;
  const i = order.indexOf(overlayLayer.value as (typeof order)[number]);
  overlayLayer.value = order[(i + 1) % order.length] ?? null;
}

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
watch(
  settings,
  (v) => {
    saveSettings(v);
    setLocale(v.locale);
  },
  { deep: true },
);

const causeChain = ref<CauseLink[] | null>(null);
/** Nhãn tên nổi trên canvas — HTML, không phải `PIXI.Text` (`§P6.9.2`). */
const labels = ref<{ id: string; text: string; left: number; top: number; self: boolean }[]>([]);

let cursor = 0;
/** Lát `z` của lô ô đang giữ — đổi lát thì phải nạp lại dù khung nhìn không dời. */
let batchZ = Number.NaN;
let stopPolling: (() => void) | null = null;

/**
 * Cư dân đang được vị thần dõi theo, nếu có.
 *
 * Trước đây chỗ này là `avatar` — thân xác của người chơi. Không còn thân xác
 * nào: người chơi là một true god, và thứ camera bám theo là **một ai đó trong
 * thế giới**, do người chơi chọn, chứ không phải chính họ.
 */
const watched = computed(() => (following.value ? godTarget.value : null));

/** Số sinh mệnh trong thế giới — vật phẩm không tính. */
const souls = computed(() => entities.value.filter((e) => e.kind === "being").length);

const phaseLabel = computed(() => {
  const m = meta.value;
  if (!m) return "";
  return t(`time.${dayPhase(m.tick)}` as "time.day");
});

async function refresh(): Promise<void> {
  const v = view.value;
  if (!v) return;

  // Bốn câu hỏi độc lập, hỏi **cùng lúc**.
  //
  // Bản trước `await` từng cái một, nên mỗi vòng làm mất bốn lần độ trễ mạng
  // cộng lại thay vì một. Trên `localhost` con số đó nhỏ, nhưng nó nhân với hai
  // lần rưỡi mỗi giây, và nó là thứ chặn luôn cả những cú kéo chuột đang chờ.
  const [m, ents, ev] = await Promise.all([api.meta(), api.entities(), api.events(cursor)]);
  meta.value = m;

  const list = ents.entities;
  entities.value = list;
  // Giữ đối tượng đang chọn đồng bộ với dữ liệu mới: không có dòng này thì
  // panel cư dân đóng băng ở trạng thái lúc bấm chọn, và người chơi tưởng NPC
  // đứng im trong khi nó đang đi.
  if (godTarget.value) {
    godTarget.value = list.find((e) => e.id === godTarget.value?.id) ?? null;
  }

  // Bám theo ai thì lấy người đó làm tâm; không thì lấy chỗ vị thần đang nhìn.
  // `followStep` có vùng chết, nên mục tiêu nhích một ô không làm bản đồ giật.
  const anchor = watched.value;
  const want = anchor ? { x: anchor.x, y: anchor.y } : { x: m.eye[0], y: m.eye[1] };
  const step = followStep(camera.value ?? want, want);
  camera.value = { x: step.x, y: step.y };
  const cx = step.x;
  const cy = step.y;
  // Lô ô là thứ đắt nhất trong vòng này (vài nghìn ô, vài trăm KB JSON). Chỉ
  // hỏi lại khi lô đang có **không còn phủ** khung nhìn, hoặc khi lát `z` đổi.
  // Vì lô được lấy dư `BATCH_MARGIN` ô mỗi phía, phần lớn cú kéo trượt bên
  // trong dữ liệu đã có và không tốn một byte nào.
  if (!v.covers(cx, cy) || batchZ !== m.z) {
    const { w, h } = v.viewportTiles();
    const batch = await api.tiles(cx - (w >> 1), cy - (h >> 1), w, h, m.z);
    batchZ = m.z;
    lastBatch.value = batch;
    v.setTerrain(batch, m.tick);
  } else {
    v.retint(m.tick);
  }
  v.setCenter(cx, cy);
  v.setEntities(list);
  refreshLabels(list);
  if (m.steps_remaining === 0) v.setPath([]);

  cursor = ev.cursor;
  if (ev.events.length) events.value = [...ev.events, ...events.value].slice(0, 200);

  status.value = t("app.running");
  // Một lỗi cũ còn nằm trên màn hình sau khi mọi thứ đã chạy lại là một lời nói
  // dối — và người chơi sẽ đi tìm một lỗi không còn tồn tại.
  errorText.value = "";
}

function refreshLabels(list: Entity[]): void {
  const v = view.value;
  if (!v) return;
  // Chỉ đặt nhãn cho sinh vật: một bãi vật phẩm sẽ phủ kín bản đồ bằng chữ.
  labels.value = list
    .filter((e) => e.kind === "being")
    .flatMap((e) => {
      const p = v.screenOf(e.x, e.y);
      return p
        ? [{ id: e.id, text: e.name, left: p.left, top: p.top, self: e.id === godTarget.value?.id }]
        : [];
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

/**
 * Khiến người đang chọn nhặt vật phẩm nằm cạnh họ.
 *
 * Vị thần không cúi xuống nhặt gì cả — ngài **bảo** ai đó nhặt. Lệnh vẫn đi qua
 * `core.take`, nên mọi điều kiện tiên quyết của thế giới (phải nằm trên đất,
 * phải trong tầm với) vẫn chặn được như thường: `§10.4` cho engine quyền từ
 * chối, và một vị thần bị từ chối vẫn đúng hơn một vị thần đi vòng qua luật.
 */
async function bidTake(): Promise<void> {
  const who = godTarget.value;
  if (!who) return;
  const near = entities.value.find(
    (x) => x.kind === "item" && Math.abs(x.x - who.x) <= 1 && Math.abs(x.y - who.y) <= 1,
  );
  if (!near) {
    godNote.value = t("panel.controls.nothingHere");
    return;
  }
  await send("core.take", { who: entityRef(who.id), what: entityRef(near.id) });
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
  const m = meta.value;
  const cv = minimapEl.value;
  if (!m || !cv) return;
  const a = camera.value ?? { x: m.eye[0], y: m.eye[1] };
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

/** Bấm vào bản đồ thu nhỏ: dời cái nhìn tới đó. */
async function onMinimapClick(e: MouseEvent): Promise<void> {
  const cv = minimapEl.value;
  if (!cv || !minimapBatch) return;
  const r = cv.getBoundingClientRect();
  const px = ((e.clientX - r.left) / r.width) * MINIMAP_SIZE;
  const py = ((e.clientY - r.top) / r.height) * MINIMAP_SIZE;
  const w = minimapToWorld(minimapBatch, MINIMAP_SIZE, px, py);
  await lookAt(w.x, w.y);
}

/**
 * Dời cái nhìn của vị thần tới một ô.
 *
 * Thôi bám theo ai đó: hai thứ cùng kéo camera sẽ giằng nhau mỗi khung hình, và
 * người chơi chỉ thấy bản đồ rung.
 */
async function lookAt(x: number, y: number): Promise<void> {
  following.value = false;
  try {
    await api.look(x, y);
    await refresh();
  } catch (err) {
    errorText.value = err instanceof Error ? err.message : String(err);
  }
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
    drawer.value = "cause";
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
  // Bàn phím chỉ giữ những thứ không có chỗ tự nhiên trên bản đồ: đổi lát `z`,
  // dừng thời gian, thôi chọn. Mọi hành động **đổi thế giới** đi qua chuột —
  // `§P6.9.5` nói mỗi hành động phải là một command có schema, và một cú bấm
  // vào đúng ô là cách nói "chỗ này" ít mơ hồ nhất.
  if (e.key === "PageUp") void changeLayer(1);
  else if (e.key === "PageDown") void changeLayer(-1);
  else if (e.key === " ") {
    // Dấu cách dừng/chạy thời gian. Một vị thần dừng được thời gian là thứ đầu
    // tiên người ta thử, và không có nó thì mọi ý chỉ phải soạn giữa lúc thế
    // giới đang trôi.
    e.preventDefault();
    void toggleTime();
  } else if (e.key === "Escape") {
    // `Esc` bỏ thứ đang cầm trước; chỉ khi tay đã trống mới mở menu tạm dừng.
    // Ngược lại thì một cú `Esc` để bỏ cây cọ cũng ném người chơi ra khỏi thế
    // giới, và đó là thứ ai cũng bấm nhầm đúng một lần rồi nhớ mãi.
    if (brush.value || guiding.value || godTarget.value || foresight.value || activePower.value) {
      brush.value = null;
      guiding.value = false;
      godTarget.value = null;
      pickedTile.value = null;
      following.value = false;
      dropPower();
    } else {
      go("esc");
    }
  } else if (e.key === "g" || e.key === "G") godOpen.value = !godOpen.value;
}

/** Dừng hoặc thả thời gian, giữ lại nấc tốc độ cũ để quay về. */
let speedBeforePause = 5;
async function toggleTime(): Promise<void> {
  if (SPEED_STEPS[speedIndex.value]?.milli === 0) {
    await applySpeed(speedBeforePause);
  } else {
    speedBeforePause = speedIndex.value;
    await applySpeed(0);
  }
}

/**
 * Bấm trái: **soi xét**, không phải đi.
 *
 * Đây là thay đổi lớn nhất so với bản trước, và nó đến từ một câu hỏi của người
 * chơi: *"tại sao mặc định true god lại có cơ thể?"*. Không có cơ thể thì không
 * có gì để đi cả — nên cú bấm trái nói *"để mắt tới cái này"*, và mọi thứ đổi
 * thế giới đều là một **ý chỉ** gửi cho ai đó hoặc cho một ô đất.
 *
 * Ba nhánh, theo đúng thứ tự người chơi mong đợi:
 * 1. đang cầm vật liệu → khắc ô đó;
 * 2. đang cầm mệnh lệnh "chỉ đường" → gửi cư dân đang chọn tới ô đó;
 * 3. còn lại → chọn thứ dưới con trỏ, hoặc dời cái nhìn tới ô trống.
 *
 * Dùng `click` chứ không phải `pointerdown`: `click` là "nhấn và thả trên cùng
 * một phần tử", nên kéo bản đồ rồi thả không bị hiểu nhầm thành một mệnh lệnh.
 * Bản đầu dùng `pointerdown` và nó **im lặng không chạy** dưới điều khiển tự
 * động — thứ chỉ lộ ra khi tự vào chơi, không lộ ra ở bất kỳ bài test nào.
 */
async function onClick(e: MouseEvent): Promise<void> {
  const v = view.value;
  const c = canvasEl.value;
  if (!v || !c) return;
  // Kéo bản đồ rồi thả không phải một cú bấm.
  if (dragMoved) return;
  const r = c.getBoundingClientRect();
  const at = v.tileAt(e.clientX - r.left, e.clientY - r.top);
  if (!at) return;

  // Đang cầm vật liệu thì cú bấm là một nhát khắc.
  if (brush.value) {
    const res = await api.build(at.x, at.y, brush.value);
    if (!res.ok) godNote.value = res.error ?? t("god.impossible");
    await refresh();
    return;
  }

  // Đang cầm mệnh lệnh "chỉ đường" thì cú bấm là đích cho người đang chọn.
  if (guiding.value && godTarget.value) {
    await guide(godTarget.value.id, at.x, at.y);
    return;
  }

  const target = entities.value.find((x) => x.x === at.x && x.y === at.y);
  if (target) {
    selectTarget(target);
  } else {
    // Ô trống: **chọn ô đó**, không dời cái nhìn. Dời cái nhìn là việc của cú
    // kéo; nếu một cú bấm cũng dời được thì mỗi lần người chơi định chỉ vào một
    // thửa ruộng, bản đồ lại nhảy đi một đoạn.
    pickedTile.value = { x: at.x, y: at.y };
    godTarget.value = null;
    following.value = false;
  }

  // Đang cầm quyền năng thì cú bấm này vừa là chọn, vừa là thi hành.
  if (activePower.value && powerReady.value && missingParams.value.length === 0) {
    await castPower();
  }
}

/** Ra lệnh cho một cư dân đi tới một ô, và vẽ đường đã lên kế hoạch. */
async function guide(who: string, x: number, y: number): Promise<void> {
  try {
    const res = await api.guide(who, x, y);
    view.value?.setPath(res.path);
    walkNote.value =
      res.outcome === "unreachable"
        ? t("panel.controls.unreachable")
        : res.outcome === "partial"
          ? t("panel.controls.partial")
          : "";
    guiding.value = false;
    await refresh();
  } catch (err) {
    errorText.value = err instanceof Error ? err.message : String(err);
  }
}

function onContextMenu(e: MouseEvent): void {
  e.preventDefault();
  // Chuột phải là "thôi": bỏ vật liệu đang cầm, bỏ mệnh lệnh đang soạn, thu
  // hồi lệnh đi của người đang chọn. Một nút hủy chung đoán được là thứ người
  // chơi thử trước khi đọc bất kỳ hướng dẫn nào.
  brush.value = null;
  guiding.value = false;
  dropPower();
  const who = godTarget.value;
  if (who) void api.halt(who.id).then(() => refresh());
}

// ── Kéo để trượt bản đồ ─────────────────────────────────────────────────────
//
// Một vị thần không đi bộ, nên cách duy nhất để "tới" chỗ khác là kéo thế giới
// về phía mình. Làm bằng chuột trái vì đó là nút người ta thử trước; cú bấm chỉ
// được tính khi con trỏ **không** trượt quá `DRAG_SLOP`.

/** Ngưỡng pixel để một cú nhấn thành một cú kéo, không phải một cú bấm. */
const DRAG_SLOP = 4;
/** Cú kéo đang chạy, hoặc `null`. */
let dragFrom: { sx: number; sy: number; cx: number; cy: number } | null = null;
/** Cú nhấn này đã trượt quá ngưỡng chưa. */
let dragMoved = false;

function onPointerDown(e: PointerEvent): void {
  const m = meta.value;
  if (!m) return;
  const c = camera.value ?? { x: m.eye[0], y: m.eye[1] };
  dragFrom = { sx: e.clientX, sy: e.clientY, cx: c.x, cy: c.y };
  dragMoved = false;
}

function onPointerUp(): void {
  const wasDragging = dragMoved && dragFrom !== null;
  dragFrom = null;
  // Báo cho server biết cái nhìn đã dừng ở đâu — **một** lần cho cả cú kéo,
  // không phải một lần cho mỗi ô.
  if (wasDragging) {
    const c = camera.value;
    if (c) void api.look(c.x, c.y).catch(() => {});
    scheduleRefetch();
  }
}

/**
 * Lăn chuột để phóng to. **Không** gọi mạng.
 *
 * Đây là chỗ đã làm người chơi nói "scroll up, down mà cứ lag lag": bản trước
 * gọi `refresh()` sau mỗi nấc lăn — bốn round-trip HTTP nối đuôi nhau cộng một
 * lần vẽ lại toàn bộ texture, cho **mỗi** sự kiện bánh xe, mà bánh xe bắn
 * khoảng hai mươi sự kiện mỗi giây. Hàng đợi dồn lại và cả giao diện đứng hình.
 *
 * Giờ `WorldView.zoom` đổi ngay trên màn hình trong khung hình này, còn việc
 * hỏi lại lô ô rộng hơn thì hoãn tới khi tay đã dừng lăn.
 */
function onWheel(e: WheelEvent): void {
  e.preventDefault();
  if (!view.value?.zoom(e.deltaY < 0 ? 1 : -1)) return;
  scheduleRefetch();
}

/** Hẹn nạp lại lô ô sau khi thao tác đã lắng. */
let refetchTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleRefetch(): void {
  if (refetchTimer !== null) clearTimeout(refetchTimer);
  refetchTimer = globalThis.setTimeout(() => {
    refetchTimer = null;
    // Ép nạp lại: khung nhìn có thể vẫn "phủ đủ" theo `covers` nhưng ở mức
    // phóng nhỏ hơn thì lô cũ không còn đủ rộng, và người chơi sẽ thấy viền đen.
    batchZ = Number.NaN;
    void refresh().catch(() => {});
  }, REFETCH_DELAY_MS);
}

/**
 * Hoãn bao lâu sau nấc lăn cuối mới hỏi lại dữ liệu.
 *
 * Đủ dài để một cú lăn liên tục chỉ tốn **một** vòng mạng, đủ ngắn để mắt
 * không kịp nhận ra là đã đợi.
 */
const REFETCH_DELAY_MS = 140;

function onPointerMove(e: PointerEvent): void {
  const v = view.value;
  const c = canvasEl.value;
  if (!v || !c) return;

  // Đang kéo thì trượt cái nhìn theo chuột, tính bằng **ô** chứ không bằng
  // pixel: một cú kéo phải trượt đúng khoảng đất nằm dưới ngón tay ở mọi mức
  // phóng, và chia ở đây là chỗ duy nhất biết cỡ ô hiện tại.
  const d = dragFrom;
  if (d) {
    const dx = e.clientX - d.sx;
    const dy = e.clientY - d.sy;
    if (!dragMoved && Math.abs(dx) + Math.abs(dy) > DRAG_SLOP) dragMoved = true;
    if (dragMoved) {
      const ts = v.tileSize;
      const nx = d.cx - Math.round(dx / ts);
      const ny = d.cy - Math.round(dy / ts);
      const cam = camera.value;
      if (!cam || cam.x !== nx || cam.y !== ny) {
        // Dời khung nhìn **ngay**, không hỏi server. Bản trước gửi một
        // `POST /api/look` cho mỗi ô trượt qua, tức hàng chục yêu cầu mỗi giây
        // giữa lúc tay đang kéo — và cú kéo giật đúng theo nhịp mạng.
        //
        // Cái nhìn là trạng thái **khung nhìn** (`§P6.8`), nên client sở hữu nó
        // được; server chỉ cần biết khi tay đã buông, để lần sau mở ra còn đúng
        // chỗ.
        camera.value = { x: nx, y: ny };
        following.value = false;
        view.value?.setCenter(nx, ny);
        if (!view.value?.covers(nx, ny)) scheduleRefetch();
      }
      return;
    }
  }

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
    // **Không** gọi `refresh()` ở đây: người chơi còn đang ở màn hình đầu, và
    // một thế giới bắt đầu trôi sau lưng một cái menu là một thế giới đã già đi
    // trước khi ai kịp nhìn nó.
    // Nhịp hỏi lại chậm hơn nhịp tick của server. Đây là chỗ WebSocket sẽ thay
    // vào khi `§P6.8` được nối đầy đủ.
    const timer = globalThis.setInterval(() => {
      // Đứng ở màn hình đầu hay đang tạm dừng thì không hỏi lại làm gì: hỏi một
      // thế giới mà không ai đang nhìn là trả giá cho thứ không ai thấy.
      if (screen.value !== "world") return;
      void refresh().catch(() => {});
    }, 400);
    // Bản đồ thu nhỏ đổi chậm hơn nhiều so với khung nhìn chính, và mỗi lần vẽ
    // là một lô 128×128 ô — hỏi lại mỗi 400 ms là trả giá cho thứ không đổi.
    const miniTimer = globalThis.setInterval(() => {
      if (screen.value !== "world") return;
      void refreshMinimap().catch(() => {});
    }, 2_500);
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
    <!-- ── Thanh trên: thế giới, thời gian, sức khỏe khu định cư ───────── -->
    <header>
      <strong>{{ t("app.title") }}</strong>
      <span v-if="meta" class="stat">{{ phaseLabel }}</span>
      <span v-if="meta" class="stat">{{ t("hud.day") }} {{ Math.floor(meta.tick / 2400) }}</span>

      <span class="spacer"></span>

      <!-- Nấc tốc độ thành nút, không phải thanh trượt: người chơi cần
           **nhảy** về ×1 hoặc về dừng, không cần rà tìm nấc đúng. -->
      <div class="speeds">
        <button
          v-for="(sp, i) in SPEED_MARKS"
          :key="sp.i"
          :class="{ on: speedIndex === sp.i }"
          :title="SPEED_STEPS[sp.i]?.label"
          @click="applySpeed(sp.i)"
        >
          {{ sp.label }}<span v-if="i < 0"></span>
        </button>
      </div>

      <span class="spacer"></span>

      <span class="stat">{{ t("hud.souls") }} {{ souls }}</span>
      <span v-if="meta" class="stat">{{ t("hud.layer") }} {{ meta.z }}</span>
      <span class="status">{{ status }}</span>
      <button class="menubtn" :title="t('menu.open')" @click="go('esc')">☰</button>
    </header>

    <div class="stage">
      <!-- ── Thanh công cụ trái ────────────────────────────────────────── -->
      <nav class="rail">
        <button
          v-for="d in DRAWERS"
          :key="d.id"
          :class="{ on: drawer === d.id }"
          :title="t(d.label)"
          @click="toggleDrawer(d.id)"
        >
          <span class="glyph">{{ d.glyph }}</span>
        </button>
      </nav>

      <main>
        <canvas
          ref="canvasEl"
          :class="{ shaping: !!brush, guiding: guiding || !!activePower }"
          @pointerdown="onPointerDown"
          @pointerup="onPointerUp"
          @pointerleave="onPointerUp"
          @pointermove="onPointerMove"
          @wheel="onWheel"
          @click="onClick"
          @contextmenu="onContextMenu"
        ></canvas>
        <div class="vignette"></div>

        <div v-if="settings.showLabels" class="labels">
          <span
            v-for="l in labels"
            :key="l.id"
            class="label"
            :class="{ self: l.self }"
            :style="{ left: `${l.left}px`, top: `${l.top}px` }"
            >{{ l.text }}</span
          >
        </div>

        <!-- Thẻ theo con trỏ: ba dòng, ngay cạnh chuột, không làm nhảy layout. -->
        <div v-if="hovered" class="hovercard">
          <b>{{ palette.label(hovered.material) }}</b>
          <span class="dim">{{ hovered.biome }} · {{ hovered.height }} m</span>
          <span v-if="hovered.drop > 0" class="dim">
            {{ t("panel.tile.depth") }} {{ hovered.drop }} m
          </span>
        </div>

        <!-- Bản đồ thu nhỏ, góc dưới phải, thu được. -->
        <div class="minicorner" :class="{ folded: !miniOpen }">
          <button class="fold" :title="t('panel.minimap')" @click="miniOpen = !miniOpen">
            {{ miniOpen ? "▾" : "▴" }}
          </button>
          <div v-show="miniOpen" class="mini">
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
        </div>

        <PowerDock
          :selected-being="godTarget?.id ?? null"
          :selected-tile="pickedTile"
          :active="activePower?.id ?? null"
          @pick="pickPower"
          @cancel="dropPower"
        />

        <div v-if="activePower && missingParams.length" class="paramform">
          <b>{{ tpRaw(`power.${activePower.id}.name`) }}</b>
          <label v-for="q in activePower.params" :key="q.key">
            <span>{{ tpRaw(`param.${q.key}`) }}</span>
            <select
              v-if="q.kind === 'choice'"
              :value="powerParams[q.key]"
              @change="powerParams[q.key] = ($event.target as HTMLSelectElement).value"
            >
              <option v-for="o in q.options" :key="o" :value="o">{{ o }}</option>
            </select>
            <input
              v-else
              :type="q.kind === 'int' ? 'number' : 'text'"
              :value="powerParams[q.key] ?? ''"
              @input="powerParams[q.key] = ($event.target as HTMLInputElement).value"
            />
          </label>
          <div class="acts">
            <button class="primary" :disabled="!powerReady" @click="castPower">
              {{ t("god.inscribe") }}
            </button>
            <button @click="dropPower">{{ t("god.withdraw") }}</button>
          </div>
        </div>

        <p v-if="errorText" class="error">{{ errorText }}</p>
      </main>

      <!-- ── Ngăn kéo, đè lên thế giới ─────────────────────────────────── -->
      <aside v-if="drawer" class="drawer">
        <div class="drawerhead">
          <b>{{ t(DRAWERS.find((d) => d.id === drawer)?.label ?? "panel.present") }}</b>
          <button class="toggle" @click="drawer = null">×</button>
        </div>

        <template v-if="drawer === 'observe'">
          <ObservePanel
            v-if="godTarget && godTarget.kind === 'being'"
            :target="godTarget"
            :events="events"
            :following="following"
            @toggle-follow="toggleFollow"
            @trace="traceCause"
          />
          <section>
            <h2>{{ t("panel.present") }} ({{ souls }})</h2>
            <ul class="list">
              <li
                v-for="e in entities"
                :key="e.id"
                :class="{ sel: godTarget?.id === e.id }"
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
        </template>

        <section v-else-if="drawer === 'layers'">
          <DataOverlay
            v-model="overlayLayer"
            :batch="lastBatch"
            :entities="entities"
            @paint="view?.setOverlay($event)"
          />
        </section>

        <section v-else-if="drawer === 'chronicle'" class="grow">
          <ChroniclePanel :events="events" :entities="entities" @trace="traceCause" />
        </section>

        <section v-else-if="drawer === 'cause'">
          <p v-if="!causeChain" class="dim">{{ t("panel.events.hint") }}</p>
          <template v-else>
            <ol class="chain">
              <li v-for="c in causeChain" :key="c.seq">
                <span class="dim">t{{ c.tick }}</span> <b>{{ c.kind }}</b>
                <div class="dim sum">{{ c.summary }}</div>
              </li>
            </ol>
            <p v-if="causeChain.length <= 1" class="dim">{{ t("panel.cause.root") }}</p>
          </template>
        </section>
      </aside>
    </div>

    <!-- ── Khay ngữ cảnh: chỉ hiện khi đã chọn thứ gì đó ─────────────────── -->
    <div v-if="godTarget || pickedTile || foresight" class="tray">
      <template v-if="godTarget">
        <div class="who">
          <b>{{ godTarget.name }}</b>
          <span v-if="godTarget.role" class="dim">{{ tRuntime("role", godTarget.role) }}</span>
          <span v-if="godTarget.intent">{{ tRuntime("intent", godTarget.intent) }}</span>
          <span v-if="godTarget.hunger !== null" class="dim">
            {{ t("panel.who.hunger") }} {{ godTarget.hunger }}
          </span>
        </div>
      </template>
      <div v-else-if="pickedTile" class="who">
        <b>{{ t("panel.tile") }}</b>
        <span class="dim">({{ pickedTile.x }}, {{ pickedTile.y }})</span>
      </div>

      <div v-if="foresight" class="foresight">
        <ul class="list">
          <li v-for="c in foresight.changes.slice(0, 4)" :key="c.id">
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

      <span class="spacer"></span>
      <span v-if="godNote" class="cmd warn">{{ godNote }}</span>
      <span v-if="walkNote" class="cmd warn">{{ walkNote }}</span>
      <button class="toggle" @click="clearSelection">×</button>
    </div>

    <!-- Các màn hình menu nằm **trên** thế giới chứ không thay thế nó: giữ
         canvas sống nghĩa là quay lại không phải dựng lại Pixi và nạp lại lô
         ô, và người chơi thấy thế giới mờ sau lớp phủ — nó vẫn ở đó. -->
    <TitleScreen
      v-if="screen === 'title'"
      @play="enterWorld"
      @settings="go('settings')"
      @codex="go('codex')"
    />
    <PauseMenu
      v-else-if="screen === 'paused' && meta"
      :seed="Number(meta.seed)"
      :tick="meta.tick"
      :state-hash="meta.state_hash"
      :population="souls"
      @resume="go('resume')"
      @settings="go('settings')"
      @codex="go('codex')"
      @quit-to-title="leaveWorld"
    />
    <SettingsPanel v-if="screen === 'settings'" v-model="settings" @close="go('esc')" />
    <CodexPanel v-if="screen === 'codex'" @close="go('esc')" />
  </div>
</template>

<style scoped>
.mow { display: flex; flex-direction: column; height: 100vh; background: #070910; color: #e6e9ef;
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif; overflow: hidden; }

/* ── Thanh trên ────────────────────────────────────────────────────────── */
header { display: flex; align-items: center; gap: 0.8rem; height: 44px; flex: none;
  padding: 0 0.8rem; border-bottom: 1px solid #1b2029; background: #0b0e14;
  font: 12px/1.4 ui-monospace, monospace; }
header strong { font-family: ui-sans-serif, system-ui, sans-serif; letter-spacing: 0.02em;
  font-size: 13px; }
.stat { color: #93a1b5; }
.spacer { flex: 1; }
.status { color: #6b7788; }
.menubtn { width: 28px; height: 26px; padding: 0; border: 1px solid #2a3441; border-radius: 4px;
  background: #12161e; color: #cfd8e3; font-size: 14px; cursor: pointer; }
.menubtn:hover { background: #1a1f28; }

/* Nấc tốc độ: bốn nút, nấc đang chạy sáng lên. Người chơi phải biết thế giới
   đang trôi nhanh hay đã dừng mà không cần đọc số. */
.speeds { display: flex; gap: 2px; }
.speeds button { min-width: 34px; height: 24px; padding: 0 6px; border: 1px solid #2a3441;
  border-radius: 3px; background: #12161e; color: #9aa7b8; font: inherit; cursor: pointer; }
.speeds button:hover { background: #1a1f28; }
.speeds button.on { border-color: #6a5426; background: #2a2415; color: #f0c674; }

/* ── Sân khấu: rail | canvas | drawer ──────────────────────────────────── */
.stage { flex: 1; min-height: 0; display: flex; position: relative; }

.rail { width: 52px; flex: none; display: flex; flex-direction: column; gap: 4px;
  padding: 6px 0; border-right: 1px solid #1b2029; background: #0b0e14; align-items: center; }
.rail button { width: 38px; height: 38px; padding: 0; border: 1px solid transparent;
  border-radius: 6px; background: transparent; color: #8a9bb0; font-size: 17px; cursor: pointer; }
.rail button:hover { background: #141922; color: #cfd8e3; }
/* Công cụ đang bật phải đọc được ở góc mắt: đổi cả nền, viền và màu chữ, không
   chỉ một viền mảnh. */
.rail button.on { border-color: #6a5426; background: #2a2415; color: #f0c674; }
.glyph { line-height: 1; }

main { flex: 1; min-width: 0; position: relative; overflow: hidden; }
/* Con trỏ nói trước cú bấm sẽ làm gì. Không có dấu hiệu này thì một cú bấm
   nhầm sẽ sửa thế giới trong khi người chơi tưởng mình đang xem. */
canvas { display: block; width: 100%; height: 100%; cursor: grab; }
canvas:active { cursor: grabbing; }
canvas.shaping, canvas.guiding { cursor: crosshair; }

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
.label.self { color: #f0c674; background: rgb(42 36 21 / 82%); }

/* Thẻ theo con trỏ: cố định một góc chứ không bám chuột. Bám chuột thì nó che
   mất chính thứ người chơi đang nhìn. */
.hovercard { position: absolute; left: 10px; top: 10px; display: flex; flex-direction: column;
  padding: 5px 9px; border: 1px solid #232833; border-radius: 5px;
  background: rgb(11 14 20 / 88%); font-size: 12px; pointer-events: none; }

.error { position: absolute; left: 1rem; bottom: 5.5rem; margin: 0; padding: 0.5rem 0.75rem;
  background: #45161a; border: 1px solid #7d2b32; border-radius: 4px; color: #ffd7d7; }

/* ── Bản đồ thu nhỏ ở góc ──────────────────────────────────────────────── */
.minicorner { position: absolute; right: 10px; bottom: 92px; width: 168px;
  border: 1px solid #232833; border-radius: 6px; background: rgb(11 14 20 / 92%);
  padding: 4px; }
.minicorner.folded { width: auto; }
.fold { position: absolute; right: 4px; top: 2px; width: 18px; height: 16px; padding: 0;
  border: 0; background: transparent; color: #7f8ea3; cursor: pointer; font-size: 11px; }
.mini { position: relative; line-height: 0; }
/* `pixelated` là bắt buộc: minimap gộp ô bằng `mode` để giữ ranh giới sắc nét,
   và một phép nội suy song tuyến ở tầng trình duyệt sẽ trung bình các pixel lân
   cận — tức là dựng lại đúng màu bùn mà module vừa tránh. */
.mini canvas { width: 100%; height: auto; display: block; border-radius: 3px;
  image-rendering: pixelated; cursor: crosshair; }
.minimark { position: absolute; width: 7px; height: 7px; margin: -4px 0 0 -4px;
  border: 2px solid #fff; border-radius: 50%; box-shadow: 0 0 0 1px #16324f;
  pointer-events: none; }

/* ── Ngăn kéo: đè lên thế giới, không thu hẹp nó ───────────────────────── */
.drawer { position: absolute; right: 0; top: 0; bottom: 0; width: 340px;
  border-left: 1px solid #232833; background: rgb(10 13 19 / 97%);
  display: flex; flex-direction: column; overflow-y: auto;
  box-shadow: -12px 0 32px rgb(0 0 0 / 40%); }
.drawerhead { display: flex; align-items: center; padding: 0.55rem 0.8rem;
  border-bottom: 1px solid #1b2029; font-size: 12px; text-transform: uppercase;
  letter-spacing: 0.08em; color: #9aa7b8; }
section { padding: 0.7rem 0.8rem; border-bottom: 1px solid #1b2029; }
section.grow { flex: 1; min-height: 0; overflow-y: auto; }
h2 { margin: 0 0 0.45rem; font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em;
  color: #7f8ea3; font-weight: 600; }

/* ── Khay ngữ cảnh dưới cùng ───────────────────────────────────────────── */
.tray { flex: none; display: flex; align-items: center; gap: 0.8rem;
  padding: 0.45rem 0.8rem; border-top: 1px solid #232833; background: #0b0e14;
  font-size: 12px; }
.tray .who { display: flex; align-items: center; gap: 0.55rem; }
.tray .foresight { display: flex; align-items: center; gap: 0.7rem; }
.tray .list { display: flex; gap: 0.7rem; }
.tray .list li { white-space: nowrap; }

.dim { color: #6b7788; }
.list { list-style: none; margin: 0; padding: 0; }
.list li { padding: 1px 0; }
.doing { padding-left: 13px; font-size: 11px; color: #8a9bb0; }
.dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 5px;
  background: #d08770; }
.dot.item { border-radius: 1px; background: #f0c674; transform: rotate(45deg); }
.toggle { margin-left: auto; width: 20px; height: 18px; padding: 0; line-height: 1;
  border: 1px solid #333c4a; border-radius: 3px; background: #171b22; color: #9aa7b8;
  cursor: pointer; }
.acts { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.acts button { padding: 3px 8px; border: 1px solid #3a4553; border-radius: 3px;
  background: #1a1f28; color: #d8dee9; font: inherit; font-size: 12px; cursor: pointer; }
.acts button:hover:not(:disabled) { background: #232a35; }
.acts button:disabled { opacity: 0.45; cursor: default; }
.acts button.primary { border-color: #6a5426; background: #2a2415; color: #f0c674; }

/* Ô nhập tham số nổi ngay trên thanh quyền năng: người chơi vừa chọn quyền
   năng ở đó, nên thứ hỏi thêm phải xuất hiện cạnh tay họ, không phải ở cột bên
   kia màn hình. */
.paramform { position: absolute; left: 50%; bottom: 92px; transform: translateX(-50%);
  display: flex; flex-direction: column; gap: 0.35rem; padding: 0.6rem 0.8rem;
  border: 1px solid #2a3441; border-radius: 6px; background: rgb(12 15 21 / 96%);
  box-shadow: 0 8px 24px rgb(0 0 0 / 45%); min-width: 220px; }
.paramform label { display: flex; align-items: center; gap: 0.5rem; font-size: 12px; }
.paramform label span { color: #7f8ea3; min-width: 4.5rem; }
.paramform input, .paramform select { flex: 1; padding: 3px 6px; border: 1px solid #333c4a;
  border-radius: 3px; background: #171b22; color: #e6e9ef; font: inherit; font-size: 12px; }

/* Chuỗi đi ngược thời gian, nên nó xuống dòng như một dòng dõi: mỗi mắt lùi
   vào một chút để mắt đọc được chiều "cái này gây ra cái trước nó". */
.chain { list-style: none; margin: 0; padding: 0; }
.chain li { padding: 2px 0 2px 10px; border-left: 2px solid #2a3441; margin-left: 3px; }
.chain .sum { font: 11px ui-monospace, monospace; }
.pick { cursor: pointer; }
.pick:hover { color: #cfd8e3; }
.pick.sel { color: #f0c674; }
.cmd { font: 11px ui-monospace, monospace; color: #9aa7b8; }
.cmd.warn { color: #e0a04a; }
</style>
