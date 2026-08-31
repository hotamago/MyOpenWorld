/**
 * Hệ biểu tượng hợp thành (`idea.md §18.14.1`, `§18.14.2`, `PA-14`).
 *
 * ## Vấn đề
 *
 * Một thế giới có hàng nghìn loại vật phẩm. Vẽ tay mỗi loại một icon là bất khả
 * thi, và tệ hơn: khi một modder thêm "rìu đồng thau", họ **không có icon nào**
 * cho tới khi có người ngồi vẽ. Thế giới mở rộng được bằng dữ liệu nhưng giao
 * diện thì không.
 *
 * ## Cách giải
 *
 * Icon không được vẽ, chúng được **hợp thành** từ khoảng một trăm bóng nguyên
 * thủy, xếp trên năm lớp:
 *
 * ```text
 * 5  annotation   số lượng, chất lượng, dấu hỏi khi chưa thẩm định
 * 4  marker       sở hữu, phe — dấu nhỏ ở góc
 * 3  state        hỏng, cháy, ướt, bị nguyền
 * 2  material     sắc thái theo vật liệu: gỗ, sắt, đồng thau
 * 1  silhouette   nó LÀ cái gì: rìu, sách, bình
 * ```
 *
 * "Rìu đồng thau bị mẻ của Aren" = `axe` + `brass` + `chipped` + `aren`. Không
 * ai phải vẽ gì cả.
 *
 * ## Vì sao khóa phải là hàm thuần
 *
 * `§18.14.2` đòi hỏi khóa icon là **hàm thuần của dữ liệu**. Đó không phải một
 * yêu cầu thẩm mỹ mà là điều kiện để atlas hoạt động:
 *
 * - Cùng dữ liệu → cùng khóa → **một ô atlas duy nhất** dùng lại cho cả nghìn
 *   cái rìu đồng thau. Nếu khóa phụ thuộc thứ tự hay thời điểm, mỗi cái rìu sẽ
 *   chiếm một ô riêng và atlas nổ tung.
 * - Khóa ổn định giữa các phiên → nướng atlas một lần lúc nạp, không phải mỗi
 *   khi có thứ mới xuất hiện trong tầm nhìn.
 */

/** Năm lớp, theo thứ tự vẽ từ dưới lên. */
export const LAYERS = ["silhouette", "material", "state", "marker", "annotation"] as const;

/** Tên một lớp. */
export type Layer = (typeof LAYERS)[number];

/** Một bóng nguyên thủy trong `content/`. */
export interface Primitive {
  /** Định danh có namespace: `core.axe`, `mypack.wand`. */
  id: string;
  /** Lớp mà nó thuộc về. */
  layer: Layer;
  /** Nội dung SVG, chỉ phần bên trong `<svg>`. */
  svg: string;
}

/** Một đặc tả icon — thứ mà dữ liệu game mô tả. */
export interface IconSpec {
  /** Hình dáng cơ bản. Bắt buộc: mọi thứ đều phải *là* một cái gì đó. */
  silhouette: string;
  /** Vật liệu, nếu có. */
  material?: string;
  /** Trạng thái, có thể nhiều. Sắp xếp trước khi tạo khóa. */
  states?: string[];
  /** Dấu sở hữu hoặc phe. */
  marker?: string;
  /** Chú thích: số lượng, chất lượng. */
  annotation?: string;
}

/** Lỗi khi hợp thành. */
export class IconError extends Error {
  constructor(
    public readonly spec: IconSpec,
    message: string,
  ) {
    super(message);
    this.name = "IconError";
  }
}

/**
 * Khóa của một icon. **Hàm thuần của đặc tả.**
 *
 * `states` được **sắp xếp** trước khi ghép: `["burnt", "wet"]` và
 * `["wet", "burnt"]` mô tả cùng một vật, nên chúng phải cho cùng một khóa.
 * Không sắp xếp thì cùng một món đồ sẽ chiếm hai ô atlas tùy vào thứ tự mà
 * effect được áp — và thứ tự đó thay đổi.
 */
export function iconKey(spec: IconSpec): string {
  const phan: string[] = [`s:${spec.silhouette}`];
  if (spec.material) phan.push(`m:${spec.material}`);
  if (spec.states?.length) {
    phan.push(`t:${[...spec.states].sort().join("+")}`);
  }
  if (spec.marker) phan.push(`k:${spec.marker}`);
  if (spec.annotation) phan.push(`a:${spec.annotation}`);
  return phan.join("|");
}

/** Sổ đăng ký bóng nguyên thủy. */
export class PrimitiveRegistry {
  #byId = new Map<string, Primitive>();

  /** Đăng ký một bóng. Trùng id là lỗi, không phải ghi đè im lặng. */
  add(p: Primitive): void {
    if (this.#byId.has(p.id)) {
      throw new Error(
        `bóng nguyên thủy trùng id \`${p.id}\`. Ghi đè phải khai báo tường minh ` +
          `trong manifest của pack (§22.29).`,
      );
    }
    if (!p.id.includes(".")) {
      throw new Error(`\`${p.id}\` thiếu namespace — mọi id phải có dạng \`<pack>.<tên>\``);
    }
    this.#byId.set(p.id, p);
  }

  /** Tra một bóng. */
  get(id: string): Primitive | undefined {
    return this.#byId.get(id);
  }

  /** Số bóng đã đăng ký. */
  get size(): number {
    return this.#byId.size;
  }

  /** Mọi id, đã sắp xếp. */
  ids(): string[] {
    return [...this.#byId.keys()].sort();
  }

  /** Mọi id thuộc một lớp, đã sắp xếp. */
  idsOfLayer(layer: Layer): string[] {
    return [...this.#byId.values()]
      .filter((p) => p.layer === layer)
      .map((p) => p.id)
      .sort();
  }
}

/** Một icon đã hợp thành. */
export interface ComposedIcon {
  key: string;
  /** SVG đầy đủ, sẵn sàng để rasterize vào atlas. */
  svg: string;
  /** Các bóng đã dùng, theo thứ tự vẽ. */
  used: string[];
}

/** Cạnh của một ô icon, tính bằng pixel ở zoom 1. */
export const ICON_SIZE = 32;

/**
 * Hợp thành một icon.
 *
 * Ném [`IconError`] khi một bóng không tồn tại. **Không** vẽ một hình mặc định
 * thay thế: một icon "dấu hỏi" trông giống hệt icon "chưa thẩm định" của
 * `§18.14.5`, và người chơi sẽ đọc một lỗi hiển thị thành một sự thật về thế
 * giới. Thà đỏ trong CI còn hơn nói dối trên màn hình.
 */
export function compose(spec: IconSpec, reg: PrimitiveRegistry): ComposedIcon {
  const used: string[] = [];
  const phan: string[] = [];

  const them = (layer: Layer, id: string) => {
    const p = reg.get(id);
    if (!p) {
      throw new IconError(
        spec,
        `không có bóng nguyên thủy \`${id}\` cho lớp \`${layer}\`. ` +
          `Mỗi định nghĩa phải giải ra được một icon; thiếu bóng là lỗi CI, ` +
          `không phải lý do để vẽ dấu hỏi (§18.14.5 đã dùng dấu hỏi cho ` +
          `"chưa thẩm định", nên dùng nó cho lỗi sẽ nói dối người chơi).`,
      );
    }
    if (p.layer !== layer) {
      throw new IconError(
        spec,
        `\`${id}\` thuộc lớp \`${p.layer}\` nhưng được dùng ở lớp \`${layer}\``,
      );
    }
    used.push(id);
    phan.push(`<g data-layer="${layer}" data-prim="${id}">${p.svg}</g>`);
  };

  // Silhouette và material trước.
  them("silhouette", spec.silhouette);
  if (spec.material) them("material", spec.material);

  // Trạng thái: **theo thứ tự đã sắp xếp**, giống như trong khóa. Nếu thứ tự vẽ
  // khác thứ tự trong khóa, hai icon có cùng khóa sẽ trông khác nhau — và cái
  // nào được nướng vào atlas là tùy vào cái nào tới trước.
  for (const st of [...(spec.states ?? [])].sort()) {
    them("state", st);
  }

  if (spec.marker) them("marker", spec.marker);
  if (spec.annotation) them("annotation", spec.annotation);

  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${ICON_SIZE} ${ICON_SIZE}" ` +
    `width="${ICON_SIZE}" height="${ICON_SIZE}">${phan.join("")}</svg>`;

  return { key: iconKey(spec), svg, used };
}

/**
 * Kiểm mọi đặc tả giải ra được icon (`PA-14`, bước CI).
 *
 * Trả danh sách lỗi. Rỗng nghĩa là đạt.
 */
export function validateAllSpecs(
  specs: IconSpec[],
  reg: PrimitiveRegistry,
): string[] {
  const loi: string[] = [];
  for (const s of specs) {
    try {
      compose(s, reg);
    } catch (e) {
      loi.push(e instanceof Error ? e.message : String(e));
    }
  }
  return loi;
}

/**
 * Nướng một tập icon thành bố cục atlas.
 *
 * Thứ tự ô là **thứ tự khóa đã sắp xếp**, không phải thứ tự gặp. Nhờ vậy hai
 * lần chạy cho cùng một atlas, và một ảnh chụp màn hình so sánh được giữa các
 * phiên bản.
 */
export function bakeAtlas(keys: string[]): {
  columns: number;
  rows: number;
  slots: Map<string, { col: number; row: number }>;
} {
  const duy_nhat = [...new Set(keys)].sort();
  const columns = Math.max(1, Math.ceil(Math.sqrt(duy_nhat.length)));
  const rows = Math.max(1, Math.ceil(duy_nhat.length / columns));
  const slots = new Map<string, { col: number; row: number }>();
  duy_nhat.forEach((k, i) => {
    slots.set(k, { col: i % columns, row: Math.floor(i / columns) });
  });
  return { columns, rows, slots };
}
