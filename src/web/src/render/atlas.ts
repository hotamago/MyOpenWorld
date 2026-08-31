/**
 * Tile atlas **suy ra từ dữ liệu vật liệu** (`idea.md §18.5.1`, `PA-11`).
 *
 * Cùng một bài toán như hệ icon, cùng một cách giải: không ai vẽ tay từng ô.
 * Diện mạo của một ô được **tính** từ định nghĩa vật liệu — màu nền, độ nhám,
 * hoa văn — rồi nướng một lần lúc nạp.
 *
 * Hệ quả là thứ `PA-11` yêu cầu: *"Modder thêm vật liệu là có tile ngay."* Họ
 * viết một file YAML mô tả đá bazan, và bazan xuất hiện trên bản đồ với diện
 * mạo nhất quán với mọi loại đá khác. Không phải chờ ai vẽ.
 *
 * ## Vì sao nướng một lần lúc nạp
 *
 * Ba lựa chọn, và hai trong số đó sai:
 *
 * - **Vẽ mỗi ô mỗi khung hình**: giết hiệu năng. Một màn hình có hàng chục
 *   nghìn ô.
 * - **Nướng lười, khi ô đầu tiên của loại đó xuất hiện**: gây khựng ngay lúc
 *   người chơi bước vào vùng mới — đúng lúc họ đang chú ý nhất.
 * - **Nướng hết lúc nạp**: tốn một chút thời gian khởi động, và sau đó không
 *   bao giờ khựng nữa. Đây là lựa chọn đúng, vì số vật liệu là hữu hạn và biết
 *   trước từ content pack.
 */

/** Định nghĩa vật liệu, như nó nằm trong `content/`. */
export interface MaterialDef {
  /** Định danh có namespace. */
  id: string;
  /** Màu nền, hex. */
  color: string;
  /**
   * Độ nhám bề mặt, `0`–`3`.
   *
   * Quyết định mật độ hạt nhiễu vẽ lên ô. Đây là thứ khiến cát trông khác đá
   * dù hai màu có thể gần nhau — và nó là **tín hiệu phụ** cho người mù màu,
   * cùng vai trò với hoa văn ở bảng màu.
   */
  roughness: number;
  /** Có phải chất lỏng không — chất lỏng được vẽ với gợn sóng. */
  liquid?: boolean;
  /** Độ sáng riêng, `0`–`3`. Magma và nấm phát quang có giá trị dương. */
  emissive?: number;
}

/** Cạnh một ô, tính bằng pixel. */
export const TILE_SIZE = 16;

/** Số biến thể của mỗi vật liệu. */
export const VARIANTS = 4;

/**
 * Khóa của một ô đã nướng. **Hàm thuần của định nghĩa cộng chỉ số biến thể.**
 *
 * Cùng lý do như [`iconKey`]: khóa ổn định là điều kiện để atlas dùng lại được
 * một ô cho hàng nghìn lần vẽ.
 *
 * [`iconKey`]: ./icons/compose.ts
 */
export function tileKey(def: MaterialDef, variant: number): string {
  return `${def.id}#${variant % VARIANTS}`;
}

/**
 * Chọn biến thể cho một ô dựa trên tọa độ.
 *
 * Phải là hàm thuần của tọa độ, không phải ngẫu nhiên. Nếu ngẫu nhiên, cùng
 * một ô sẽ đổi diện mạo mỗi khi chunk được nạp lại — mặt đất nhấp nháy khi
 * người chơi đi qua đi lại.
 *
 * Băm rẻ tiền: đủ tốt để mắt không thấy hoa văn lặp, và đủ nhanh để chạy cho
 * mọi ô trên màn hình.
 */
export function variantAt(x: bigint, y: bigint): number {
  // Trộn hai tọa độ bằng số nguyên tố lớn rồi lấy vài bit giữa. `BigInt.asUintN`
  // giữ phép toán trong 64 bit thay vì để bigint lớn vô hạn.
  let h = BigInt.asUintN(64, x * 0x9e37_79b9_7f4a_7c15n + y * 0xc2b2_ae3d_27d4_eb4fn);
  h = BigInt.asUintN(64, h ^ (h >> 29n));
  h = BigInt.asUintN(64, h * 0xbf58_476d_1ce4_e5b9n);
  h = BigInt.asUintN(64, h ^ (h >> 32n));
  return Number(h % BigInt(VARIANTS));
}

/** Một ô đã nướng: dữ liệu pixel RGBA. */
export interface BakedTile {
  key: string;
  width: number;
  height: number;
  pixels: Uint8ClampedArray;
}

function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) throw new Error(`vật liệu \`${hex}\` không phải màu hợp lệ (#rrggbb)`);
  const v = Number.parseInt(m[1]!, 16);
  return [(v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff];
}

/** Băm xác định cho hạt nhiễu trong ô. */
function grain(seed: number, px: number, py: number): number {
  let h = (seed * 374761393 + px * 668265263 + py * 2246822519) >>> 0;
  h = (h ^ (h >>> 13)) >>> 0;
  h = Math.imul(h, 1274126177) >>> 0;
  return (h ^ (h >>> 16)) >>> 0;
}

/**
 * Nướng một ô.
 *
 * Thuần, xác định, không phụ thuộc canvas — nên nó chạy được trong test và
 * trong Web Worker, không chỉ trên luồng chính.
 */
export function bakeTile(def: MaterialDef, variant: number): BakedTile {
  const [r, g, b] = hexToRgb(def.color);
  const px = new Uint8ClampedArray(TILE_SIZE * TILE_SIZE * 4);

  // Biên độ hạt theo độ nhám. `0` cho mặt phẳng tuyệt đối như băng.
  const bien_do = [0, 6, 14, 24][Math.min(3, Math.max(0, def.roughness))]!;
  const seed = variant * 7919 + 1;

  for (let y = 0; y < TILE_SIZE; y++) {
    for (let x = 0; x < TILE_SIZE; x++) {
      const i = (y * TILE_SIZE + x) * 4;
      let d = 0;
      if (bien_do > 0) {
        d = (grain(seed, x, y) % (bien_do * 2 + 1)) - bien_do;
      }
      if (def.liquid) {
        // Gợn sóng theo hàng: chất lỏng phải nhận ra được **bằng hình**, không
        // chỉ bằng màu — người mù màu vẫn phải phân biệt nước với đồng cỏ.
        d += ((x + y * 3 + variant) % 8 < 4 ? 5 : -5);
      }
      const e = (def.emissive ?? 0) * 12;
      px[i] = r + d + e;
      px[i + 1] = g + d + e;
      px[i + 2] = b + d + Math.floor(e / 2);
      px[i + 3] = 255;
    }
  }

  return { key: tileKey(def, variant), width: TILE_SIZE, height: TILE_SIZE, pixels: px };
}

/** Atlas đã nướng. */
export class TileAtlas {
  #tiles = new Map<string, BakedTile>();
  #index = new Map<string, number>();
  #order: string[] = [];

  /**
   * Nướng toàn bộ vật liệu.
   *
   * Duyệt theo **id đã sắp xếp**, nên chỉ số ô trong atlas ổn định giữa các lần
   * chạy. Không có nó, một ảnh chụp màn hình so sánh giữa hai phiên bản sẽ khác
   * nhau chỉ vì thứ tự đọc thư mục khác nhau.
   */
  bake(defs: MaterialDef[]): void {
    for (const def of [...defs].sort((a, b) => a.id.localeCompare(b.id))) {
      for (let v = 0; v < VARIANTS; v++) {
        const t = bakeTile(def, v);
        if (!this.#tiles.has(t.key)) {
          this.#index.set(t.key, this.#order.length);
          this.#order.push(t.key);
          this.#tiles.set(t.key, t);
        }
      }
    }
  }

  /** Số ô đã nướng. */
  get size(): number {
    return this.#tiles.size;
  }

  /** Một ô. */
  get(key: string): BakedTile | undefined {
    return this.#tiles.get(key);
  }

  /** Chỉ số của một ô trong atlas. */
  indexOf(key: string): number | undefined {
    return this.#index.get(key);
  }

  /** Ô cho một vật liệu tại một tọa độ. */
  tileAt(def: MaterialDef, x: bigint, y: bigint): BakedTile {
    const k = tileKey(def, variantAt(x, y));
    const t = this.#tiles.get(k);
    if (!t) {
      throw new Error(
        `vật liệu \`${def.id}\` chưa được nướng. Atlas được nướng một lần lúc ` +
          `nạp từ toàn bộ content pack; một vật liệu xuất hiện mà không có trong ` +
          `atlas nghĩa là nó không được khai báo trong pack nào.`,
      );
    }
    return t;
  }

  /** Mọi khóa, theo thứ tự chỉ số. */
  keys(): string[] {
    return [...this.#order];
  }
}
