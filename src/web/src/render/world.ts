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
import { ambientSprites, type AmbientSprite } from "./ambient";
import { drawFigure, figureOf } from "./figure";
import { paintTerrain, skyTint } from "./terrain";

/**
 * Trần số hạt môi trường mỗi khung nhìn.
 *
 * Đo trên khung 87×41: cảnh dày nhất (sa mạc, băng hà) sinh ~250 hạt, cảnh
 * thường ~120–170. 240 nghĩa là gần như không bao giờ tỉa ở cảnh thật, và chỉ
 * tỉa nhẹ ở hai chỗ dày nhất.
 */
const AMBIENT_BUDGET = 240;

/** Các mức phóng, tính bằng pixel một ô. Rời rạc để lưới luôn khớp pixel. */
export const ZOOM_STEPS = [4, 6, 9, 12, 18, 26, 36] as const;

export interface TileCoord {
  x: number;
  y: number;
}

/** Một thực thể sẽ đổi, như server mô tả trong diff xem trước. */
export interface DiffChange {
  id: string;
  name: string;
  from: [number, number] | null;
  to: [number, number] | null;
  moved: boolean;
  attrs: string[];
}

export class WorldView {
  private app: Application | null = null;
  private terrainLayer = new Container();
  /** Đường đi và ô đích. Nằm giữa nền và thực thể để không che nhân vật. */
  private pathLayer = new Container();
  /** Hạt môi trường. Dưới thực thể để không che nhân vật. */
  private ambientLayer = new Container();
  private overlayLayer = new Container();
  private overlaySprite: Sprite | null = null;
  private overlayTexture: Texture | null = null;
  /** Diff của một can thiệp đang xem trước. Trên cùng: nó là thứ cần đọc. */
  private diffLayer = new Container();
  private diffChanges: DiffChange[] = [];
  private entityLayer = new Container();
  /**
   * Vị trí ở lần vẽ trước của từng thực thể, theo `id` — dùng suy `facing`
   * trong `figureOf`. Dọn trong `setEntities` mỗi khi một `id` không còn xuất
   * hiện nữa, để map này không phình vô hạn suốt đời phiên chơi.
   */
  private prevPos = new Map<string, { x: number; y: number }>();
  private plannedPath: [number, number][] = [];
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
    app.stage.addChild(this.overlayLayer);
    app.stage.addChild(this.ambientLayer);
    app.stage.addChild(this.pathLayer);
    app.stage.addChild(this.entityLayer);
    app.stage.addChild(this.diffLayer);
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
    this.redrawAmbient(tick);
    this.redrawPath();
    this.redrawDiff();
    this.reposition();
  }

  /**
   * Vẽ lại lớp hạt môi trường.
   *
   * Một `Graphics` duy nhất cho cả lớp, không phải một sprite mỗi hạt: 240 hạt
   * thành 240 đối tượng Pixi là 240 lần thêm/xóa mỗi tick, và chi phí đó lớn
   * hơn hẳn chi phí vẽ chúng.
   */
  private redrawAmbient(tick: number): void {
    for (const c of this.ambientLayer.removeChildren()) c.destroy();
    const batch = this.batch;
    if (!batch) return;

    const ts = this.tileSize;
    // Dưới một ngưỡng phóng, hạt nhỏ hơn một pixel và chỉ thành nhiễu.
    if (ts < 9) return;

    const sprites: AmbientSprite[] = ambientSprites(batch, this.palette, tick, AMBIENT_BUDGET);
    const g = new Graphics();
    for (const p of sprites) {
      const cx = (p.x - batch.x) * ts;
      const cy = (p.y - batch.y) * ts;
      const r = Math.max(0.6, ts * 0.09 * p.scale);
      switch (p.kind) {
        case "foam":
          g.circle(cx, cy, r * 1.2).fill({ color: 0xdff0f7, alpha: p.alpha * 0.75 });
          break;
        case "ripple":
          g.circle(cx, cy, r * 1.6).stroke({ width: Math.max(0.5, ts * 0.03), color: 0xbfe0f2, alpha: p.alpha * 0.5 });
          break;
        case "dust":
          g.circle(cx, cy, r * 0.8).fill({ color: 0xe6d9b8, alpha: p.alpha * 0.4 });
          break;
        case "sparkle":
          // Chữ thập nhỏ, không phải chấm: lấp lánh đọc ra là "ánh sáng" chứ
          // không phải "một hạt bụi sáng màu".
          g.moveTo(cx - r * 1.6, cy).lineTo(cx + r * 1.6, cy)
            .moveTo(cx, cy - r * 1.6).lineTo(cx, cy + r * 1.6)
            .stroke({ width: Math.max(0.5, ts * 0.035), color: 0xffffff, alpha: p.alpha * 0.8 });
          break;
      }
    }
    this.ambientLayer.addChild(g);
  }

  /**
   * Vẽ lại lớp thực thể.
   *
   * Hình dạng — thân, đầu, dụng cụ, dấu ý định — tính ở `figure.ts`, hàm
   * thuần và có test riêng. Ở đây chỉ còn việc của một nơi vẽ: cắt theo khung
   * nhìn, sắp thứ tự chồng lớp, và nhớ vị trí trước để suy hướng nhìn.
   */
  setEntities(entities: Entity[]): void {
    for (const c of this.entityLayer.removeChildren()) c.destroy();

    // Dọn vị trí của thực thể đã biến mất — chết, rời server, hay bị gộp vào
    // một sự kiện khác. Không dọn thì `prevPos` phình vô hạn suốt đời phiên
    // chơi, dù bản đồ tại một thời điểm chỉ có vài chục thực thể.
    const stillHere = new Set(entities.map((e) => e.id));
    for (const id of this.prevPos.keys()) {
      if (!stillHere.has(id)) this.prevPos.delete(id);
    }

    const batch = this.batch;
    if (!batch) return;

    const ts = this.tileSize;
    // Sắp theo `y` rồi `x`: ai đứng thấp hơn trên màn hình thì vẽ sau, nên
    // chồng lên. Không sắp thì thứ tự vẽ theo thứ tự mảng và hai thực thể cạnh
    // nhau nhấp nháy qua lại mỗi tick.
    const sorted = [...entities].sort((p, q) => p.y - q.y || p.x - q.x);

    for (const e of sorted) {
      const prev = this.prevPos.get(e.id);
      // Ghi trước khi cắt theo khung nhìn: vị trí thật của thực thể vẫn đổi dù
      // nó đang ở ngoài màn hình, và bỏ qua bước này sẽ làm nó luôn "quay mặt
      // về phải" đúng lúc vừa xuất hiện lại — sai ngay cú vẽ đầu tiên.
      this.prevPos.set(e.id, { x: e.x, y: e.y });

      const gx = e.x - batch.x;
      const gy = e.y - batch.y;
      if (gx < 0 || gy < 0 || gx >= batch.w || gy >= batch.h) continue;

      const g = new Graphics();
      const cx = gx * ts + ts / 2;
      const cy = gy * ts + ts / 2;
      drawFigure(g, figureOf(e, prev), cx, cy, ts);
      this.entityLayer.addChild(g);
    }
    this.reposition();
  }

  /**
   * Đặt đường đi đang theo, để vẽ ra.
   *
   * Đây là phản hồi cho câu hỏi *"nó có hiểu tôi bấm vào đâu không"*. Không có
   * nó, một cú bấm vào chỗ không tới được và một cú bấm chưa kịp xử lý trông
   * giống hệt nhau — và người chơi bấm lại lần nữa.
   */
  setPath(path: [number, number][]): void {
    this.plannedPath = path;
    this.redrawPath();
  }

  private redrawPath(): void {
    for (const c of this.pathLayer.removeChildren()) c.destroy();
    const batch = this.batch;
    if (!batch || this.plannedPath.length === 0) return;

    const ts = this.tileSize;
    const g = new Graphics();
    const last = this.plannedPath.length - 1;

    for (const [i, [wx, wy]] of this.plannedPath.entries()) {
      const gx = wx - batch.x;
      const gy = wy - batch.y;
      if (gx < 0 || gy < 0 || gx >= batch.w || gy >= batch.h) continue;
      const cx = gx * ts + ts / 2;
      const cy = gy * ts + ts / 2;

      if (i === last) {
        // Ô đích: một khung, không phải một chấm. Khung đọc được ở mọi mức
        // phóng còn chấm thì biến mất khi thu nhỏ.
        const h = ts * 0.42;
        g.rect(cx - h, cy - h, h * 2, h * 2).stroke({ width: Math.max(1.5, ts * 0.1), color: 0x9ec1ff });
      } else {
        // Chấm mờ dần về phía đích: mắt đọc được **chiều** đi mà không cần mũi tên.
        const k = 1 - (i / Math.max(1, last)) * 0.55;
        g.circle(cx, cy, Math.max(1, ts * 0.1)).fill({ color: 0x9ec1ff, alpha: 0.28 * k + 0.2 });
      }
    }
    this.pathLayer.addChild(g);
  }

  /**
   * Đặt diff đang xem trước, để vẽ lên bản đồ.
   *
   * `§18.12` và lời khuyên rõ nhất tôi nhận được về console này: diff chính
   * phải nằm **trên bản đồ**, không phải trong một bảng chữ. Người chơi cần
   * thấy "ô nào đổi, ai chết" trong ba giây, và một danh sách văn bản buộc họ
   * đọc để tìm ra hậu quả.
   *
   * Ba màu, và mỗi màu đi kèm một **hình dạng** khác nhau: `§18.6` cấm để màu
   * làm kênh duy nhất mang thông tin.
   */
  setDiff(changes: DiffChange[]): void {
    this.diffChanges = changes;
    this.redrawDiff();
  }

  private redrawDiff(): void {
    for (const c of this.diffLayer.removeChildren()) c.destroy();
    const batch = this.batch;
    if (!batch || this.diffChanges.length === 0) return;

    const ts = this.tileSize;
    const g = new Graphics();
    const at = (p: [number, number]) => ({
      x: (p[0] - batch.x) * ts + ts / 2,
      y: (p[1] - batch.y) * ts + ts / 2,
    });
    const inside = (p: [number, number]) =>
      p[0] >= batch.x && p[1] >= batch.y && p[0] < batch.x + batch.w && p[1] < batch.y + batch.h;

    for (const c of this.diffChanges) {
      // Biến mất: khung đỏ tím gạch chéo. Đây là hậu quả nặng nhất, nên nó có
      // hình dạng riêng chứ không chỉ màu riêng.
      if (c.from && !c.to && inside(c.from)) {
        const p = at(c.from);
        const h = ts * 0.45;
        g.rect(p.x - h, p.y - h, h * 2, h * 2).stroke({ width: Math.max(1.5, ts * 0.1), color: 0xc35b8a });
        g.moveTo(p.x - h, p.y - h).lineTo(p.x + h, p.y + h)
          .moveTo(p.x + h, p.y - h).lineTo(p.x - h, p.y + h)
          .stroke({ width: Math.max(1, ts * 0.07), color: 0xc35b8a });
        continue;
      }
      // Xuất hiện: khung lam nét liền.
      if (!c.from && c.to && inside(c.to)) {
        const p = at(c.to);
        const h = ts * 0.45;
        g.rect(p.x - h, p.y - h, h * 2, h * 2).stroke({ width: Math.max(1.5, ts * 0.1), color: 0x6fa8dc });
        continue;
      }
      // Dịch chuyển: mũi tên từ chỗ cũ tới chỗ mới.
      if (c.moved && c.from && c.to) {
        if (inside(c.from) && inside(c.to)) {
          const a = at(c.from);
          const b = at(c.to);
          g.moveTo(a.x, a.y).lineTo(b.x, b.y).stroke({ width: Math.max(1.5, ts * 0.09), color: 0xe0a04a });
          g.circle(b.x, b.y, ts * 0.16).fill(0xe0a04a);
        }
        continue;
      }
      // Đổi thuộc tính mà không dịch chuyển: vòng hổ phách quanh ô.
      const p0 = c.to ?? c.from;
      if (p0 && inside(p0)) {
        const p = at(p0);
        g.circle(p.x, p.y, ts * 0.5).stroke({ width: Math.max(1.5, ts * 0.09), color: 0xe0a04a });
      }
    }
    this.diffLayer.addChild(g);
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

  /**
   * Phủ một lớp dữ liệu lên nền, hoặc gỡ nó ra khi truyền `null`.
   *
   * Lớp riêng chứ không trộn vào texture nền, vì hai lý do. Thứ nhất `§18.5`:
   * màu nền của một ô mang **vật liệu và chỉ vật liệu**; trộn cao độ vào đó là
   * làm một kênh mang hai nghĩa, và người chơi hết cách biết cái nào đang nói.
   * Thứ hai: đổi lớp dữ liệu không nên buộc vẽ lại địa hình, vốn là phần đắt.
   *
   * Đặt **dưới** lớp ánh sáng môi trường có chủ ý: dữ liệu vẫn phải chịu ngày
   * đêm như mọi thứ khác, nếu không thì ban đêm nó là thứ duy nhất sáng và mắt
   * không rời được nó.
   */
  setOverlay(rgba: Uint8ClampedArray | null): void {
    const batch = this.batch;
    if (!batch || !rgba) {
      this.overlaySprite?.destroy();
      this.overlaySprite = null;
      this.overlayTexture?.destroy(true);
      this.overlayTexture = null;
      this.overlayLayer.removeChildren();
      return;
    }

    const cv = document.createElement("canvas");
    cv.width = batch.w;
    cv.height = batch.h;
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    const img = ctx.createImageData(batch.w, batch.h);
    img.data.set(rgba);
    ctx.putImageData(img, 0, 0);

    this.overlayTexture?.destroy(true);
    this.overlayTexture = Texture.from(cv);
    this.overlayTexture.source.scaleMode = "nearest";
    if (!this.overlaySprite) {
      this.overlaySprite = new Sprite(this.overlayTexture);
      this.overlayLayer.addChild(this.overlaySprite);
    } else {
      this.overlaySprite.texture = this.overlayTexture;
    }
    this.overlaySprite.width = batch.w * this.tileSize;
    this.overlaySprite.height = batch.h * this.tileSize;
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
    this.overlayLayer.position.set(x, y);
    this.ambientLayer.position.set(x, y);
    this.pathLayer.position.set(x, y);
    this.entityLayer.position.set(x, y);
    this.diffLayer.position.set(x, y);
  }

  destroy(): void {
    this.overlayTexture?.destroy(true);
    this.terrainTexture?.destroy(true);
    this.app?.destroy(true, { children: true });
    this.app = null;
  }
}
