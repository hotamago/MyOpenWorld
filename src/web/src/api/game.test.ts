/**
 * Bài kiểm cho `decodeTiles` — giải mã lô ô từ dây về `TileBatch`.
 *
 * `/api/tiles` giờ gửi chỉ mục trỏ vào bảng `names` thay vì ba mảng chuỗi lặp
 * lại (xem tài liệu `tiles` ở `mow-server/src/api.rs` và tài liệu
 * `decodeTiles` ở `game.ts`). Bài này không kiểm băng thông — cái đó đã có
 * bài đo phía Rust — mà kiểm đúng ba điều `decodeTiles` phải giữ: giải mã
 * đúng, chỉ mục ngoài phạm vi không lọt `undefined` xuống renderer, và định
 * dạng cũ (server chưa khởi động lại) vẫn chạy được.
 */
import { describe, expect, it } from "vitest";
import { decodeTiles } from "./game";

describe("decodeTiles — định dạng mới (chỉ mục + bảng tra)", () => {
  it("giải mã chỉ mục thành đúng chuỗi qua `names`", () => {
    const raw = {
      x: 0,
      y: 0,
      w: 2,
      h: 2,
      z: 10,
      names: ["air", "topsoil", "grassland"],
      material: [0, 1, 1, 0],
      surface: [1, 1, 1, 1],
      biome: [2, 2, 2, 2],
      drop: [0, 0, 0, 0],
      built: [0, 0, 1, 0],
      height: [10, 10, 10, 10],
      river: [0, 0, 0, 0],
    };

    const batch = decodeTiles(raw);

    expect(batch.material).toEqual(["air", "topsoil", "topsoil", "air"]);
    expect(batch.surface).toEqual(["topsoil", "topsoil", "topsoil", "topsoil"]);
    expect(batch.biome).toEqual(["grassland", "grassland", "grassland", "grassland"]);
    // Các mảng số phải đi qua nguyên vẹn, không bị đổi ý nghĩa.
    expect(batch.built).toEqual([0, 0, 1, 0]);
    expect(batch.height).toEqual([10, 10, 10, 10]);
    expect(batch.x).toBe(0);
    expect(batch.w).toBe(2);
  });

  it("chỉ mục ngoài phạm vi bảng cho ra `\"?\"`, không phải `undefined`", () => {
    // Một `undefined` lọt vào tầng vẽ sẽ thành một màu tím không giải thích
    // được — và người ta sẽ đổ lỗi cho renderer thay vì cho bảng chỉ mục sai.
    const raw = {
      x: 0,
      y: 0,
      w: 1,
      h: 1,
      z: 0,
      names: ["air"],
      material: [99],
      surface: [-1],
      biome: [0],
      drop: [0],
      built: [0],
      height: [0],
      river: [0],
    };

    const batch = decodeTiles(raw);

    expect(batch.material).toEqual(["?"]);
    expect(batch.surface).toEqual(["?"]);
    expect(batch.material[0]).not.toBeUndefined();
  });

  it("bảng `names` rỗng vẫn không sinh ra `undefined`", () => {
    const raw = {
      x: 0,
      y: 0,
      w: 1,
      h: 1,
      z: 0,
      names: [],
      material: [0],
      surface: [0],
      biome: [0],
      drop: [0],
      built: [0],
      height: [0],
      river: [0],
    };

    expect(decodeTiles(raw).material).toEqual(["?"]);
  });
});

describe("decodeTiles — định dạng cũ (server chưa khởi động lại cùng client)", () => {
  it("vẫn chạy khi `material`/`surface`/`biome` đã là chuỗi thẳng", () => {
    // Trong lúc phát triển, client và server không phải lúc nào cũng khởi
    // động lại cùng nhau. Một client mới nói chuyện với một server cũ không
    // có `names` phải vẫn vẽ được bản đồ, không phải một màn hình trắng
    // không nói cho ai biết vì sao.
    const raw = {
      x: 0,
      y: 0,
      w: 2,
      h: 1,
      z: 5,
      material: ["air", "topsoil"],
      surface: ["topsoil", "topsoil"],
      biome: ["grassland", "grassland"],
      drop: [0, 0],
      built: [0, 1],
      height: [5, 5],
      river: [0, 0],
    };

    const batch = decodeTiles(raw);

    expect(batch.material).toEqual(["air", "topsoil"]);
    expect(batch.surface).toEqual(["topsoil", "topsoil"]);
    expect(batch.biome).toEqual(["grassland", "grassland"]);
    expect(batch.built).toEqual([0, 1]);
  });
});
