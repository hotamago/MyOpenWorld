/**
 * Bộ giải văn phạm blazon — chiều ngược của `heraldry.blazon()`
 * (`idea.md §18.14.3`, `PF-18`).
 *
 * `PD-20` chỉ cần bảng tĩnh cộng dấu nhánh thứ để **đọc được huyết thống**.
 * Đây là phần đắt tách ra: nhận một chuỗi blazon do người viết và trả về
 * [`Arms`] — thứ cho phép một content pack, một người chơi, hoặc một tài liệu
 * lịch sử trong game khai một lá cờ bằng đúng ngôn ngữ mà huy hiệu học đã dùng
 * suốt bảy trăm năm.
 *
 * ## Vì sao cần chiều ngược
 *
 * Vì `generateArms(seed)` chỉ sinh được những lá cờ nằm trong không gian seed.
 * Một dòng họ có thật trong lore, một hiệp ước nhắc tới huy hiệu của một nhà đã
 * tuyệt tự, một content pack muốn đưa vào cờ riêng — cả ba đều cần đường đi từ
 * **chữ** sang **dữ liệu**, và không cái nào đi được bằng cách dò seed.
 *
 * ## Bộ giải phải **từ chối**, không phải đoán
 *
 * Đây là nguyên tắc trung tâm. Một bộ giải rộng lượng — bỏ qua từ không hiểu,
 * đoán màu gần đúng — sẽ nhận `"gules, một lion gules"` và trả về một lá cờ
 * **vi phạm luật màu**: lion đỏ trên nền đỏ, không nhìn thấy gì từ xa. Luật
 * màu ở `§18.14.3` là một chuẩn tương phản; một bộ giải bỏ qua nó sẽ lặng lẽ
 * phá đúng cái mà `§18.6` phải kiểm bằng máy.
 *
 * Nên [`parseBlazon`] trả về `{ ok: false, errors }` với **mọi** lỗi, và
 * không có tham số nào nới lỏng.
 *
 * ## Vòng khép kín
 *
 * `blazon(parseBlazon(s)) === s` với mọi `s` hợp lệ, và
 * `parseBlazon(blazon(a)) === a` với mọi `a`. Hai chiều khớp nhau là điều kiện
 * để một lá cờ đi qua văn bản mà không mất gì — và văn bản là thứ duy nhất
 * người đọc màn hình đọc được.
 */

import {
  CADENCY,
  CHARGES,
  COLOURS,
  DIVISIONS,
  METALS,
  blazon,
  contrasts,
  type Arms,
  type CadencyMark,
  type Charge,
  type ChargeTincture,
  type Division,
  type Tincture,
} from "./heraldry";

/** Mọi tincture hợp lệ. */
const TINCTURES: readonly Tincture[] = [...METALS, ...COLOURS];

/** Một lỗi cú pháp hoặc lỗi luật, kèm chỗ hỏng. */
export interface BlazonError {
  /** Mã ổn định để UI tra và dịch. */
  code:
    | "empty"
    | "unknown_tincture"
    | "unknown_division"
    | "unknown_charge"
    | "unknown_cadency"
    | "missing_charge"
    | "tincture_rule"
    | "field_count";
  /** Đoạn văn bản gây lỗi, nguyên văn. */
  at: string;
  /** Câu giải thích cho người viết. */
  detail: string;
}

/** Kết quả giải. */
export type ParseResult =
  | { ok: true; arms: Arms }
  | { ok: false; errors: BlazonError[] };

/** Chuẩn hóa khoảng trắng, giữ nguyên dấu tiếng Việt. */
function chuan(s: string): string {
  return s.trim().replace(/\s+/g, " ");
}

function laTincture(w: string): w is Tincture {
  return (TINCTURES as readonly string[]).includes(w);
}

/**
 * Tách phần trường khỏi phần hình.
 *
 * Blazon dùng dấu phẩy làm ranh giới giữa mô tả trường và mô tả hình — cùng
 * quy ước với văn bản huy hiệu học thật. Tách bằng dấu phẩy **đầu tiên** chứ
 * không phải mọi dấu phẩy: phần dấu nhánh thứ ở sau cũng có dấu phẩy.
 */
function tach(s: string): { truong: string; con_lai: string } {
  const i = s.indexOf(",");
  if (i < 0) return { truong: s, con_lai: "" };
  return { truong: s.slice(0, i), con_lai: s.slice(i + 1) };
}

/**
 * Giải phần trường: `"gules"` hoặc `"per pale or và azure"`.
 *
 * Trả `null` kèm lỗi nếu không khớp. Không đoán: `"per pale or"` thiếu vế thứ
 * hai là lỗi, không phải một trường một màu có thêm chữ thừa.
 */
function giaiTruong(
  s: string,
  errors: BlazonError[],
): { division: Division; field: [Tincture] | [Tincture, Tincture] } | null {
  const t = chuan(s);
  if (t === "") {
    errors.push({ code: "empty", at: s, detail: "không có phần trường nào" });
    return null;
  }

  // Trường một màu.
  if (laTincture(t)) {
    return { division: "plain", field: [t] };
  }

  // Trường chia: `<division> <a> và <b>`. Division trong `DIVISIONS` dùng dấu
  // gạch dưới (`per_pale`), còn văn bản dùng khoảng trắng — `blazon()` thay
  // gạch dưới bằng khoảng trắng, nên ở đây làm ngược lại.
  const chia = DIVISIONS.filter((d) => d !== "plain")
    .map((d) => ({ d, van: d.replace(/_/g, " ") }))
    .sort((a, b) => b.van.length - a.van.length) // khớp cái dài trước
    .find(({ van }) => t.startsWith(`${van} `));

  if (!chia) {
    errors.push({
      code: "unknown_division",
      at: t,
      detail: `không nhận ra cách chia trường; hợp lệ: ${DIVISIONS.join(", ")}`,
    });
    return null;
  }

  const phan = t.slice(chia.van.length + 1);
  const ve = phan.split(" và ").map(chuan);
  if (ve.length !== 2) {
    errors.push({
      code: "field_count",
      at: phan,
      detail: `trường chia cần đúng hai màu nối bằng "và", nhận được ${ve.length}`,
    });
    return null;
  }

  const mau: Tincture[] = [];
  for (const v of ve) {
    if (!laTincture(v)) {
      errors.push({
        code: "unknown_tincture",
        at: v,
        detail: `không phải tincture; hợp lệ: ${TINCTURES.join(", ")}`,
      });
      continue;
    }
    mau.push(v);
  }
  const [a, b] = mau;
  if (a === undefined || b === undefined) return null;

  return { division: chia.d, field: [a, b] };
}

/**
 * Giải phần hình: `"một lion or"` hoặc `"một lion counterchanged"`.
 */
function giaiHinh(
  s: string,
  errors: BlazonError[],
): { charge: Charge; chargeTincture: ChargeTincture } | null {
  const t = chuan(s);
  if (!t.startsWith("một ")) {
    errors.push({
      code: "missing_charge",
      at: t,
      detail: 'phần hình phải bắt đầu bằng "một"',
    });
    return null;
  }
  const con = t.slice("một ".length);

  // Khớp charge dài trước để `"sư tử"` không bị `"sư"` cắt mất.
  const hinh = [...CHARGES]
    .sort((a, b) => b.length - a.length)
    .find((c) => con.startsWith(`${c} `));
  if (!hinh) {
    errors.push({
      code: "unknown_charge",
      at: con,
      detail: `không nhận ra hình; hợp lệ: ${CHARGES.join(", ")}`,
    });
    return null;
  }

  const sac = chuan(con.slice(hinh.length + 1));
  if (sac === "counterchanged") {
    return { charge: hinh, chargeTincture: "counterchanged" };
  }
  if (!laTincture(sac)) {
    errors.push({
      code: "unknown_tincture",
      at: sac,
      detail: `sắc của hình không hợp lệ; hợp lệ: ${TINCTURES.join(", ")}, counterchanged`,
    });
    return null;
  }
  return { charge: hinh, chargeTincture: sac };
}

/**
 * Giải phần dấu nhánh thứ: `", khác biệt bởi label rồi crescent"`.
 *
 * Chuỗi rỗng là hợp lệ — đó là nhánh chính, và nó phải phân biệt được với một
 * lỗi cú pháp.
 */
function giaiDauNhanh(
  s: string,
  errors: BlazonError[],
): CadencyMark[] | null {
  const t = chuan(s);
  if (t === "") return [];

  const dau_hieu = "khác biệt bởi ";
  if (!t.startsWith(dau_hieu)) {
    errors.push({
      code: "unknown_cadency",
      at: t,
      detail: 'phần dấu nhánh thứ phải bắt đầu bằng "khác biệt bởi"',
    });
    return null;
  }

  const cac = t
    .slice(dau_hieu.length)
    .split(" rồi ")
    .map(chuan)
    .filter((x) => x !== "");

  const ra: CadencyMark[] = [];
  for (const c of cac) {
    if (!(CADENCY as readonly string[]).includes(c)) {
      errors.push({
        code: "unknown_cadency",
        at: c,
        detail: `không nhận ra dấu nhánh thứ; hợp lệ: ${CADENCY.join(", ")}`,
      });
      continue;
    }
    ra.push(c as CadencyMark);
  }
  return ra.length === cac.length ? ra : null;
}

/**
 * Giải một chuỗi blazon thành [`Arms`] (`§18.14.3`, `PF-18`).
 *
 * **Từ chối, không đoán.** Mọi lỗi trả về cùng lúc để người viết sửa một lần,
 * và luật màu được kiểm **sau** khi cú pháp đúng — báo "sai luật màu" cho một
 * chuỗi chưa phân tích được là một thông báo gây lạc hướng.
 */
export function parseBlazon(text: string): ParseResult {
  const errors: BlazonError[] = [];
  const t = chuan(text);
  if (t === "") {
    return {
      ok: false,
      errors: [{ code: "empty", at: text, detail: "chuỗi rỗng" }],
    };
  }

  const { truong, con_lai } = tach(t);
  const truong_ra = giaiTruong(truong, errors);

  // Phần còn lại: `<hình>[, khác biệt bởi …]`.
  const { truong: phan_hinh, con_lai: phan_dau } = tach(con_lai);
  const hinh_ra = phan_hinh.trim() === "" ? null : giaiHinh(phan_hinh, errors);
  if (phan_hinh.trim() === "") {
    errors.push({
      code: "missing_charge",
      at: con_lai,
      detail: "không có phần hình",
    });
  }
  const dau_ra = giaiDauNhanh(phan_dau, errors);

  if (!truong_ra || !hinh_ra || dau_ra === null) {
    return { ok: false, errors };
  }

  const arms: Arms = {
    division: truong_ra.division,
    field: truong_ra.field,
    charge: hinh_ra.charge,
    chargeTincture: hinh_ra.chargeTincture,
    cadency: dau_ra,
  };

  // ── Luật màu, kiểm **sau** khi cú pháp đã đúng ──
  //
  // `counterchanged` không cần kiểm: nó lấy sắc đối của phần trường bên dưới,
  // nên nó tương phản theo định nghĩa. Đó chính là lý do huy hiệu học phát
  // minh ra nó.
  if (arms.chargeTincture !== "counterchanged") {
    const sac = arms.chargeTincture;
    const dung = arms.field.every((f) => contrasts(f, sac));
    if (!dung) {
      errors.push({
        code: "tincture_rule",
        at: `${arms.field.join(" và ")} / ${arms.chargeTincture}`,
        detail:
          "vi phạm luật màu: không đặt kim loại lên kim loại, không đặt màu lên màu — " +
          "luật này là một chuẩn tương phản, không phải một quy ước trang trí",
      });
    }
  }

  if (errors.length > 0) return { ok: false, errors };
  return { ok: true, arms };
}

/**
 * Vòng khép kín: giải rồi viết lại phải ra đúng chuỗi ban đầu.
 *
 * Dùng trong test và trong `pack validate`. Một content pack khai blazon mà
 * không khép vòng thì lá cờ nó khai **không phải** lá cờ nó nghĩ.
 */
export function roundTrips(text: string): boolean {
  const r = parseBlazon(text);
  if (!r.ok) return false;
  return blazon(r.arms) === chuan(text);
}
