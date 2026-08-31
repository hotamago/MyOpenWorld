/**
 * Nội suy vị trí vẽ của thực thể giữa hai ô — chỉ để mắt nhìn mượt hơn.
 *
 * ## Vấn đề: server tick chậm hơn màn hình cần vẽ
 *
 * `App.vue` hỏi lại thế giới mỗi 400 ms; màn hình thì vẽ ở 60 khung/giây. Nếu
 * cứ đặt sprite thẳng vào ô nguyên mỗi lần hỏi lại, một bước đi một ô là một
 * cú nhảy giật giữa hai vị trí cố định, đứng yên suốt 400 ms rồi giật tiếp.
 * Nội suy lấp khoảng 400 ms đó bằng một đường trượt ngắn.
 *
 * ## Đây KHÔNG phải optimistic UI
 *
 * `§P6.9.5` cấm suy đoán trạng thái thế giới trước khi engine xác nhận: không
 * được tự dịch avatar rồi hoàn tác nếu lệnh bị từ chối. `MotionTrack` không vi
 * phạm điều đó vì nó không **suy đoán vị trí mới** — nó chỉ vẽ mượt lại giữa
 * hai vị trí **đã được server xác nhận**. Điểm đến luôn là số server vừa gửi;
 * cái duy nhất bịa ra là các điểm nằm *giữa* hai lần xác nhận, và chúng chỉ
 * tồn tại trên canvas, không bao giờ chảy ngược vào bất cứ quyết định nào
 * (đường đi, va chạm, câu hỏi "ai đang đứng ở ô này"). Vì vậy `at()` trả về
 * một toạ độ **chỉ để vẽ**: đừng dùng nó ở nơi nào coi vị trí là sự thật.
 *
 * ## Nhảy xa thì snap, không trượt
 *
 * Trượt có ý nghĩa cho một bước đi (1 ô). Nó vô nghĩa — và trông như lỗi —
 * cho một cú dịch chuyển hay đổi khung nhìn, nơi "vị trí cũ" và "vị trí mới"
 * chẳng liên quan gì đến nhau. Ngưỡng `SNAP_DISTANCE_TILES` cắt hai trường hợp
 * đó ra khỏi nhau.
 *
 * ## Thời gian là tham số, không phải `Date.now()`
 *
 * Mọi hàm ở đây nhận `nowMs` từ bên gọi. Nhờ vậy file này test được bằng số
 * tay trong Node, không cần giả lập đồng hồ hệ thống hay `requestAnimationFrame`.
 */

/** Vị trí vẽ của một thực thể, nội suy giữa hai ô. */
export interface Motion {
  x: number;
  y: number;
}

/** Thời gian trượt từ ô cũ sang ô mới, tính bằng mili giây. */
export const GLIDE_MS = 260;

/**
 * Nhảy xa hơn ngần này (tính bằng ô, theo khoảng cách Chebyshev — trục nào xa
 * hơn quyết định) thì coi là dịch chuyển/đổi khung nhìn, không phải một bước
 * đi. Một bước đi thường của cư dân là 1 ô; 3 chừa dư cho trường hợp một tick
 * server gộp vài bước liền trước khi client kịp hỏi lại.
 */
const SNAP_DISTANCE_TILES = 3;

/** Trạng thái trượt của một thực thể: đang đi từ đâu, đến đâu, bắt đầu khi nào. */
interface Track {
  fromX: number;
  fromY: number;
  toX: number;
  toY: number;
  startMs: number;
}

/**
 * Hàm giảm tốc bậc ba (ease-out): nhanh lúc bắt đầu, chậm dần lúc chạm đích.
 *
 * Chọn ease-out chứ không phải tuyến tính vì một bước chân thật cũng vậy —
 * chân rời đất nhanh rồi khựng lại khi chạm ô mới. Tuyến tính đọc được là
 * "trượt băng", không phải "bước đi".
 *
 * Kẹp `t` vào `[0, 1]` ngay trong hàm: đây là hàm công khai, có test riêng gọi
 * thẳng với giá trị ngoài khoảng để chắc nó không vọt ra ngoài `[0, 1]` — thứ
 * sẽ làm thực thể vẽ vọt qua khỏi đích rồi lùi lại.
 */
export function ease(t: number): number {
  const c = t < 0 ? 0 : t > 1 ? 1 : t;
  const inv = 1 - c;
  return 1 - inv * inv * inv;
}

/**
 * Theo dõi và nội suy vị trí vẽ cho một tập thực thể.
 *
 * **Chỉ dùng kết quả của `at()` để vẽ.** Không nơi nào khác — không đường đi,
 * không luật va chạm, không "ô này ai đang đứng" — được phép đọc nó, vì đó là
 * một vị trí ước lượng giữa hai lần server xác nhận, có thể lệch vài phần mười
 * ô so với sự thật tại đúng khoảnh khắc đó. Logic luôn phải dùng toạ độ
 * `Entity.x/y` nguyên bản.
 */
export class MotionTrack {
  private tracks = new Map<string, Track>();

  /** Ghi nhận vị trí quyền uy mới (đơn vị ô, số nguyên) tại thời điểm `nowMs`. */
  update(id: string, x: number, y: number, nowMs: number): void {
    const cur = this.tracks.get(id);

    if (!cur) {
      // Lần đầu thấy `id`: không có "chỗ cũ" nào để trượt từ đó. Xuất hiện
      // thẳng tại vị trí quyền uy, giống hệt hành vi trước khi có nội suy.
      this.tracks.set(id, { fromX: x, fromY: y, toX: x, toY: y, startMs: nowMs });
      return;
    }

    if (cur.toX === x && cur.toY === y) {
      // Vị trí quyền uy không đổi so với lần trước — trường hợp thường gặp
      // nhất, vì `refresh()` hỏi lại mỗi 400 ms bất kể thực thể có bước đi hay
      // không. Không chạm gì cả: nếu một glide trước đó chưa trượt xong (dồn
      // dập vài lần hỏi lại), đụng vào `startMs` ở đây sẽ làm nó khựng lại rồi
      // chạy lại từ đầu — giật đúng chỗ lẽ ra phải mượt nhất.
      return;
    }

    const dx = Math.abs(x - cur.toX);
    const dy = Math.abs(y - cur.toY);
    if (Math.max(dx, dy) > SNAP_DISTANCE_TILES) {
      // Nhảy xa: đổi khung nhìn, dịch chuyển, hoặc một khoảng mất kết nối dài.
      // Trượt qua cả quãng đó trông như một lỗi hiển thị, không phải một bước
      // đi — snap thẳng, không glide.
      this.tracks.set(id, { fromX: x, fromY: y, toX: x, toY: y, startMs: nowMs });
      return;
    }

    // Điểm xuất phát của glide mới là vị trí **đang vẽ** ngay tại `nowMs`, chứ
    // không phải đích cũ (`cur.toX/toY`). Nếu lấy đích cũ, một lệnh cập nhật
    // đến giữa lúc glide trước còn dang dở sẽ làm thực thể giật lùi về đích cũ
    // trước khi trượt tiếp — đúng kiểu lỗi hình ảnh mà nội suy sinh ra để
    // tránh, không phải để tạo thêm.
    const at = interpolate(cur, nowMs);
    this.tracks.set(id, { fromX: at.x, fromY: at.y, toX: x, toY: y, startMs: nowMs });
  }

  /** Vị trí nên vẽ tại `nowMs`. Chưa từng thấy `id` thì trả `null`. */
  at(id: string, nowMs: number): Motion | null {
    const cur = this.tracks.get(id);
    if (!cur) return null;
    return interpolate(cur, nowMs);
  }

  /** Quên những id không còn trong danh sách. */
  retain(ids: Iterable<string>): void {
    const keep = new Set(ids);
    for (const id of this.tracks.keys()) {
      if (!keep.has(id)) this.tracks.delete(id);
    }
  }

  size(): number {
    return this.tracks.size;
  }
}

/** Nội suy một `Track` tại `nowMs`, chịu được `nowMs` lùi lại hoặc đứng yên. */
function interpolate(t: Track, nowMs: number): Motion {
  const elapsed = nowMs - t.startMs;
  // `elapsed <= 0` gộp cả trường hợp đồng hồ đứng yên lẫn lùi lại: kẹp về 0
  // thay vì để phép chia cho ra một số âm rồi `ease()` ngoại suy lùi trước
  // điểm xuất phát. `GLIDE_MS` là hằng số dương cố định nên phép chia dưới
  // đây không bao giờ chia cho 0.
  const t01 = elapsed <= 0 ? 0 : Math.min(1, elapsed / GLIDE_MS);
  const k = ease(t01);
  return {
    x: t.fromX + (t.toX - t.fromX) * k,
    y: t.fromY + (t.toY - t.fromY) * k,
  };
}
