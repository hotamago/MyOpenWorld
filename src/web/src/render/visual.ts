/**
 * Ngôn ngữ thị giác (`idea.md §18.5`, `PB-17`).
 *
 * ## Kênh thị giác được **phân bổ cố định**
 *
 * Mắt người đọc được vài kênh song song: màu, độ sáng, hoa văn, hình dáng, kích
 * thước, chuyển động. Đó là một **ngân sách**, và nó nhỏ hơn người ta tưởng.
 *
 * Nếu mỗi hệ thống tự chọn kênh cho mình, thì tới hệ thống thứ tư mọi kênh đã
 * bị chiếm và thứ năm phải dùng lại — lúc đó một ô đỏ có thể nghĩa là nóng,
 * hoặc nguy hiểm, hoặc thuộc phe đỏ, và người chơi không có cách nào biết.
 *
 * Nên phân bổ nằm ở đây, **một chỗ**, và mọi hệ thống phải xin từ bảng này.
 *
 * ## Overlay là nhóm loại trừ
 *
 * Đã thi hành ở [`OverlayGroup`]. Nhắc lại ở đây vì đó là hệ quả trực tiếp của
 * ngân sách kênh: hai overlay cùng bật là hai hệ thống cùng chiếm kênh màu.
 *
 * [`OverlayGroup`]: ./overlays/datatexture.ts
 */

/** Các kênh thị giác mà mắt đọc được song song. */
export const CHANNELS = [
  "hue",
  "lightness",
  "pattern",
  "shape",
  "size",
  "motion",
  "outline",
] as const;

/** Tên một kênh. */
export type Channel = (typeof CHANNELS)[number];

/**
 * Phân bổ kênh cố định.
 *
 * Bảng này là **hợp đồng**. Đổi một dòng ở đây là đổi cách người chơi đọc bản
 * đồ, nên nó cần cân nhắc như một thay đổi luật chơi, không phải như một chỉnh
 * sửa thẩm mỹ.
 */
export const CHANNEL_ASSIGNMENT: Record<Channel, string> = {
  // Nền môi trường: biome, vật liệu. Chiếm sắc màu vì nó là lớp dưới cùng và
  // phủ toàn bản đồ.
  hue: "terrain",
  // Độ cao và overlay đang bật. Độ sáng sống sót qua mù màu và qua in đen
  // trắng, nên nó dành cho thứ quan trọng nhất phải đọc được.
  lightness: "elevation_and_active_overlay",
  // Hoa văn: tín hiệu phụ cho biome, và tín hiệu **chính** cho người mù màu.
  pattern: "biome_texture",
  // Hình dáng: loại thực thể. Đây là kênh mạnh nhất và nó dành cho câu hỏi
  // được hỏi nhiều nhất — "đó là cái gì".
  shape: "entity_kind",
  // Kích thước: quy mô, số lượng trong một chồng.
  size: "quantity",
  // Chuyển động: **chỉ dành cho thứ đang thay đổi ngay lúc này**. Dùng nó cho
  // trạng thái tĩnh sẽ làm bản đồ nhấp nháy không ngừng và mắt không nghỉ được.
  motion: "active_change",
  // Viền: định danh — phe, sở hữu. Kênh nhỏ nhưng đủ để tách khỏi nền.
  outline: "identity",
};

/** Lỗi khi hai hệ thống tranh cùng một kênh. */
export class ChannelConflict extends Error {
  constructor(
    public readonly channel: Channel,
    public readonly owner: string,
    public readonly requester: string,
  ) {
    super(
      `hệ thống \`${requester}\` xin kênh \`${channel}\` nhưng nó đã thuộc về ` +
        `\`${owner}\`. Ngân sách kênh thị giác là hữu hạn (§18.5); dùng lại một ` +
        `kênh làm cùng một tín hiệu mang hai nghĩa, và người chơi không có cách ` +
        `nào biết nghĩa nào đang áp dụng.`,
    );
    this.name = "ChannelConflict";
  }
}

/** Xin một kênh. Ném [`ChannelConflict`] nếu nó đã có chủ. */
export function claimChannel(channel: Channel, system: string): void {
  const chu = CHANNEL_ASSIGNMENT[channel];
  if (chu !== system) {
    throw new ChannelConflict(channel, chu, system);
  }
}

/** Kênh nào thuộc về một hệ thống. */
export function channelsOf(system: string): Channel[] {
  return CHANNELS.filter((c) => CHANNEL_ASSIGNMENT[c] === system);
}

/**
 * Vật thể nhiều tầng: dấu hiệu che khuất (`§18.5`).
 *
 * Khi một thứ ở tầng dưới bị thứ ở tầng trên che, nó **phải có dấu hiệu**. Vẽ
 * nó mờ đi thôi thì không đủ: người chơi sẽ tưởng nó ở xa, hoặc tưởng nó là một
 * hiệu ứng, chứ không hiểu là "có mái nhà ở trên".
 */
export interface Occlusion {
  /** Thứ bị che.  */
  hidden: string;
  /** Bị che bởi cái gì. */
  by: string;
  /** Chênh tầng. */
  layersAbove: number;
}

/** Chế độ cắt lớp để nhìn vào trong nhà. */
export type CutawayMode =
  /** Không cắt. */
  | "off"
  /** Cắt mọi thứ trên tầng camera. */
  | "above_camera"
  /** Chỉ cắt phần che thực thể đang theo dõi. */
  | "follow_target";

/**
 * Những gì cần vẽ ở chế độ cắt lớp.
 *
 * `follow_target` chỉ cắt phần che **thực thể đang theo dõi**, không cắt cả
 * mái nhà. Cắt cả mái làm mất bối cảnh: người xem không còn biết nhân vật đang
 * ở trong nhà hay ngoài trời.
 */
export function cutawayHides(
  mode: CutawayMode,
  occlusions: Occlusion[],
  cameraLayer: number,
  targetLayer: number | null,
): string[] {
  switch (mode) {
    case "off":
      return [];
    case "above_camera":
      return occlusions
        .filter((o) => o.layersAbove > 0 && cameraLayer < cameraLayer + o.layersAbove)
        .map((o) => o.by);
    case "follow_target":
      if (targetLayer === null) return [];
      return occlusions
        .filter((o) => o.hidden === String(targetLayer) || o.layersAbove > 0)
        .map((o) => o.by);
  }
}

/**
 * Dấu hiệu che khuất cho một thực thể bị che.
 *
 * Trả về `null` nếu không bị che. Trả về một **nhãn**, không phải một mức mờ:
 * độ mờ là cách vẽ, nhãn là cái ý nghĩa, và giao diện cần cái thứ hai.
 */
export function occlusionMarker(o: Occlusion | null): string | null {
  if (!o) return null;
  return o.layersAbove === 1 ? "bị che bởi tầng trên" : `bị che bởi ${o.layersAbove} tầng`;
}

/** Ưu tiên vẽ theo tầng `z`, rồi theo loại. */
export const DRAW_ORDER: readonly string[] = [
  "terrain",
  "terrain_overlay",
  "floor_item",
  "structure",
  "entity",
  "effect",
  "identity_marker",
  "label",
] as const;

/** Chỉ số vẽ của một loại; `-1` nếu không biết. */
export function drawIndex(kind: string): number {
  return DRAW_ORDER.indexOf(kind);
}
