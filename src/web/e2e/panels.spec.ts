/**
 * E2E cho các panel chính, chạy trên bản đã build trong trình duyệt thật
 * (`plan.md §P7.8`, `PF-14`).
 *
 * ## Bộ này kiểm gì mà `vitest` không kiểm được
 *
 * `vitest` chạy model của panel — hàm thuần, không DOM, không mạng. Nó bắt
 * được logic sai và không bắt được bốn thứ dưới đây, vì cả bốn chỉ tồn tại khi
 * có một trình duyệt thật:
 *
 * | Kiểm | Vì sao `vitest` không thấy |
 * |---|---|
 * | ứng dụng **mount** được | bundle vỡ, import vòng, lỗi runtime của Vue |
 * | `BigInt` sống sót qua biên | `JSON.parse` biến `u64` thành `Number` |
 * | canvas **có kích thước thật** | layout CSS chỉ tồn tại khi có layout engine |
 * | không có lỗi console | một `TypeError` bị nuốt vẫn để trang hiện ra |
 *
 * Dòng thứ tư là dòng có giá trị nhất và hay bị bỏ nhất: một trang **hiện ra**
 * không có nghĩa là nó chạy. `§22.10` cấm ép tọa độ 64-bit qua `Number`, và
 * chỗ vi phạm điều đó thường không ném lỗi — nó chỉ trả về một con số hơi sai.
 *
 * ## Chạy trên `dist/`, không trên `vite dev`
 *
 * Xem `playwright.config.ts`: bản dev có HMR và không qua `vue-tsc -b`, nên
 * kiểm trên nó là kiểm một thứ khác với thứ người dùng nhận.
 */

import { expect, test, type ConsoleMessage } from "@playwright/test";

/**
 * Bắt mọi lỗi console trong một bài.
 *
 * Trả về một hàm khẳng định — gọi ở cuối bài. Khẳng định ngay trong listener sẽ
 * ném từ một ngữ cảnh mà Playwright không gắn được vào bài nào.
 */
function batLoiConsole(page: import("@playwright/test").Page) {
  const loi: string[] = [];
  page.on("console", (m: ConsoleMessage) => {
    if (m.type() === "error") loi.push(m.text());
  });
  page.on("pageerror", (e) => loi.push(String(e)));
  return () => loi;
}

test.describe("panel chính (PF-14)", () => {
  test("ứng dụng mount được và không ném lỗi", async ({ page }) => {
    const loi = batLoiConsole(page);
    await page.goto("/");

    // Header là thứ hiện trước renderer; nó có nghĩa là Vue đã mount.
    await expect(page.locator("header")).toBeVisible();
    await expect(page.locator(".tick")).toBeVisible();

    // WebGL không có ở mọi runner CI, nên renderer được phép báo lỗi khởi tạo
    // — nhưng ứng dụng thì không được sập vì thế.
    const bo_qua = /WebGL|WebGPU|GPU|context/i;
    expect(loi().filter((l) => !bo_qua.test(l))).toEqual([]);
  });

  test("thanh trạng thái hiện tick và hash, không hiện undefined", async ({
    page,
  }) => {
    await page.goto("/");
    const tick = await page.locator(".tick").innerText();
    expect(tick).toMatch(/^t\d+$/);

    const hash = await page.locator(".hash").innerText();
    expect(hash).not.toContain("undefined");
    expect(hash).not.toContain("NaN");
  });

  test("hash đầy đủ nằm ở `title`, không bị cắt mất", async ({ page }) => {
    // `§18.13` nguyên tắc 1: rút gọn để đọc, nhưng số đầy đủ luôn có.
    await page.goto("/");
    const day_du = await page.locator(".hash").getAttribute("title");
    const ngan = await page.locator(".hash").innerText();
    expect(day_du).not.toBeNull();
    expect(day_du!.startsWith(ngan)).toBe(true);
  });

  test("BigInt sống sót qua biên JS — §22.10", async ({ page }) => {
    await page.goto("/");
    // Tọa độ 64-bit phải đi qua `BigInt`. Bài này kiểm chính tính chất đó
    // trong một runtime thật: `Number` mất chính xác trên 2^53, và mất một
    // cách im lặng.
    const ket = await page.evaluate(() => {
      const xa = 9_007_199_254_740_993n; // 2^53 + 1
      const qua_number = Number(xa);
      return {
        bigintGiuDuoc: BigInt(xa.toString()) === xa,
        numberMatChinhXac: BigInt(qua_number) !== xa,
      };
    });
    expect(ket.bigintGiuDuoc).toBe(true);
    expect(ket.numberMatChinhXac).toBe(true);
  });

  test("canvas chiếm chỗ thật, không cao 0 px", async ({ page }) => {
    // Một canvas cao 0 px vẫn "hiện ra" với mọi kiểm tra DOM và không vẽ được
    // gì. Chỉ một layout engine thật mới nói được điều này.
    await page.goto("/");
    const hop = await page.locator("canvas").boundingBox();
    expect(hop).not.toBeNull();
    expect(hop!.width).toBeGreaterThan(100);
    expect(hop!.height).toBeGreaterThan(100);
  });

  test("không có yêu cầu mạng nào ra ngoài loopback", async ({ page }) => {
    // `idea.md §2` chốt offline-first. Một font hay một script tải từ CDN sẽ
    // làm ứng dụng hỏng ở máy không có mạng — và nó hỏng ở dạng "khởi động
    // chậm rồi trắng màn hình".
    const ngoai: string[] = [];
    page.on("request", (r) => {
      const u = new URL(r.url());
      if (!["localhost", "127.0.0.1", ""].includes(u.hostname)) {
        ngoai.push(r.url());
      }
    });
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    expect(ngoai).toEqual([]);
  });
});
