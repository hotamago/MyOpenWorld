/**
 * Cấu hình Playwright (`plan.md §P7.8`, `PF-14`, `PF-12`).
 *
 * Hai project, và ranh giới giữa chúng là ranh giới mà `§P7.8` vạch ra:
 *
 * | Project | Chạy ở đâu | Kiểm gì |
 * |---|---|---|
 * | `web` | Chromium thật | panel, overlay, BigInt coord, WS reconnect |
 * | `desktop` | `tauri-driver` | đường dẫn file, quyền, sidecar, CSP của WebView |
 *
 * Tách vì ba thứ ở dòng dưới **không tồn tại trong một trình duyệt**. Chạy
 * chúng bằng Chromium sẽ xanh và không chứng minh gì.
 *
 * ## `webServer` dùng bản đã build, không dùng `vite dev`
 *
 * Bản dev có HMR, source map, và không qua bước `vue-tsc -b`. Kiểm trên nó
 * nghĩa là kiểm một thứ khác với thứ người dùng nhận. `preview` phục vụ đúng
 * `dist/` mà `pnpm build` vừa tạo.
 */

import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  // Một bài e2e treo là một CI job treo. Trần này cắt sớm.
  timeout: 30_000,
  expect: { timeout: 5_000 },

  // Không cho `test.only` lọt vào CI: nó làm cả bộ còn lại **không chạy** mà
  // vẫn báo xanh — cách hỏng im lặng nhất mà một bộ e2e có.
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",

  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "on-first-retry",
  },

  projects: [
    {
      name: "web",
      testIgnore: /desktop\.spec\.ts/,
      use: { ...devices["Desktop Chrome"] },
    },
    {
      // Đường riêng của Tauri. Bỏ qua khi không có `tauri-driver` — xem ghi chú
      // đầu `e2e/desktop.spec.ts`.
      name: "desktop",
      testMatch: /desktop\.spec\.ts/,
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  webServer: {
    // `--host 127.0.0.1` là bắt buộc, không phải trang trí: mặc định `vite
    // preview` bind `localhost`, và trên Windows `localhost` giải ra `::1`
    // trước — nên một `url` viết `127.0.0.1` sẽ chờ tới hết giờ trong khi
    // server đã sẵn sàng từ lâu.
    command: "pnpm exec vite preview --port 4173 --strictPort --host 127.0.0.1",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
