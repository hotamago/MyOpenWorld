/**
 * Bài kiểm vệ sinh cây nguồn.
 *
 * ## Vì sao một bài test lại đi đọc thư mục
 *
 * Vì lỗi này không hiện ra ở bất kỳ bài test nào khác, và nó đã cắn hai lần.
 *
 * `moduleResolution: "bundler"` ưu tiên `.js` hơn `.ts`. Nên một file `.js` cũ
 * nằm cạnh `.ts` của chính nó **che** mất nguồn thật: Vite phục vụ bản `.js`,
 * và mọi thay đổi trong `.ts` biến mất một cách im lặng. Không lỗi, không cảnh
 * báo, không dòng nào đỏ.
 *
 * Lần gần nhất: 92 file `.js` do một lần chạy `tsc` để lại. `@/i18n` giải ra
 * bản `.js` thiếu những khóa vừa thêm, nên `t("yuu.hint")` trả về chuỗi rỗng —
 * trong khi `import()` thẳng file `.ts` trả về đúng chữ. Cả một panel hiện ra
 * trống rỗng và không có gì nói vì sao.
 *
 * `tsconfig.json` đã đặt `noEmit: true` để chặn nguồn gây ra nó. Bài này là
 * lớp thứ hai: nếu ai đó gỡ cờ kia, hoặc một công cụ khác ghi ra `.js`, thì có
 * một dòng đỏ thay vì một buổi chiều gỡ lỗi.
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

/** Mọi file dưới `dir`, đệ quy. Bỏ qua những thư mục không phải nguồn. */
function walk(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "dist" || name.startsWith(".")) continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...walk(p));
    else out.push(p);
  }
  return out;
}

describe("cây nguồn không có file dịch sẵn", () => {
  it("không có `.js` nào nằm trong `src/`", () => {
    const strays = walk("src").filter((p) => p.endsWith(".js"));
    expect(
      strays,
      "có file `.js` trong `src/` — chúng **che** mất `.ts` cùng tên và mọi sửa " +
        "đổi sẽ biến mất im lặng. Xoá chúng, rồi tìm xem công cụ nào đã ghi ra.",
    ).toEqual([]);
  });

  it("`tsconfig.json` vẫn cấm `tsc` ghi ra file", () => {
    // Bỏ dòng bình luận trước khi phân tích: `tsconfig.json` cho phép `//`, còn
    // `JSON.parse` thì không.
    const raw = readFileSync("tsconfig.json", "utf8").replace(/^\s*\/\/.*$/gm, "");
    const cfg = JSON.parse(raw) as { compilerOptions?: { noEmit?: boolean } };
    expect(cfg.compilerOptions?.noEmit, "`noEmit` đã bị gỡ khỏi tsconfig").toBe(true);
  });
});
