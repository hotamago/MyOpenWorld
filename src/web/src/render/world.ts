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
 * | nhãn | khi tên/vị trí đổi, hoặc khi camera/zoom đổi | vài chục `Text`, tái dùng theo `id` |
 *
 * Gộp chúng nghĩa là dựng lại cả bản đồ mỗi khi một NPC nhấc chân.
 *
 * ## Nhãn giờ nằm trong Pixi, không còn là `<div>` HTML
 *
 * Bản trước đặt nhãn bằng `<div>` tuyệt đối đè lên canvas, đúng khuyến nghị cũ
 * (`§P6.9.2`: `PIXI.Text` tạo một texture cho **mỗi chuỗi**, sợ vài trăm nhãn
 * ăn hết bộ nhớ texture). Cái giá phải trả hoá ra nặng hơn cái nó tránh: mỗi
 * lần cập nhật vị trí, trình duyệt phải chạy lại Layout/Style cho hàng chục
 * phần tử `<div>` — một reflow DOM đúng vào giữa lúc người chơi đang kéo bản
 * đồ, tức là đúng lúc CPU đã bận nhất.
 *
 * Ở đây `nameplate.ts` chặn số nhãn ở một con số nhỏ (`DEFAULT_LABEL_LIMIT`,
 * xem lời giải ở đó) và mỗi `id` chỉ giữ **một** `Text` sống suốt vòng đời của
 * nó — cùng khuôn tái dùng-theo-`id` mà `entityGraphics` đã dùng cho thực thể.
 * Nỗi lo "vài trăm nhãn" không còn xảy ra được nữa vì giới hạn đó, nên chi phí
 * một texture-mỗi-chuỗi của `PIXI.Text` không còn là vấn đề, còn chi phí reflow
 * DOM thì biến mất hẳn vì không còn `<div>` nào cả.
 *
 * ## Nội suy chỉ để vẽ, không phải optimistic UI
 *
 * Thực thể **quyền uy** vẫn nhảy từ ô sang ô đúng như server nói — không có gì
 * đoán trước bị `§P6.9.5` cấm ở đây. Cái đổi là cách **vẽ** bước nhảy đó: thay
 * vì đặt sprite thẳng vào ô mới và đứng im 400 ms tới lần hỏi lại kế tiếp,
 * `MotionTrack` (`./motion.ts`) trượt vị trí vẽ mượt trong một khoảng ngắn.
 * Toạ độ nội suy không bao giờ rời khỏi hàm vẽ: không đường đi, không luật va
 * chạm, không panel nào đọc nó — chúng vẫn dùng thẳng `Entity.x/y`.
 *
 * ## Vẽ lại texture chỉ khi nội dung đổi, không phải mỗi lần hỏi server
 *
 * `App.vue` hỏi lại thế giới mỗi 400 ms bất kể địa hình có đổi hay không — lý
 * do là polling, không phải WebSocket (`§P6.8` sẽ thay khi nối xong). Phần lớn
 * các lần hỏi đó trả về đúng y hệt lô ô cũ. Tạo canvas mới + tải texture mới
 * lên GPU cho một tấm ảnh không đổi là trả giá GPU cho không khí, nên
 * `setTerrain` chỉ vẽ lại khi một dấu vân rẻ của lô (xem `fingerprintOf`) đổi;
 * `tint` theo ngày/đêm thì luôn áp lại vì nó chỉ là một phép nhân màu trên
 * sprite, không đụng texture.
 */

import { Application, Container, Culler, Graphics, Sprite, Text, Texture } from "pixi.js";
import type { TextStyleOptions } from "pixi.js";
import type { Entity, TileBatch } from "@/api/game";
import type { BlockPalette } from "./blocks";
import { ambientSprites, type AmbientSprite } from "./ambient";
import { drawFigure, figureOf, type FigureSpec } from "./figure";
import { chimneys, phaseAt, smokePuff, sway } from "./liveliness";
import { MotionTrack } from "./motion";
import { type Nameplate, visibleLabels } from "./nameplate";
import { paintTerrain, skyTint } from "./terrain";

/**
 * Băm nội dung một chuỗi bằng vòng lặp ký tự.
 *
 * Dùng cho `fingerprintOf`: vật liệu chỉ vài chục loại và tên trung bình dưới
 * mười ký tự, nên vòng lặp ký tự ở đây rẻ hơn hẳn một lần `paintTerrain` (vốn
 * còn lấy mẫu ô lân cận, tính hillshade, hạt vật liệu, bóng công trình cho
 * từng ô) — không cần một hàm băm "xịn" hơn cho việc chỉ để phát hiện đổi.
 */
function stringHash(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (Math.imul(h, 31) + s.charCodeAt(i)) | 0;
  return h;
}

/**
 * `%` giữ dấu toán hạng trái trong JS; các vòng lặp thời gian của lớp sống
 * động (khe khói, cửa sổ lấp lánh) cần phần dư luôn dương bất kể `nowMs` hay
 * độ lệch pha âm tới đâu.
 */
function safeModWorld(a: number, m: number): number {
  return ((a % m) + m) % m;
}

/**
 * Dấu vân rẻ của một lô ô: đổi khi và chỉ khi hình ảnh render ra sẽ đổi.
 *
 * ## Vì sao phải quét toàn bộ mảng, không lấy mẫu
 *
 * Một trong những lần "địa hình đổi" quan trọng nhất là người chơi khắc lại
 * **đúng một ô**. Lấy mẫu một phần lô ô (mỗi ô thứ N) sẽ bỏ sót đúng trường
 * hợp đó phần lớn thời gian — rẻ hơn nhưng sai, và sai theo kiểu im lặng: bản
 * đồ hiện đúng, chỉ là hiện đúng cái cũ. Phải kiểm đủ.
 *
 * ## Vì sao quét đủ vẫn rẻ
 *
 * Quét đủ không có nghĩa là đắt như `paintTerrain`: hàm băm ở đây chỉ cộng và
 * nhân số nguyên, không lấy mẫu ô lân cận, không lượng giác, không tính bóng.
 * Với một khung nhìn cỡ 87×41 (~3500 ô), tổng số ký tự phải đọc chỉ khoảng vài
 * chục nghìn — rẻ hơn nhiều so với chi phí thật sự cần tránh: tạo canvas mới,
 * `Texture.from`, và tải lên GPU.
 *
 * `built` gộp vào thay vì bỏ qua vì khắc một ô đôi khi chỉ đổi cờ `built` mà
 * giữ nguyên `material`/`surface` (ví dụ đóng dấu "đã xây" lên nền có sẵn).
 */
function fingerprintOf(batch: TileBatch): string {
  let h = 0;
  const n = batch.w * batch.h;
  for (let i = 0; i < n; i++) {
    h = Math.imul(h, 0x0100_0193) ^ stringHash(batch.material[i] ?? "");
    h = Math.imul(h, 0x0100_0193) ^ stringHash(batch.surface[i] ?? "");
    h = Math.imul(h, 0x0100_0193) ^ (batch.built[i] ?? 0);
  }
  return `${batch.x}:${batch.y}:${batch.w}:${batch.h}:${batch.z}:${h >>> 0}`;
}

/**
 * So hai `FigureSpec` để biết có cần dựng lại `Graphics` hay không.
 *
 * So từng trường nguyên thủy thay vì `JSON.stringify` hai bên: rẻ hơn, và
 * đúng luôn — `FigureSpec` chỉ có số/chuỗi, không có cấu trúc lồng cần so sâu.
 */
function figureSpecEqual(a: FigureSpec, b: FigureSpec): boolean {
  return (
    a.shape === b.shape &&
    a.scale === b.scale &&
    a.body === b.body &&
    a.outline === b.outline &&
    a.tool === b.tool &&
    a.head === b.head &&
    a.facing === b.facing &&
    a.mark === b.mark
  );
}

/**
 * Cỡ chữ nhãn tên, tính bằng pixel màn hình — **không** theo `tileSize`.
 *
 * Nhãn là chữ để đọc, không phải hoạ tiết của ô: nó cần đúng một cỡ vật lý dễ
 * đọc ở mọi mức phóng, không phải "phóng to theo bản đồ" tới mức chiếm nửa màn
 * hình hay co lại thành chấm mực. Cố định cỡ còn tránh việc mỗi lần đổi zoom
 * lại bắt Pixi dựng lại texture chữ của mọi nhãn đang hiện cùng lúc — thứ
 * `rescale()` vốn đã phải làm cho toàn bộ hình thực thể, không cần thêm nhãn
 * vào danh sách đó.
 */
const LABEL_FONT_SIZE = 12;
const LABEL_FONT_SIZE_HIGHLIGHT = 13;

/**
 * Kiểu chữ cho một nhãn tên, theo trạng thái `highlight`.
 *
 * ## Viền chữ, không phải nền mờ, để đọc được trên mọi nền
 *
 * Một nền mờ phía sau đọc được cũng tốt, nhưng nó đòi thêm một `Graphics` cho
 * MỖI nhãn — gấp đôi số đối tượng Pixi phải tạo, tái dùng theo `id`, và dọn khi
 * thực thể biến mất, cho một ích lợi mà `stroke` trong `TextStyle` đã cho sẵn:
 * một viền tối quanh chữ sáng đọc được trên cả tuyết trắng lẫn đêm đen, bằng
 * đúng một thuộc tính có sẵn của chính đối tượng `Text` đã cần vẽ.
 *
 * Trả một object mới mỗi lần gọi — không phải hai hằng số style dùng chung —
 * vì `Text.style` sở hữu style của nó (gán một style đang bị `Text` khác dùng
 * sẽ làm chúng đổi lẫn nhau khi một bên bị dọn).
 */
function labelStyle(highlight: boolean): TextStyleOptions {
  return {
    fontFamily: "sans-serif",
    fontSize: highlight ? LABEL_FONT_SIZE_HIGHLIGHT : LABEL_FONT_SIZE,
    fontWeight: highlight ? "700" : "500",
    fill: highlight ? 0xffe28a : 0xf2f4f8,
    stroke: { color: 0x10131a, width: highlight ? 4 : 3 },
  };
}

/**
 * Trần số hạt môi trường mỗi khung nhìn.
 *
 * Đo trên khung 87×41: cảnh dày nhất (sa mạc, băng hà) sinh ~250 hạt, cảnh
 * thường ~120–170. 240 nghĩa là gần như không bao giờ tỉa ở cảnh thật, và chỉ
 * tỉa nhẹ ở hai chỗ dày nhất.
 */
const AMBIENT_BUDGET = 240;

const TAU = Math.PI * 2;

/**
 * Ngưỡng phóng dưới đó mọi chuyển động vi mô (khói, cỏ lay, lấp lánh, nhịp đi,
 * bóng tiếp xúc) đều tắt hẳn.
 *
 * Dùng chung một con số với `redrawAmbient` (`ts < 9`, xem ở đó): dưới ngưỡng
 * này biên độ vài pixel của các hiệu ứng này còn nhỏ hơn một ô màn hình, nên
 * chúng không còn đọc được là chuyển động — chỉ còn là nhiễu run rẩy trên một
 * texture đã đủ nhỏ để mắt gộp các ô lại với nhau.
 */
const LIVELINESS_MIN_TILE_SIZE = 9;

/**
 * Trần cứng tổng số phần tử vi-chuyển-động vẽ mỗi khung hình: tối đa 5 ống
 * khói × tối đa 5 hạt khói mỗi ống (`SMOKE_SLOTS`) = 25, cộng tối đa 90 tép
 * lúa/cỏ lay, cộng tối đa 65 vệt lấp lánh mặt nước — 25 + 90 + 65 = 180.
 *
 * Ba loại chia sẻ **một** ngân sách chứ không phải ba trần riêng cộng lại vô
 * điều kiện: một khung nhìn toàn ruộng lúa sát biển vẫn phải dừng ở đúng con
 * số này, không phải 90 + 65 cộng dồn với bất cứ gì khói bếp cần thêm.
 */
const LIVELINESS_BUDGET = 180;
const CHIMNEY_TILE_LIMIT = 5;
const CROP_TUFT_LIMIT = 90;
const WATER_GLINT_LIMIT = 65;

/** Cỏ/lúa: biên độ và chu kỳ khớp đúng buổi tư vấn — xem `liveliness.sway`. */
const CROP_TUFT_COLOR = 0xcdeaa0;

/**
 * Khoảng cách giữa hai hạt khói sinh liên tiếp từ cùng một ống khói, cộng số
 * "khe" xét mỗi khung hình.
 *
 * `SMOKE_SLOTS * SMOKE_SPACING_MS` (2 750 ms) lớn hơn `SMOKE_LIFE_MS`
 * (2 200 ms — xem `liveliness.ts`) một chút: mỗi khe có một quãng nghỉ ngắn
 * trước khi hạt tiếp theo của chính nó sinh ra, nhưng vì 5 khe lệch pha đều
 * nhau, tại bất kỳ thời điểm nào luôn có 3–5 hạt của các khe khác nhau đang
 * sống — đúng hiệu ứng "thỉnh thoảng nhả 3–5 hạt" mà không cần giữ danh sách
 * hạt đang sống ở đâu cả (mỗi khung hình tính lại từ `performance.now()`).
 */
const SMOKE_SLOTS = 5;
const SMOKE_SPACING_MS = 550;
const SMOKE_CYCLE_MS = SMOKE_SLOTS * SMOKE_SPACING_MS;

/** Mặt nước lấp lánh: chu kỳ và cửa sổ "đang sáng" của vệt chạy. */
const WATER_GLINT_PERIOD_MS = 5_000;
const WATER_GLINT_WINDOW = 0.18;

/** Nhịp bước: một cú nảy nhẹ mỗi nửa chu kỳ (`Math.abs(sin)`), chỉ khi đang trượt ô. */
const WALK_BOB_PERIOD_MS = 420;
const WALK_BOB_AMPLITUDE = 1.5;
const WALK_TILT_DEG = 4;

/**
 * Hướng đổ bóng, khớp `SUN` của `terrain.ts` (trên–trái, tức `x`/`y` âm).
 *
 * Chép tay hai thành phần ngang của `SUN`, chuẩn hoá lại trong mặt phẳng
 * `(x, y)` — không import được vì `SUN` không export khỏi `terrain.ts` (module
 * đó thuộc quyền sửa của người khác trong phiên làm việc này). Nếu `terrain.ts`
 * đổi hướng nắng, hằng số này phải đổi theo tay; `terrain.test.ts` là nơi phát
 * hiện lệch nếu có ai quên.
 */
const SHADOW_DIR_X = -0.663;
const SHADOW_DIR_Y = -0.748;
/** Co lại còn ngần này khi nhân vật đang ở giữa bước — "nhấc chân khỏi đất". */
const SHADOW_STRIDE_SCALE = 0.82;

/**
 * Số ô lấy dư mỗi phía so với khung nhìn.
 *
 * Lấy dư là trả trước một ít băng thông để **không** phải hỏi lại server mỗi
 * lần người chơi nhích bản đồ một ô. Tám ô mỗi phía phủ được một cú kéo cỡ vừa,
 * và với lô 90×45 thì nó thêm khoảng 40% dữ liệu — rẻ hơn nhiều so với một vòng
 * mạng giữa lúc tay đang kéo.
 */
const BATCH_MARGIN = 8;

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
  /**
   * Chuyển động vi mô của "sự sống": khói bếp, cỏ/lúa lay, mặt nước lấp lánh.
   * Tách khỏi `ambientLayer` dù cùng vẽ bằng một `Graphics` mỗi khung — xem lời
   * giải "vì sao module này tồn tại tách khỏi ambient.ts" ở `liveliness.ts`:
   * lớp này chạy theo đồng hồ thật mỗi khung hình (`onTick`), còn `ambientLayer`
   * chỉ vẽ lại theo nhịp `tick` (mỗi lần `retint`/`setTerrain`).
   */
  private livelinessLayer = new Container();
  /**
   * Bóng tiếp xúc của thực thể, lệch theo hướng nắng và co lại giữa bước chân.
   * Nằm ngay dưới `entityLayer` để nhân vật luôn "đứng trên" bóng của chính nó.
   * Vẽ riêng khỏi bóng cố định đã có trong `drawFigure` (`figure.ts`) vì module
   * này không được sửa `figure.ts` — hai bóng cộng lại đọc ra như một bóng mềm
   * có phần lõi đậm (bóng tiếp đất cũ) và phần đổ dài theo nắng (bóng mới),
   * không phải hai vật thể tách rời, miễn là bóng mới nhạt và lệch ít.
   */
  private shadowLayer = new Container();
  private shadowGraphics = new Graphics();
  private overlayLayer = new Container();
  private overlaySprite: Sprite | null = null;
  private overlayTexture: Texture | null = null;
  /** Diff của một can thiệp đang xem trước. Trên cùng: nó là thứ cần đọc. */
  private diffLayer = new Container();
  private diffChanges: DiffChange[] = [];
  private entityLayer = new Container();
  /**
   * Nhãn tên. Trên `entityLayer` để chữ không bị nhân vật che, dưới `diffLayer`
   * vì diff xem trước là thứ khẩn cấp hơn cần đọc ngay (xem lời giải ở
   * `diffLayer`) — hai lớp không tranh nhau vì chúng hiếm khi cùng dày đặc một
   * lúc: diff chỉ có khi đang xem trước một ý chỉ.
   */
  private labelLayer = new Container();
  /**
   * Vị trí ở lần vẽ trước của từng thực thể, theo `id` — dùng suy `facing`
   * trong `figureOf`. Dọn trong `setEntities` mỗi khi một `id` không còn xuất
   * hiện nữa, để map này không phình vô hạn suốt đời phiên chơi.
   */
  private prevPos = new Map<string, { x: number; y: number }>();
  /**
   * Vị trí **vẽ** của mỗi thực thể, trượt mượt giữa hai lần server xác nhận.
   * Chỉ đọc trong `syncEntityPositions` — mọi nơi khác (facing, culling, va
   * chạm) vẫn phải dùng `Entity.x/y` nguyên bản, xem lời giải ở đầu file.
   */
  private motion = new MotionTrack();
  /**
   * Một `Graphics` mỗi thực thể, tái dùng giữa các lần `setEntities`. Dựng lại
   * chỉ khi `FigureSpec` đổi hoặc cỡ ô đổi (`entityTileSize`); vị trí thì đổi
   * mỗi khung hình qua `g.position`, không đụng vào nội dung đã vẽ.
   */
  private entityGraphics = new Map<string, Graphics>();
  /** `FigureSpec` đã dùng để vẽ lần gần nhất — để biết khi nào phải vẽ lại. */
  private entitySpecs = new Map<string, FigureSpec>();
  /** Cỡ ô lúc vẽ `entityGraphics` lần gần nhất; đổi zoom thì mọi hình phải vẽ lại. */
  private entityTileSize = 0;
  /**
   * Danh sách thực thể của lần cập nhật gần nhất.
   *
   * Giữ lại để `rescale` dựng lại hình theo cỡ ô mới mà không phải hỏi server:
   * phóng to là một câu hỏi về khung nhìn, không phải về thế giới.
   */
  private lastEntities: Entity[] = [];
  /**
   * Một `Text` mỗi nhãn đang hiện, tái dùng theo `id` — cùng khuôn với
   * `entityGraphics`. Chỉ những nhãn `visibleLabels` chọn mới có mặt ở đây;
   * phần còn lại của `lastLabels` không tốn một đối tượng Pixi nào.
   */
  private labelObjects = new Map<string, Text>();
  /** Nội dung + `highlight` đã áp cho mỗi `Text` — để biết khi nào phải viết lại, thay vì viết lại mỗi lần gọi. */
  private labelCache = new Map<string, { text: string; highlight: boolean }>();
  /**
   * Toàn bộ nhãn `setLabels` nhận được gần nhất, **trước khi** lọc.
   *
   * Giữ lại để `reposition()` — chạy mỗi khi camera hoặc `tileSize` đổi — có
   * thể tính lại `visibleLabels` mà không phải đợi `App.vue` gọi `setLabels`
   * lần nữa: nhãn nào lọt qua ngưỡng ẩn hay khung nhìn là một câu hỏi về
   * *camera*, không phải về dữ liệu, nên nó phải phản ứng ngay khi camera đổi.
   */
  private lastLabels: Nameplate[] = [];
  /**
   * Độ phân giải canvas của mọi nhãn tên, chốt một lần khi `attach()` — không
   * đổi theo `tileSize` hay theo từng lần vẽ. `Text` mặc định tự dò lại độ
   * phân giải mỗi khi style đổi; cố định nó ở đây tránh việc bật/tắt highlight
   * (đổi `fontSize`, xem `labelStyle`) vô tình kéo theo một lần dò lại không
   * liên quan, và khớp đúng tinh thần "resolution cố định" mà `Application`
   * cũng áp cho canvas chính (xem `attach`).
   */
  private readonly labelResolution = globalThis.devicePixelRatio ?? 1;
  private plannedPath: [number, number][] = [];
  private terrainSprite: Sprite | null = null;
  private terrainTexture: Texture | null = null;
  /** Canvas nền tái dùng qua các lần vẽ lại — chỉ thay khi kích thước lô đổi. */
  private terrainCanvas: HTMLCanvasElement | null = null;
  private terrainCtx: CanvasRenderingContext2D | null = null;
  /** Dấu vân của lô đã vẽ lần gần nhất — xem `fingerprintOf`. */
  private terrainFingerprint = "";
  private batch: TileBatch | null = null;
  private centerX = 0;
  private centerY = 0;
  private zoomIndex = 4;
  /**
   * `tick` thế giới của lần `setTerrain`/`retint` gần nhất — `onTick` chạy mỗi
   * khung hình và cần biết đêm hay ngày (`chimneys` phụ thuộc `tick`) mà không
   * đợi lần hỏi server kế tiếp truyền lại con số này.
   */
  private currentTick = 0;
  /**
   * Đặt lại vị trí sprite thực thể theo `MotionTrack`, rồi cắt (cull) hai lớp
   * đông đối tượng — thực thể và nhãn — mỗi khung hình. Gắn vào `app.ticker`
   * (nó vốn đã chạy để render) thay vì tự mở một `requestAnimationFrame`
   * riêng — một vòng lặp thời gian thực cho cùng một component là đủ.
   *
   * ## Vì sao gọi `Culler` tay ở đây, không đăng ký `CullerPlugin`
   *
   * Pixi có sẵn `CullerPlugin` tự cắt **toàn bộ** `app.stage` trước mỗi lần
   * render, nhưng bật nó đòi `extensions.add(CullerPlugin)` — một đăng ký
   * **toàn cục**, áp dụng cho mọi `Application` được tạo sau đó trong tiến
   * trình, không riêng gì `WorldView`. Gọi thẳng `Culler.shared.cull(...)` ở
   * đây cắt đúng hai lớp cần cắt (`entityLayer`, `labelLayer` — những lớp có
   * thể có hàng chục đối tượng ngoài khung nhìn cùng lúc) mà không đụng tới
   * `terrainLayer`/`pathLayer`/`diffLayer`, vốn chỉ có một `Graphics`/`Sprite`
   * mỗi lớp nên cắt chúng không tiết kiệm được gì, chỉ thêm một lần tính bounds
   * toàn cục vô ích.
   *
   * Thứ tự chạy đúng vì `app.ticker` gọi các listener theo `priority`:
   * `TickerPlugin` của Pixi tự đăng ký lệnh render ở mức `UPDATE_PRIORITY.LOW`
   * (`-25`), còn `onTick` đăng ký không kèm priority — tức `NORMAL` (`0`),
   * cao hơn `LOW`. `onTick` luôn chạy xong, đặt cờ `culled` cho khung hình này,
   * rồi renderer mới đọc cờ đó — không có cách nào cờ bị đọc trước khi được
   * đặt trong cùng một khung hình.
   */
  private readonly onTick = (): void => {
    const now = performance.now();
    this.syncEntityPositions(now);
    // Vẽ lại khói/cỏ lay/lấp lánh mỗi khung hình, không đợi `retint`/
    // `setTerrain` (những hàm đó chỉ chạy theo nhịp hỏi server ~400ms) — xem
    // lời giải "vì sao module này tồn tại tách khỏi ambient.ts" ở
    // `liveliness.ts`. Đây vẫn là `app.ticker` đã có, không phải một
    // `requestAnimationFrame` thứ hai.
    this.redrawLiveliness(now);
    const app = this.app;
    if (!app) return;
    // `renderer.screen` là khung nhìn tính bằng pixel **logic** (đã chia cho
    // `resolution`) — cùng đơn vị với toạ độ toàn cục của mọi sprite trong
    // stage, vì `reposition()` cũng đặt vị trí layer bằng đúng đơn vị đó.
    Culler.shared.cull(this.entityLayer, app.renderer.screen);
    Culler.shared.cull(this.labelLayer, app.renderer.screen);
  };

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
    app.stage.addChild(this.livelinessLayer);
    app.stage.addChild(this.pathLayer);
    app.stage.addChild(this.shadowLayer);
    this.shadowLayer.addChild(this.shadowGraphics);
    app.stage.addChild(this.entityLayer);
    app.stage.addChild(this.labelLayer);
    app.stage.addChild(this.diffLayer);
    // `cullableChildren` cho phép `Culler` đi xuống từng con để đọc cờ
    // `cullable` của chính nó (mặc định `false` trên mọi đối tượng, xem
    // `cullingMixin` của Pixi) — đặt trên container thôi thì con vẫn luôn vẽ.
    // Từng `Graphics`/`Text` được bật `cullable = true` khi tạo, trong
    // `setEntities`/`redrawLabels`.
    this.entityLayer.cullableChildren = true;
    this.labelLayer.cullableChildren = true;
    this.app = app;
    // Chỉ đặt lại toạ độ sprite theo `MotionTrack` — không dựng lại `Graphics`.
    // Dựng lại mỗi khung hình là đổi lỗi "giật vì đứng yên 400ms" lấy lỗi
    // "vẽ lại toàn bộ hình 60 lần/giây", vốn còn tốn hơn.
    app.ticker.add(this.onTick);
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
    // Đổi **ngay trên màn hình**, không đợi một vòng mạng nào.
    //
    // Đây là chỗ đã gây ra "lăn chuột thì lag": bản trước để `App.vue` gọi
    // `refresh()` sau mỗi nấc lăn, tức bốn round-trip HTTP nối đuôi nhau cộng
    // một lần vẽ lại toàn bộ texture — cho **mỗi** sự kiện bánh xe, mà bánh xe
    // bắn khoảng hai mươi sự kiện mỗi giây.
    //
    // Phóng to là một phép biến hình của **khung nhìn**, không phải một câu hỏi
    // về thế giới: cùng lô ô đó, vẽ to hơn. Nên nó thuộc về đây.
    this.rescale();
    return true;
  }

  /**
   * Chỉ đổi sắc trời theo nhịp, không đụng tới texture nền.
   *
   * Ngày đêm là một phép nhân màu trên **một** sprite — rẻ tới mức làm mỗi
   * khung hình cũng được. Vẽ lại cả lô ô chỉ vì mặt trời nhích một chút là trả
   * giá bốn nghìn ô cho một thay đổi mà GPU làm được miễn phí.
   */
  retint(tick: number): void {
    this.currentTick = tick;
    if (this.terrainSprite) this.terrainSprite.tint = skyTint(tick);
    this.redrawAmbient(tick);
  }

  /** Áp lại cỡ ô lên mọi lớp mà không hỏi lại dữ liệu. */
  private rescale(): void {
    const b = this.batch;
    if (b && this.terrainSprite) {
      this.terrainSprite.width = b.w * this.tileSize;
      this.terrainSprite.height = b.h * this.tileSize;
    }
    // Đổi mức phóng là đúng lúc `nearest`/`linear` phải đổi theo — xem
    // chế độ lọc. Gọi ở đây, không đợi lần `setTerrain` kế
    // tiếp, để hết răng cưa ngay trên khung hình vừa lăn chuột, không phải sau
    // vòng hỏi server 400ms kế tiếp.
    this.applyTerrainFilter();
    if (b && this.overlaySprite) {
      this.overlaySprite.width = b.w * this.tileSize;
      this.overlaySprite.height = b.h * this.tileSize;
    }
    // Hình thực thể vẽ theo cỡ ô, nên chúng phải dựng lại — nhưng đó là vài
    // chục `Graphics`, không phải bốn nghìn ô, và nó chạy đồng bộ trong khung
    // hình này chứ không sau một vòng mạng.
    this.setEntities(this.lastEntities);
    this.redrawPath();
    this.redrawDiff();
    this.reposition();
  }

  /** Kích thước khung nhìn tính bằng ô, cộng một viền để không lộ mép. */
  viewportTiles(): { w: number; h: number } {
    const app = this.app;
    if (!app) return { w: 33, h: 33 };
    const px = app.renderer.resolution * this.tileSize;
    return {
      w: Math.max(9, Math.ceil(app.renderer.width / px) + 2 * BATCH_MARGIN),
      h: Math.max(9, Math.ceil(app.renderer.height / px) + 2 * BATCH_MARGIN),
    };
  }

  /**
   * Lô ô hiện có còn phủ hết khung nhìn quanh `(cx, cy)` không.
   *
   * Đây là câu hỏi quyết định một cú kéo chuột có phải trả giá bằng một vòng
   * mạng hay không. Vì lô được lấy rộng hơn khung nhìn `BATCH_MARGIN` ô mỗi
   * phía, phần lớn cú kéo trượt **bên trong** dữ liệu đã có, và câu trả lời
   * đúng là "không cần hỏi lại gì cả".
   */
  covers(cx: number, cy: number): boolean {
    const b = this.batch;
    if (!b) return false;
    const { w, h } = this.viewportTiles();
    const needL = cx - (w >> 1) + BATCH_MARGIN;
    const needT = cy - (h >> 1) + BATCH_MARGIN;
    const needR = needL + w - 2 * BATCH_MARGIN;
    const needB = needT + h - 2 * BATCH_MARGIN;
    return needL >= b.x && needT >= b.y && needR <= b.x + b.w && needB <= b.y + b.h;
  }

  /** Mức phóng hiện tại, để chỗ gọi nhớ lại sau khi nạp lô mới. */
  zoomLevel(): number {
    return this.zoomIndex;
  }

  setCenter(x: number, y: number): void {
    this.centerX = x;
    this.centerY = y;
    this.reposition();
  }

  /**
   * Nạp một lô ô. Chỉ vẽ lại texture nền khi nội dung thật sự đổi (xem
   * `fingerprintOf`); `tint` ngày/đêm thì luôn áp lại vì nó rẻ.
   */
  setTerrain(batch: TileBatch, tick: number): void {
    this.batch = batch;
    this.currentTick = tick;

    const fp = fingerprintOf(batch);
    if (fp !== this.terrainFingerprint) {
      this.terrainFingerprint = fp;
      this.repaintTerrain(batch);
    }

    if (this.terrainSprite) {
      this.terrainSprite.width = batch.w * this.tileSize;
      this.terrainSprite.height = batch.h * this.tileSize;
      // Ngày đêm áp bằng `tint` trên sprite có sẵn: không phải vẽ lại texture,
      // và nó cũng không chạm vào màu vật liệu đã bake trong đó.
      this.terrainSprite.tint = skyTint(tick);
    }
    this.redrawAmbient(tick);
    this.redrawPath();
    this.redrawDiff();
    this.reposition();
  }

  /**
   * Vẽ lại texture nền lên canvas tái dùng, rồi báo GPU cập nhật tại chỗ.
   *
   * ## Một canvas, một texture, sống suốt vòng đời `WorldView`
   *
   * Bản trước tạo một `<canvas>` và một `Texture` mới **mỗi lần gọi** — hai
   * lần cấp phát lớn cộng một lần tải GPU, 2.5 lần/giây theo nhịp `refresh()`
   * của `App.vue`, cho một tấm ảnh gần như không đổi. Ở đây canvas chỉ được
   * tạo lại khi kích thước lô đổi (đổi khung nhìn/resize cửa sổ — hiếm), còn
   * lại thì vẽ đè lên canvas cũ và gọi `TextureSource.update()` để báo GPU
   * "nội dung đổi rồi, tải lại đi" mà không cấp phát texture mới. Đây là cách
   * Pixi 8 phơi ra cho đúng nhu cầu "cập nhật tại chỗ" — không có API
   * `resize`-nội-dung nào rẻ hơn cho một `CanvasSource`.
   */
  private repaintTerrain(batch: TileBatch): void {
    if (!this.terrainCanvas || this.terrainCanvas.width !== batch.w || this.terrainCanvas.height !== batch.h) {
      this.terrainCanvas = document.createElement("canvas");
      this.terrainCanvas.width = batch.w;
      this.terrainCanvas.height = batch.h;
      this.terrainCtx = this.terrainCanvas.getContext("2d");
      // Đổi kích thước nghĩa là canvas cũ không còn dùng được — đây là nhánh
      // hiếm (không chạy mỗi 400ms) nên vẫn trả giá một texture mới ở đây,
      // đúng như bản cũ làm cho *mọi* lần gọi.
      this.terrainTexture?.destroy(true);
      this.terrainTexture = Texture.from(this.terrainCanvas);
      // `autoGenerateMipmaps`: mỗi lần `source.update()` báo nội dung đổi (vài
      // dòng dưới), Pixi tự tính lại các mip level qua GPU — không cần tự gọi
      // `updateMipmaps()`. Đặt cờ này một lần ở đây là đủ cho suốt vòng đời
      // texture; `applyTerrainFilter()` sau đó chỉ còn việc chọn `scaleMode`
      // theo mức phóng.
      // **Không** bật mipmap cho texture nền, và điều đó không phải một thiếu
      // sót. Texture này có đúng **một texel mỗi ô**, và cỡ ô nhỏ nhất là 4 px
      // (`ZOOM_STEPS`) — nên nó luôn được **phóng to**, chưa bao giờ thu nhỏ.
      // Mipmap chỉ giúp khi thu nhỏ; ở đây nó là bộ nhớ và một tầng lấy mẫu
      // thừa, cộng một cách để hình bị nhòe mà không ai giải thích được.
      this.terrainTexture.source.autoGenerateMipmaps = false;
      this.applyTerrainFilter();
    }

    const ctx = this.terrainCtx;
    if (!ctx || !this.terrainTexture) return;

    const rgba = paintTerrain(batch, this.palette);
    // `ctx.createImageData` rồi `set` thay vì `new ImageData(rgba, w, h)`: kiểu
    // `ImageDataArray` của lib DOM không nhận thẳng `Uint8ClampedArray` ở mọi
    // phiên bản TypeScript, và ép kiểu ở đây sẽ giấu mất một lỗi thật nếu sau
    // này buffer đổi kiểu.
    const img = ctx.createImageData(batch.w, batch.h);
    img.data.set(rgba);
    ctx.putImageData(img, 0, 0);

    // Cập nhật tại chỗ: không tạo texture mới, không tải lại toàn bộ pipeline
    // ràng buộc GPU (bind group, v.v.) như một `Texture.from` mới sẽ kéo theo.
    // Vì `autoGenerateMipmaps` đã bật, lệnh này cũng kéo theo việc GPU tính
    // lại mip level cho đúng nội dung mới — ảnh cũ không bị "bóng ma" ở các
    // mip xa sau khi khắc một ô.
    this.terrainTexture.source.update();

    if (!this.terrainSprite) {
      this.terrainSprite = new Sprite(this.terrainTexture);
      this.terrainLayer.addChild(this.terrainSprite);
    } else if (this.terrainSprite.texture !== this.terrainTexture) {
      this.terrainSprite.texture = this.terrainTexture;
    }
  }

  /**
   * Chọn `nearest` hay `linear` cho texture nền theo mức phóng hiện tại — xem
   * đánh đổi đầy đủ ở `TERRAIN_LINEAR_MAX_TILE_SIZE`.
   *
   * Gọi lại mỗi khi `tileSize` đổi (`rescale()`), không chỉ một lần lúc tạo
   * texture: người chơi lăn chuột qua lại giữa các mức phóng suốt phiên chơi,
   * và bộ lọc phải theo kịp chứ không đứng yên ở trạng thái lúc tải trang.
   */
  private applyTerrainFilter(): void {
    const source = this.terrainTexture?.source;
    if (!source) return;
    // Luôn `nearest`. Mỗi texel là một ô, và ranh giới giữa hai vật liệu là
    // **thông tin**, không phải răng cưa: nội suy nó ra một màu ở giữa là dựng
    // lại đúng màu bùn mà `minimap.ts` đã phải tránh, chỉ ở tầng khác. Một
    // ngưỡng đổi sang `linear` khi thu nhỏ nghe hợp lý nhưng sai ở đây: thứ bị
    // làm mượt không phải pixel, mà là ranh giới vật liệu.
    source.scaleMode = "nearest";
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
   * Vẽ lại lớp "sự sống": khói bếp, cỏ/lúa lay, mặt nước lấp lánh.
   *
   * Chạy mỗi khung hình (gọi từ `onTick`, xem lời giải ở đó) — khác
   * `redrawAmbient` vốn chỉ chạy theo nhịp `tick`. `nowMs` là đồng hồ thật,
   * dùng cho phần hoạt hình mượt (`sway`, `smokePuff` — xem lời giải "vì sao
   * module này tồn tại tách khỏi ambient.ts" ở `liveliness.ts`); `this.currentTick`
   * là `tick` thế giới gần nhất, dùng cho phần quyết định trạng thái
   * ("ống khói nào đang cháy" — một sự thật của thế giới, không phải của đồng
   * hồ máy người chơi).
   *
   * Một `Graphics` duy nhất cho cả ba hiệu ứng, cùng khuôn `redrawAmbient`, và
   * cùng chia sẻ một trần cứng `LIVELINESS_BUDGET` (xem hằng số) qua biến đếm
   * `budget` — một khung nhìn toàn ruộng lúa hay toàn mặt hồ đều dừng ở đúng
   * ngân sách đó, không phải ba trần riêng cộng dồn vô điều kiện.
   */
  private redrawLiveliness(nowMs: number): void {
    for (const c of this.livelinessLayer.removeChildren()) c.destroy();
    const batch = this.batch;
    if (!batch) return;

    const ts = this.tileSize;
    // Cùng ngưỡng và cùng lý do `redrawAmbient` tắt hạt môi trường: dưới đây
    // biên độ vài pixel nhỏ hơn một ô màn hình và chỉ còn là nhiễu.
    if (ts < LIVELINESS_MIN_TILE_SIZE) return;

    const g = new Graphics();
    let budget = LIVELINESS_BUDGET;

    // ── Khói bếp ───────────────────────────────────────────────────────────
    for (const chim of chimneys(batch, this.currentTick, CHIMNEY_TILE_LIMIT)) {
      if (budget <= 0) break;
      // Lệch pha khởi động riêng cho từng ống khói, tái dùng `phaseAt` thay vì
      // viết thêm một hàm băm mới trong file vẽ — xem lời giải `SMOKE_SLOTS`.
      const tileOffsetMs = (phaseAt(chim.x, chim.y) / TAU) * SMOKE_CYCLE_MS;
      for (let k = 0; k < SMOKE_SLOTS; k++) {
        if (budget <= 0) break;
        const age = safeModWorld(nowMs - tileOffsetMs - k * SMOKE_SPACING_MS, SMOKE_CYCLE_MS);
        const seed = (chim.x * 92_821 + chim.y * 68_917 + k * 104_729) | 0;
        // `null` nghĩa là khe này đang ở quãng nghỉ giữa hai hạt (`age` vượt
        // `SMOKE_LIFE_MS`, xem `liveliness.smokePuff`) — không phải lỗi.
        const puff = smokePuff(seed, age);
        if (!puff) continue;
        const cx = (chim.x - batch.x + 0.5 + puff.dx) * ts;
        // 0.1 ô: miệng ống khói nằm gần mép trên của ô mái, không phải giữa ô.
        const cy = (chim.y - batch.y + 0.1 + puff.dy) * ts;
        g.circle(cx, cy, Math.max(0.6, puff.r * ts)).fill({ color: 0xd7d2c8, alpha: puff.alpha });
        budget--;
      }
    }

    // ── Lúa/cỏ lay ─────────────────────────────────────────────────────────
    if (budget > 0) {
      const candidates: { gx: number; gy: number; off: number; rank: number }[] = [];
      for (let gy = 0; gy < batch.h; gy++) {
        for (let gx = 0; gx < batch.w; gx++) {
          const i = gy * batch.w + gx;
          const material = batch.material[i] ?? "air";
          const visible = material !== "air" ? material : (batch.surface[i] ?? "air");
          if (visible !== "crop_green") continue;
          const wx = batch.x + gx;
          const wy = batch.y + gy;
          // `sway` tự cổng ~15% ô; giá trị đúng bằng 0 nghĩa là ô này chưa từng
          // được chọn (hoặc đang đúng điểm cân bằng — hai trường hợp không
          // phân biệt được bằng mắt, xem lời giải ở `liveliness.ts`).
          const off = sway(wx, wy, nowMs);
          if (off === 0) continue;
          candidates.push({ gx, gy, off, rank: phaseAt(wx, wy) });
        }
      }
      if (candidates.length > CROP_TUFT_LIMIT) {
        // Tỉa theo hạng băm, không theo thứ tự quét — cùng lý do `ambient.ts`
        // dùng cho hạt môi trường: cắt đuôi mảng sẽ luôn xoá sạch nửa dưới
        // khung nhìn trước.
        candidates.sort((a, b) => a.rank - b.rank);
        candidates.length = CROP_TUFT_LIMIT;
      }
      for (const c of candidates) {
        if (budget <= 0) break;
        const baseX = (c.gx + 0.5) * ts;
        const baseY = (c.gy + 0.78) * ts;
        const topX = baseX + c.off;
        const topY = (c.gy + 0.35) * ts;
        g.moveTo(baseX, baseY)
          .lineTo(topX, topY)
          .stroke({ width: Math.max(0.6, ts * 0.05), color: CROP_TUFT_COLOR, alpha: 0.55 });
        budget--;
      }
    }

    // ── Mặt nước lấp lánh ──────────────────────────────────────────────────
    if (budget > 0) {
      const candidates: { gx: number; gy: number; shine: number; rank: number }[] = [];
      for (let gy = 0; gy < batch.h; gy++) {
        for (let gx = 0; gx < batch.w; gx++) {
          const i = gy * batch.w + gx;
          const material = batch.material[i] ?? "air";
          const visible = material !== "air" ? material : (batch.surface[i] ?? "air");
          const wet = this.palette.isLiquid(visible) || (batch.river[i] ?? 0) === 1;
          if (!wet) continue;
          const wx = batch.x + gx;
          const wy = batch.y + gy;
          const p = phaseAt(wx, wy) / TAU;
          // Răng cưa: mỗi ô "sáng" đúng `WATER_GLINT_WINDOW` phần chu kỳ của
          // nó, lệch pha riêng — kết quả là một dải sáng chạy chậm qua mặt
          // nước, không phải cả mặt nước nhấp nháy cùng lúc.
          const cyc = safeModWorld(nowMs / WATER_GLINT_PERIOD_MS + p, 1);
          if (cyc >= WATER_GLINT_WINDOW) continue;
          const k = cyc / WATER_GLINT_WINDOW;
          const shine = Math.sin(Math.PI * k);
          candidates.push({ gx, gy, shine, rank: p });
        }
      }
      if (candidates.length > WATER_GLINT_LIMIT) {
        candidates.sort((a, b) => a.rank - b.rank);
        candidates.length = WATER_GLINT_LIMIT;
      }
      for (const c of candidates) {
        if (budget <= 0) break;
        const cx = (c.gx + 0.5) * ts;
        const cy = (c.gy + 0.5) * ts;
        g.ellipse(cx, cy, ts * 0.3, ts * 0.1).fill({ color: 0xeaf6ff, alpha: 0.35 * c.shine });
        budget--;
      }
    }

    this.livelinessLayer.addChild(g);
  }

  /**
   * Vẽ lại lớp thực thể.
   *
   * Hình dạng — thân, đầu, dụng cụ, dấu ý định — tính ở `figure.ts`, hàm
   * thuần và có test riêng. Ở đây chỉ còn việc của một nơi vẽ: cắt theo khung
   * nhìn, sắp thứ tự chồng lớp, và nhớ vị trí trước để suy hướng nhìn.
   *
   * ## Một `Graphics` sống suốt vòng đời một thực thể
   *
   * Bản trước phá hết `Graphics` cũ và dựng lại toàn bộ mỗi lần gọi — hợp lý
   * khi `setEntities` chạy mỗi 400 ms, vô lý nếu chạy mỗi khung hình. Ở đây
   * `entityGraphics` giữ một `Graphics` cho mỗi `id` qua nhiều lần gọi; nội
   * dung (`drawFigure`) chỉ vẽ lại khi `FigureSpec` hoặc cỡ ô đổi
   * (`figureSpecEqual`), còn vị trí thì `syncEntityPositions` đặt lại mỗi
   * khung hình qua `g.position`, tách hẳn khỏi nội dung đã vẽ.
   */
  setEntities(entities: Entity[]): void {
    this.lastEntities = entities;
    const now = performance.now();
    const stillHere = new Set(entities.map((e) => e.id));

    // Dọn vị trí của thực thể đã biến mất — chết, rời server, hay bị gộp vào
    // một sự kiện khác. Không dọn thì các map này phình vô hạn suốt đời phiên
    // chơi, dù bản đồ tại một thời điểm chỉ có vài chục thực thể.
    for (const id of this.prevPos.keys()) {
      if (!stillHere.has(id)) this.prevPos.delete(id);
    }
    this.motion.retain(stillHere);
    for (const [id, g] of this.entityGraphics) {
      if (!stillHere.has(id)) {
        g.destroy();
        this.entityGraphics.delete(id);
        this.entitySpecs.delete(id);
      }
    }

    const batch = this.batch;
    if (!batch) return;

    const ts = this.tileSize;
    // Đổi zoom đổi kích thước mọi hình dù `FigureSpec` không đổi một trường
    // nào — `figureSpecEqual` không thấy được việc này nên phải kiểm riêng.
    const sizeChanged = ts !== this.entityTileSize;
    this.entityTileSize = ts;

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
      // Cũng cập nhật trước khi cắt: một thực thể ngoài màn hình vẫn phải giữ
      // đúng vị trí quyền uy, để nếu nó lọt lại vào khung nhìn thì `MotionTrack`
      // biết đường snap/trượt đúng, không tính từ một chỗ đã lỗi thời.
      this.motion.update(e.id, e.x, e.y, now);

      const gx = e.x - batch.x;
      const gy = e.y - batch.y;
      const visible = gx >= 0 && gy >= 0 && gx < batch.w && gy < batch.h;
      const existing = this.entityGraphics.get(e.id);

      if (!visible) {
        // Ngoài khung nhìn: gỡ khỏi tầng vẽ nhưng KHÔNG hủy — thực thể vẫn
        // "còn sống", chỉ đang ở ngoài màn hình, và hủy rồi dựng lại tốn đúng
        // chi phí mà cả cơ chế tái dùng này sinh ra để tránh.
        if (existing?.parent) this.entityLayer.removeChild(existing);
        continue;
      }

      const spec = figureOf(e, prev);
      const cached = this.entitySpecs.get(e.id);
      const needsRedraw = !existing || sizeChanged || !cached || !figureSpecEqual(cached, spec);

      const g = existing ?? new Graphics();
      if (!existing) {
        // Bật culling từng thực thể một: xem lời giải "vì sao gọi `Culler` tay"
        // ở `onTick`. Không set trên `entityLayer` (nó đã đúng bằng cả khung
        // nhìn hầu hết thời gian, cắt cả container chẳng tiết kiệm được gì).
        g.cullable = true;
        this.entityGraphics.set(e.id, g);
      }
      if (needsRedraw) {
        g.clear();
        // Vẽ quanh gốc cục bộ `(0, 0)`: vị trí thật gán bằng `g.position` ở
        // `syncEntityPositions`, không bake vào lệnh vẽ. Nhờ vậy trượt giữa
        // hai ô là một phép gán toạ độ mỗi khung hình, không phải một lần vẽ
        // lại `Graphics` mỗi khung hình.
        drawFigure(g, spec, 0, 0, ts);
        this.entitySpecs.set(e.id, spec);
      }
      // `addChild` trên một con đã thuộc đúng container này chỉ dời nó lên
      // cuối danh sách vẽ — dùng để vừa đảm bảo nó có mặt (trường hợp vừa từ
      // ngoài màn hình quay lại) vừa giữ đúng thứ tự chồng lớp theo `sorted`.
      this.entityLayer.addChild(g);
    }

    this.syncEntityPositions(now);
    this.reposition();
  }

  /**
   * Đặt lại `g.position` của mọi sprite thực thể đang hiện theo `MotionTrack`,
   * thêm nhịp đi khi đang trượt ô, và vẽ lại bóng tiếp xúc của chúng.
   *
   * Chạy mỗi khung hình (`app.ticker`) nhưng KHÔNG đụng tới `Graphics` đã vẽ —
   * chỉ một phép gán toạ độ/góc xoay mỗi thực thể hiện có trên màn hình, rẻ
   * hơn hẳn dựng lại hình học 60 lần/giây.
   *
   * ## Vì sao nhịp đi và bóng nằm ở đây, không phải trong `figure.ts`
   *
   * `figureOf`/`drawFigure` chỉ chạy lại khi `FigureSpec` đổi (đứng yên phần
   * lớn thời gian, xem `setEntities`) — đúng cho hình dạng, sai cho một hoạt
   * ảnh phải mượt mỗi khung hình. Nhịp đi và bóng vì vậy là phép biến hình đặt
   * **lên trên** hình đã vẽ (`g.position`/`g.rotation`, và một `Graphics` bóng
   * riêng), không phải nội dung của chính hình đó.
   *
   * ## "Đang đi" nghĩa là gì khi không có state máy trạng thái
   *
   * Không có cờ "đang đi" nào từ server hay từ `MotionTrack`. Nhưng
   * `MotionTrack.at()` chỉ khác vị trí quyền uy (`Entity.x/y`) đúng lúc đang
   * trượt giữa hai ô — `ease(1) = 1` nên khi trượt xong, toạ độ nội suy khớp
   * số nguyên với đích tuyệt đối. So sánh hai số đó là đủ để biết "đang đi" mà
   * không cần `MotionTrack` phơi thêm state riêng của nó.
   */
  private syncEntityPositions(nowMs: number): void {
    const batch = this.batch;
    this.shadowGraphics.clear();
    if (!batch) return;
    const ts = this.tileSize;
    // Dưới ngưỡng phóng, nhịp đi (biên độ tuyệt đối, không theo `tileSize`) và
    // bóng lệch vài pixel đều nhỏ hơn một ô — tắt hẳn, cùng lý do
    // `redrawLiveliness` tắt khói/cỏ/lấp lánh ở ngưỡng này.
    const microFx = ts >= LIVELINESS_MIN_TILE_SIZE;
    const targets = new Map(this.lastEntities.map((e) => [e.id, e] as const));

    for (const [id, g] of this.entityGraphics) {
      if (!g.parent) continue; // ngoài khung nhìn: không hiện, không cần đặt lại vị trí
      const m = this.motion.at(id, nowMs);
      if (!m) continue;
      const baseX = (m.x - batch.x) * ts + ts / 2;
      const baseY = (m.y - batch.y) * ts + ts / 2;
      const target = targets.get(id);
      const walking = microFx && target !== undefined && (m.x !== target.x || m.y !== target.y);

      if (walking) {
        // Lệch pha theo `id`, tái dùng `stringHash` đã có cho `fingerprintOf`:
        // hai thực thể cùng đi một lúc không được nảy cùng nhịp, nếu không cả
        // đoàn người trông như một đội diễu hành, không phải từng người đi.
        const idPhase = ((stringHash(id) >>> 0) / 4_294_967_296) * TAU;
        const wobble = Math.sin((nowMs / WALK_BOB_PERIOD_MS) * TAU + idPhase);
        const facing = this.entitySpecs.get(id)?.facing ?? "right";
        const dir = facing === "right" ? 1 : -1;
        // `Math.abs`: chân nảy lên rồi chạm đất, không lún xuống dưới mặt đất
        // — hai cú nảy mỗi chu kỳ sin, khớp nhịp hai bước chân trái/phải.
        g.position.set(baseX, baseY - Math.abs(wobble) * WALK_BOB_AMPLITUDE);
        g.rotation = (wobble * WALK_TILT_DEG * dir * Math.PI) / 180;
      } else {
        g.position.set(baseX, baseY);
        g.rotation = 0;
      }

      if (microFx) {
        // Neo bóng vào đúng điểm `drawFigure` (`figure.ts`) đã đặt bóng tiếp
        // đất cũ (`cy + ts*0.3`), rồi lệch thêm theo hướng nắng của
        // `terrain.ts` — hai bóng cộng lại đọc ra một bóng mềm có lõi đậm
        // (bóng cũ) và phần đổ dài theo nắng (bóng này), xem lời giải ở khai
        // báo `shadowLayer`.
        const scale = walking ? SHADOW_STRIDE_SCALE : 1;
        const sx = baseX + SHADOW_DIR_X * ts * 0.16;
        const sy = baseY + ts * 0.3 + SHADOW_DIR_Y * ts * 0.16;
        this.shadowGraphics
          .ellipse(sx, sy, ts * 0.34 * scale, ts * 0.13 * scale)
          .fill({ color: 0x000000, alpha: 0.22 });
      }
    }
  }

  /**
   * Đặt danh sách nhãn tên. `App.vue` gọi hàm này thay cho việc tự tay đặt
   * `<div>` lên canvas như trước — xem lời giải "Nhãn giờ nằm trong Pixi" ở
   * đầu file.
   *
   * Chỉ lưu **toàn bộ** danh sách rồi giao việc lọc cho `redrawLabels`/
   * `visibleLabels`: hàm này không tự quyết nhãn nào đáng hiện, để luật đó nằm
   * đúng một chỗ (`nameplate.ts`) thay vì lặp lại ở đây.
   */
  setLabels(list: Nameplate[]): void {
    this.lastLabels = list;
    this.redrawLabels();
  }

  /**
   * Vẽ lại lớp nhãn tên theo danh sách `lastLabels` đã lọc qua `visibleLabels`.
   *
   * ## Vì sao `PIXI.Text`, không phải `BitmapText`
   *
   * `BitmapText` nhanh hơn, nhưng đòi cài trước một tập ký tự cố định qua
   * `BitmapFont.install({ chars: [...] })`. Tên trong thế giới này không phải
   * chữ La-tinh trần — coi `figure.test.ts` (`"Ai đó"`) hay
   * `overlays/field.test.ts` (`"táo"`): tiếng Việt có vài chục dấu thanh và
   * nguyên âm có dấu, và một bộ ký tự liệt kê tay rất dễ bỏ sót một dấu hiếm
   * gặp — mà khi bỏ sót, ký tự đó không vẽ ra gì cả, lặng lẽ, không có lỗi nào
   * để bắt. `Text` vẽ qua canvas font hệ thống nên nhận đúng mọi ký tự Unicode
   * font hệ điều hành có, không cần liệt kê trước.
   *
   * Cái giá đổi lại — mỗi chuỗi một texture riêng — không còn là vấn đề ở đây:
   * `visibleLabels` đã chặn số nhãn ở `DEFAULT_LABEL_LIMIT`, và mỗi `id` chỉ
   * giữ một `Text` sống suốt vòng đời của nó (`labelObjects`), giống hệt
   * `entityGraphics`. Số texture tối đa tại một thời điểm có trần, không phụ
   * thuộc thế giới đông cỡ nào.
   *
   * ## Chỉ viết lại `.text`/style khi đổi, luôn viết lại `.position`
   *
   * Cùng khuôn với `setEntities`: dựng `Text` (viết `.text` mới) là chỗ đắt,
   * tốn một lần vẽ canvas + tải texture lên GPU; đặt `.position` là một phép
   * gán số, gần như miễn phí. Nên `.text` và style chỉ đổi khi nội dung thật
   * sự khác `labelCache`, còn `.position` được gán lại vô điều kiện mỗi lần
   * hàm này chạy — kể cả khi tên và trạng thái `highlight` không đổi gì.
   */
  private redrawLabels(): void {
    const batch = this.batch;
    if (!batch) {
      for (const t of this.labelLayer.removeChildren()) t.destroy();
      this.labelObjects.clear();
      this.labelCache.clear();
      return;
    }

    const visible = visibleLabels(this.lastLabels, {
      tileSize: this.tileSize,
      viewport: batch,
      centerX: this.centerX,
      centerY: this.centerY,
    });

    const stillHere = new Set(visible.map((l) => l.id));
    for (const [id, t] of this.labelObjects) {
      if (!stillHere.has(id)) {
        t.destroy();
        this.labelObjects.delete(id);
        this.labelCache.delete(id);
      }
    }

    const ts = this.tileSize;
    for (const l of visible) {
      const existing = this.labelObjects.get(l.id);
      const cached = this.labelCache.get(l.id);
      const t =
        existing ??
        new Text({
          text: l.text,
          style: labelStyle(l.highlight),
          resolution: this.labelResolution,
          // Neo giữa theo chiều ngang, đáy theo chiều dọc: `.position` dưới
          // đây đặt đúng điểm ngay trên đầu ô, chữ mọc lên từ đó.
          anchor: { x: 0.5, y: 1 },
        });

      if (!existing) {
        t.cullable = true; // xem lời giải "vì sao gọi `Culler` tay" ở `onTick`.
        this.labelObjects.set(l.id, t);
      } else {
        if (!cached || cached.text !== l.text) t.text = l.text;
        if (!cached || cached.highlight !== l.highlight) t.style = labelStyle(l.highlight);
      }
      this.labelCache.set(l.id, { text: l.text, highlight: l.highlight });
      // `addChild` trên một con đã thuộc đúng container này chỉ dời nó lên
      // cuối danh sách vẽ — cùng lý do dùng nó ở `setEntities`: vừa đảm bảo có
      // mặt (trường hợp mới tạo, hoặc vừa quay lại sau khi `visibleLabels`
      // từng loại nó ra) vừa giữ thứ tự vẽ ổn định.
      this.labelLayer.addChild(t);

      // Cùng công thức `screenOf` từng dùng cho nhãn HTML: giữa ô theo chiều
      // ngang, nhô lên trên đầu nhân vật theo chiều dọc — giữ nguyên cảm giác
      // hình ảnh cũ dù nhãn giờ sống trong Pixi.
      t.position.set((l.x - batch.x) * ts + ts / 2, (l.y - batch.y) * ts - ts * 0.45);
    }
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

  /**
   * Đặt mọi lớp sao cho ô tâm nằm giữa màn hình, rồi vẽ lại nhãn tên.
   *
   * `redrawLabels()` đứng ở cuối hàm này — không phải trong `setLabels` — vì
   * "nhãn nào đáng hiện" phụ thuộc `tileSize` và khung nhìn hiện tại
   * (`visibleLabels`), và cả hai thứ đó đổi đúng những lúc `reposition()`
   * chạy: kéo camera (`setCenter`), phóng to/nhỏ (`rescale`), nạp lô mới
   * (`setTerrain`). Đặt nó ở đây nghĩa là zoom ra tới ngưỡng ẩn nhãn có tác
   * dụng ngay trên khung hình kế tiếp, không phải đợi `App.vue` gọi lại
   * `setLabels` ở lần hỏi server sau.
   */
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
    this.livelinessLayer.position.set(x, y);
    this.pathLayer.position.set(x, y);
    this.shadowLayer.position.set(x, y);
    this.entityLayer.position.set(x, y);
    this.labelLayer.position.set(x, y);
    this.diffLayer.position.set(x, y);
    this.redrawLabels();
  }

  destroy(): void {
    // Gỡ khỏi ticker trước khi phá `app`: `app.destroy()` cũng dừng ticker của
    // nó, nhưng gỡ tường minh ở đây là chỗ duy nhất khẳng định "vòng lặp đặt
    // lại vị trí thực thể không còn sống sau khi component đã tháo" — không
    // phải suy luận ngầm từ thứ tự bên trong Pixi.
    this.app?.ticker.remove(this.onTick);
    this.overlayTexture?.destroy(true);
    this.terrainTexture?.destroy(true);
    this.app?.destroy(true, { children: true });
    this.app = null;
  }
}
