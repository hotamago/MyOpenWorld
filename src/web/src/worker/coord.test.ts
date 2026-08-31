import { describe, expect, it } from "vitest";
import {
  chunkKey,
  chunkOf,
  CoordRangeError,
  FloatingOrigin,
  formatCoord,
  localInChunk,
  parseCoord,
  parsePoint,
  REBASE_RADIUS,
  SAFE_INT,
} from "./coord";

describe("§22.10 — tọa độ không bị ép qua Number", () => {
  it("từ chối tọa độ tới dưới dạng number", () => {
    // Đây là bài quan trọng nhất của file. Nếu hàm này chấp nhận `number`, mất
    // chính xác đã xảy ra ở `JSON.parse` — trước khi ta có cơ hội can thiệp.
    expect(() => parseCoord(123)).toThrow(/number/);
    expect(() => parseCoord(123)).toThrow(/chuỗi/);
  });

  it("đọc được số vượt 2^53 từ chuỗi mà không mất bit nào", () => {
    // 2^53 + 1 = 9007199254740993. Phải là số **lẻ**: quá 2^53, `f64` chỉ còn
    // biểu diễn được số chẵn, nên một số chẵn sẽ đi qua `Number` nguyên vẹn và
    // bài test sẽ chứng minh nhầm rằng không có vấn đề gì.
    const vuot = SAFE_INT + 2n;
    expect(vuot % 2n).toBe(1n);

    const doc = parseCoord(vuot.toString());
    expect(doc).toBe(vuot);

    // Chứng minh vì sao phải làm thế: qua `Number` thì mất đúng một đơn vị.
    expect(BigInt(Number(vuot))).toBe(vuot - 1n);
  });

  it("đi và về nguyên vẹn ở biên i64", () => {
    const bien = [(1n << 62n) - 1n, -(1n << 62n), 0n, -1n];
    for (const v of bien) {
      expect(parseCoord(formatCoord(v))).toBe(v);
    }
  });

  it("đọc điểm từ payload", () => {
    const p = parsePoint({ x: "9007199254740993", y: "-42", z: 3 });
    expect(p.x).toBe(9007199254740993n);
    expect(p.y).toBe(-42n);
    expect(p.z).toBe(3);
  });

  it("báo lỗi rõ khi payload sai hình dạng", () => {
    expect(() => parsePoint(null)).toThrow(/không phải object/);
    expect(() => parsePoint({ x: 1, y: "2" })).toThrow(/point\.x/);
  });
});

describe("chunkOf — cái bẫy quanh gốc tọa độ", () => {
  it("ô âm rơi vào đúng chunk", () => {
    // `/` của bigint cắt về 0, nên nếu không xử lý dấu thì ô -1 và ô 0 sẽ rơi
    // vào cùng chunk. Lưới lệch một ô quanh gốc, và không bài test nào chạy
    // quanh (0,0) phát hiện được.
    expect(chunkOf(0n, 32)).toBe(0n);
    expect(chunkOf(31n, 32)).toBe(0n);
    expect(chunkOf(32n, 32)).toBe(1n);
    expect(chunkOf(-1n, 32)).toBe(-1n);
    expect(chunkOf(-32n, 32)).toBe(-1n);
    expect(chunkOf(-33n, 32)).toBe(-2n);
  });

  it("vị trí trong chunk luôn không âm", () => {
    expect(localInChunk(-1n, 32)).toBe(31);
    expect(localInChunk(-32n, 32)).toBe(0);
    expect(localInChunk(0n, 32)).toBe(0);
    expect(localInChunk(33n, 32)).toBe(1);
  });

  it("ô luôn thuộc chunk của chính nó, kể cả ở xa", () => {
    const size = 32;
    for (const x of [0n, -1n, 12345n, -98765n, 1n << 55n, -(1n << 55n)]) {
      const c = chunkOf(x, size);
      const l = localInChunk(x, size);
      expect(c * BigInt(size) + BigInt(l)).toBe(x);
      expect(l).toBeGreaterThanOrEqual(0);
      expect(l).toBeLessThan(size);
    }
  });

  it("khóa chunk phân biệt được", () => {
    expect(chunkKey(1n, 2n, 0)).not.toBe(chunkKey(2n, 1n, 0));
    expect(chunkKey(1n, 2n, 0)).not.toBe(chunkKey(1n, 2n, 1));
  });
});

describe("FloatingOrigin — §18.4", () => {
  it("tọa độ cục bộ là hiệu với gốc", () => {
    const o = new FloatingOrigin({ x: 1000n, y: 2000n, z: 0 });
    expect(o.toLocal({ x: 1010n, y: 1990n, z: 0 })).toEqual({ x: 10, y: -10, z: 0 });
  });

  it("đi và về nguyên vẹn", () => {
    const o = new FloatingOrigin({ x: 1n << 55n, y: -(1n << 55n), z: 2 });
    const p = { x: (1n << 55n) + 137n, y: -(1n << 55n) - 42n, z: 5 };
    expect(o.toWorld(o.toLocal(p))).toEqual(p);
  });

  it("dời gốc khi camera đi quá xa", () => {
    const o = new FloatingOrigin();
    expect(o.generation).toBe(0);

    // Trong bán kính thì không dời.
    expect(o.recenterIfNeeded({ x: 100n, y: 100n, z: 0 })).toBe(false);
    expect(o.generation).toBe(0);

    // Vượt bán kính thì dời.
    const xa = BigInt(REBASE_RADIUS) + 1n;
    expect(o.recenterIfNeeded({ x: xa, y: 0n, z: 0 })).toBe(true);
    expect(o.generation).toBe(1);
    expect(o.origin.x).toBe(xa);
  });

  it("đổi tầng cũng phải dời gốc", () => {
    const o = new FloatingOrigin();
    expect(o.recenterIfNeeded({ x: 0n, y: 0n, z: 3 })).toBe(true);
  });

  it("sau khi dời, tọa độ cục bộ lại nhỏ", () => {
    // Đây là toàn bộ mục đích của floating origin: giữ cho tọa độ vẽ nằm trong
    // khoảng mà `f32` của WebGL còn chính xác.
    const o = new FloatingOrigin();
    const camera = { x: 1n << 55n, y: 1n << 55n, z: 0 };
    o.recenterIfNeeded(camera);
    const gan_camera = { x: camera.x + 50n, y: camera.y - 30n, z: 0 };
    const l = o.toLocal(gan_camera);
    expect(Math.abs(l.x)).toBeLessThan(1000);
    expect(Math.abs(l.y)).toBeLessThan(1000);
  });

  it("điểm quá xa gốc là lỗi, không phải im lặng sai", () => {
    const o = new FloatingOrigin();
    expect(() => o.toLocal({ x: 1n << 55n, y: 0n, z: 0 })).toThrow(CoordRangeError);
  });

  it("generation tăng mỗi lần dời để renderer biết phải vẽ lại", () => {
    const o = new FloatingOrigin();
    o.recenter({ x: 1n, y: 1n, z: 0 });
    o.recenter({ x: 2n, y: 2n, z: 0 });
    expect(o.generation).toBe(2);
  });
});
