/**
 * Chân dung tối thiểu (`idea.md §18.14.4`, `PC-17`).
 *
 * > Deterministic từ `genotype_seed` cộng phenotype: cùng một cá thể luôn ra
 * > cùng chân dung, và **con cái trông giống cha mẹ** vì lớp hình thái lấy từ
 * > cùng bộ gen.
 *
 * ## Vì sao "giống cha mẹ" không tự nhiên có
 *
 * Cách hiển nhiên là băm `entity_id` rồi lấy lớp theo số dư. Nó cho ra chân dung
 * ổn định — cùng cá thể luôn cùng mặt — và **hỏng đúng cái đáng giá nhất**: hai
 * anh em ruột nhận hai `entity_id` khác nhau, nên trông không liên quan gì tới
 * nhau. Một thế giới có dòng họ mà các dòng họ không có gương mặt thì cái công
 * phu của `§9.5.2` không nhìn thấy được ở đâu cả.
 *
 * Nên khóa của các lớp **hình thái** là `genotype_seed`, thứ mà con thừa hưởng
 * từ cha mẹ qua tái tổ hợp. Còn các lớp **trạng thái** — gầy vì đói, xanh vì
 * bệnh, sẹo sau trận — lấy từ phenotype hiện tại, và chúng đổi theo thời gian.
 *
 * ```text
 *  genotype_seed ──► loài, sắc da, tóc, mắt, nét mặt      (đời không đổi)
 *  phenotype     ──► thể trạng, tuổi, dấu hiệu effect     (đổi theo tick)
 * ```
 *
 * ## Bộ tối thiểu ở đây, bộ 15 lớp ở `PF-19`
 *
 * `PC-17` chỉ cần đủ để nhận ra: loài, tuổi, và trạng thái thấy được. Tách như
 * vậy vì bộ đầy đủ cần trang phục theo văn hóa và theo địa vị — hai thứ mà
 * `§12.3` và `§12.10` chưa có ở giai đoạn này. Vẽ chúng bằng dữ liệu bịa ra bây
 * giờ thì sau này phải gỡ, và trong lúc đó người chơi đã học sai một ngôn ngữ
 * hình ảnh mà `§18.14.6` nói là không được đổi.
 */

/** Một lớp trong chân dung. */
export const PORTRAIT_LAYERS = [
  "species",
  "build",
  "age",
  "skin",
  "hair",
  "eyes",
  "expression",
  "condition",
] as const;

/** Tên lớp. */
export type PortraitLayer = (typeof PORTRAIT_LAYERS)[number];

/**
 * Lớp nào lấy khóa từ gen, lớp nào lấy từ trạng thái hiện tại.
 *
 * Bảng này **là** cơ chế "con giống cha mẹ". Chuyển một lớp từ `genotype` sang
 * `phenotype` sẽ làm lớp đó thôi di truyền, và không có gì báo lỗi — chân dung
 * vẫn dựng được, chỉ là các dòng họ mất dần nét chung.
 */
export const LAYER_SOURCE: Record<PortraitLayer, "genotype" | "phenotype"> = {
  species: "genotype",
  build: "phenotype",
  age: "phenotype",
  skin: "genotype",
  hair: "genotype",
  eyes: "genotype",
  expression: "phenotype",
  condition: "phenotype",
};

/** Kiểu hình quan sát được ngay lúc này. */
export interface Phenotype {
  /** Định danh loài, có namespace. */
  species: string;
  /** Tuổi tính bằng năm thế giới. */
  ageYears: number;
  /** Mức dinh dưỡng `0`–`1000`; thấp thì gầy đi. */
  nutrition: number;
  /** Mức bệnh `0`–`1000`; cao thì xanh xao. */
  illness: number;
  /** Số sẹo nhìn thấy được. */
  scars: number;
  /** Tâm trạng `-1000`..`1000`, quyết định biểu cảm. */
  mood: number;
}

/** Một lớp đã chọn xong phương án. */
export interface ResolvedLayer {
  layer: PortraitLayer;
  /** Định danh phương án, ổn định — `§18.14.6` cấm đổi nghĩa cái đã phát hành. */
  variant: string;
}

/** Chân dung đã dựng. */
export interface Portrait {
  layers: ResolvedLayer[];
  /** Khóa để cache và để so sánh. */
  key: string;
}

/**
 * Băm 64-bit trên `BigInt`, xác định và giống nhau trên mọi máy.
 *
 * Dùng `BigInt` chứ không dùng `number` vì `genotype_seed` là `u64`, và
 * `Number` mất chính xác trên 2^53 — hai bộ gen khác nhau sẽ băm ra cùng một số
 * và hai người xa lạ bỗng giống hệt nhau. Đây là cùng lý do mà `§22.10` bắt
 * dùng `BigInt` ở biên JS.
 */
function bam(seed: bigint, salt: string): bigint {
  // FNV-1a 64-bit. Chọn nó vì nó ngắn, không có bảng tra, và **cùng kết quả ở
  // mọi ngôn ngữ** — Rust sinh chân dung phía server sẽ ra đúng cái này.
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

/** Chọn một phương án trong `n` phương án, xác định theo seed. */
function chon(seed: bigint, salt: string, n: number): number {
  if (n <= 0) throw new RangeError(`lớp '${salt}' không có phương án nào`);
  return Number(bam(seed, salt) % BigInt(n));
}

/**
 * Số phương án của mỗi lớp hình thái.
 *
 * Con số nhỏ có chủ đích: `§18.14.4` nói "mỗi lớp vài phương án". Nhiều phương
 * án làm mỗi khuôn mặt độc nhất và làm **mất** cảm giác họ hàng, vì hai anh em
 * chia nhau một nửa bộ gen vẫn sẽ rơi vào hai phương án khác nhau.
 */
const SO_PHUONG_AN: Record<string, number> = {
  skin: 6,
  hair: 8,
  eyes: 5,
};

/** Thể trạng theo dinh dưỡng. */
function theTrang(nutrition: number): string {
  if (nutrition < 200) return "emaciated";
  if (nutrition < 450) return "gaunt";
  if (nutrition > 850) return "stout";
  return "normal";
}

/** Nhóm tuổi. Ngưỡng theo loài là việc của `PF-19`; ở đây là bộ tối thiểu. */
function nhomTuoi(years: number): string {
  if (years < 3) return "infant";
  if (years < 13) return "child";
  if (years < 20) return "adolescent";
  if (years < 45) return "adult";
  if (years < 65) return "middle";
  return "elder";
}

/** Biểu cảm theo tâm trạng. */
function bieuCam(mood: number): string {
  if (mood < -600) return "anguished";
  if (mood < -200) return "grim";
  if (mood > 600) return "elated";
  if (mood > 200) return "content";
  return "neutral";
}

/**
 * Dấu hiệu trạng thái thấy được.
 *
 * Chỉ **một** lớp, và nó chọn cái nặng nhất. Chồng cả bệnh lẫn sẹo lẫn đói lên
 * cùng một khuôn mặt nhỏ 32px cho ra một mớ không đọc được — và `§18.13` nguyên
 * tắc 4 nói thẳng: đổ hết mọi thứ ra cùng lúc là cách chắc chắn nhất khiến
 * không ai đọc gì.
 */
function tinhTrang(p: Phenotype): string {
  if (p.illness > 700) return "gravely_ill";
  if (p.illness > 300) return "sickly";
  if (p.scars >= 3) return "scarred_heavy";
  if (p.scars >= 1) return "scarred";
  return "none";
}

/**
 * Dựng chân dung.
 *
 * **Hàm thuần** của `(genotypeSeed, phenotype)`. Không đọc đồng hồ, không có
 * ngẫu nhiên, không đọc `entity_id`. Đó là ba điều kiện để cùng một cá thể luôn
 * ra cùng chân dung trên mọi máy, mọi lần chạy.
 */
export function buildPortrait(genotypeSeed: bigint, p: Phenotype): Portrait {
  const layers: ResolvedLayer[] = [
    { layer: "species", variant: p.species },
    { layer: "build", variant: theTrang(p.nutrition) },
    { layer: "age", variant: nhomTuoi(p.ageYears) },
    { layer: "skin", variant: `skin_${chon(genotypeSeed, "skin", SO_PHUONG_AN.skin!)}` },
    { layer: "hair", variant: `hair_${chon(genotypeSeed, "hair", SO_PHUONG_AN.hair!)}` },
    { layer: "eyes", variant: `eyes_${chon(genotypeSeed, "eyes", SO_PHUONG_AN.eyes!)}` },
    { layer: "expression", variant: bieuCam(p.mood) },
    { layer: "condition", variant: tinhTrang(p) },
  ];
  return { layers, key: layers.map((l) => `${l.layer}:${l.variant}`).join("|") };
}

/**
 * Hai chân dung giống nhau tới mức nào ở **phần di truyền**, `0`–`1`.
 *
 * Chỉ tính các lớp `genotype`. Tính cả lớp trạng thái sẽ làm hai người lạ cùng
 * đang ốm trông "họ hàng" hơn hai anh em mà một người khỏe — nghĩa là số đo
 * không còn đo cái nó định đo.
 */
export function kinshipResemblance(a: Portrait, b: Portrait): number {
  const gen = PORTRAIT_LAYERS.filter((l) => LAYER_SOURCE[l] === "genotype");
  const lay = (p: Portrait, l: PortraitLayer) =>
    p.layers.find((x) => x.layer === l)?.variant;
  const trung = gen.filter((l) => lay(a, l) !== undefined && lay(a, l) === lay(b, l)).length;
  return trung / gen.length;
}

/**
 * Chân dung của một người **chưa từng gặp** (`§18.14.5`).
 *
 * > Người chưa từng gặp hiện bóng chung với đúng những gì đã quan sát được,
 * > không phải chân dung đầy đủ.
 *
 * Nên hàm này nhận `observed` — tập lớp mà người xem thật sự nhận ra — và mọi
 * lớp khác thành `unknown`. Nó **không** nhận `genotypeSeed`: không có seed thì
 * không có cách nào lỡ tay vẽ ra khuôn mặt thật.
 */
export function strangerPortrait(observed: Partial<Record<PortraitLayer, string>>): Portrait {
  const layers: ResolvedLayer[] = PORTRAIT_LAYERS.map((l) => ({
    layer: l,
    variant: observed[l] ?? "unknown",
  }));
  return { layers, key: layers.map((l) => `${l.layer}:${l.variant}`).join("|") };
}
