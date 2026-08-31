/**
 * Camera với floating origin (`§18.4`, `PA-06`).
 *
 * Camera giữ vị trí ở `bigint` và cung cấp phép biến đổi sang tọa độ màn hình.
 * Phần khó nằm ở [`FloatingOrigin`], đã có test riêng; ở đây chỉ là phần ghép.
 */

import { FloatingOrigin, type WorldPoint } from "@/worker/coord";

/** Giới hạn zoom, tính bằng pixel trên mỗi ô. */
export const MIN_ZOOM = 1;
export const MAX_ZOOM = 64;

/** Trạng thái camera. */
export class Camera {
  /** Vị trí trong thế giới. */
  at: WorldPoint = { x: 0n, y: 0n, z: 0 };
  /** Số pixel trên mỗi ô. */
  zoom = 16;
  /** Kích thước khung nhìn, pixel. */
  viewportWidth = 1;
  viewportHeight = 1;

  readonly origin = new FloatingOrigin();

  /**
   * Dời camera một quãng tính bằng **pixel màn hình**.
   *
   * Nhận pixel chứ không nhận ô, vì đó là thứ chuột cung cấp. Quy đổi ở đây,
   * một chỗ, thay vì ở mỗi chỗ xử lý input — nếu không, một lần đổi công thức
   * zoom sẽ phải sửa năm chỗ và sẽ sót một.
   */
  panByPixels(dxPx: number, dyPx: number): void {
    this.at = {
      x: this.at.x - BigInt(Math.round(dxPx / this.zoom)),
      y: this.at.y - BigInt(Math.round(dyPx / this.zoom)),
      z: this.at.z,
    };
  }

  /**
   * Zoom quanh một điểm màn hình, giữ ô dưới con trỏ đứng yên.
   *
   * Giữ ô đứng yên là thứ khiến zoom "cảm thấy đúng". Zoom quanh tâm màn hình
   * thì rẻ hơn nhưng người dùng sẽ phải kéo lại sau mỗi lần cuộn.
   */
  zoomAt(factor: number, screenX: number, screenY: number): void {
    const truoc = this.screenToWorld(screenX, screenY);
    this.zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, this.zoom * factor));
    const sau = this.screenToWorld(screenX, screenY);
    this.at = {
      x: this.at.x + (truoc.x - sau.x),
      y: this.at.y + (truoc.y - sau.y),
      z: this.at.z,
    };
  }

  /** Đổi tọa độ màn hình sang tọa độ thế giới. */
  screenToWorld(px: number, py: number): WorldPoint {
    const dx = Math.floor((px - this.viewportWidth / 2) / this.zoom);
    const dy = Math.floor((py - this.viewportHeight / 2) / this.zoom);
    return { x: this.at.x + BigInt(dx), y: this.at.y + BigInt(dy), z: this.at.z };
  }

  /**
   * Đổi tọa độ thế giới sang tọa độ màn hình.
   *
   * Đi qua [`FloatingOrigin`], nên nó ném lỗi khi điểm nằm quá xa — thay vì
   * trả về một số vô nghĩa mà renderer sẽ vẽ ở đâu đó ngẫu nhiên.
   */
  worldToScreen(p: WorldPoint): { x: number; y: number } {
    const l = this.origin.toLocal(p);
    const c = this.origin.toLocal(this.at);
    return {
      x: (l.x - c.x) * this.zoom + this.viewportWidth / 2,
      y: (l.y - c.y) * this.zoom + this.viewportHeight / 2,
    };
  }

  /** Vùng thế giới đang nhìn thấy, kèm một viền đệm. */
  visibleBounds(padCells = 2): { min: WorldPoint; max: WorldPoint } {
    const nua_w = BigInt(Math.ceil(this.viewportWidth / this.zoom / 2) + padCells);
    const nua_h = BigInt(Math.ceil(this.viewportHeight / this.zoom / 2) + padCells);
    return {
      min: { x: this.at.x - nua_w, y: this.at.y - nua_h, z: this.at.z },
      max: { x: this.at.x + nua_w, y: this.at.y + nua_h, z: this.at.z },
    };
  }

  /**
   * Đồng bộ gốc trôi với vị trí camera.
   *
   * Trả `true` nếu gốc đã dời — renderer phải vẽ lại **toàn bộ**, vì mọi vị trí
   * đã lưu trong bộ đệm đều tính theo gốc cũ.
   */
  syncOrigin(): boolean {
    return this.origin.recenterIfNeeded(this.at);
  }
}
