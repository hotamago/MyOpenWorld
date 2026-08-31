/**
 * Cấu hình ESLint (`plan.md §P10.1`).
 *
 * `Makefile` liệt eslint trong `make lint` từ lâu, nhưng chưa bao giờ có cấu
 * hình — nên bước đó im lặng không làm gì. File này đóng khoảng trống đó.
 *
 * ## Chỉ những luật bắt được lỗi thật
 *
 * Không bật bộ `stylistic`: định dạng là việc của Prettier/`vite`, và một
 * linter cãi nhau với formatter là một linter người ta tắt đi.
 *
 * Hai luật đáng nói:
 *
 * - `no-restricted-globals` cấm `parseInt`/`Number` trên tọa độ. `§22.10` cấm
 *   ép tọa độ 64-bit qua `Number`, và chỗ vi phạm điều đó **không ném lỗi** —
 *   nó chỉ trả về một con số hơi sai. Một linter là chỗ rẻ nhất để bắt.
 * - `@typescript-eslint/no-explicit-any` bật ở mức lỗi. `web/` không được chứa
 *   luật authoritative (`§19.2`), nên mọi thứ nó đụng tới phải là kiểu sinh từ
 *   schema — và một `any` là chỗ kiểu đó rơi mất.
 */

import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "dist/**",
      "node_modules/**",
      "playwright-report/**",
      "test-results/**",
      "src/**/generated/**",
      // Đầu ra của `vue-tsc -b`: nó phát `.js` và `.d.ts` cạnh nguồn. Lint bản
      // biên dịch sẽ báo lỗi ở mã không ai viết, và sửa những lỗi đó là sửa
      // nhầm chỗ.
      "**/*.js",
      "**/*.d.ts",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.ts"],
    rules: {
      // TypeScript đã kiểm biến không tồn tại, và nó kiểm đúng hơn — `no-undef`
      // không biết `lib.dom.d.ts`, nên nó báo `WebSocket` và `TextEncoder` là
      // không định nghĩa. Đây là khuyến nghị chính thức của `typescript-eslint`.
      "no-undef": "off",
      "@typescript-eslint/no-explicit-any": "error",
      // Biến không dùng có tiền tố `_` là cố ý — thường là tham số của một
      // chữ ký cố định.
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      eqeqeq: ["error", "always"],
      "no-console": ["error", { allow: ["warn", "error"] }],
    },
  },
  {
    // Test được phép dùng `console.log` để chẩn đoán, và được phép `any` khi
    // đang cố tình dựng dữ liệu sai để kiểm hàm từ chối nó.
    files: ["**/*.test.ts", "e2e/**/*.ts"],
    rules: {
      "no-console": "off",
      "@typescript-eslint/no-explicit-any": "off",
    },
  },
);
