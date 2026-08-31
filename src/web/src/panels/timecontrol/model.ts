/**
 * Điều khiển thời gian (`idea.md §18.8`, `PB-18`).
 *
 * ## `pause-on-ready` là tính năng quan trọng nhất ở đây
 *
 * Một thế giới chạy liên tục ở tốc độ ×16 thì người chơi không kịp thấy gì. Một
 * thế giới dừng mỗi tick thì không đi tới đâu.
 *
 * `pause-on-ready` giải cả hai: chạy nhanh cho tới khi **một thứ đáng chú ý xảy
 * ra**, rồi dừng lại và nói ra đó là gì. Người chơi bỏ qua ba tháng yên bình
 * trong hai giây, và có mặt đúng lúc ngôi làng bốc cháy.
 *
 * ## Lý do dừng phải lấy từ **event có thật**
 *
 * `§22.17` cấm tường thuật thêm sự kiện không có trong nhật ký. Khi mô phỏng tự
 * dừng, giao diện phải nói *"dừng vì `crime.committed` tại tick 4820"* và bấm
 * được sang chuỗi nhân quả — chứ không phải *"có chuyện gì đó xảy ra"*.
 */

/** Tốc độ chạy. */
export const SPEEDS = [1, 4, 16, 64] as const;

/** Một tốc độ hợp lệ. */
export type Speed = (typeof SPEEDS)[number];

/** Trạng thái đồng hồ. */
export interface ClockState {
  paused: boolean;
  speed: Speed;
  tick: string;
  divineTick: string;
}

/** Một mốc khiến mô phỏng tự dừng. */
export interface StopMarker {
  /** Loại event làm dừng. */
  kind: string;
  /** Nhãn cho người đọc. */
  label: string;
  /** Có bật không. */
  enabled: boolean;
}

/** Bộ mốc mặc định. */
export function defaultMarkers(): StopMarker[] {
  return [
    { kind: "crime.committed", label: "có người phạm pháp", enabled: true },
    { kind: "life.died", label: "có người chết", enabled: true },
    { kind: "disease.outbreak", label: "dịch bùng phát", enabled: true },
    { kind: "war.declared", label: "chiến tranh nổ ra", enabled: true },
    { kind: "settlement.founded", label: "khu định cư mới", enabled: false },
    { kind: "knowledge.discovered", label: "phát kiến mới", enabled: false },
  ];
}

/** Vì sao mô phỏng dừng lại. */
export interface StopReason {
  /** Loại event. */
  kind: string;
  /** Nhãn đọc được. */
  label: string;
  /** Tick xảy ra. */
  tick: string;
  /** `seq` để bấm sang chuỗi nhân quả — **event có thật**. */
  seq: string;
}

/** Một event tối thiểu, đủ để quyết định có dừng không. */
export interface TickEvent {
  kind: string;
  tick: string;
  seq: string;
}

/**
 * Quyết định có dừng không sau khi tiến một tick.
 *
 * Trả `null` nếu không có gì đáng dừng. Trả về lý do **lấy từ event thật**, kèm
 * `seq` — không có đường nào tạo ra một `StopReason` mà không có event tương ứng.
 */
export function shouldStop(events: TickEvent[], markers: StopMarker[]): StopReason | null {
  const bat = new Map(markers.filter((m) => m.enabled).map((m) => [m.kind, m]));
  // Duyệt theo thứ tự `seq`: nếu hai mốc cùng kích hoạt trong một tick, cái nào
  // xảy ra trước thì báo cái đó. Thứ tự nhận được từ mạng không quyết định.
  const theo_seq = [...events].sort((a, b) => a.seq.localeCompare(b.seq, undefined, { numeric: true }));
  for (const e of theo_seq) {
    const m = bat.get(e.kind);
    if (m) {
      return { kind: e.kind, label: m.label, tick: e.tick, seq: e.seq };
    }
  }
  return null;
}

/** Một vị từ "chạy đến khi". */
export interface RunUntil {
  /** Biểu thức, cùng ngữ pháp với scenario DSL. */
  predicate: string;
  /** Trần số tick, để không chạy vô hạn. */
  maxTicks: number;
}

/** Kết quả một lần "chạy đến khi". */
export type RunUntilResult =
  | { outcome: "matched"; atTick: string; reason: StopReason | null }
  | { outcome: "marker"; reason: StopReason }
  /**
   * Hết trần mà vị từ chưa đúng.
   *
   * Đây là một kết quả **có nghĩa**, không phải một lỗi: nó nói rằng điều bạn
   * chờ đã không xảy ra trong khoảng thời gian đó, và đó thường là thông tin
   * quan trọng hơn cả việc nó xảy ra.
   */
  | { outcome: "timeout"; ticksRun: number };

/** Bộ điều khiển thời gian. */
export class TimeController {
  #state: ClockState = { paused: true, speed: 1, tick: "0", divineTick: "0" };
  #markers: StopMarker[] = defaultMarkers();
  #lastStop: StopReason | null = null;

  /** Trạng thái hiện tại. */
  get state(): Readonly<ClockState> {
    return this.#state;
  }

  /** Lý do dừng gần nhất. */
  get lastStop(): StopReason | null {
    return this.#lastStop;
  }

  /** Các mốc tự dừng. */
  get markers(): readonly StopMarker[] {
    return this.#markers;
  }

  /** Bật hoặc tắt một mốc. */
  toggleMarker(kind: string, enabled: boolean): void {
    const m = this.#markers.find((x) => x.kind === kind);
    if (m) m.enabled = enabled;
  }

  /** Tạm dừng. */
  pause(): void {
    this.#state.paused = true;
  }

  /** Chạy tiếp ở một tốc độ. */
  resume(speed: Speed = 1): void {
    this.#state.paused = false;
    this.#state.speed = speed;
    this.#lastStop = null;
  }

  /**
   * Tiến đúng một tick rồi dừng.
   *
   * Luôn dừng sau đó, kể cả khi đang chạy. `step` là một hành động có chủ đích
   * của người xem, và nó nghĩa là "tôi muốn nhìn kỹ".
   */
  step(): void {
    this.#state.paused = true;
  }

  /** Cập nhật đồng hồ và kiểm mốc dừng. */
  onTick(tick: string, divineTick: string, events: TickEvent[]): StopReason | null {
    this.#state.tick = tick;
    this.#state.divineTick = divineTick;
    if (this.#state.paused) return null;

    const ly_do = shouldStop(events, this.#markers);
    if (ly_do) {
      this.#state.paused = true;
      this.#lastStop = ly_do;
    }
    return ly_do;
  }

  /**
   * Câu giải thích cho người xem.
   *
   * Luôn có `seq`, nên giao diện luôn bấm được sang chuỗi nhân quả. Không có
   * biến thể nào trả về một câu chung chung.
   */
  explainStop(): string | null {
    const s = this.#lastStop;
    return s ? `Dừng tại t${s.tick}: ${s.label} (${s.kind} #${s.seq})` : null;
  }
}

/** Tốc độ kế tiếp trong vòng lặp, để gán một phím tắt. */
export function nextSpeed(current: Speed): Speed {
  const i = SPEEDS.indexOf(current);
  return SPEEDS[(i + 1) % SPEEDS.length]!;
}
