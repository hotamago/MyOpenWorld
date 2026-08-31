/**
 * Hình thực thể: một cư dân đọc được **vai trò** ở mức phóng nhỏ nhất.
 *
 * ## Vì sao một chấm tròn không đủ
 *
 * Bản trước vẽ mọi cư dân bằng cùng một hình tròn, khác nhau đúng một chi
 * tiết: avatar có viền xanh. Ở mức phóng 12px — mức người chơi dùng nhiều
 * nhất để bao quát cả làng — hai chục chấm tròn cùng cỡ trông như một chuồng
 * bi, không phải một ngôi làng có nông dân, thợ rèn, thợ săn. `§18.6` cấm màu
 * là kênh duy nhất mang thông tin; ở bản cũ vấn đề còn nặng hơn thế — ngoài
 * màu ra không có kênh nào khác cả.
 *
 * ## Tách thuần khỏi vẽ, cùng ranh giới với `ambient.ts`
 *
 * `figureOf` không đụng Pixi: nó nhận một `Entity` và trả về một `FigureSpec`
 * — vài con số và vài nhãn hình dạng. Nhờ vậy quy tắc "thợ săn cầm cung, không
 * cầm cái gì khác" là một hàm thuần kiểm bằng `vitest` trong Node, không phải
 * một khẳng định phải soi bằng mắt trên canvas. Lỗi tính sai vai và lỗi vẽ sai
 * nét là hai loại lỗi khác nhau, cần hai cách gỡ khác nhau — gộp chúng vào một
 * hàm nghĩa là mọi lần sửa hình đều phải chạy cả trình duyệt để biết còn đúng
 * không.
 *
 * ## Hình dạng, không phải màu, mang thông tin vai trò
 *
 * Mỗi vai được gán một **cặp** dụng cụ + mũ không trùng với vai nào khác —
 * cặp, chứ không phải từng phần riêng: hai vai được phép chia sẻ một dụng cụ
 * hay một kiểu mũ, miễn là không chia sẻ cả hai cùng lúc (`elder` và `hunter`
 * cùng đội `hood` nhưng cầm dụng cụ khác nhau). Cặp đó đọc được cả khi in đen
 * trắng, khi mù màu, hay khi màn hình ám vàng lúc hoàng hôn — còn `body` và
 * `outline` chỉ là một lớp phụ giúp mắt quen nhanh hơn, không phải chỗ để giấu
 * thông tin không thể thiếu.
 *
 * Avatar là trường hợp đặc biệt duy nhất: nó luôn đội `crown` và tay không,
 * bất kể vai nền của người chơi trong dữ liệu server là gì. Không cư dân nào
 * khác được cấp `crown`, nên avatar không bao giờ lẫn vào đám đông — kể cả khi
 * người chơi tự gán cho mình vai nông dân để hóa trang.
 */

import type { Entity } from "@/api/game";
import type { Graphics } from "pixi.js";

/** Dụng cụ cầm tay — kênh hình dạng chính phân biệt các vai lao động. */
export type FigureTool = "none" | "staff" | "hammer" | "bow" | "hoe" | "none_child";

/** Mũ hoặc tóc — kênh hình dạng thứ hai, độc lập với dụng cụ. */
export type FigureHead = "bare" | "hat" | "hood" | "crown";

/** Ý định rút gọn thành một ký hiệu hình học, không phải một chuỗi chữ. */
export type FigureMark = "none" | "eat" | "sleep" | "work" | "talk" | "walk";

/**
 * Mô tả hình dạng đã tính xong của một thực thể — không còn logic nào để
 * `drawFigure` phải quyết định, chỉ còn tọa độ và lựa chọn hình học.
 */
export interface FigureSpec {
  /**
   * Người hay vật phẩm. Vật phẩm dùng hẳn một bộ hình khác (hình thoi cũ),
   * không phải một biến thể của bộ dụng-cụ/mũ — cờ này là chỗ `drawFigure`
   * biết đường rẽ nhánh mà không phải đoán qua `tool === "none"`, thứ vốn
   * cũng là giá trị hợp lệ của một cư dân tay không.
   */
  shape: "being" | "item";
  /** Bán kính thân, theo tỉ lệ của cỡ ô. */
  scale: number;
  /** Màu áo — chỉ để phân biệt phe/vai phụ, KHÔNG mang thông tin duy nhất. */
  body: number;
  outline: number;
  /** Dụng cụ cầm tay, phân biệt bằng **hình**: `§18.6` cấm màu là kênh duy nhất. */
  tool: FigureTool;
  /** Mũ / tóc, cũng là hình. */
  head: FigureHead;
  /** Hướng nhìn, suy từ bước đi gần nhất. */
  facing: "left" | "right";
  /** Dấu ý định nổi trên đầu: một ký hiệu hình học, không phải chữ. */
  mark: FigureMark;
}

/** Bán kính thân của một cư dân trưởng thành, theo tỉ lệ cỡ ô. */
const ADULT_SCALE = 0.29;

/**
 * Trẻ con nhỏ hơn rõ rệt — nhân với tỉ lệ chứ không phải một hằng số tuyệt
 * đối, để "nhỏ hơn người lớn bao nhiêu" vẫn đúng nếu sau này chỉnh cỡ chung.
 */
const CHILD_SCALE = ADULT_SCALE * 0.72;

/** Avatar lớn hơn một chút — giữ đúng mức chênh mà bản cũ dùng (`0.34` so với `0.29`). */
const AVATAR_SCALE = 0.33;

/** Bán kính thân của vật phẩm — giữ nguyên từ bản cũ. */
const ITEM_SCALE = 0.26;

interface RoleShape {
  tool: FigureTool;
  head: FigureHead;
  body: number;
  outline: number;
  scale: number;
}

/**
 * Hình dạng gắn với từng vai — một bảng tra, không phải một chuỗi `if/else`.
 *
 * Bất biến kiểm trong `figure.test.ts`: không hai vai nào trùng cả `tool` lẫn
 * `head` cùng lúc. Bảng là nơi duy nhất bất biến đó có thể vỡ vì gõ nhầm một
 * dòng, nên nó cũng là nơi duy nhất cần đọc lại khi thêm một vai mới.
 */
const ROLE_SHAPE: Record<string, RoleShape> = {
  elder: { tool: "staff", head: "hood", body: 0xb9b0cf, outline: 0x332e44, scale: ADULT_SCALE },
  smith: { tool: "hammer", head: "bare", body: 0xb0564a, outline: 0x3a1810, scale: ADULT_SCALE },
  hunter: { tool: "bow", head: "hood", body: 0x6f8a5c, outline: 0x263420, scale: ADULT_SCALE },
  farmer: { tool: "hoe", head: "hat", body: 0xd0a24a, outline: 0x4a3110, scale: ADULT_SCALE },
  child: { tool: "none_child", head: "bare", body: 0xe8c990, outline: 0x5c3d20, scale: CHILD_SCALE },
};

/**
 * Hình dạng dùng khi vai không khớp bảng trên.
 *
 * Rơi về một cư dân trung tính — không dụng cụ, đầu trần — chứ không ném lỗi:
 * một content pack có thể gán một vai mà bảng này chưa biết, và một cư dân lạ
 * vai thì vẫn phải vẽ ra được, giống cách `role_from` phía server rơi về
 * `Farmer` thay vì panic.
 */
const FALLBACK_SHAPE: RoleShape = {
  tool: "none",
  head: "bare",
  body: 0xd08770,
  outline: 0x3a1f16,
  scale: ADULT_SCALE,
};

/**
 * Rút ý định thành một dấu hình học.
 *
 * `e.intent` là **khóa ổn định** do server đặt: `"eat"`, `"sleep"`, `"work"`,
 * `"socialize"`, `"idle"`, hoặc `"goto.<nơi>"`. Nó cố tình **không** phải
 * `format!("{:?}")` của enum Rust — bản đầu đúng là như vậy và chuỗi
 * `GoTo { place: Field }` đã lọt thẳng lên màn hình người chơi. Một khóa
 * snake_case thì dịch được, ổn định qua các phiên bản, và khớp được mà không
 * phải phân tích một định dạng gỡ lỗi.
 *
 * Chỉ đọc phần trước dấu chấm: nơi đến là việc của bảng chú giải, còn dấu trên
 * đầu chỉ cần nói "người này đang đi đâu đó".
 *
 * Khóa lạ — kể cả một ý định tương lai chưa từng thấy — rơi về `"none"`. Ném
 * lỗi ở đây là sai chỗ: một dấu vẽ thiếu không đáng để dừng cả khung hình.
 */
function markOf(intent: string | null): FigureMark {
  const head = intent === null ? "" : (intent.split(".")[0] ?? "");
  switch (head) {
    case "eat":
      return "eat";
    case "sleep":
      return "sleep";
    case "work":
      return "work";
    case "socialize":
      return "talk";
    case "goto":
      return "walk";
    default:
      return "none";
  }
}

/**
 * Dựng `FigureSpec` cho một thực thể tại tick hiện tại.
 *
 * `prev` là vị trí của **cùng** thực thể ở lần vẽ trước — bên gọi (`WorldView`)
 * giữ một `Map<id, {x,y}>` vì đây là hàm thuần, nó không được phép tự nhớ gì
 * giữa hai lần gọi.
 */
export function figureOf(e: Entity, prev?: { x: number; y: number }): FigureSpec {
  const facing: FigureSpec["facing"] = prev !== undefined && e.x < prev.x ? "left" : "right";

  if (e.kind === "item") {
    // Vật phẩm không có "vai trò" để đọc, và ép nó vào bộ dụng-cụ/mũ ở dưới sẽ
    // vẽ ra một người tí hon cầm một viên kim cương — sai cả hai hướng cùng
    // lúc. Hình thoi giữ nguyên từ bản cũ: khác **hình dạng** với con người,
    // không chỉ khác màu (`§18.6`).
    return {
      shape: "item",
      scale: ITEM_SCALE,
      body: 0xf0c674,
      outline: 0x2a2110,
      tool: "none",
      head: "bare",
      facing,
      mark: "none",
    };
  }

  const mark = markOf(e.intent);

  if (e.is_avatar) {
    // Avatar ghi đè toàn bộ hình dạng theo vai — kể cả khi server gán cho nó
    // một vai cư dân thật (một vị thần vẫn có thể đứng tên "farmer" trong dữ
    // liệu định cư). Ghi đè hoàn toàn, không hợp nhất một phần, là cách duy
    // nhất đảm bảo avatar không bao giờ trùng `tool`+`head` với bất kỳ ai.
    return {
      shape: "being",
      scale: AVATAR_SCALE,
      body: 0xf5f7fa,
      outline: 0x2b6cb0,
      tool: "none",
      head: "crown",
      facing,
      mark,
    };
  }

  const rs = (e.role !== null ? ROLE_SHAPE[e.role] : undefined) ?? FALLBACK_SHAPE;
  return {
    shape: "being",
    scale: rs.scale,
    body: rs.body,
    outline: rs.outline,
    tool: rs.tool,
    head: rs.head,
    facing,
    mark,
  };
}

// ─────────────────────────────────────────────────────────────────────────
// Phần vẽ. Từ đây trở xuống là `Graphics` thật — không có logic nào đáng kiểm
// bằng test thuần, chỉ còn hình học. `drawFigure` chỉ đọc `spec`; nó không tự
// suy vai hay ý định. Nếu một hình vẽ sai, câu hỏi đầu tiên là "`figureOf` có
// tính sai không", không phải "hàm vẽ có bug logic không".
// ─────────────────────────────────────────────────────────────────────────

/** Màu và viền cố định của dấu ý định — không đổi theo vai hay avatar. */
const MARK_FILL = 0xf4ecd8;
const MARK_OUTLINE = 0x2a2216;

/**
 * Vẽ một `FigureSpec` đã tính sẵn lên `Graphics`, tâm tại `(cx, cy)`.
 *
 * `ts` (cỡ ô hiện tại) là đơn vị duy nhất mọi kích thước quy về — cùng cách
 * `terrain.ts` và `ambient.ts` làm, để phóng to/nhỏ không cần vẽ lại bằng công
 * thức khác.
 */
export function drawFigure(g: Graphics, spec: FigureSpec, cx: number, cy: number, ts: number): void {
  // Bóng tiếp đất: giữ nguyên hình dạng và vị trí của bản cũ, cho mọi hình
  // dạng như nhau. Không có nó thì mọi thứ trông như dán lên bản đồ chứ không
  // đứng trên nó — và bóng không mang thông tin vai trò nên không cần đổi.
  g.ellipse(cx, cy + ts * 0.3, ts * 0.32, ts * 0.12).fill({ color: 0x000000, alpha: 0.35 });

  const lineW = Math.max(1, ts * 0.06);

  if (spec.shape === "item") {
    const r = ts * spec.scale;
    g.poly([cx, cy - r, cx + r, cy, cx, cy + r, cx - r, cy])
      .fill(spec.body)
      .stroke({ width: lineW, color: spec.outline });
    return;
  }

  const s = ts * spec.scale;
  // Chân đứng ngay trên tâm bóng, không phải trên tâm ô: nếu thân vẽ giữa ở
  // `cy` thì phần chân "trôi" khỏi bóng mỗi khi `scale` đổi theo vai, và trẻ
  // con — vai duy nhất nhỏ hơn hẳn — sẽ trông như lơ lửng phía trên bóng của
  // chính nó.
  const feetY = cy + ts * 0.2;
  const bodyW = s * 1.3;
  const bodyH = s * 2.15;
  const bodyTop = feetY - bodyH;
  const headR = s * 0.62;
  const headY = bodyTop - headR * 0.75;

  // Thân viên thuốc: `roundRect` với bán kính bo bằng nửa bề rộng biến hai đầu
  // thành nửa hình tròn, cho dáng "một người đứng thẳng" — khác hẳn một hình
  // tròn, thứ chỉ đọc ra "một cục" chứ không đọc ra "một cơ thể".
  g.roundRect(cx - bodyW / 2, bodyTop, bodyW, bodyH, bodyW / 2)
    .fill(spec.body)
    .stroke({ width: lineW, color: spec.outline });

  g.circle(cx, headY, headR).fill(spec.body).stroke({ width: lineW, color: spec.outline });

  drawHead(g, spec.head, cx, headY, headR, spec.outline, lineW);
  drawTool(g, spec.tool, spec.facing, cx, headY, headR, feetY, bodyW, s, spec.outline, lineW);
  if (spec.mark !== "none") drawMark(g, spec.mark, cx, headY - headR, s, spec.facing, lineW);
}

/** Vẽ phần mũ/tóc chồng lên đầu tròn đã vẽ sẵn. */
function drawHead(
  g: Graphics,
  head: FigureHead,
  cx: number,
  headY: number,
  headR: number,
  outline: number,
  lineW: number,
): void {
  switch (head) {
    case "bare":
      // Đầu trần: hình tròn cơ sở đã đủ. Thêm nét ở đây sẽ làm "không đội gì"
      // trông giống "đội một cái mũ không màu" — hai thứ phải khác nhau.
      return;
    case "hat": {
      // Vành nón rộng của nông dân: một ellipse dẹt cắt ngang đầu, rộng hơn cả
      // đầu để đọc ra "vành", cộng một chóp nhỏ phía trên cho dáng nón lá.
      const y = headY - headR * 0.15;
      g.ellipse(cx, y, headR * 0.95, headR * 0.42).fill(outline);
      g.circle(cx, y - headR * 0.35, headR * 0.4).fill(outline);
      return;
    }
    case "hood": {
      // Mũ trùm nhọn của thợ săn/người già: một tam giác phủ từ đỉnh đầu
      // xuống ngang vai — nhọn, không dẹt, để không lẫn với vành nón.
      g.poly([
        cx,
        headY - headR * 1.5,
        cx + headR * 0.95,
        headY + headR * 0.5,
        cx - headR * 0.95,
        headY + headR * 0.5,
      ]).fill(outline);
      return;
    }
    case "crown": {
      // Ba mũi nhọn trên một dải nền: hình dạng không cư dân nào khác được
      // cấp, nên avatar đọc ra được ngay cả khi in đen trắng và mất hết màu.
      const w = headR * 0.9;
      const base = headY - headR * 0.6;
      const valley = base - headR * 0.35;
      const tipSide = base - headR * 0.75;
      const tipMid = base - headR * 1.05;
      g.poly([
        cx - w,
        base,
        cx - w,
        valley,
        cx - w * 0.55,
        tipSide,
        cx - w * 0.28,
        valley,
        cx,
        tipMid,
        cx + w * 0.28,
        valley,
        cx + w * 0.55,
        tipSide,
        cx + w,
        valley,
        cx + w,
        base,
      ])
        .fill(0xf0d060)
        .stroke({ width: lineW, color: outline });
      return;
    }
  }
}

/** Vẽ dụng cụ cầm tay, lệch sang phía đang quay mặt tới. */
function drawTool(
  g: Graphics,
  tool: FigureTool,
  facing: "left" | "right",
  cx: number,
  headY: number,
  headR: number,
  feetY: number,
  bodyW: number,
  s: number,
  outline: number,
  lineW: number,
): void {
  // Trẻ con và avatar tay không — đúng nghĩa đen, không có gì để vẽ thêm.
  if (tool === "none" || tool === "none_child") return;

  const side = facing === "right" ? 1 : -1;
  const handX = cx + side * bodyW * 0.7;

  switch (tool) {
    case "staff": {
      // Gậy dài chạm đất: nét đọc được ngay ở 12px vì nó dài hơn cả thân —
      // khác hẳn dụng cụ cầm gọn của thợ rèn hay thợ săn.
      const top = headY - headR * 0.4;
      g.moveTo(handX, top).lineTo(handX, feetY).stroke({ width: lineW, color: outline });
      g.circle(handX, top, s * 0.14).fill(outline);
      return;
    }
    case "hammer": {
      // Cán ngắn cộng một khối vuông trên đầu cán: "ngắn + có khối" là điều
      // phân biệt nó với cây gậy dài, không cần màu khác.
      const top = headY + headR * 0.1;
      const bottom = feetY - s * 0.3;
      g.moveTo(handX, top).lineTo(handX, bottom).stroke({ width: lineW, color: outline });
      g.rect(handX - s * 0.32, top - s * 0.22, s * 0.64, s * 0.3).fill(outline);
      return;
    }
    case "bow": {
      // Một cung cong thật — không phải một đường thẳng, thứ sẽ đọc nhầm
      // thành cây gậy. Cong bằng nội suy bậc hai quanh một điểm phình ra
      // ngoài thân, không dùng `arc` vì cần cung phình theo đúng phía `facing`
      // mà không phải tính giao điểm hai đường tròn.
      const top = headY + headR * 0.2;
      const bottom = feetY - s * 0.15;
      const mid = (top + bottom) / 2;
      const bulgeX = handX + side * s * 0.85;
      const steps = 8;
      for (let i = 0; i <= steps; i++) {
        const t = i / steps;
        const mt = 1 - t;
        const x = mt * mt * handX + 2 * mt * t * bulgeX + t * t * handX;
        const y = mt * mt * top + 2 * mt * t * mid + t * t * bottom;
        if (i === 0) g.moveTo(x, y);
        else g.lineTo(x, y);
      }
      g.stroke({ width: lineW, color: outline });
      // Dây cung: nét thẳng nối hai đầu, để hình đọc ra "một cây cung" chứ
      // không phải "một nét cong bất kỳ".
      g.moveTo(handX, top).lineTo(handX, bottom).stroke({ width: Math.max(1, lineW * 0.6), color: outline });
      return;
    }
    case "hoe": {
      const top = headY + headR * 0.1;
      g.moveTo(handX, top).lineTo(handX, feetY).stroke({ width: lineW, color: outline });
      g.rect(handX - s * 0.36, feetY - s * 0.16, s * 0.72, s * 0.2).fill(outline);
      return;
    }
  }
}

/** Vẽ dấu ý định nổi phía trên đầu, tại `topY` (mép trên của đầu tròn). */
function drawMark(
  g: Graphics,
  mark: Exclude<FigureMark, "none">,
  cx: number,
  topY: number,
  s: number,
  facing: "left" | "right",
  lineW: number,
): void {
  const r = s * 0.42;
  const y = topY - s * 0.22 - r;

  switch (mark) {
    case "eat":
      // Tam giác hướng lên: một mô thức ăn. Hướng ngược hẳn với mũi tên của
      // `walk` để hai dấu không lẫn nhau dù cùng là tam giác.
      g.poly([cx, y - r, cx + r * 0.9, y + r * 0.75, cx - r * 0.9, y + r * 0.75])
        .fill(MARK_FILL)
        .stroke({ width: lineW, color: MARK_OUTLINE });
      return;
    case "sleep":
      // Bán nguyệt: một cung nửa vòng cộng một cạnh thẳng đóng lại thành hình
      // trăng khuyết — không cần phép trừ hình học giữa hai vòng tròn, thứ
      // Pixi `Graphics` không phơi ra một cách chắc chắn qua API tiện dụng.
      g.arc(cx, y, r, -Math.PI / 2, Math.PI / 2, false)
        .lineTo(cx, y - r)
        .closePath()
        .fill(MARK_FILL)
        .stroke({ width: lineW, color: MARK_OUTLINE });
      return;
    case "work":
      // Hình vuông: khác cả tam giác lẫn bán nguyệt, và không trùng hình thoi
      // đã dành riêng cho vật phẩm.
      g.rect(cx - r * 0.8, y - r * 0.8, r * 1.6, r * 1.6)
        .fill(MARK_FILL)
        .stroke({ width: lineW, color: MARK_OUTLINE });
      return;
    case "talk":
      // Bong bóng thoại: một hình chữ nhật bo góc cộng một mũi nhỏ trỏ xuống
      // đầu — hình duy nhất trong bộ có "cái đuôi", nên không lẫn với `work`.
      g.roundRect(cx - r, y - r * 0.7, r * 2, r * 1.4, r * 0.4)
        .fill(MARK_FILL)
        .stroke({ width: lineW, color: MARK_OUTLINE });
      g.poly([cx - r * 0.25, y + r * 0.55, cx + r * 0.1, y + r * 0.55, cx - r * 0.15, y + r * 1.05]).fill(MARK_FILL);
      return;
    case "walk": {
      // Tam giác chỉ sang hướng đang đi: dùng lại `facing` đã tính cho thân,
      // để dấu và bước chân luôn nói cùng một hướng.
      const side = facing === "right" ? 1 : -1;
      g.poly([cx - side * r * 0.7, y - r * 0.85, cx - side * r * 0.7, y + r * 0.85, cx + side * r * 0.9, y])
        .fill(MARK_FILL)
        .stroke({ width: lineW, color: MARK_OUTLINE });
      return;
    }
  }
}
