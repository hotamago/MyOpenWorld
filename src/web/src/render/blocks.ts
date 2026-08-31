/**
 * Bảng vật liệu, **nạp từ dữ liệu** chứ không viết cứng trong mã.
 *
 * ## Vì sao
 *
 * Bản đầu tiên có một `Record<string, number>` gồm 11 màu nằm ngay trong mã vẽ.
 * Nó chạy, và nó đóng cứng trò chơi: thêm một vật liệu đòi sửa một enum Rust,
 * một bảng màu TypeScript, và một hàm phân loại — ba chỗ, hai ngôn ngữ, cho một
 * thứ đáng lẽ là **một thư mục dữ liệu**.
 *
 * `§8.2` đã định nghĩa material là dữ liệu và `§19.7` đã định nghĩa content
 * pack. Module này chỉ là phía đọc: server trả về bảng vật liệu của pack đang
 * nạp, client dựng bảng tra từ đó.
 *
 * ## Vẫn có bảng dự phòng, và nó không phải thừa
 *
 * Nếu server chưa trả bảng (bản cũ, hoặc pack hỏng), giao diện phải vẽ được
 * **một cái gì đó** thay vì một màn hình đen. Nhưng bảng dự phòng dùng màu tím
 * cho vật liệu lạ — nhìn là biết ngay "cái này chưa có định nghĩa", thay vì
 * đoán một màu hợp lý và giấu luôn vấn đề.
 */

import type { BlockInfo } from "@/api/game";
import { localized } from "@/i18n";

export interface BlockDef {
  id: string;
  name: Partial<Record<"en" | "vi", string>>;
  /** Màu nền ô, `0xRRGGBB`. */
  color: number;
  /** Chất lỏng: ảnh hưởng đổ bóng và đường bờ. */
  liquid: boolean;
  /** Đi xuyên qua được không. */
  walkable: boolean;
  tags: string[];
}

/** Màu cho vật liệu không có định nghĩa. Cố tình chói. */
export const UNDEFINED_COLOR = 0xff00ff;

/**
 * Bảng dự phòng, dùng khi server chưa trả bảng vật liệu.
 *
 * Giữ đồng bộ với `content/core/blocks/<id>/metadata.yaml`. Lệch nhau thì thứ
 * người chơi thấy khác thứ pack khai — nên có một bài test so hai bên.
 */
const FALLBACK: readonly BlockDef[] = [
  { id: "air", name: { en: "Air", vi: "Không khí" }, color: 0x0d1014, liquid: false, walkable: true, tags: ["gas"] },
  { id: "water", name: { en: "Water", vi: "Nước" }, color: 0x2c5c8a, liquid: true, walkable: true, tags: ["liquid"] },
  { id: "ice", name: { en: "Ice", vi: "Băng" }, color: 0xcfe6f0, liquid: false, walkable: false, tags: ["solid"] },
  { id: "sand", name: { en: "Sand", vi: "Cát" }, color: 0xd8c48a, liquid: false, walkable: false, tags: ["soil", "loose"] },
  { id: "topsoil", name: { en: "Topsoil", vi: "Đất mặt" }, color: 0x6b5a3e, liquid: false, walkable: false, tags: ["soil"] },
  { id: "clay", name: { en: "Clay", vi: "Sét" }, color: 0x8c6b4f, liquid: false, walkable: false, tags: ["soil"] },
  { id: "sedimentary", name: { en: "Sedimentary rock", vi: "Đá trầm tích" }, color: 0x8a8578, liquid: false, walkable: false, tags: ["rock"] },
  { id: "metamorphic", name: { en: "Metamorphic rock", vi: "Đá biến chất" }, color: 0x6f6a72, liquid: false, walkable: false, tags: ["rock"] },
  { id: "igneous", name: { en: "Igneous rock", vi: "Đá macma" }, color: 0x55535a, liquid: false, walkable: false, tags: ["rock"] },
  { id: "ore", name: { en: "Ore", vi: "Quặng" }, color: 0xb08a3a, liquid: false, walkable: false, tags: ["rock", "valuable"] },
  { id: "magma", name: { en: "Magma", vi: "Magma" }, color: 0xd4562a, liquid: true, walkable: false, tags: ["liquid", "hot"] },
];

export class BlockPalette {
  private byId = new Map<string, BlockDef>();

  constructor(defs: readonly BlockDef[] = FALLBACK) {
    for (const d of defs) this.byId.set(d.id, d);
  }

  get(id: string): BlockDef | undefined {
    return this.byId.get(id);
  }

  color(id: string): number {
    return this.byId.get(id)?.color ?? UNDEFINED_COLOR;
  }

  isLiquid(id: string): boolean {
    return this.byId.get(id)?.liquid ?? false;
  }

  /** Tên hiển thị theo ngôn ngữ đang dùng. */
  label(id: string): string {
    return localized(this.byId.get(id)?.name, id);
  }

  /**
   * Biên độ nhiễu độ sáng của vật liệu.
   *
   * Nước gần như phẳng, cát rất hạt, đá ở giữa. Một biên độ chung cho mọi vật
   * liệu làm mặt nước lấm tấm như giấy nhám — sai về cảm giác vật chất.
   */
  grain(id: string): number {
    const tags = this.byId.get(id)?.tags ?? [];
    if (tags.includes("liquid")) return 0.02;
    if (tags.includes("loose")) return 0.09;
    if (tags.includes("soil")) return 0.06;
    if (tags.includes("rock")) return 0.07;
    return 0.0;
  }

  ids(): string[] {
    return [...this.byId.keys()].sort();
  }
}

/** Bảng dự phòng dạng mảng, cho test so sánh với content pack. */
export const FALLBACK_BLOCKS = FALLBACK;

/**
 * Dựng bảng từ dữ liệu server trả về.
 *
 * Màu tới dưới dạng chuỗi `#rrggbb` chứ không phải số: đọc được bằng mắt khi gỡ
 * lỗi, và không có chuyện `0x0d1014` bị một tầng JSON nào đó đọc thành số thập
 * phân. Đổi sang số đúng một lần, ở đây.
 */
export function paletteFrom(infos: readonly BlockInfo[]): BlockPalette {
  const defs: BlockDef[] = infos.map((b) => ({
    id: b.id,
    name: b.name,
    color: Number.parseInt(b.color.replace("#", ""), 16) || UNDEFINED_COLOR,
    liquid: b.liquid,
    walkable: b.walkable,
    tags: b.tags,
  }));
  return new BlockPalette(defs);
}
