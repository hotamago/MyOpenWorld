import { describe, expect, it } from "vitest";
import { GLIDE_MS, MotionTrack, ease } from "./motion";

describe("ease", () => {
  it("chạm đúng hai đầu mút", () => {
    expect(ease(0)).toBe(0);
    expect(ease(1)).toBe(1);
  });

  it("tăng dần và giảm tốc (ease-out): nửa đường đã đi hơn một nửa quãng", () => {
    const mid = ease(0.5);
    expect(mid).toBeGreaterThan(0.5);
    expect(mid).toBeLessThan(1);
    expect(ease(0.25)).toBeLessThan(ease(0.75));
  });

  it("kẹp giá trị ngoài [0, 1] thay vì ngoại suy vọt khỏi đích", () => {
    expect(ease(-5)).toBe(0);
    expect(ease(5)).toBe(1);
  });
});

describe("MotionTrack.at trên id chưa từng thấy", () => {
  it("trả về null", () => {
    const m = new MotionTrack();
    expect(m.at("ghost", 1_000)).toBeNull();
  });
});

describe("MotionTrack — lần đầu thấy một id", () => {
  it("xuất hiện thẳng tại vị trí quyền uy, không glide từ hư không", () => {
    const m = new MotionTrack();
    m.update("a", 5, 7, 1_000);
    expect(m.at("a", 1_000)).toEqual({ x: 5, y: 7 });
    // Kể cả hỏi lại muộn hơn nhiều: không có gì để trượt tới nên vị trí đứng yên.
    expect(m.at("a", 5_000)).toEqual({ x: 5, y: 7 });
  });
});

describe("MotionTrack — bước đi một ô", () => {
  it("trượt dần từ ô cũ sang ô mới trong đúng GLIDE_MS", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("a", 1, 0, 0); // bước sang phải một ô, ngay tại t=0

    expect(m.at("a", 0)).toEqual({ x: 0, y: 0 });

    const mid = m.at("a", GLIDE_MS / 2);
    expect(mid).not.toBeNull();
    expect(mid!.x).toBeGreaterThan(0);
    expect(mid!.x).toBeLessThan(1);
    expect(mid!.y).toBe(0);

    expect(m.at("a", GLIDE_MS)).toEqual({ x: 1, y: 0 });
    // Quá thời gian trượt: kẹp tại đích, không vọt tiếp.
    expect(m.at("a", GLIDE_MS + 10_000)).toEqual({ x: 1, y: 0 });
  });

  it("càng gần đích thì bước tiến mỗi mili giây càng nhỏ (giảm tốc thật)", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("a", 1, 0, 0);
    const early = m.at("a", GLIDE_MS * 0.25)!.x;
    const mid = m.at("a", GLIDE_MS * 0.5)!.x;
    const late = m.at("a", GLIDE_MS * 0.75)!.x;
    expect(mid - early).toBeGreaterThan(late - mid);
  });
});

describe("MotionTrack — cập nhật lặp lại cùng một vị trí", () => {
  it("không làm giật lại một glide đang chạy dở", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("a", 1, 0, 0); // bắt đầu glide (0,0) -> (1,0) tại t=0

    const before = m.at("a", 50);

    // Server xác nhận lại đúng cùng vị trí (1, 0) — refresh 400ms không có gì
    // mới. Nếu điều này reset `startMs`, vị trí tại t=50 sẽ tụt về gần 0 lần
    // nữa thay vì tiếp tục đúng đường cũ.
    m.update("a", 1, 0, 50);

    const after = m.at("a", 50);
    expect(after).toEqual(before);
  });
});

describe("MotionTrack — đổi hướng giữa chừng", () => {
  it("glide mới xuất phát từ vị trí đang vẽ, không giật lùi về đích cũ", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("a", 1, 0, 0); // glide (0,0) -> (1,0) bắt đầu tại t=0

    const midway = m.at("a", 100)!;
    expect(midway.x).toBeGreaterThan(0);
    expect(midway.x).toBeLessThan(1);

    // Trước khi glide cũ trượt xong, server báo một đích mới.
    m.update("a", 2, 0, 100);

    // Ngay tại thời điểm đổi hướng, vị trí vẽ phải liền mạch với `midway` —
    // không nhảy về (1, 0) rồi mới bắt đầu trượt tiếp.
    expect(m.at("a", 100)).toEqual(midway);

    expect(m.at("a", 100 + GLIDE_MS)).toEqual({ x: 2, y: 0 });
  });
});

describe("MotionTrack — nhảy xa", () => {
  it("đúng ngưỡng (3 ô) vẫn trượt, không snap", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("a", 3, 0, 0);
    const mid = m.at("a", GLIDE_MS / 2)!;
    expect(mid.x).toBeGreaterThan(0);
    expect(mid.x).toBeLessThan(3);
  });

  it("quá ngưỡng thì snap ngay lập tức, không trượt qua quãng đó", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("a", 40, 0, 0); // đổi khung nhìn / dịch chuyển

    // Snap: đã ở đích ngay tại t=0, không phải đợi GLIDE_MS.
    expect(m.at("a", 0)).toEqual({ x: 40, y: 0 });
    expect(m.at("a", 1)).toEqual({ x: 40, y: 0 });
  });

  it("nhảy xa theo một trục duy nhất cũng snap (khoảng cách Chebyshev)", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("a", 0, 10, 0);
    expect(m.at("a", 0)).toEqual({ x: 0, y: 10 });
  });
});

describe("MotionTrack — thời gian đứng yên hoặc lùi lại", () => {
  it("nowMs bằng lúc bắt đầu cho vị trí xuất phát, không NaN", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 1_000);
    m.update("a", 1, 0, 1_000);
    const pos = m.at("a", 1_000)!;
    expect(pos.x).toBe(0);
    expect(Number.isNaN(pos.x)).toBe(false);
  });

  it("nowMs lùi lại so với lúc bắt đầu vẫn kẹp về vị trí xuất phát", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 1_000);
    m.update("a", 1, 0, 1_000);
    const pos = m.at("a", 500)!;
    expect(pos).toEqual({ x: 0, y: 0 });
    expect(Number.isNaN(pos.x)).toBe(false);
    expect(Number.isNaN(pos.y)).toBe(false);
  });
});

describe("MotionTrack.retain", () => {
  it("quên id không còn trong danh sách, giữ nguyên id còn lại", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.update("b", 1, 1, 0);
    expect(m.size()).toBe(2);

    m.retain(["b"]);

    expect(m.size()).toBe(1);
    expect(m.at("a", 0)).toBeNull();
    expect(m.at("b", 0)).toEqual({ x: 1, y: 1 });
  });

  it("danh sách rỗng thì quên hết", () => {
    const m = new MotionTrack();
    m.update("a", 0, 0, 0);
    m.retain([]);
    expect(m.size()).toBe(0);
  });
});
