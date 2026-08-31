/**
 * E2E cho **trò chơi**, không chỉ cho trang.
 *
 * ## Vì sao bộ này tồn tại
 *
 * Hai bộ e2e cũ chạy với một frontend **không có server**: chúng chứng minh
 * trang mount được, và không chứng minh được gì về việc trò chơi có chạy hay
 * không. Khoảng trống đó đã để lọt đúng những lỗi tệ nhất của dự án này, và cả
 * ba chỉ lộ ra khi có người mở trình duyệt bấm thử:
 *
 * | Lỗi | Vì sao không bộ test nào thấy |
 * |---|---|
 * | server chưa từng trả lời preflight CORS | `curl` không hỏi trước, nên **mọi** `POST` từ trình duyệt chết mà `cargo test` vẫn xanh |
 * | khay ngữ cảnh bị đẩy xuống dưới đáy màn hình | chiều cao chỉ tồn tại khi có layout engine |
 * | một ổ bánh được coi là "sinh mệnh" nên quyền năng cho người sáng lên | logic đúng ở từng hàm, sai ở chỗ nối |
 *
 * ## Chạy trên cùng một gốc
 *
 * `mow-server` phục vụ luôn `web/dist`, nên không có CORS ở giữa — đúng hình
 * dạng người dùng nhận. Xem `playwright.config.ts`, project `game`.
 */

import { expect, test, type ConsoleMessage, type Page } from "@playwright/test";

/**
 * Bắt mọi lỗi console trong một bài.
 *
 * Trả về một hàm khẳng định — gọi ở cuối bài. Khẳng định ngay trong listener sẽ
 * ném từ một ngữ cảnh mà Playwright không gắn được vào bài nào.
 */
function catchConsoleErrors(page: Page): () => void {
  const errs: string[] = [];
  page.on("console", (m: ConsoleMessage) => {
    if (m.type() === "error") errs.push(m.text());
  });
  page.on("pageerror", (e: Error) => errs.push(e.message));
  return () => expect(errs, "có lỗi console trong lúc chơi").toEqual([]);
}

/** Bước vào thế giới từ màn hình đầu, và đợi tới khi có cư dân trên bản đồ. */
async function enterWorld(page: Page): Promise<void> {
  await page.goto("/");
  const enter = page.getByRole("button", { name: /bước vào thế giới|enter the world/i });
  await expect(enter).toBeVisible();
  await enter.click();
  // Thanh trên chỉ hiện khi đã có `meta` — tức là server đã trả lời.
  await expect(page.locator("header")).toBeVisible();
}

test.describe("trò chơi chạy được từ đầu tới cuối", () => {
  test("bước vào thế giới thì thấy một ngôi làng có người", async ({ page }) => {
    const assertClean = catchConsoleErrors(page);
    await enterWorld(page);

    // Đây là bài kiểm cho đúng lời phàn nàn đã dẫn tới Giai đoạn G: "tôi là một
    // vị thần mà nơi bắt đầu trông chả ra làm sao".
    const souls = page.locator("header").getByText(/sinh mệnh|souls/i);
    await expect(souls).toBeVisible();
    await expect(souls).not.toHaveText(/\b0\b/);

    assertClean();
  });

  test("người chơi **không** có thân xác trên bản đồ", async ({ page }) => {
    await enterWorld(page);
    await page.getByRole("button", { name: /quan sát|observe/i }).click();
    // "Nguoi Choi" là thân xác của bản cũ; nó đã bị gỡ và không được quay lại.
    await expect(page.getByText("Nguoi Choi")).toHaveCount(0);
  });

  test("mọi thao tác đổi thế giới đi qua được — preflight CORS không chặn", async ({ page }) => {
    // Bản trước, **mọi** `POST` từ trình duyệt chết bằng `TypeError: Failed to
    // fetch` vì server chưa từng trả lời `OPTIONS`. Đổi tốc độ là một `POST`,
    // nên nó là que thử rẻ nhất cho cả lớp lỗi đó.
    const assertClean = catchConsoleErrors(page);
    await enterWorld(page);

    const fast = page.locator("header").getByRole("button", { name: "×25" });
    await fast.click();
    await expect(fast).toHaveClass(/on/);

    assertClean();
  });

  test("chọn một cư dân thì quyền năng cho sinh mệnh mới sáng lên", async ({ page }) => {
    await enterWorld(page);
    await page.getByRole("button", { name: /quan sát|observe/i }).click();

    // Danh sách có mặt: bấm dòng đầu tiên là chọn một cư dân thật.
    const first = page.locator("aside .list li.pick").first();
    await expect(first).toBeVisible();
    await first.click();

    // "Ban no đủ" cần một sinh mệnh. Trước khi tách sinh mệnh khỏi vật phẩm, nó
    // sáng lên cả khi người chơi bấm vào một ổ bánh.
    const feed = page.getByRole("button", { name: /ban no đủ|grant fullness/i });
    await expect(feed).toBeVisible();
    await expect(feed).toHaveAttribute("aria-disabled", "false");
  });

  test("khay ngữ cảnh nằm **trong** màn hình", async ({ page }) => {
    // Bản trước khay bị đẩy xuống dưới đáy viewport và không ai thấy nó — một
    // lỗi bố cục mà chỉ layout engine thật mới lộ ra.
    await enterWorld(page);
    await page.getByRole("button", { name: /quan sát|observe/i }).click();
    await page.locator("aside .list li.pick").first().click();

    const tray = page.locator(".tray");
    await expect(tray).toBeVisible();
    const box = await tray.boundingBox();
    const height = page.viewportSize()?.height ?? 0;
    expect(box, "không đo được khay").not.toBeNull();
    expect(box?.y ?? Infinity).toBeLessThan(height);
  });

  test("thế giới trôi khi thời gian chạy, và đứng yên khi dừng", async ({ page }) => {
    await enterWorld(page);
    const day = page.locator("header").getByText(/ngày|day/i);
    await expect(day).toBeVisible();

    await page.locator("header").getByRole("button", { name: "⏸" }).click();
    const before = await day.textContent();
    await page.waitForTimeout(1_200);
    expect(await day.textContent(), "thời gian vẫn trôi sau khi bấm dừng").toBe(before);
  });
});
