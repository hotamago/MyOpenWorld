/**
 * Đường riêng của bản desktop, chạy qua `tauri-driver` (`plan.md §P7.8`,
 * `§P3.4`, `PF-12`).
 *
 * `§P7.8` chốt ranh giới:
 *
 * > UI: `vitest` + Playwright — panel, overlay, BigInt coord, WS reconnect.
 * > **Chạy trên bản web trong trình duyệt**; đường riêng của Tauri (đường dẫn
 * > file, quyền, sidecar) test bằng `tauri-driver`.
 *
 * Nên file này **không** kiểm lại panel hay overlay — chúng đã có ở `vitest`,
 * và chạy lại qua một WebView thật chỉ làm CI chậm mà không bắt thêm lỗi nào.
 * Nó kiểm đúng ba thứ chỉ tồn tại ở bản desktop, và cả ba là những thứ
 * `deploy/tauri/README.md` liệt là "chỉ lộ ra khi đóng gói thật":
 *
 * | Kiểm | Hỏng thế nào nếu không có |
 * |---|---|
 * | đường dẫn tài nguyên | chạy ở dev, hỏng ở bản phát hành |
 * | quyền ghi save | Windows từ chối, sau khi đóng gói |
 * | CSP của WebView | WebSocket bị chặn **im lặng** |
 *
 * ## Vì sao cần `tauri-driver` chứ không phải Playwright thường
 *
 * Ba thứ trên không tồn tại trong một trình duyệt. Không có thư mục tài nguyên
 * cạnh binary, không có thư mục dữ liệu ứng dụng, và CSP của WebView khác CSP
 * của Chrome. Chạy chúng bằng Playwright thường sẽ **xanh** và không chứng minh
 * gì.
 *
 * ## Chạy
 *
 * ```bash
 * cargo install tauri-driver --locked
 * pnpm --filter web build
 * cd deploy/tauri/src-tauri && cargo build
 * pnpm --filter web test:e2e:desktop
 * ```
 *
 * Bỏ qua khi thiếu `tauri-driver` thay vì báo đỏ: một bộ test đỏ vì thiếu công
 * cụ là một bộ test người ta học cách bỏ qua, và từ đó nó không bắt được gì
 * nữa.
 */

import { expect, test } from "@playwright/test";

/** Có `tauri-driver` trong môi trường không. */
const CO_DRIVER = process.env.TAURI_DRIVER === "1";

test.describe("đường riêng của bản desktop (PF-12)", () => {
  test.skip(
    !CO_DRIVER,
    "cần `tauri-driver`; đặt TAURI_DRIVER=1 sau khi cài để chạy",
  );

  test("tài nguyên đọc từ cạnh binary, không từ thư mục làm việc", async ({
    page,
  }) => {
    // Ứng dụng in đường dẫn đã phân giải ra stderr lúc khởi động
    // (`main.rs::tracing_lite`). Ở đây kiểm phía WebView: frontend nạp được
    // asset của nó qua giao thức tùy chỉnh của Tauri, không qua `file://`.
    await page.goto("/");
    const src = await page.evaluate(() => document.location.protocol);
    expect(src).not.toBe("file:");
  });

  test("WebSocket tới loopback KHÔNG bị CSP chặn", async ({ page }) => {
    // Đây là lớp lỗi tệ nhất trong ba lớp: WebView chặn `connect-src` mặc
    // định, và nó chặn **im lặng**. Không có bài này thì lỗi hiện ra dưới dạng
    // "bản desktop không cập nhật thế giới" mà không có gì trong log.
    await page.goto("/");
    const ket = await page.evaluate(async () => {
      try {
        const ws = new WebSocket("ws://127.0.0.1:9/echo");
        return await new Promise<string>((resolve) => {
          ws.addEventListener("error", () => resolve("error"));
          ws.addEventListener("open", () => resolve("open"));
          setTimeout(() => resolve("timeout"), 2000);
        });
      } catch (e) {
        // `SecurityError` nghĩa là CSP chặn — khác hẳn với "không kết nối
        // được", và đó là phân biệt mà bài này tồn tại để làm.
        return e instanceof Error ? e.name : "unknown";
      }
    });
    expect(ket).not.toBe("SecurityError");
  });

  test("save ghi được vào thư mục dữ liệu ứng dụng", async ({ page }) => {
    // Bản Rust đã `assert!` điều này lúc `setup()` (`main.rs`), nên nếu đường
    // save nằm trong thư mục cài đặt thì ứng dụng không khởi động nổi và bài
    // này không tới được `goto`. Đó là chủ đích: kiểm ở chỗ rẻ nhất.
    await page.goto("/");
    await expect(page.locator("body")).toBeVisible();
  });
});
