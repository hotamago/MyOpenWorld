/**
 * Vẽ thế giới: nền là một texture, thực thể là sprite (`§18.4`, `§P6.9.2`).
 *
 * ## Một texture, không phải 3500 hình chữ nhật
 *
 * Cách hiển nhiên là vẽ mỗi ô một `Graphics.rect()`. Nó chạy, và nó hỏng đúng
 * lúc cần chạy: một khung nhìn 87×41 là hơn 3500 lệnh vẽ mỗi lần dựng lại.
 *
 * Ở đây một ô là **một pixel** trong một texture, rồi phóng to bằng `nearest`.
 * Cả bản đồ là một sprite: một lệnh vẽ, không phụ thuộc số ô, và phóng to không
 * tốn gì thêm. Đây đúng là "chunk texture" mà `§18.4` mô tả.
 *
 * ## Ba lớp, ba nhịp đổi khác nhau
 *
 * | Lớp | Đổi khi | Chi phí dựng lại |
 * |---|---|---|
 * | nền | camera qua ranh giới ô, đổi lát `z` | một texture |
 * | thực thể | mỗi tick | vài chục `Graphics` |
 * | nhãn | khi tên/vị trí đổi | DOM, ngoài canvas |
 *
 * Gộp chúng nghĩa là dựng lại cả bản đồ mỗi khi một NPC nhấc chân.
 *
 * ## Nhãn nằm ở HTML, không phải `PIXI.Text`
 *
 * `§P6.9.2` nói rõ: `PIXI.Text` tạo một texture cho **mỗi chuỗi**, nên một bản
 * đồ vài trăm nhãn ăn hết bộ nhớ texture. Nhãn ở đây là `<div>` tuyệt đối phía
 * trên canvas — chọn được, đọc được bằng trình đọc màn hình, và không phá batch.
 *
 * ## Không nội suy vị trí
 *
 * Thực thể nhảy từ ô sang ô, không trượt mượt. Trượt mượt đòi client đoán vị trí
 * giữa hai tick, và đoán là một dạng optimistic UI mà `§P6.9.5` cấm: màn hình sẽ
 * hiện một thế giới mà engine chưa bao giờ ở trong đó.
 */

import { Application, Container, Graphics, Sprite, Texture } from "pixi.js";
import type { Entity, TileBatch } from "@/api/game";
import type { BlockPalette } from "./blocks";
import { paintTerrain, skyTint } from "./terrain";

/** Các mức phóng, tính bằng pixel một ô. Rời rạc để lưới luôn khớp pixel. */
export const ZOOM_STEPS = [4, 6, 9, 12, 18, 26, 36] as const;

export interface TileCoord {
  x: number;
  y: number;
}

export class WorldView {
  private app: Application | null = null;
  private terrainLayer = new Container();
  private entityLayer = new Container();
  private terrainSprite: Sprite | null = null;
  private terrainTexture: Texture | null = null;
  private batch: TileBatch | null = null;
  private centerX = 0;
  private centerY = 0;
  private zoomIndex = 4;

  constructor(private palette: BlockPalette) {}

  async attach(canvas: HTMLCanvasElement): Promise<void> {
    const app = new Application();
    const parent = canvas.parentElement;
    await app.init({
      canvas,
      preference: "webgl",
      antialias: false,
      // `autoDensity` + `resolution` giữ nét trên màn hình HiDPI. Thiếu nó thì
      // texture `nearest` bị mờ đúng ở chỗ nó cần sắc nhất.
      resolution: globalThis.devicePixelRatio ?? 1,
      autoDensity: true,
      background: 0x070910,
      ...(parent ? { resizeTo: parent } : {}),
    });
    app.stage.addChild(this.terrainLayer);
    app.stage.addChild(this.entityLayer);
    this.app = app;
  }

  setPalette(p: BlockPalette): void {
    this.palette = p;
  }

  /** Pixel một ô ở mức phóng hiện tại. */
  get tileSize(): number {
    return ZOOM_STEPS[this.zoomIndex] ?? 18;
  }

  /** Đổi mức phóng. Trả về `true` nếu có đổi thật. */
  zoom(delta: number): boolean {
    const next = Math.min(ZOOM_STEPS.length - 1, Math.max(0, this.zoomIndex + delta));
    if (next === this.zoomIndex) return false;
    this.zoomIndex = next;
    return true;
  }

  /** Kích thước khung nhìn tính bằng ô, cộng một viền để không lộ mép. */
  viewportTiles(): { w: number; h: number } {
    const app = this.app;
    if (!app) return { w: 33, h: 33 };
    const px = app.renderer.resolution * this.tileSize;
    return {
      w: Math.max(9, Math.ceil(app.renderer.width / px) + 2),
      h: Math.max(9, Math.ceil(app.renderer.height / px) + 2),
    };
  }

  setCenter(x: number, y: number): void {
    this.centerX = x;
    this.centerY = y;
    this.reposition();
  }

  /** Nạp một lô ô và dựng lại texture nền. */
  setTerrain(batch: TileBatch, tick: number): void {
    this.batch = batch;
    const cv = document.createElement("canvas");
    cv.width = batch.w;
    cv.height = batch.h;
    const ctx = cv.getContext("2d");
    if (!ctx) return;

    const rgba = paintTerrain(batch, this.palette);
    // `ctx.createImageData` rồi `set` thay vì `new ImageData(rgba, w, h)`: kiểu
    // `ImageDataArray` của lib DOM không nhận thẳng `Uint8ClampedArray` ở mọi
    // phiên bản TypeScript, và ép kiểu ở đây sẽ giấu mất một lỗi thật nếu sau
    // này buffer đổi kiểu.
    const img = ctx.createImageData(batch.w, batch.h);
    img.data.set(rgba);
    ctx.putImageData(img, 0, 0);

    this.terrainTexture?.destroy(true);
    this.terrainTexture = Texture.from(cv);
    this.terrainTexture.source.scaleMode = "nearest";

    if (!this.terrainSprite) {
      this.terrainSprite = new Sprite(this.terrainTexture);
      this.terrainLayer.addChild(this.terrainSprite);
    } else {
      this.terrainSprite.texture = this.terrainTexture;
    }
    this.terrainSprite.width = batch.w * this.tileSize;
    this.terrainSprite.height = batch.h * this.tileSize;
    // Ngày đêm áp bằng `tint` trên một sprite duy nhất: không phải vẽ lại
    // texture, và nó cũng không chạm vào màu vật liệu đã bake trong đó.
    this.terrainSprite.tint = skyTint(tick);
    this.reposition();
  }

  /** Vẽ lại lớp thực thể. */
  setEntities(entities: Entity[]): void {
    for (const c of this.entityLayer.removeChildren()) c.destroy();
    const batch = this.batch;
    if (!batch) return;

    const ts = this.tileSize;
    // Sắp theo `y` rồi `x`: ai đứng thấp hơn trên màn hình thì vẽ sau, nên
    // chồng lên. Không sắp thì thứ tự vẽ theo thứ tự mảng và hai thực thể cạnh
    // nhau nhấp nháy qua lại mỗi tick.
    const sorted = [...entities].sort((p, q) => p.y - q.y || p.x - q.x);

    for (const e of sorted) {
      const gx = e.x - batch.x;
      const gy = e.y - batch.y;
      if (gx < 0 || gy < 0 || gx >= batch.w || gy >= batch.h) continue;

      const g = new Graphics();
      const cx = gx * ts + ts / 2;
      const cy = gy * ts + ts / 2;

      // Bóng tiếp đất: một ellipse mờ dưới chân. Không có nó thì mọi thứ trông
      // như dán lên bản đồ chứ không đứng trên nó.
      g.ellipse(cx, cy + ts * 0.3, ts * 0.32, ts * 0.12).fill({ color: 0x000000, alpha: 0.35 });

      if (e.kind === "item") {
        // Hình thoi cho vật phẩm: khác **hình dạng**, không chỉ khác màu —
        // `§18.6` cấm để màu làm kênh duy nhất mang thông tin.
        const r = ts * 0.26;
        g.poly([cx, cy - r, cx + r, cy, cx, cy + r, cx - r, cy])
          .fill(0xf0c674)
          .stroke({ width: Math.max(1, ts * 0.06), color: 0x2a2110 });
      } else if (e.is_avatar) {
        g.circle(cx, cy, ts * 0.34)
          .fill(0xf5f7fa)
          .stroke({ width: Math.max(1.5, ts * 0.09), color: 0x2b6cb0 });
        // Một chấm nhỏ ở tâm: ở mức phóng nhỏ nhất, viền và thân nhòe vào nhau
        // và avatar biến mất giữa đám NPC.
        g.circle(cx, cy, Math.max(1, ts * 0.1)).fill(0x2b6cb0);
      } else {
        g.circle(cx, cy, ts * 0.29)
          .fill(0xd08770)
          .stroke({ width: Math.max(1, ts * 0.06), color: 0x3a1f16 });
      }
      this.entityLayer.addChild(g);
    }
    this.reposition();
  }

  /** Ô nằm dưới một điểm trên canvas. */
  tileAt(px: number, py: number): TileCoord | null {
    const batch = this.batch;
    if (!batch) return null;
    const ts = this.tileSize;
    const gx = Math.floor((px - this.terrainLayer.x) / ts);
    const gy = Math.floor((py - this.terrainLayer.y) / ts);
    if (gx < 0 || gy < 0 || gx >= batch.w || gy >= batch.h) return null;
    return { x: batch.x + gx, y: batch.y + gy };
  }

  /** Vị trí trên canvas của một ô thế giới — dùng đặt nhãn HTML. */
  screenOf(x: number, y: number): { left: number; top: number } | null {
    const batch = this.batch;
    if (!batch) return null;
    const gx = x - batch.x;
    const gy = y - batch.y;
    if (gx < 0 || gy < 0 || gx >= batch.w || gy >= batch.h) return null;
    const ts = this.tileSize;
    return {
      left: this.terrainLayer.x + gx * ts + ts / 2,
      top: this.terrainLayer.y + gy * ts - ts * 0.45,
    };
  }

  /** Chỉ số trong lô của một ô thế giới, hoặc `-1`. */
  indexOf(x: number, y: number): number {
    const batch = this.batch;
    if (!batch) return -1;
    const gx = x - batch.x;
    const gy = y - batch.y;
    if (gx < 0 || gy < 0 || gx >= batch.w || gy >= batch.h) return -1;
    return gy * batch.w + gx;
  }

  currentBatch(): TileBatch | null {
    return this.batch;
  }

  /** Đặt hai lớp sao cho ô tâm nằm giữa màn hình. */
  private reposition(): void {
    const app = this.app;
    const batch = this.batch;
    if (!app || !batch) return;
    const ts = this.tileSize;
    const w = app.renderer.width / app.renderer.resolution;
    const h = app.renderer.height / app.renderer.resolution;
    // Làm tròn về pixel nguyên: một sprite `nearest` đặt ở tọa độ lẻ sẽ có
    // hàng pixel bị nhân đôi, và lưới trông méo.
    const x = Math.round(w / 2 - (this.centerX - batch.x) * ts - ts / 2);
    const y = Math.round(h / 2 - (this.centerY - batch.y) * ts - ts / 2);
    this.terrainLayer.position.set(x, y);
    this.entityLayer.position.set(x, y);
  }

  destroy(): void {
    this.terrainTexture?.destroy(true);
    this.app?.destroy(true, { children: true });
    this.app = null;
  }
}
