// `vitest/config` chu khong phai `vite`: chi ban nay biet khoa `test`.
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  worker: {
    // Worker phai la ES module: `net.worker.ts` import tu `coord.ts`, va ban
    // classic se phai goi `importScripts` — thu khong dung duoc voi TypeScript
    // da bien dich theo module.
    format: "es",
  },
  build: {
    target: "es2022", // BigInt literal va `at()` deu can muc nay.
    sourcemap: true,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
