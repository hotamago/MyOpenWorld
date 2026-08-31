/**
 * Kênh WebSocket tới thế giới.
 *
 * ## Vì sao không hỏi lại mỗi 400 ms nữa
 *
 * Thế giới chạy theo tick và **nó** biết khi nào mình đổi. Bắt client hỏi lại
 * theo nhịp riêng là sai ở cả hai đầu: hỏi quá thưa thì màn hình hiện một quá
 * khứ, hỏi quá dày thì phần lớn câu trả lời giống hệt câu trước. Cả hai đều
 * tốn, và không cái nào đúng.
 *
 * Server đẩy một khung `state` mỗi khi có gì đổi, và **bỏ qua** khung tin khi
 * `state_hash` không đổi và không có sự kiện mới — nên một thế giới đang tạm
 * dừng không tốn một byte nào.
 *
 * ## Cái gì **không** đi qua đây
 *
 * Lô ô (`/api/tiles`). Nó lớn, nó chỉ đổi khi khung nhìn dời, và nó là một câu
 * hỏi có câu trả lời — đúng hình dạng của một `GET`. Đẩy nó theo nhịp là gửi
 * hàng trăm KB cho một thứ không đổi. Xem `api.tiles`.
 *
 * ## Đường lui
 *
 * Nối hỏng thì [`connect`] gọi `onDown`, và chỗ gọi quay về hỏi lại bằng HTTP.
 * Một trò chơi không chạy được vì WebSocket bị một proxy chặn là một trò chơi
 * hỏng vì lý do không liên quan gì tới nó.
 */

import type { Entity, WorldEvent, WorldMeta } from "./game";

/** Khung tin server đẩy xuống mỗi khi thế giới đổi. */
export interface StateFrame {
  meta: WorldMeta;
  entities: Entity[];
  /** Chỉ những sự kiện **mới** kể từ khung trước; server nhớ con trỏ cho mỗi kết nối. */
  events: WorldEvent[];
}

/** Những gì chỗ gọi muốn nghe. */
export interface SocketHandlers {
  onHello?: (meta: WorldMeta) => void;
  onState: (frame: StateFrame) => void;
  /** Kênh đứt hoặc không nối được. Chỗ gọi nên bật đường lui HTTP. */
  onDown?: (reason: string) => void;
  /** Kênh đã nối lại được sau khi đứt. */
  onUp?: () => void;
}

/** Tay cầm để đóng kênh và gửi lệnh. */
export interface Socket {
  /** Gửi một lệnh và đợi `ack`. Cùng hình dạng như `POST` cùng đường dẫn. */
  request<T>(path: string, body: unknown): Promise<T>;
  /** Kênh đang mở không. */
  alive(): boolean;
  close(): void;
}

/** Trần thời gian chờ một `ack`, tính bằng mili giây. */
const ACK_TIMEOUT_MS = 8_000;

/**
 * Khoảng cách giữa hai lần thử nối lại, tăng dần.
 *
 * Thử lại ngay lập tức và mãi mãi là cách biến một server đang khởi động lại
 * thành một trận bão yêu cầu. Tăng dần rồi dừng ở 5 giây: đủ thưa để không làm
 * phiền, đủ dày để người chơi không phải tải lại trang.
 */
const BACKOFF_MS = [250, 500, 1_000, 2_500, 5_000] as const;

interface Pending {
  resolve: (v: unknown) => void;
  reject: (e: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

/**
 * Mở kênh tới `origin` và giữ nó mở.
 *
 * `origin` là gốc HTTP (ví dụ `http://localhost:17777`, hoặc chuỗi rỗng khi
 * trang và server cùng gốc); hàm này tự đổi `http`→`ws`.
 */
export function connect(origin: string, h: SocketHandlers): Socket {
  const url = toWs(origin);
  let ws: WebSocket | null = null;
  let closed = false;
  let attempt = 0;
  let nextId = 1;
  const pending = new Map<number, Pending>();

  function open(): void {
    if (closed) return;
    let sock: WebSocket;
    try {
      sock = new WebSocket(url);
    } catch (e) {
      fail(e instanceof Error ? e.message : String(e));
      return;
    }
    ws = sock;

    sock.onopen = () => {
      // Chỉ đặt lại bộ đếm **sau khi** đã mở thật. Đặt lại lúc bắt đầu thử sẽ
      // biến một server từ chối liên tục thành một vòng lặp 250 ms.
      attempt = 0;
      h.onUp?.();
    };

    sock.onmessage = (ev) => {
      if (typeof ev.data !== "string") return;
      let msg: unknown;
      try {
        msg = JSON.parse(ev.data);
      } catch {
        // Một khung hỏng không đáng để đóng cả kênh: khung sau vẫn có thể tốt.
        return;
      }
      dispatch(msg);
    };

    sock.onclose = () => fail("kênh đã đóng");
    sock.onerror = () => fail("lỗi kênh");
  }

  function dispatch(msg: unknown): void {
    if (typeof msg !== "object" || msg === null) return;
    const m = msg as Record<string, unknown>;
    switch (m["t"]) {
      case "hello":
        h.onHello?.(m["meta"] as WorldMeta);
        break;
      case "state":
        h.onState({
          meta: m["meta"] as WorldMeta,
          entities: (m["entities"] as Entity[]) ?? [],
          events: (m["events"] as WorldEvent[]) ?? [],
        });
        break;
      case "ack": {
        const id = typeof m["id"] === "number" ? m["id"] : -1;
        const p = pending.get(id);
        if (!p) return;
        pending.delete(id);
        clearTimeout(p.timer);
        if (m["ok"] === true) p.resolve(m["body"] ?? {});
        else p.reject(new Error(String(m["error"] ?? "lệnh bị từ chối")));
        break;
      }
      default:
        break;
    }
  }

  function fail(reason: string): void {
    if (closed) return;
    ws = null;
    // Mọi lệnh đang chờ sẽ không bao giờ có `ack`. Báo hỏng ngay thay vì để
    // chúng treo tới khi hết giờ: chỗ gọi cần biết để thử lại bằng HTTP.
    for (const [, p] of pending) {
      clearTimeout(p.timer);
      p.reject(new Error(reason));
    }
    pending.clear();
    h.onDown?.(reason);
    const wait = BACKOFF_MS[Math.min(attempt, BACKOFF_MS.length - 1)] ?? 5_000;
    attempt += 1;
    setTimeout(open, wait);
  }

  open();

  return {
    request<T>(path: string, body: unknown): Promise<T> {
      const sock = ws;
      if (!sock || sock.readyState !== WebSocket.OPEN) {
        return Promise.reject(new Error("kênh chưa mở"));
      }
      const id = nextId++;
      return new Promise<T>((resolve, reject) => {
        const timer = setTimeout(() => {
          pending.delete(id);
          reject(new Error(`${path}: quá hạn chờ`));
        }, ACK_TIMEOUT_MS);
        pending.set(id, {
          resolve: resolve as (v: unknown) => void,
          reject,
          timer,
        });
        sock.send(JSON.stringify({ t: "cmd", id, path, body }));
      });
    },
    alive: () => ws !== null && ws.readyState === WebSocket.OPEN,
    close(): void {
      closed = true;
      for (const [, p] of pending) clearTimeout(p.timer);
      pending.clear();
      ws?.close();
      ws = null;
    },
  };
}

/**
 * Đổi một gốc HTTP thành gốc WebSocket.
 *
 * Gốc rỗng nghĩa là "cùng gốc với trang": lấy từ `location`. Viết tường minh
 * thay vì dựa vào đường dẫn tương đối, vì `new WebSocket("/ws")` không hợp lệ.
 */
export function toWs(origin: string): string {
  const base = origin || globalThis.location?.origin || "http://localhost";
  return `${base.replace(/^http/, "ws")}/ws`;
}
