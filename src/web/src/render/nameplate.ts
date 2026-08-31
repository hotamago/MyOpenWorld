/**
 * Chọn nhãn tên nào nên hiện — thuần, không đụng Pixi (cùng ranh giới với
 * `figure.ts`, xem lời giải ở đó).
 *
 * ## Vì sao cần một hàm lọc riêng, không vẽ hết những gì `App.vue` gửi
 *
 * `World.setLabels` giờ nhận thẳng một mảng nhãn từ `App.vue` mỗi lần hỏi lại
 * thế giới. Vẽ nguyên xi mảng đó có hai cách hỏng:
 *
 * 1. **Chữ chồng chữ ở mức thu nhỏ.** Khi một ô chỉ còn vài pixel, tâm hai
 *    nhãn cạnh nhau gần như trùng lên nhau — kết quả không phải "nhiều tên",
 *    mà là một vệt xám không đọc được chữ nào, trong khi vẫn trả đủ giá vẽ
 *    của từng ấy đối tượng.
 * 2. **Một đám đông sinh ra hàng trăm `Text`.** Một lễ hội tụ tập 300 dân làng
 *    ở cùng một khoảng sân là 300 texture chữ tạo/hủy liên tục — đúng cái giá
 *    mà `world.ts` từng tránh bằng cách không dùng `PIXI.Text` (xem lời giải ở
 *    đầu file đó). Giới hạn số nhãn giữ cái giá đó có trần, bất kể thế giới
 *    đông đến đâu.
 *
 * ## Vì sao là hàm thuần
 *
 * "Nhãn nào đáng hiện" là một câu hỏi có thể trả lời chỉ bằng số — tọa độ, cỡ
 * ô, có phải đối tượng đang chọn hay không — không cần Pixi, không cần DOM,
 * không cần trình duyệt. Tách nó ra khỏi `world.ts` nghĩa là luật "ưu tiên
 * nhãn đang chọn" hay "cắt ở đúng 40 nhãn" là thứ `vitest` kiểm được thẳng
 * bằng số tay trong Node, không phải một khẳng định phải soi bằng mắt trên
 * canvas sau khi zoom ra zoom vào.
 */

/** Một nhãn tên trước khi lọc — đúng hình dạng `World.setLabels` nhận vào. */
export interface Nameplate {
  id: string;
  text: string;
  /** Toạ độ **ô** trong thế giới, không phải pixel màn hình. */
  x: number;
  y: number;
  /** Nhãn của đối tượng người chơi đang chọn — luôn được ưu tiên giữ lại. */
  highlight: boolean;
}

/**
 * Vùng ô đang tải, dùng làm "khung nhìn" để lọc thô.
 *
 * Chấp một hình chữ nhật ô (`x, y, w, h`) thay vì một cặp toạ độ màn hình vì
 * đó đúng là thứ `World` đã có sẵn (`TileBatch`, xem `api/game.ts`) và đúng là
 * ranh giới `setEntities` đã dùng để quyết định thực thể nào được vẽ. Dùng
 * chung một ranh giới nghĩa là nhãn của một thực thể **không hề được vẽ** thì
 * không bao giờ có nhãn trôi nổi một mình — hai lớp luôn đồng bộ theo cùng một
 * phép so sánh, không phải hai phép so sánh phải tự tay giữ cho khớp nhau.
 */
export interface Viewport {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface VisibleLabelsOptions {
  /** Pixel một ô ở mức phóng hiện tại — xem `World.tileSize`. */
  tileSize: number;
  viewport: Viewport;
  /** Ô tâm màn hình, dùng để đo "gần" khi phải cắt bớt. */
  centerX: number;
  centerY: number;
  /** Ngưỡng ẩn toàn bộ nhãn. Mặc định `DEFAULT_MIN_LABEL_TILE_SIZE`. */
  minTileSize?: number;
  /** Số nhãn tối đa còn lại sau khi cắt. Mặc định `DEFAULT_LABEL_LIMIT`. */
  limit?: number;
}

/**
 * Ngưỡng `tileSize` dưới đó nhãn bị ẩn hoàn toàn.
 *
 * Dưới mức này một ô rơi xuống vài pixel — cùng cỡ với khoảng cách giữa tâm
 * hai thực thể đứng cạnh nhau. Chữ ở cỡ đọc được (dù đã cố định theo màn hình,
 * xem `world.ts`) sẽ tràn lấn sang ô bên cạnh và chồng lên nhau. Ẩn hẳn ở đây
 * rẻ hơn — và trung thực hơn — so với vẽ ra một thứ không ai đọc nổi.
 */
export const DEFAULT_MIN_LABEL_TILE_SIZE = 9;

/**
 * Trần số nhãn hiện cùng lúc.
 *
 * 40 là một con số rộng rãi cho cảnh sinh hoạt thường ngày (một ngôi làng vài
 * chục dân) nhưng vẫn chặn được trường hợp xấu nhất — một lễ hội, một đám đông
 * tụ tập — nơi số thực thể trong khung nhìn có thể lên tới hàng trăm.
 */
export const DEFAULT_LABEL_LIMIT = 40;

/** `true` nếu `(x, y)` nằm trong `viewport`, biên trên/trái đóng, biên dưới/phải mở. */
function insideViewport(x: number, y: number, viewport: Viewport): boolean {
  const gx = x - viewport.x;
  const gy = y - viewport.y;
  return gx >= 0 && gy >= 0 && gx < viewport.w && gy < viewport.h;
}

/**
 * Khoảng cách bình phương tới tâm màn hình.
 *
 * Bình phương, không `Math.hypot`: thứ tự so sánh giữ nguyên mà không cần
 * `sqrt`, và hàm này có thể chạy trên hàng trăm nhãn mỗi lần `setLabels`.
 */
function distSqToCenter(l: Nameplate, centerX: number, centerY: number): number {
  const dx = l.x - centerX;
  const dy = l.y - centerY;
  return dx * dx + dy * dy;
}

/**
 * Nhãn nào nên hiện, theo thứ tự: lọc mức phóng → lọc khung nhìn → cắt số lượng.
 *
 * ## Vì sao ưu tiên `highlight` rồi mới tới khoảng cách
 *
 * Nhãn của đối tượng người chơi đang chọn là nhãn **họ vừa bấm vào để xem** —
 * mất nó ngay giữa lúc đang theo dõi đọc như một lỗi, không phải một lượt cắt
 * hợp lý. Toàn bộ nhãn còn lại thì xếp theo khoảng cách tới tâm màn hình: đó
 * là chỗ mắt người chơi đang nhìn, nên khi phải bỏ bớt, cái bị bỏ trước phải
 * là cái ở rìa tầm mắt, không phải một lựa chọn ngẫu nhiên theo thứ tự mảng.
 */
export function visibleLabels(list: readonly Nameplate[], opts: VisibleLabelsOptions): Nameplate[] {
  const minTileSize = opts.minTileSize ?? DEFAULT_MIN_LABEL_TILE_SIZE;
  if (opts.tileSize < minTileSize) return [];

  const inView = list.filter((l) => insideViewport(l.x, l.y, opts.viewport));

  const limit = opts.limit ?? DEFAULT_LABEL_LIMIT;
  if (inView.length <= limit) return inView;

  // Chỉ sắp và tính khoảng cách khi thật sự phải cắt bớt — trường hợp thường
  // gặp nhất (một cảnh vài chục dân) không trả giá cho việc này.
  const ranked = inView
    .map((l) => ({ l, d: distSqToCenter(l, opts.centerX, opts.centerY) }))
    .sort((a, b) => {
      if (a.l.highlight !== b.l.highlight) return a.l.highlight ? -1 : 1;
      return a.d - b.d;
    });
  return ranked.slice(0, limit).map((r) => r.l);
}
