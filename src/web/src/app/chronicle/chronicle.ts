/**
 * Dệt nhật ký sự kiện thô (`WorldEvent[]`) thành một cuốn sử đọc được.
 *
 * ## Vì sao "gộp rồi xếp hạng", không phải "lọc"
 *
 * Panel Sự kiện cũ liệt kê thẳng `WorldEvent[]` — đúng nhưng vô nghĩa: một
 * cuộc đi từ nhà ra đồng phát ra một `core.entity.moved` mỗi tick, nên mười
 * bước chân là mười dòng giống hệt nhau. Lọc bớt sẽ mất thông tin ("có đi
 * chưa?"); phải **gộp** chúng thành một dòng mang số bước, rồi **xếp hạng**
 * để một bàn tay thần can thiệp không bị chìm giữa một trăm bước chân tầm
 * thường. Đó là lý do [`Chapter`] có `count` (gộp) và `weight` (hạng), còn
 * `WorldEvent` thì không có cái nào.
 *
 * ## Vì sao module này thuần
 *
 * Không Vue, không DOM, không `fetch`, không đọc `locale()`. Quyết định gộp
 * bao nhiêu tick thì tính là "cùng một cuộc đi", và biến cố nào đáng nổi bật —
 * đó là logic có thể sai, nên nó phải kiểm được bằng dữ liệu bịa, không cần
 * dựng cả Vue lẫn server giả. Chữ hiển thị thật (tiếng Việt/Anh) nằm ở
 * `strings.ts`; ở đây chỉ có `key` — một chuỗi ổn định để tra chữ sau.
 *
 * ## Vì sao chỉ tin đúng những `kind` đã xác nhận có thật
 *
 * `core.entity.spawned`, `core.entity.moved`, `core.need.set`, `npc.intended`,
 * `core.item.taken`, `core.item.eaten`, `core.speech.uttered`,
 * `core.act.committed`, `truegod.intervened` — chín loại này được xác nhận
 * trong `mow-scenario/src/{slice,testing}.rs`, tức bộ handler thật mà
 * `mow-server` dùng để dựng thế giới (`slice::build_empty_world` gọi
 * `slice_handlers()`, và hàm đó gộp cả `testing::handlers()` vào). Loại nào
 * ngoài chín cái này rơi vào nhánh mặc định ở cuối [`buildChapter`] — vẫn ra
 * một chương đọc được, mang theo chính `kind`, chứ không bịa thêm nghĩa cho
 * một sự kiện chưa từng thấy.
 *
 * ## Vì sao có sự kiện luôn "vô danh"
 *
 * Server chỉ gửi `actor` qua `/api/events`, không gửi `subject` (`Event` ở
 * `mow-core` có cả hai trường, nhưng `mow-server/src/api.rs::events()` chỉ
 * đưa `actor` lên dây — xem thêm giải thích y hệt ở `app/observe.ts`).
 * `core.entity.spawned`, `core.need.set`, `truegod.intervened` chỉ gắn
 * `.on(subject)` chứ không `.by(actor)`, nên `actor` của chúng **luôn**
 * `null` — không phải một trường hợp hiếm cần né, mà là hình dạng bình
 * thường của ba loại này. Vì vậy chúng không có mẫu chữ mang `{who}`, và
 * không cố suy đoán ai đứng sau — một cái tên đoán ra thì tệ hơn không có,
 * vì người đọc sử sẽ tin nó.
 */

import type { WorldEvent } from "@/api/game";

/** Một dòng trong cuốn sử. */
export interface Chapter {
  /** Khóa ổn định để hiện chữ, ví dụ `"chronicle.journey"`. */
  key: string;
  /** Tick sớm nhất và muộn nhất mà mắt sử này bao trùm. */
  from: number;
  to: number;
  /** Ngày thứ mấy (tính từ tick 0). Luôn tính theo `to` — mốc gần nhất mà
   * chương này còn đúng — để một chương đang gộp dần không bị kẹt ở ngày của
   * bước chân đầu tiên trong khi những bước sau đã sang ngày mới. */
  day: number;
  /** Ai là chủ thể, nếu biết. */
  who: string | null;
  /** Số sự kiện đã gộp vào đây. */
  count: number;
  /** Sự kiện tiêu biểu, để truy ngược nhân quả — luôn là sự kiện **mới nhất**
   * trong nhóm đã gộp, vì "vì sao" người đọc muốn hỏi là vì sao chuyện *vừa
   * xảy ra*, và chuỗi nhân quả của bước cuối vẫn kéo được về tới gốc. */
  seq: number;
  /** Mức đáng chú ý, `0` là nền, `2` là biến cố. */
  weight: 0 | 1 | 2;
  /** Dữ liệu để điền vào chuỗi chữ. */
  slots: Record<string, string | number>;
}

/**
 * Số tick một ngày. Chép tay từ `TICKS_PER_DAY` của `render/terrain.ts`,
 * **không import** — nhiệm vụ này bị cấm đụng `render/*`, và module thuần ở
 * đây phải đứng độc lập được với tầng vẽ dù sao đi nữa (test không cần biết
 * `render/*` tồn tại). Hai chỗ cùng giữ `2400` là một trade-off có chủ đích,
 * không phải sao chép quên đồng bộ; nếu đồng hồ trong `Worldgen` đổi nhịp,
 * đây là chỗ thứ hai cần sửa theo.
 */
const DEFAULT_TICKS_PER_DAY = 2_400;

/**
 * Cửa sổ gộp mặc định, tính bằng tick.
 *
 * `core.walk` phát một `core.entity.moved` mỗi bước, và giữa hai bước liên
 * tiếp của một cuộc đi bình thường thường chỉ cách nhau một tick. `20` đủ
 * rộng để chịu một nhịp khựng ngắn (đứng lại tránh người khác, chờ đường)
 * mà không nối liền hai cuộc đi cách nhau một quãng nghỉ thật sự.
 */
const DEFAULT_WINDOW = 20;

/** Tính ngày từ tick, không vỡ khi `ticksPerDay` không dương. */
function dayOf(tick: number, ticksPerDay: number): number {
  // `0`, số âm, hoặc `NaN` đều rơi về ngày `0` thay vì `NaN`/`Infinity` — một
  // dải nhịp hoạt động vỡ ở phép chia còn tệ hơn một dải gộp sai ngày.
  if (!(ticksPerDay > 0)) return 0;
  return Math.floor(tick / ticksPerDay);
}

/** Đọc `payload` (kiểu `unknown`) như một object phẳng, không ép kiểu ẩu. */
function payloadRecord(payload: unknown): Record<string, unknown> | null {
  if (payload === null || typeof payload !== "object" || Array.isArray(payload)) return null;
  return payload as Record<string, unknown>;
}

/** Trường chuỗi trong `payload`, hoặc `null` nếu không có hay sai kiểu. */
function textAt(payload: unknown, key: string): string | null {
  const v = payloadRecord(payload)?.[key];
  return typeof v === "string" ? v : null;
}

/** Trường số trong `payload`, hoặc `null` nếu không có hay sai kiểu. */
function numberAt(payload: unknown, key: string): number | null {
  const v = payloadRecord(payload)?.[key];
  return typeof v === "number" ? v : null;
}

/**
 * Tên hiển thị của một chủ thể, hoặc `null` nếu không biết.
 *
 * Hai lý do dẫn tới `null`, và cả hai đều phải ra cùng một kết quả: không có
 * `actor` (sự kiện vốn vô danh, xem đầu file), hoặc có `actor` nhưng
 * `names` — bảng tra do nơi gọi cấp, dựng từ `Entity[]` hiện có — không chứa
 * id đó (thực thể đã biến mất, hoặc chưa từng nằm trong tầm nhìn). Không
 * bao giờ trả về chính `actor` (id thô): một chuỗi số 64-bit hiện lên thay
 * tên trông như một tên thật trong khi nó chỉ là một định danh không tra
 * được — dối hơn là nói thẳng "không biết".
 */
function nameOf(actor: string | null, names: Map<string, string>): string | null {
  if (actor === null) return null;
  return names.get(actor) ?? null;
}

/** Hạng đáng chú ý theo `kind`. Loại lạ mặc định hạng `1`: không nền (im
 * lặng trôi qua), cũng không tự nhận là biến cố — chỗ giữa an toàn cho một
 * sự kiện chưa ai biết trước tầm quan trọng. */
const WEIGHT_BY_KIND: Readonly<Record<string, 0 | 1 | 2>> = {
  "core.entity.moved": 0,
  "core.need.set": 0,
  "npc.intended": 1,
  "core.item.taken": 1,
  "core.item.eaten": 1,
  "core.speech.uttered": 1,
  "core.act.committed": 1,
  "core.entity.spawned": 2,
  "truegod.intervened": 2,
};

function weightOf(kind: string): 0 | 1 | 2 {
  return WEIGHT_BY_KIND[kind] ?? 1;
}

/** Phần chung của một chương đơn (chưa gộp gì thêm ngoài chính nó). */
function baseOf(e: WorldEvent, who: string | null): Omit<Chapter, "key" | "slots"> {
  return {
    from: e.tick,
    to: e.tick,
    day: dayOf(e.tick, DEFAULT_TICKS_PER_DAY),
    who,
    count: 1,
    seq: e.seq,
    weight: weightOf(e.kind),
  };
}

/**
 * Dựng một chương từ đúng một sự kiện (mọi loại trừ `core.entity.moved`, loại
 * đó có đường gộp riêng ở [`weave`]).
 *
 * Mỗi nhánh chọn giữa mẫu "biết `who`" và mẫu "`.unknown`" — hai khóa khác
 * nhau thay vì một khóa với `{who}` rỗng, để `strings.ts` viết được một câu
 * trọn vẹn cho trường hợp vô danh ("có ai đó…") thay vì một câu cụt thiếu
 * chủ ngữ.
 */
function buildChapter(e: WorldEvent, names: Map<string, string>): Chapter {
  const who = nameOf(e.actor, names);
  const base = baseOf(e, who);

  switch (e.kind) {
    case "npc.intended": {
      const intent = textAt(e.payload, "intent") ?? "?";
      return who !== null
        ? { ...base, key: "chronicle.intent", slots: { who, intent } }
        : { ...base, key: "chronicle.intent.unknown", slots: { intent } };
    }
    case "core.item.taken":
      return who !== null
        ? { ...base, key: "chronicle.itemTaken", slots: { who } }
        : { ...base, key: "chronicle.itemTaken.unknown", slots: {} };
    case "core.item.eaten": {
      const nutrition = numberAt(e.payload, "nutrition") ?? 0;
      return who !== null
        ? { ...base, key: "chronicle.itemEaten", slots: { who, nutrition } }
        : { ...base, key: "chronicle.itemEaten.unknown", slots: { nutrition } };
    }
    case "core.speech.uttered": {
      const text = textAt(e.payload, "text") ?? "…";
      return who !== null
        ? { ...base, key: "chronicle.speech", slots: { who, text } }
        : { ...base, key: "chronicle.speech.unknown", slots: { text } };
    }
    case "core.act.committed": {
      const act = textAt(e.payload, "act") ?? "?";
      return who !== null
        ? { ...base, key: "chronicle.actCommitted", slots: { who, act } }
        : { ...base, key: "chronicle.actCommitted.unknown", slots: { act } };
    }
    case "core.entity.spawned": {
      const kind = textAt(e.payload, "kind") ?? "?";
      return { ...base, key: "chronicle.spawned", slots: { kind } };
    }
    case "core.need.set": {
      const need = textAt(e.payload, "need") ?? "?";
      const value = numberAt(e.payload, "value") ?? 0;
      return { ...base, key: "chronicle.needSet", slots: { need, value } };
    }
    case "truegod.intervened": {
      const key = textAt(e.payload, "key") ?? "?";
      return { ...base, key: "chronicle.intervened", slots: { key } };
    }
    default:
      // Loại chưa biết tới (content pack tương lai — `§19.7` — hay chính
      // `core.entity.moved` lỡ trôi tới đây vì mang `actor: null`, thứ không
      // nên xảy ra nhưng không được phép làm sập chỗ này nếu nó xảy ra): vẫn
      // phải ra một chương đọc được, mang theo `kind` thật, không được biến
      // mất im lặng và không được hiện `undefined`.
      return who !== null
        ? { ...base, key: "chronicle.other", slots: { who, kind: e.kind } }
        : { ...base, key: "chronicle.other.unknown", slots: { kind: e.kind } };
  }
}

/** Mở một chương "cuộc đi" mới từ một `core.entity.moved` đầu tiên của run. */
function openJourney(e: WorldEvent, who: string | null): Chapter {
  const slots: Record<string, string | number> = { count: 1 };
  if (who !== null) slots.who = who;
  const x = numberAt(e.payload, "x");
  const y = numberAt(e.payload, "y");
  if (x !== null) slots.x = x;
  if (y !== null) slots.y = y;
  return {
    key: who !== null ? "chronicle.journey" : "chronicle.journey.unknown",
    from: e.tick,
    to: e.tick,
    day: dayOf(e.tick, DEFAULT_TICKS_PER_DAY),
    who,
    count: 1,
    seq: e.seq,
    weight: 0,
    slots,
  };
}

/**
 * Gộp dòng sự kiện thô thành các chương.
 * `names` là bảng tra `id -> tên`, vì `WorldEvent` chỉ mang định danh.
 */
export function weave(
  events: WorldEvent[],
  names: Map<string, string>,
  opts?: {
    /** Khoảng tick tối đa để gộp hai sự kiện cùng loại cùng người. */
    window?: number;
    /** Số chương tối đa trả về. */
    limit?: number;
  },
): Chapter[] {
  const window = opts?.window ?? DEFAULT_WINDOW;

  // `[...events]` trước khi `sort`: `Array.prototype.sort` sửa tại chỗ, và
  // `events` là dữ liệu chia sẻ với vòng lặp `refresh()` của nơi gọi — sửa
  // nó ở đây là một tác dụng phụ mà nơi gọi không ngờ tới. Sắp tăng dần theo
  // thời gian để gộp đúng thứ tự xảy ra; đảo lại thành "mới nhất trước" ở
  // bước cuối, sau khi đã gộp xong.
  const ordered = [...events].sort((a, b) => a.tick - b.tick || a.seq - b.seq);

  const chapters: Chapter[] = [];
  // Chương "cuộc đi" đang mở của mỗi `(kind, actor)`. Chỉ `core.entity.moved`
  // đi qua nhánh gộp — đúng yêu cầu thiết kế: một lời nói, một lần nhặt, một
  // ý định đều đáng đọc riêng từng cái, không nên trôi mất vào một con số gộp.
  const openRuns = new Map<string, Chapter>();

  for (const e of ordered) {
    if (e.kind === "core.entity.moved" && e.actor !== null) {
      const runKey = `${e.kind} ${e.actor}`;
      const who = nameOf(e.actor, names);
      const open = openRuns.get(runKey);
      if (open !== undefined && e.tick - open.to <= window) {
        open.to = e.tick;
        open.day = dayOf(open.to, DEFAULT_TICKS_PER_DAY);
        open.count += 1;
        open.seq = e.seq;
        open.slots.count = open.count;
        const x = numberAt(e.payload, "x");
        const y = numberAt(e.payload, "y");
        if (x !== null) open.slots.x = x;
        if (y !== null) open.slots.y = y;
        continue;
      }
      const chapter = openJourney(e, who);
      chapters.push(chapter);
      openRuns.set(runKey, chapter);
      continue;
    }
    chapters.push(buildChapter(e, names));
  }

  // Mới nhất trước, theo `to` (mốc gần nhất chương này còn bao trùm); `seq`
  // là chốt phụ để hai chương cùng chốt ở một tick vẫn có thứ tự xác định.
  chapters.sort((a, b) => b.to - a.to || b.seq - a.seq);

  return opts?.limit === undefined ? chapters : chapters.slice(0, opts.limit);
}

/** Đếm theo ngày, để vẽ một dải nhịp hoạt động. */
export function pulse(
  events: WorldEvent[],
  ticksPerDay: number = DEFAULT_TICKS_PER_DAY,
): { day: number; count: number }[] {
  const counts = new Map<number, number>();
  for (const e of events) {
    const day = dayOf(e.tick, ticksPerDay);
    counts.set(day, (counts.get(day) ?? 0) + 1);
  }
  // Tăng dần theo ngày: một dải nhịp đọc trái sang phải theo chiều thời gian.
  // Ngược hướng với `weave()` một cách có chủ đích — ở đó "mới nhất trước"
  // phục vụ đọc lướt tin mới, còn một dải nhịp chạy ngược thời gian thì trông
  // như một lỗi vẽ, không phải một lựa chọn.
  return [...counts.entries()].map(([day, count]) => ({ day, count })).sort((a, b) => a.day - b.day);
}
