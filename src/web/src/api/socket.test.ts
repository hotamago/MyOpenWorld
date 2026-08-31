/**
 * Bài kiểm cho phần thuần của kênh WebSocket.
 *
 * Không giả lập cả một `WebSocket`: một bản giả đủ trung thực để đáng tin thì
 * đã là một bài kiểm cho chính bản giả đó. Thứ đáng kiểm ở đây là phép đổi gốc
 * — chỗ duy nhất có thể sai lặng lẽ, vì `new WebSocket` với một gốc sai chỉ
 * hỏng lúc chạy, trên máy người khác.
 */
import { describe, expect, it } from "vitest";
import { toWs } from "./socket";

describe("toWs", () => {
  it("đổi http thành ws và https thành wss", () => {
    expect(toWs("http://localhost:17777")).toBe("ws://localhost:17777/ws");
    expect(toWs("https://mow.example")).toBe("wss://mow.example/ws");
  });

  it("gốc rỗng nghĩa là cùng gốc với trang", () => {
    // `new WebSocket("/ws")` **không** hợp lệ, nên đường dẫn tương đối không
    // phải một lựa chọn; gốc phải viết tường minh.
    const got = toWs("");
    expect(got.startsWith("ws://") || got.startsWith("wss://")).toBe(true);
    expect(got.endsWith("/ws")).toBe(true);
  });
});
