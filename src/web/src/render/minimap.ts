/**
 * Bản đồ thu nhỏ (`§18.3`, panel `panel.minimap`).
 *
 * Nhận một lô ô rộng và ép nó xuống một ảnh vuông `size × size` để người chơi
 * biết mình đang ở đâu trong thế giới. Kết quả là buffer RGBA, chỗ gọi bọc vào
 * `ImageData` rồi `putImageData` lên canvas của panel.
 *
 * ## Vì sao **mode**, không phải trung bình RGB
 *
 * Đây là toàn bộ lý do module này tồn tại như một file riêng thay vì ba dòng
 * trong `App.vue`.
 *
 * Thu nhỏ một ảnh thường là lấy trung bình: gộp 9 pixel thành 1 thì cộng chín
 * màu rồi chia chín. Với ảnh chụp thì đúng, vì màu ở đó là một **đại lượng liên
 * tục** — trung bình của hai sắc da vẫn là một sắc da.
 *
 * Màu nền ô ở đây **không phải** đại lượng liên tục. `§18.5` quy định nó chở
 * đúng một thứ: vật liệu. Nó là nhãn **phân loại**, và trung bình của hai nhãn
 * không phải là một nhãn — nó là một màu không ứng với vật liệu nào cả.
 *
 * Con số cụ thể, với bảng dự phòng: nước `#2c5c8a` là `(44, 92, 138)`, đất mặt
 * `#6b5a3e` là `(107, 90, 62)`. Gộp 5 ô nước với 4 ô đất, lấy trung bình ra
 * `(72, 91, 104)` — một màu bùn xám-lam không có trong bảng vật liệu. Cả một
 * đường bờ biển dài sẽ thành một dải bùn như thế, và bờ biển chính là đường nét
 * mà mắt bám vào để định vị trên bản đồ thu nhỏ. Bản đồ vẫn "có màu", chỉ là
 * không còn nói gì.
 *
 * Nên ở đây mỗi pixel lấy **giá trị xuất hiện nhiều nhất** trong nhóm ô nó gộp.
 * 5 ô nước thắng 4 ô đất thì pixel đó là màu nước, đúng `#2c5c8a` như trong
 * bảng. Bờ biển vẫn răng cưa và vẫn là bờ biển.
 *
 * ## Vì sao sông là ngoại lệ của mode
 *
 * Sông thường rộng một ô. Trong một nhóm gộp 8×8, một ô sông là 1/64 — mode
 * loại nó ngay từ vòng đầu, và **toàn bộ hệ thống sông biến mất** khỏi bản đồ
 * thu nhỏ. Mà sông lại đúng là thứ người chơi dùng để định hướng: nó là đường
 * dài, liên tục, có hình dạng nhớ được, khác hẳn các mảng biome tròn trịa
 * giống nhau.
 *
 * Nên sông dùng luật **any**, không phải mode: bất kỳ ô con nào là sông thì
 * pixel đó vẽ sông. Kết quả là sông trên bản đồ thu nhỏ **dày hơn sự thật** —
 * đó là chủ ý. Một bản đồ thu nhỏ đúng tỉ lệ tới từng ô mà không đọc được thì
 * thua một bản đồ phóng đại nét chính mà đọc được.
 *
 * ## Đại lượng nào thì được phép lấy trung bình
 *
 * Độ cao. Nó **là** đại lượng liên tục: trung bình của 80 m và 120 m là 100 m,
 * và 100 m là một độ cao thật. Đó chính là ranh giới phân biệt hai loại dữ
 * liệu, và cũng là câu trả lời cho "vì sao chỗ này trung bình được mà chỗ kia
 * thì không".
 */

import type { TileBatch } from "@/api/game";
import type { BlockPalette } from "./blocks";

/**
 * Biên độ đổ bóng theo độ cao. `±0.10` cho hệ số nằm trong `[0.90, 1.10]`.
 *
 * Cố tình rất nhẹ. `§18.5` bắt màu nền ô chở vật liệu và chỉ chở vật liệu; một
 * lớp bóng mạnh sẽ làm "nước sâu" và "đá trong bóng núi" trùng màu nhau, tức là
 * đổ bóng nuốt mất chính kênh mà nó đang phụ họa. Ở mức 10% thì mắt vẫn đọc ra
 * hình khối địa hình, còn việc phân loại vật liệu thì không ai phải đoán.
 */
const HEIGHT_SHADE_RANGE = 0.1;

/**
 * Thang độ cao của đường cong đổ bóng, mét.
 *
 * Dùng `tanh` với thang này thay vì chuẩn hóa theo min/max của lô, vì chuẩn hóa
 * theo lô là **tương đối**: cùng một quả đồi sẽ đổi độ sáng mỗi khi người chơi
 * đi qua một ngọn núi cao hơn lọt vào lô. Nhìn ra hệt một lỗi nhấp nháy, và tốn
 * rất nhiều thời gian để loại trừ. `tanh` neo ở mực nước biển (`sea_level_m`
 * mặc định là 0) nên độ sáng của một ô là hàm của **chỉ ô đó**, ổn định khi kéo
 * bản đồ.
 *
 * `300` m: biên độ địa hình của worldgen là `[2400, 1200, 220, 40]` m, nhưng
 * phần lớn đất người chơi đi lại nằm trong vài trăm mét quanh mực nước biển.
 * Đặt thang ở đó thì độ dốc của đường cong rơi đúng vào khoảng có dữ liệu; núi
 * 2400 m vẫn không cháy vì `tanh` bão hòa chứ không kẹp cứng.
 */
const HEIGHT_SHADE_SCALE_M = 300;

/**
 * Sắc phủ lòng sông. Cùng sắc với `terrain.ts` để hai khung nhìn là **một thế
 * giới**, chứ không phải hai bản đồ khác nhau của cùng một chỗ.
 */
const RIVER_TINT: readonly [number, number, number] = [0x35, 0x7a, 0xb8];

/**
 * Độ trộn sắc sông, mạnh hơn `terrain.ts` (`0.45`) một cách có chủ ý.
 *
 * Ở khung nhìn chính, một con sông là một dải rộng nhiều pixel: hình dạng của
 * nó đã tự nói nó là sông, nên sắc chỉ cần gợi ý. Trên bản đồ thu nhỏ nó chỉ
 * còn **một pixel** cạnh một pixel đất trơn — không còn hình dạng nào để dựa
 * vào, nên sắc phải làm toàn bộ việc nhận dạng.
 */
const RIVER_MIX = 0.6;

/** Vật liệu dùng khi lô không nói gì về ô đó. */
const EMPTY_MATERIAL = "air";

const clamp8 = (v: number): number => (v < 0 ? 0 : v > 255 ? 255 : v);

/**
 * Khoảng ô `[lo, hi)` mà một hàng/cột pixel gộp vào.
 *
 * `hi` luôn `>= lo + 1`. Đó không phải trang trí: khi `size > tiles` (người chơi
 * phóng to bản đồ thu nhỏ hơn số ô có thật), phép chia thẳng cho ra `lo === hi`
 * ở phần lớn pixel, tức là **không có ô nào để lấy mode** và pixel đó thành màu
 * rác. Ép rộng tối thiểu một ô biến trường hợp đó thành lấy mẫu gần nhất, vốn
 * là hành vi đúng khi phóng to.
 */
function tileSpan(pixel: number, tiles: number, size: number): readonly [number, number] {
  const lo = Math.floor((pixel * tiles) / size);
  const hi = Math.max(lo + 1, Math.floor(((pixel + 1) * tiles) / size));
  // Kẹp về trong mảng: chỉ số ngoài biên ở đây không ném lỗi, nó trả `undefined`
  // và lặng lẽ tô một pixel sai — đúng loại lỗi khó thấy nhất.
  return [Math.min(lo, tiles - 1), Math.min(hi, tiles)] as const;
}

/**
 * Pixel chứa một ô, **nghịch đảo chính xác** của [`tileSpan`].
 *
 * `tileSpan` gán cho pixel `p` các ô từ `floor(p * tiles / size)`. Pixel chứa ô
 * `tile` do đó là chỉ số `p` lớn nhất còn thỏa `floor(p * tiles / size) <= tile`
 * — giải ra là `ceil((tile + 1) * size / tiles) - 1`.
 *
 * Viết bằng phép chia đúng một lần thay vì dò lại `tileSpan` trong vòng lặp:
 * nếu hai công thức lệch nhau dù chỉ một pixel thì dấu vị trí người chơi sẽ
 * đứng cạnh ô của chính nó chứ không nằm trên nó, và không có bài test nào bắt
 * được điều đó nếu cả hai bên cùng dùng chung một hàm sai.
 */
function pixelOfTile(tile: number, tiles: number, size: number): number {
  const p = Math.ceil(((tile + 1) * size) / tiles) - 1;
  return p < 0 ? 0 : p >= size ? size - 1 : p;
}

/** Số pixel hợp lệ, hoặc `null` nếu tham số vô nghĩa. */
function normalizeSize(size: number): number | null {
  const n = Math.floor(size);
  return Number.isFinite(n) && n >= 1 ? n : null;
}

/**
 * Tô bản đồ thu nhỏ. Trả buffer RGBA dài đúng `size * size * 4`.
 *
 * Ảnh **vuông** kể cả khi lô không vuông: chỗ gọi chỉnh tỉ lệ khi vẽ lên canvas
 * (thuộc tính `width`/`height` của phần tử, hoặc `drawImage`), vì đó là chỗ duy
 * nhất biết panel rộng bao nhiêu. Trả về một ảnh có viền đen để giữ tỉ lệ thì
 * sẽ đốt pixel vào chỗ không chở thông tin gì, mà bản đồ thu nhỏ vốn đã thiếu
 * pixel.
 *
 * Alpha luôn `255`. Bản đồ thu nhỏ nằm trên một panel mờ, và một pixel trong
 * suốt ở đó không đọc ra là "không có dữ liệu" — nó đọc ra là màu của panel.
 */
export function paintMinimap(
  batch: TileBatch,
  palette: BlockPalette,
  size: number,
): Uint8ClampedArray {
  const n = normalizeSize(size);
  if (n === null) {
    throw new RangeError(`paintMinimap: size phải là số nguyên >= 1, nhận ${size}`);
  }

  const out = new Uint8ClampedArray(n * n * 4);
  const { w, h } = batch;

  // Lô rỗng: vẫn trả ảnh đặc màu "không khí" thay vì mảng toàn 0. Mảng toàn 0
  // là màu đen **và** alpha 0 — hai cách hiểu khác nhau cho cùng một buffer, và
  // chỗ gọi sẽ chọn nhầm cách.
  if (w <= 0 || h <= 0) {
    const blank = palette.color(EMPTY_MATERIAL);
    for (let i = 0; i < n * n; i++) {
      out[i * 4] = (blank >> 16) & 0xff;
      out[i * 4 + 1] = (blank >> 8) & 0xff;
      out[i * 4 + 2] = blank & 0xff;
      out[i * 4 + 3] = 255;
    }
    return out;
  }

  // Một `Map` dùng lại cho mọi pixel. Cấp phát một `Map` mỗi pixel là `size²`
  // lần cấp phát cho mỗi khung — với `size = 146` là hơn hai vạn, đủ để bộ thu
  // gom rác nhận ra trong một panel đáng lẽ gần như miễn phí.
  const counts = new Map<string, number>();

  for (let py = 0; py < n; py++) {
    const [y0, y1] = tileSpan(py, h, n);
    for (let px = 0; px < n; px++) {
      const [x0, x1] = tileSpan(px, w, n);

      counts.clear();
      let winner = EMPTY_MATERIAL;
      let winnerCount = 0;
      let hasRiver = false;
      let heightSum = 0;
      let samples = 0;

      for (let ty = y0; ty < y1; ty++) {
        for (let tx = x0; tx < x1; tx++) {
          const i = ty * w + tx;
          // Cùng luật "nhìn từ trên xuống" với `terrain.ts`: ô là không khí thì
          // cái người chơi thực sự thấy là mặt đất bên dưới. Nếu bản đồ thu nhỏ
          // đếm "không khí" là một vật liệu thì đứng ở lát cao sẽ cho ra một
          // hình chữ nhật đen đặc, không phải bản đồ.
          const material = batch.material[i] ?? EMPTY_MATERIAL;
          const surface = batch.surface[i] ?? EMPTY_MATERIAL;
          const visible = material !== EMPTY_MATERIAL ? material : surface;

          const c = (counts.get(visible) ?? 0) + 1;
          counts.set(visible, c);
          // So sánh `>` chứ không `>=`: hòa thì giữ vật liệu gặp trước, mà thứ
          // tự quét là hàng-trước-cột cố định. Nhờ vậy kết quả xác định — cùng
          // đầu vào cho cùng buffer, không phụ thuộc thứ tự duyệt `Map`.
          if (c > winnerCount) {
            winnerCount = c;
            winner = visible;
          }

          // Luật **any** cho sông, xem phần đầu file.
          if ((batch.river[i] ?? 0) === 1) hasRiver = true;

          heightSum += batch.height[i] ?? 0;
          samples++;
        }
      }

      const base = palette.color(winner);
      let r = (base >> 16) & 0xff;
      let g = (base >> 8) & 0xff;
      let b = base & 0xff;

      if (hasRiver) {
        r = r * (1 - RIVER_MIX) + RIVER_TINT[0] * RIVER_MIX;
        g = g * (1 - RIVER_MIX) + RIVER_TINT[1] * RIVER_MIX;
        b = b * (1 - RIVER_MIX) + RIVER_TINT[2] * RIVER_MIX;
      }

      // Trung bình độ cao là hợp lệ ở đây và chỉ ở đây — độ cao liên tục, vật
      // liệu thì không.
      const meanHeight = samples > 0 ? heightSum / samples : 0;
      const shade = 1 + HEIGHT_SHADE_RANGE * Math.tanh(meanHeight / HEIGHT_SHADE_SCALE_M);

      const o = (py * n + px) * 4;
      out[o] = clamp8(Math.round(r * shade));
      out[o + 1] = clamp8(Math.round(g * shade));
      out[o + 2] = clamp8(Math.round(b * shade));
      out[o + 3] = 255;
    }
  }

  return out;
}

/**
 * Tọa độ thế giới → pixel bản đồ thu nhỏ. `null` khi điểm nằm ngoài lô.
 *
 * Dùng để vẽ dấu vị trí avatar, đích đang đi tới, hay các thực thể đáng chú ý.
 *
 * `null` chứ không phải kẹp về mép: một dấu bị kẹp nằm ở mép bản đồ **trông y
 * hệt** một dấu thật ở mép bản đồ. Người chơi sẽ đọc nó là "có người ở góc kia"
 * và đi tới đó. Không vẽ gì cả là câu trả lời trung thực cho "chỗ này chưa nạp
 * dữ liệu"; muốn có mũi tên chỉ hướng ra ngoài thì đó là quyết định của chỗ gọi
 * chứ không phải của phép biến đổi tọa độ.
 */
export function minimapMarker(
  batch: TileBatch,
  size: number,
  worldX: number,
  worldY: number,
): { x: number; y: number } | null {
  const n = normalizeSize(size);
  if (n === null || batch.w <= 0 || batch.h <= 0) return null;

  // `floor` chứ không `round`: tọa độ thế giới có thể là phân số (vị trí camera
  // nội suy giữa hai ô), và ô chứa điểm `12.9` là ô `12`, không phải ô `13`.
  const tx = Math.floor(worldX) - batch.x;
  const ty = Math.floor(worldY) - batch.y;
  if (tx < 0 || ty < 0 || tx >= batch.w || ty >= batch.h) return null;

  return { x: pixelOfTile(tx, batch.w, n), y: pixelOfTile(ty, batch.h, n) };
}

/**
 * Pixel bản đồ thu nhỏ → tọa độ thế giới. Dùng cho "bấm vào để nhảy camera".
 *
 * Trả ô ở **giữa** nhóm ô mà pixel đó gộp, không phải ô ở góc trên-trái. Ở mức
 * thu nhỏ 8 ô một pixel, lấy góc làm camera lệch bốn ô so với chỗ ngón tay chỉ
 * — đủ để cảm thấy bản đồ "trả lời sai".
 *
 * Kẹp `px`/`py` về trong ảnh thay vì trả `null`: nguồn của chúng là toạ độ chuột
 * chia cho tỉ lệ hiển thị, và phép chia đó cho ra đúng `size` khi bấm vào cạnh
 * phải của canvas. Ném lỗi hay trả `null` ở một pixel biên là biến một cú bấm
 * hợp lệ thành một lỗi.
 */
export function minimapToWorld(
  batch: TileBatch,
  size: number,
  px: number,
  py: number,
): { x: number; y: number } {
  const n = normalizeSize(size);
  if (n === null || batch.w <= 0 || batch.h <= 0) return { x: batch.x, y: batch.y };

  const cx = Math.min(n - 1, Math.max(0, Math.floor(px)));
  const cy = Math.min(n - 1, Math.max(0, Math.floor(py)));

  const [x0, x1] = tileSpan(cx, batch.w, n);
  const [y0, y1] = tileSpan(cy, batch.h, n);

  // `>> 1` thay vì `/ 2`: kết quả phải là số nguyên, và một tọa độ ô phân số đi
  // vào `api.goto` sẽ bị server làm tròn theo cách riêng của nó.
  return {
    x: batch.x + x0 + ((x1 - x0 - 1) >> 1),
    y: batch.y + y0 + ((y1 - y0 - 1) >> 1),
  };
}
