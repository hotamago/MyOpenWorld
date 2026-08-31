/**
 * Web Worker mang: nhan WebSocket va giai ma **ngoai luong chinh** (`PA-05`).
 *
 * Vi sao trong worker chu khong o luong chinh: mot khung delta co the chua hang
 * nghin thuc the va vai chunk. Giai ma no tren luong chinh se lam rot khung
 * hinh dung luc the gioi dang chuyen dong nhieu nhat — tuc dung luc nguoi choi
 * chu y nhat.
 *
 * Worker cung la noi **toa do duoc doc thanh `bigint`** (`§22.10`). Lam o day
 * nghia la luong chinh khong bao gio cham vao chuoi toa do tho, nen khong co
 * cho nao de mot `Number()` vo tinh lot vao.
 */

import { decodeServer, encodeClient, type ClientMessage, type ServerMessage } from "@/api/protocol";

/** Thong diep tu luong chinh gui vao worker. */
type ToWorker =
  | { kind: "connect"; url: string }
  | { kind: "send"; message: ClientMessage }
  | { kind: "close" };

/** Thong diep tu worker gui ra. */
type FromWorker =
  | { kind: "open" }
  | { kind: "closed"; code: number }
  | { kind: "error"; detail: string }
  | { kind: "message"; message: ServerMessage };

let ws: WebSocket | null = null;

function gui(m: FromWorker): void {
  // `bigint` khong qua duoc structured clone trong moi trinh duyet cu, va
  // `ServerMessage` co chua no. Chuyen ve chuoi truoc khi gui ra.
  self.postMessage(JSON.parse(JSON.stringify(m, (_k, v) =>
    typeof v === "bigint" ? v.toString() : v)));
}

self.onmessage = (ev: MessageEvent<ToWorker>) => {
  const m = ev.data;
  switch (m.kind) {
    case "connect": {
      ws?.close();
      ws = new WebSocket(m.url);
      ws.onopen = () => gui({ kind: "open" });
      ws.onclose = (e) => gui({ kind: "closed", code: e.code });
      ws.onerror = () => gui({ kind: "error", detail: "loi WebSocket" });
      ws.onmessage = (e) => {
        try {
          gui({ kind: "message", message: decodeServer(String(e.data)) });
        } catch (err) {
          // Khong nuot: mot thong diep khong doc duoc nghia la giao thuc da
          // lech giua hai ben, va do la thu phai sua chu khong phai bo qua.
          gui({ kind: "error", detail: err instanceof Error ? err.message : String(err) });
        }
      };
      break;
    }
    case "send":
      ws?.send(encodeClient(m.message));
      break;
    case "close":
      ws?.close();
      ws = null;
      break;
  }
};

export type { FromWorker, ToWorker };
