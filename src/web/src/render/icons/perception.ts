/**
 * Biểu tượng tuân thủ tri giác (`idea.md §18.14.5`, `PC-18`).
 *
 * > Đây là chỗ dễ rò nhất, cùng loại với `§18.9` và `§8.10.3`.
 *
 * ## Vì sao rò ở đây khó thấy hơn rò ở panel
 *
 * Một panel hiện sai thì có người đọc và có người thắc mắc. Một **icon** hiện
 * sai thì không ai đọc — người chơi chỉ *biết*, bằng liếc mắt, rằng cái rương
 * kia là đồ gian hay thanh kiếm kia bị nguyền. Họ không bao giờ hình thành câu
 * hỏi "sao mình biết điều này?", nên họ không bao giờ báo lỗi. Thông tin đã rò,
 * gameplay đã hỏng, và không có ai phàn nàn.
 *
 * Vì vậy module này đứng **trước** [`compose`]: nó biến một đặc tả *đầy đủ*
 * thành một đặc tả *được phép thấy*, và chỗ dựng icon không có cách nào bỏ qua
 * nó ngoài việc cố tình.
 *
 * ```text
 *  sự thật đầy đủ ──► gate(spec, viewer) ──► IconSpec ──► compose ──► atlas
 *                     ▲
 *                     └── chưa thẩm định → dấu hỏi
 *                         phép ẩn không perceptible → bỏ hẳn
 *                         cải trang → bóng của lớp cải trang
 *                         claim chưa biết → bỏ dấu "đồ gian"
 * ```
 *
 * ## Dấu hỏi chỉ có **một** nghĩa
 *
 * `§18.14.5` dùng dấu hỏi cho "chưa thẩm định". `contentkit` đã ghi chú rằng lỗi
 * hợp thành **không** được vẽ dấu hỏi vì lý do đó. Ở đây nguyên tắc ấy được giữ
 * bằng cách chỉ có đúng một chỗ sinh ra `unappraised`.
 */

import type { IconSpec } from "./compose";

/**
 * Người xem biết gì.
 *
 * Mọi trường **bắt buộc**. Không có mặc định nào, vì mặc định ở đây luôn sai
 * theo hướng nguy hiểm: nếu quên truyền `knowsClaimDispute`, giá trị mặc định
 * `false` thì icon thiếu thông tin (khó chịu), còn `true` thì icon **rò** thông
 * tin (hỏng gameplay). Bắt buộc cả hai để không ai phải đoán bên nào an toàn.
 */
export interface Viewer {
  /** Đang nhìn qua chế độ nào (`§18.9`). */
  mode: "embodied" | "observer" | "true_god";
  /** Có kỹ năng thẩm định, hoặc có người biết xem bên cạnh (`§8.6.4`). */
  canAppraise: boolean;
  /** Những dấu hiệu effect mà người này **nhận biết được** theo `perceptible_as`. */
  perceives: ReadonlySet<string>;
  /** Có biết món này đang tranh chấp quyền sở hữu không (`§12.8.1`). */
  knowsClaimDispute: boolean;
  /** Có nhìn ra lớp cải trang không. */
  seesThroughDisguise: boolean;
}

/** Sự thật đầy đủ về một vật, trước khi lọc. */
export interface Truth {
  /** Bóng thật. */
  silhouette: string;
  /** Bóng mà lớp cải trang trình ra, nếu đang cải trang. */
  disguisedAs?: string | undefined;
  material?: string | undefined;
  /**
   * Dấu chất lượng thật.
   *
   * Chỉ hiện khi người xem thẩm định được. Chưa thẩm định thì **dấu hỏi**, không
   * phải bỏ trống: bỏ trống trông y hệt "đồ thường", và người chơi sẽ học rằng
   * mọi thứ nhặt được đều là đồ thường.
   */
  quality?: string | undefined;
  /**
   * Các dấu hiệu trạng thái, kèm tên `perceptible_as` của chúng.
   *
   * Khóa là dấu hiệu để vẽ; giá trị là điều kiện để thấy nó. `null` nghĩa là ai
   * cũng thấy — ướt, cháy, gãy là những thứ nhìn là biết.
   */
  states: ReadonlyArray<{ sign: string; perceptibleAs: string | null }>;
  /** Đang có tranh chấp quyền sở hữu. */
  claimDisputed: boolean;
  /** Dấu phe hoặc sở hữu, nếu công khai. */
  marker?: string | undefined;
  /** Số lượng, nếu là chồng. */
  stackAnnotation?: string | undefined;
}

/** Kết quả lọc: đặc tả được phép vẽ, kèm những gì đã bị giấu. */
export interface Gated {
  spec: IconSpec;
  /**
   * Những gì đã bị lọc bỏ, để test và để chế độ True God đối chiếu.
   *
   * Không dùng cho việc vẽ. Nó tồn tại để một test khẳng định được rằng *đã có*
   * thứ bị giấu — một bộ lọc không giấu gì bao giờ trông y hệt một bộ lọc đúng.
   */
  withheld: string[];
}

/**
 * Lọc sự thật xuống thứ người xem được phép thấy.
 *
 * Ở chế độ quan sát và True God, mọi thứ hiện đầy đủ — nhưng `§18.14.5` đòi
 * **ghi nhãn rõ là sự thật của thế giới**. Nhãn đó là việc của read model
 * (`Certainty`); ở đây chỉ cần không giấu gì.
 */
export function gate(truth: Truth, viewer: Viewer): Gated {
  const withheld: string[] = [];
  const toan_tri = viewer.mode !== "embodied";

  // ── Cải trang ──────────────────────────────────────────────────────────────
  // Hiện bóng của lớp cải trang, cho tới khi có ai đó nhìn ra.
  let silhouette = truth.silhouette;
  if (truth.disguisedAs !== undefined && !toan_tri && !viewer.seesThroughDisguise) {
    silhouette = truth.disguisedAs;
    withheld.push(`silhouette:${truth.silhouette}`);
  }

  // ── Chất lượng ─────────────────────────────────────────────────────────────
  let annotation: string | undefined;
  if (truth.quality !== undefined) {
    if (toan_tri || viewer.canAppraise) {
      annotation = truth.quality;
    } else {
      // Đúng một chỗ trong toàn bộ mã sinh ra `unappraised`. Xem docstring.
      annotation = "unappraised";
      withheld.push(`quality:${truth.quality}`);
    }
  } else if (truth.stackAnnotation !== undefined) {
    annotation = truth.stackAnnotation;
  }

  // ── Trạng thái ─────────────────────────────────────────────────────────────
  // Phép ẩn và lời nguyền chỉ hiện huy hiệu nếu người xem nhận biết được nó.
  const states: string[] = [];
  for (const s of truth.states) {
    const thay = toan_tri || s.perceptibleAs === null || viewer.perceives.has(s.perceptibleAs);
    if (thay) {
      states.push(s.sign);
    } else {
      withheld.push(`state:${s.sign}`);
    }
  }

  // ── Dấu "đồ gian" ──────────────────────────────────────────────────────────
  let marker = truth.marker;
  if (truth.claimDisputed) {
    if (toan_tri || viewer.knowsClaimDispute) {
      marker = "stolen";
    } else {
      withheld.push("marker:stolen");
    }
  }

  const spec: IconSpec = { silhouette };
  if (truth.material !== undefined) spec.material = truth.material;
  if (states.length > 0) spec.states = states;
  if (marker !== undefined) spec.marker = marker;
  if (annotation !== undefined) spec.annotation = annotation;

  return { spec, withheld };
}

/**
 * Hai người xem khác nhau có thấy cùng một icon không.
 *
 * Công cụ chẩn đoán cho test và cho `mow-devtool`: nếu một người biết bí mật và
 * một người không mà cả hai thấy icon giống hệt nhau, thì hoặc bí mật đó không
 * ảnh hưởng gì tới hình ảnh (được), hoặc bộ lọc đã không làm gì (hỏng).
 */
export function sameIcon(truth: Truth, a: Viewer, b: Viewer): boolean {
  return JSON.stringify(gate(truth, a).spec) === JSON.stringify(gate(truth, b).spec);
}

/** Người xem "không biết gì" — dùng làm mốc trong test và cho người lạ. */
export function naiveViewer(): Viewer {
  return {
    mode: "embodied",
    canAppraise: false,
    perceives: new Set(),
    knowsClaimDispute: false,
    seesThroughDisguise: false,
  };
}
