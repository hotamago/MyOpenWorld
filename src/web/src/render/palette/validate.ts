/**
 * Bộ kiểm bảng màu, chạy trong CI (`PA-13`, `§18.6.2`–`§18.6.4`).
 *
 * ## Ngưỡng đến từ đâu
 *
 * Bản kế hoạch trước chốt *"bảng dùng cho bản đồ quá 3 định danh làm CI fail"*,
 * dựa trên một phép đo ΔE. Đo lại bằng chính bộ này — CIEDE2000, lấy giá trị
 * **nhỏ nhất qua cả ba dạng mù màu** — cho một bức tranh khác:
 *
 * ```text
 * #eda100 ↔ #eb6834          ΔE₀₀ 24.2  thị giác thường
 *                            ΔE₀₀  9.6  deuteranopia     ← đây mới là ràng buộc
 *
 * Okabe–Ito, đủ 8 màu        ΔE₀₀ 10.9  (#e69f00 ↔ #cc79a7, tritanopia)
 * ```
 *
 * Hai điều rút ra, và điều thứ hai lật một phần kết luận cũ:
 *
 * 1. **Ràng buộc thật nằm ở mù màu, không ở thị giác thường.** Cặp cam kia cách
 *    nhau rõ ràng với hầu hết mọi người và gần như trùng nhau với người
 *    protanopia. Kiểm ở thị giác thường sẽ cho nó qua.
 * 2. **Trần "3 màu" không có cơ sở.** Bảng phân loại an toàn kinh điển giữ
 *    nguyên 11.6 tới tận tám màu. Con số 3 là một **biến thay thế** cho ngưỡng —
 *    và biến thay thế thì sai ở cả hai phía: nó cấm những bộ tốt có 4 màu, và
 *    cho qua những bộ tồi có 3.
 *
 * Nên luật ở đây là **ngưỡng**, không phải số đếm. Ngưỡng **10** nằm giữa cặp
 * đã biết là dễ nhầm (9.6) và bảng an toàn kinh điển (10.9). Khoảng đó hẹp, và
 * sự hẹp đó là thông tin: **không có nhiều chỗ trống**. Một ngưỡng cao hơn sẽ
 * loại luôn cả Okabe–Ito, tức là loại thứ tốt nhất mà ngành từng tìm ra.
 *
 * Trần 8 màu vẫn còn, nhưng với một lý do khác hẳn: quá tám thì không bảng nào
 * từng biết giữ nổi ngưỡng, nên định danh phải chuyển sang icon và nhãn thay vì
 * màu.
 *
 * ## Vì sao vẫn có **hai** luật
 *
 * | Loại | Luật | Vì sao |
 * |---|---|---|
 * | `identity_critical` | **mọi cặp** ΔE₀₀ ≥ 10 qua mọi thị giác | Nhầm phe với phe là **lỗi gameplay**. Người chơi ra quyết định sai dựa trên nó. |
 * | `environment` | **cặp kề nhau** ΔE₀₀ ≥ 8, bắt buộc có hoa văn | Địa hình đã có tín hiệu phụ — hoa văn, độ cao, nhãn. Nhầm "rừng ôn đới" với "rừng lá kim" trong một giây thì không ai mất gì. |
 *
 * Ràng buộc chặt đi theo **hậu quả của việc nhầm**, không theo việc thứ đó có
 * phải màu hay không.
 */

import {
  contrastRatio,
  deltaE,
  minDeltaEAcrossVision,
  parseHex,
  toLab,
  type Cvd,
} from "./color";

/** Loại bảng màu. */
export type PaletteKind = "identity_critical" | "environment" | "sequential" | "diverging";

/** Một bảng màu như nó nằm trong `content/`. */
export interface Palette {
  id: string;
  kind: PaletteKind;
  /** Nền mà bảng này được vẽ lên, để kiểm tương phản. */
  background: string;
  /** Các mục, **theo thứ tự có nghĩa** — thứ tự này là một phần của dữ liệu. */
  entries: PaletteEntry[];
  /** Chế độ: `light` hoặc `dark`. */
  mode: "light" | "dark";
  /**
   * Màu của trạng thái "không có dữ liệu", với thang tuần tự và phân kỳ.
   *
   * Đây là thứ mà hai đầu mút của thang phải phân biệt được — **không phải**
   * nền trang. Một overlay được vẽ lên bản đồ, không lên trang; hỏi nó có
   * tương phản với nền trang hay không là hỏi sai câu, và trả lời câu hỏi sai
   * đó sẽ buộc mọi thang phải tối đi cho tới khi bản đồ thành một tấm kính màu.
   */
  noData?: string;
}

/** Một mục trong bảng. */
export interface PaletteEntry {
  id: string;
  color: string;
  /**
   * Hoa văn thay màu (`§18.6.3`).
   *
   * Bắt buộc với `environment`: đó là điều khiến việc nới ngưỡng ΔE trở nên an
   * toàn. Không có hoa văn thì màu là **tín hiệu duy nhất**, và lúc đó ngưỡng
   * lỏng là một lời hứa suông với người mù màu.
   */
  pattern?: string;
}

/** Một vi phạm. */
export interface Violation {
  palette: string;
  rule: string;
  detail: string;
}

/** Ngưỡng, gom một chỗ để CI và tài liệu không lệch nhau. */
export const THRESHOLDS = {
  /**
   * ΔE₀₀ tối thiểu giữa **mọi cặp** trong bảng định danh, lấy min qua mọi dạng
   * thị giác.
   *
   * Nằm giữa cặp đã biết là dễ nhầm (9.6) và bảng an toàn kinh điển Okabe–Ito
   * (10.9). Khoảng hẹp đó là thông tin, không phải sự tùy tiện: nâng ngưỡng lên
   * sẽ loại luôn thứ tốt nhất mà ngành từng tìm ra.
   */
  identityAllPairs: 10,
  /**
   * Số màu tối đa trong bảng định danh.
   *
   * Không phải một giới hạn tri giác mà là một giới hạn **thực nghiệm**: quá
   * tám màu thì không bảng nào từng biết giữ nổi ngưỡng trên, nên định danh
   * phải chuyển sang icon và nhãn.
   */
  identityMaxColors: 8,
  /** ΔE₀₀ tối thiểu giữa hai mục **kề nhau** trong bảng môi trường. */
  environmentAdjacent: 8,
  /** Tương phản tối thiểu giữa một dấu hiệu và nền nó nằm trên. */
  markContrast: 3,
  /** Bước sáng tối thiểu giữa hai bậc của thang tuần tự. */
  sequentialLightnessStep: 4,
} as const;

/** Kiểm một bảng màu. */
export function validatePalette(p: Palette): Violation[] {
  const v: Violation[] = [];
  const nen = parseHex(p.background);
  const mau = p.entries.map((e) => ({ ...e, rgb: parseHex(e.color) }));

  if (mau.length === 0) {
    v.push({ palette: p.id, rule: "empty", detail: "bảng màu rỗng" });
    return v;
  }

  // ── Tách khỏi nền: ba loại dấu hiệu, ba câu hỏi khác nhau ─────────────────
  //
  // Đây là phân biệt dễ bỏ qua nhất trong cả file, và bỏ qua nó thì tốn kém.
  //
  // **Dấu hiệu vẽ LÊN nền** (huy hiệu phe): dùng tương phản WCAG. Chúng nhỏ,
  // nằm chồng lên thứ khác, và mắt phải tách được hình khỏi nền.
  //
  // **Mảng LẤP ĐẦY nền** (màu biome): không hỏi gì cả. Chúng *chính là* bản đồ.
  // Đòi màu biome tương phản 3:1 với nền trang là một lỗi phân loại, và tuân
  // theo nó sẽ buộc mọi quần xã sinh vật phải tối hoặc bão hòa.
  //
  // **Thang phủ lên bản đồ** (độ cao, bất thường): hai đầu mút phải phân biệt
  // được với **màu "không có dữ liệu"**, không phải với nền trang — và bằng ΔE
  // chứ không phải WCAG, vì đây là hai mảng màu cạnh nhau, không phải hình trên
  // nền.
  if (p.kind === "identity_critical") {
    for (const m of mau) {
      const c = contrastRatio(m.rgb, nen);
      if (c < THRESHOLDS.markContrast) {
        v.push({
          palette: p.id,
          rule: "mark-contrast",
          detail: `\`${m.id}\` (${m.color}) tương phản ${c.toFixed(2)}:1 với nền ${p.background}, cần ≥ ${THRESHOLDS.markContrast}:1`,
        });
      }
    }
  } else if (p.kind === "sequential" || p.kind === "diverging") {
    if (!p.noData) {
      v.push({
        palette: p.id,
        rule: "no-data-required",
        detail:
          "thang phủ lên bản đồ phải khai báo `noData` — nếu không, không có cách nào " +
          "phân biệt \"giá trị nhỏ nhất\" với \"chưa có dữ liệu\".",
      });
    } else {
      const nd = parseHex(p.noData);
      for (const m of [mau[0]!, mau[mau.length - 1]!]) {
        const { value, vision } = minDeltaEAcrossVision(m.rgb, nd);
        if (value < THRESHOLDS.environmentAdjacent) {
          v.push({
            palette: p.id,
            rule: "no-data-separation",
            detail: `\`${m.id}\` ↔ noData: ΔE ${value.toFixed(1)} < ${THRESHOLDS.environmentAdjacent} (${vision})`,
          });
        }
      }
    }
  }

  switch (p.kind) {
    case "identity_critical": {
      if (mau.length > THRESHOLDS.identityMaxColors) {
        v.push({
          palette: p.id,
          rule: "identity-max-colors",
          detail:
            `${mau.length} màu, tối đa ${THRESHOLDS.identityMaxColors}. Bảng định danh ` +
            `(phe, chủ sở hữu) phải phân biệt được ở MỌI cặp, và quá chừng đó thì ` +
            `không bảng nào từng biết giữ nổi ngưỡng ΔE₀₀ ${THRESHOLDS.identityAllPairs} ` +
            `qua mọi dạng mù màu. Nếu cần nhiều hơn, dùng icon hoặc nhãn làm tín hiệu ` +
            `chính, không dùng màu.`,
        });
      }
      // Mọi cặp, qua mọi dạng thị giác.
      for (let i = 0; i < mau.length; i++) {
        for (let j = i + 1; j < mau.length; j++) {
          const a = mau[i]!;
          const b = mau[j]!;
          const { value, vision } = minDeltaEAcrossVision(a.rgb, b.rgb);
          if (value < THRESHOLDS.identityAllPairs) {
            v.push({
              palette: p.id,
              rule: "identity-all-pairs",
              detail:
                `\`${a.id}\` ↔ \`${b.id}\`: ΔE ${value.toFixed(1)} < ${THRESHOLDS.identityAllPairs} ` +
                `(ở thị giác ${vision}). Nhầm hai định danh này là lỗi gameplay, không phải ` +
                `bất tiện thẩm mỹ.`,
            });
          }
        }
      }
      break;
    }

    case "environment": {
      // Chỉ cặp **kề nhau** trong thứ tự đã khai báo. Biome cách xa nhau trong
      // thang thì hiếm khi nằm cạnh nhau trên bản đồ, và khi nằm cạnh thì đã có
      // ranh giới địa hình phân định.
      for (let i = 0; i + 1 < mau.length; i++) {
        const a = mau[i]!;
        const b = mau[i + 1]!;
        const { value, vision } = minDeltaEAcrossVision(a.rgb, b.rgb);
        if (value < THRESHOLDS.environmentAdjacent) {
          v.push({
            palette: p.id,
            rule: "environment-adjacent",
            detail: `\`${a.id}\` ↔ \`${b.id}\` kề nhau: ΔE ${value.toFixed(1)} < ${THRESHOLDS.environmentAdjacent} (${vision})`,
          });
        }
      }
      // Hoa văn là điều kiện để nới ngưỡng ΔE. Thiếu nó thì màu là tín hiệu
      // duy nhất, và ngưỡng lỏng thành một lời hứa suông.
      for (const m of mau) {
        if (!m.pattern) {
          v.push({
            palette: p.id,
            rule: "environment-pattern-required",
            detail:
              `\`${m.id}\` thiếu \`pattern\`. Bảng môi trường được nới ngưỡng ΔE **vì** ` +
              `có hoa văn làm tín hiệu phụ (§18.6.3); thiếu nó thì phải theo luật của ` +
              `bảng định danh.`,
          });
        }
      }
      break;
    }

    case "sequential": {
      // Độ sáng phải đơn điệu: đó là thứ khiến thang tuần tự đọc được **mà
      // không cần legend**, và cũng là thứ khiến nó còn đọc được khi in đen
      // trắng hay khi người xem mù màu.
      const L = mau.map((m) => toLab(m.rgb).L);
      const tang = L.every((x, i) => i === 0 || x > L[i - 1]!);
      const giam = L.every((x, i) => i === 0 || x < L[i - 1]!);
      if (!tang && !giam) {
        v.push({
          palette: p.id,
          rule: "sequential-monotonic-lightness",
          detail: `độ sáng không đơn điệu: [${L.map((x) => x.toFixed(1)).join(", ")}]`,
        });
      }
      for (let i = 0; i + 1 < L.length; i++) {
        const d = Math.abs(L[i + 1]! - L[i]!);
        if (d < THRESHOLDS.sequentialLightnessStep) {
          v.push({
            palette: p.id,
            rule: "sequential-lightness-step",
            detail: `bậc ${i}→${i + 1} chỉ chênh ${d.toFixed(1)} L*, cần ≥ ${THRESHOLDS.sequentialLightnessStep}`,
          });
        }
      }
      break;
    }

    case "diverging": {
      // Hai nhánh, sáng nhất ở giữa. Điểm giữa phải thật sự là trung tính —
      // một thang phân kỳ lệch tâm sẽ khiến người xem đọc sai dấu của dữ liệu.
      const L = mau.map((m) => toLab(m.rgb).L);
      const giua = Math.floor(L.length / 2);
      const sang_nhat = L.indexOf(Math.max(...L));
      if (Math.abs(sang_nhat - giua) > 1) {
        v.push({
          palette: p.id,
          rule: "diverging-center",
          detail: `bậc sáng nhất ở vị trí ${sang_nhat}, tâm ở ${giua} — thang lệch tâm làm đọc sai dấu`,
        });
      }
      if (L.length % 2 === 0) {
        v.push({
          palette: p.id,
          rule: "diverging-odd",
          detail: `${L.length} bậc — thang phân kỳ cần số bậc lẻ để có một bậc trung tính thật sự`,
        });
      }
      break;
    }
  }

  return v;
}

/** Kiểm một tập bảng màu. */
export function validateAll(palettes: Palette[]): Violation[] {
  const v = palettes.flatMap(validatePalette);

  // Mỗi bảng phải có cả hai chế độ. `§18.6.4`: chế độ tối là **thang riêng đã
  // qua kiểm tra**, không phải bảng sáng bị đảo ngược — đảo ngược làm hỏng thứ
  // tự sáng của thang tuần tự và làm màu bão hòa trở nên chói.
  const theo_id = new Map<string, Set<string>>();
  for (const p of palettes) {
    if (!theo_id.has(p.id)) theo_id.set(p.id, new Set());
    theo_id.get(p.id)!.add(p.mode);
  }
  for (const [id, modes] of [...theo_id].sort()) {
    if (modes.size < 2) {
      v.push({
        palette: id,
        rule: "both-modes-required",
        detail: `chỉ có chế độ ${[...modes].join(", ")}. Chế độ tối phải là thang riêng đã qua kiểm tra (§18.6.4).`,
      });
    }
  }
  return v;
}

/** Báo cáo đọc được. */
export function formatReport(v: Violation[]): string {
  if (v.length === 0) return "bảng màu: không vi phạm";
  const theo_bang = new Map<string, Violation[]>();
  for (const x of v) {
    if (!theo_bang.has(x.palette)) theo_bang.set(x.palette, []);
    theo_bang.get(x.palette)!.push(x);
  }
  const dong: string[] = [`bảng màu: ${v.length} vi phạm`];
  for (const [ten, ds] of [...theo_bang].sort()) {
    dong.push(`  ${ten}`);
    for (const x of ds) dong.push(`    [${x.rule}] ${x.detail}`);
  }
  return dong.join("\n");
}

/** Tiện ích cho test: ΔE giữa hai hex. */
export function deltaEHex(a: string, b: string): number {
  return deltaE(parseHex(a), parseHex(b));
}

export type { Cvd };
