/**
 * Huy hiệu: dòng dõi đọc được từ lá cờ (`idea.md §18.14.3`, `PD-20`).
 *
 * ## Hai quy tắc, và cả hai có **công dụng cơ học**
 *
 * **1. Luật màu — không kim loại lên kim loại, không màu lên màu.**
 *
 * > Luật này tồn tại suốt nhiều thế kỷ vì đúng một lý do — **để nhìn rõ từ xa**.
 * > Nó chính là một chuẩn tương phản có trước khi ai đó nghĩ ra chữ "tương
 * > phản", và nó cho ta miễn phí thứ mà `§18.6` phải kiểm tra bằng máy.
 *
 * **2. Nhánh thứ thừa kế huy hiệu của nhánh chính, cộng đúng một dấu khác biệt.**
 *
 * > Đây là chỗ hệ thống này trở nên đáng giá: huy hiệu **tự nó mã hóa đồ thị
 * > huyết thống**. Nhìn hai lá cờ là biết hai bên cùng dòng máu và bên nào là
 * > nhánh thứ, **trước khi có bất kỳ dòng thoại nào giải thích**.
 *
 * ## Giải ràng buộc, **không quay xúc xắc rồi thử lại**
 *
 * `§18.14.3` nói thẳng cách cài:
 *
 * > Mỗi thành phần chọn màu từ tập hợp lệ đối với thứ nó nằm lên. Vòng lặp
 * > thử-lại có thể chạy **số lần khác nhau giữa hai lần chạy** và sẽ phá
 * > determinism.
 *
 * Khác biệt rất cụ thể: `while (!valid) { roll() }` tiêu một số lượng ngẫu nhiên
 * **không đoán trước được** từ dòng RNG, nên mọi thứ rút sau đó lệch đi. Ở đây
 * [`pick`] lọc tập hợp lệ **trước**, rồi lấy đúng một số — luôn đúng một số.
 */

/** Tinctures chia làm hai nhóm, và luật màu là luật giữa hai nhóm. */
export const METALS = ["or", "argent"] as const;
/** Nhóm màu. */
export const COLOURS = ["gules", "azure", "sable", "vert", "purpure"] as const;

/** Một tincture. */
export type Tincture = (typeof METALS)[number] | (typeof COLOURS)[number];

/** Nhóm của một tincture. */
export function classOf(t: Tincture): "metal" | "colour" {
  return (METALS as readonly string[]).includes(t) ? "metal" : "colour";
}

/**
 * Luật màu: hai tincture có được đặt cạnh/chồng nhau không.
 *
 * Kim loại trên màu và màu trên kim loại thì được. Cùng nhóm thì không — và đó
 * chính là ràng buộc tương phản.
 */
export function contrasts(a: Tincture, b: Tincture): boolean {
  return classOf(a) !== classOf(b);
}

/** Cách chia trường. */
export const DIVISIONS = [
  "plain",
  "per_pale",
  "per_fess",
  "per_bend",
  "quarterly",
  "per_chevron",
] as const;

/** Tên một cách chia. */
export type Division = (typeof DIVISIONS)[number];

/** Hình đặt trên trường. */
export const CHARGES = [
  "lion",
  "eagle",
  "tower",
  "hammer",
  "star",
  "wheat",
  "fish",
  "oak",
] as const;

/** Tên một hình. */
export type Charge = (typeof CHARGES)[number];

/**
 * **Dấu khác biệt** của nhánh thứ (`cadency mark`).
 *
 * Thứ tự có nghĩa: con trưởng mang `label`, con thứ hai `crescent`, và cứ thế.
 * Nhìn dấu là biết thứ bậc trong nhà — đó là toàn bộ điểm của `§18.14.3` quy
 * tắc 2.
 */
export const CADENCY = [
  "label",
  "crescent",
  "mullet",
  "martlet",
  "annulet",
  "fleur_de_lis",
  "rose",
  "cross_moline",
] as const;

/** Một dấu khác biệt. */
export type CadencyMark = (typeof CADENCY)[number];

/**
 * Màu của hình trên trường.
 *
 * ## Vì sao có `"counterchanged"`
 *
 * Luật màu và trường chia đôi **mâu thuẫn nhau** nếu không có nó, và mâu thuẫn
 * đó là toàn phần chứ không phải hiếm gặp:
 *
 * - Quy tắc 1 đòi hai nửa trường phải khác nhóm — một kim loại, một màu.
 * - Cùng quy tắc đó đòi hình phải khác nhóm với **cả hai** nửa.
 * - Chỉ có hai nhóm. Nên **không tồn tại** màu nào thỏa cả hai vế.
 *
 * Bản đầu của module này không nhận ra điều đó và ném lỗi ở mọi lá cờ có trường
 * chia — tức là gần hết.
 *
 * Huy hiệu học đã giải bài này từ nhiều thế kỷ trước, và lời giải là
 * **counterchanged**: hình đổi màu theo từng nửa, lấy đúng màu của nửa đối diện.
 * Nhờ vậy nó tương phản ở mọi chỗ, và nhìn từ xa vẫn rõ — đúng mục đích mà cả
 * luật màu tồn tại vì nó.
 */
export type ChargeTincture = Tincture | "counterchanged";

/** Một huy hiệu. */
export interface Arms {
  /** Cách chia trường. */
  division: Division;
  /** Màu nền. Hai màu nếu trường được chia. */
  field: [Tincture] | [Tincture, Tincture];
  /** Hình trên trường. */
  charge: Charge;
  /** Màu của hình. */
  chargeTincture: ChargeTincture;
  /**
   * Dấu khác biệt, nếu đây là nhánh thứ.
   *
   * Chuỗi dấu, không phải một dấu: nhánh thứ của nhánh thứ mang **hai** dấu, và
   * độ dài chuỗi chính là số đời kể từ nhánh chính.
   */
  cadency: CadencyMark[];
}

/**
 * Chọn một phần tử từ tập **đã lọc hợp lệ**, tiêu đúng một số ngẫu nhiên.
 *
 * Ném khi tập rỗng thay vì rơi về một giá trị mặc định: một huy hiệu không có
 * màu hợp lệ nào là một lỗi dữ liệu, và im lặng thay bằng `or` sẽ tạo ra hàng
 * loạt lá cờ giống hệt nhau mà không ai biết vì sao.
 */
function pick<T>(from: readonly T[], roll: number): T {
  if (from.length === 0) {
    throw new RangeError("tập lựa chọn rỗng: không có màu nào hợp luật");
  }
  return from[roll % from.length]!;
}

/**
 * Băm xác định trên `BigInt` — cùng lý do như `portrait.ts`.
 *
 * Một `seed` là `u64`; `Number` mất chính xác trên `2^53`, và hai dòng họ khác
 * nhau sẽ nhận cùng một lá cờ.
 */
function hash(seed: bigint, salt: string): bigint {
  const PRIME = 0x100000001b3n;
  const MASK = 0xffffffffffffffffn;
  let h = 0xcbf29ce484222325n;
  for (const b of new TextEncoder().encode(salt)) {
    h = ((h ^ BigInt(b)) * PRIME) & MASK;
  }
  let s = seed;
  for (let i = 0; i < 8; i++) {
    h = ((h ^ (s & 0xffn)) * PRIME) & MASK;
    s >>= 8n;
  }
  return h;
}

/**
 * Sinh huy hiệu cho một dòng họ **gốc**.
 *
 * Hàm thuần của `seed`. Không có vòng thử-lại: mỗi bước lọc tập hợp lệ rồi lấy
 * đúng một số.
 */
export function generateArms(seed: bigint): Arms {
  const division = pick(DIVISIONS, Number(hash(seed, "division") % 1000n));
  const field0 = pick(
    [...METALS, ...COLOURS],
    Number(hash(seed, "field0") % 1000n),
  );

  // Trường thứ hai (nếu có) phải tương phản với trường thứ nhất.
  const field: [Tincture] | [Tincture, Tincture] =
    division === "plain"
      ? [field0]
      : [
          field0,
          pick(
            [...METALS, ...COLOURS].filter((t) => contrasts(t, field0)),
            Number(hash(seed, "field1") % 1000n),
          ),
        ];

  // Hình phải nhìn rõ trên **mọi** nửa trường nó vắt qua.
  //
  // Trường không chia: chọn một màu khác nhóm với nền.
  // Trường có chia: **counterchanged** — xem `ChargeTincture`. Không có màu đơn
  // nào thỏa được cả hai nửa, nên đây không phải một lựa chọn thẩm mỹ mà là lời
  // giải duy nhất.
  const chargeTincture: ChargeTincture =
    field.length === 1
      ? pick(
          [...METALS, ...COLOURS].filter((t) => contrasts(t, field[0])),
          Number(hash(seed, "charge_tincture") % 1000n),
        )
      : "counterchanged";

  return {
    division,
    field,
    charge: pick(CHARGES, Number(hash(seed, "charge") % 1000n)),
    chargeTincture,
    cadency: [],
  };
}

/**
 * Huy hiệu của một **nhánh thứ**: thừa kế nguyên vẹn, cộng **đúng một** dấu.
 *
 * `birthOrder` là thứ tự sinh, bắt đầu từ 0 cho con trưởng.
 *
 * Không đổi màu, không đổi hình, không đổi cách chia. Đổi bất kỳ thứ nào trong
 * đó là phá chính cái làm hệ thống này đáng giá: nhìn hai lá cờ mà biết ngay hai
 * bên cùng dòng máu.
 */
export function cadetArms(parent: Arms, birthOrder: number): Arms {
  return {
    ...parent,
    field: [...parent.field] as [Tincture] | [Tincture, Tincture],
    cadency: [...parent.cadency, CADENCY[birthOrder % CADENCY.length]!],
  };
}

/** Hai huy hiệu có **cùng gốc** không — tức là cùng dòng máu. */
export function sameLineage(a: Arms, b: Arms): boolean {
  return (
    a.division === b.division &&
    a.charge === b.charge &&
    a.chargeTincture === b.chargeTincture &&
    a.field.length === b.field.length &&
    a.field.every((t, i) => t === b.field[i])
  );
}

/**
 * Cách nhánh chính bao nhiêu đời.
 *
 * `0` là chính nhánh. Đây là con số mà người chơi đọc được **từ lá cờ**, không
 * cần mở bảng gia phả.
 */
export function generationsFromMain(a: Arms): number {
  return a.cadency.length;
}

/**
 * Kiểm một huy hiệu có tuân **luật màu** không.
 *
 * Dùng trong test và trong bộ kiểm content pack. Một huy hiệu vi phạm luật màu
 * là một huy hiệu không đọc được từ xa — và `§18.6` gọi đó là lỗi, không phải
 * lựa chọn thẩm mỹ.
 */
export function violatesTincture(a: Arms): string[] {
  const loi: string[] = [];
  if (a.field.length === 2 && !contrasts(a.field[0], a.field[1]!)) {
    loi.push(`hai nửa trường cùng nhóm: ${a.field[0]} / ${a.field[1]}`);
  }
  // Counterchanged tương phản ở mọi nửa theo cách nó được dựng, nên không có gì
  // để kiểm — kiểm nó bằng luật màu sẽ luôn báo lỗi cho một lá cờ hoàn toàn hợp lệ.
  if (a.chargeTincture !== "counterchanged") {
    for (const f of a.field) {
      if (!contrasts(a.chargeTincture, f)) {
        loi.push(`hình ${a.chargeTincture} không tương phản với trường ${f}`);
      }
    }
  }
  return loi;
}

/**
 * Mô tả bằng lời — dùng cho tooltip và cho người đọc màn hình.
 *
 * `§18.13`: triệu chứng trước, con số sau. Một lá cờ phải nói được thành lời,
 * nếu không nó chỉ là trang trí với người không phân biệt được màu.
 */
export function blazon(a: Arms): string {
  const truong =
    a.field.length === 1
      ? a.field[0]
      : `${a.division.replace(/_/g, " ")} ${a.field[0]} và ${a.field[1]}`;
  const dau =
    a.cadency.length === 0
      ? ""
      : `, khác biệt bởi ${a.cadency.join(" rồi ")}`;
  return `${truong}, một ${a.charge} ${a.chargeTincture}${dau}`;
}
