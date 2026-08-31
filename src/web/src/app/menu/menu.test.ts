/**
 * Bài kiểm cho `menu.ts`.
 *
 * Ba nhóm, đúng ba thứ được hứa trong tài liệu module: `sanitize` chịu được
 * dữ liệu bẩn từng trường một, `loadSettings` không làm sập trò chơi khi
 * `localStorage` ném lỗi, và `nextScreen` khép kín đúng như bảng chuyển đã
 * viết trong `menu.ts`.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_SETTINGS,
  loadSettings,
  MAX_SPEED_INDEX,
  nextScreen,
  sanitize,
  saveSettings,
  type Screen,
  type Settings,
} from "./menu";

// ─────────────────────────────────────────────────────────────────────────────
// sanitize — mỗi trường tự đứng hoặc tự ngã, không phải cả object
// ─────────────────────────────────────────────────────────────────────────────

describe("sanitize", () => {
  it("dữ liệu hợp lệ đi qua nguyên vẹn", () => {
    const good: Settings = {
      locale: "en",
      speedIndex: 7,
      showLabels: false,
      showGrid: true,
      reduceMotion: true,
      uiScale: 115,
    };
    expect(sanitize(good)).toEqual(good);
  });

  it("null, chuỗi, mảng, số — không phải object — rơi hết về mặc định", () => {
    expect(sanitize(null)).toEqual(DEFAULT_SETTINGS);
    expect(sanitize(undefined)).toEqual(DEFAULT_SETTINGS);
    expect(sanitize("khong phai object")).toEqual(DEFAULT_SETTINGS);
    expect(sanitize(42)).toEqual(DEFAULT_SETTINGS);
    expect(sanitize([1, 2, 3])).toEqual(DEFAULT_SETTINGS);
  });

  it("một trường hỏng không kéo các trường tốt khác xuống theo", () => {
    // Đây là quy tắc chính của module: hỏng `locale` không được xóa sạch
    // `uiScale` đã chỉnh đúng ở bên cạnh.
    const raw = { locale: "fr", uiScale: 115, showLabels: false };
    const s = sanitize(raw);
    expect(s.locale).toBe(DEFAULT_SETTINGS.locale);
    expect(s.uiScale).toBe(115);
    expect(s.showLabels).toBe(false);
  });

  it("locale sai kiểu hoặc lạ thì về mặc định", () => {
    expect(sanitize({ locale: 1 }).locale).toBe("vi");
    expect(sanitize({ locale: "fr" }).locale).toBe("vi");
    expect(sanitize({ locale: null }).locale).toBe("vi");
  });

  it("speedIndex ngoài khoảng, âm, thập phân, hoặc sai kiểu thì về mặc định", () => {
    expect(sanitize({ speedIndex: -1 }).speedIndex).toBe(DEFAULT_SETTINGS.speedIndex);
    expect(sanitize({ speedIndex: MAX_SPEED_INDEX + 1 }).speedIndex).toBe(
      DEFAULT_SETTINGS.speedIndex,
    );
    expect(sanitize({ speedIndex: 2.5 }).speedIndex).toBe(DEFAULT_SETTINGS.speedIndex);
    expect(sanitize({ speedIndex: "5" }).speedIndex).toBe(DEFAULT_SETTINGS.speedIndex);
    expect(sanitize({ speedIndex: Number.NaN }).speedIndex).toBe(DEFAULT_SETTINGS.speedIndex);
    // Biên hợp lệ vẫn phải đi qua được, không bị làm tròn lố.
    expect(sanitize({ speedIndex: 0 }).speedIndex).toBe(0);
    expect(sanitize({ speedIndex: MAX_SPEED_INDEX }).speedIndex).toBe(MAX_SPEED_INDEX);
  });

  it("cờ boolean sai kiểu thì về mặc định, không ép kiểu truthy/falsy", () => {
    // `"false"` là chuỗi khác rỗng — JS coi nó truthy. Chấp nhận ép kiểu ở
    // đây sẽ biến một chuỗi ghi tay `"false"` thành `true`, đúng thứ trái
    // ngược ý người chỉnh.
    expect(sanitize({ showLabels: "false" }).showLabels).toBe(DEFAULT_SETTINGS.showLabels);
    expect(sanitize({ showGrid: 1 }).showGrid).toBe(DEFAULT_SETTINGS.showGrid);
    expect(sanitize({ reduceMotion: null }).reduceMotion).toBe(DEFAULT_SETTINGS.reduceMotion);
  });

  it("uiScale ngoài ba nấc hợp lệ thì về mặc định", () => {
    expect(sanitize({ uiScale: 105 }).uiScale).toBe(DEFAULT_SETTINGS.uiScale);
    expect(sanitize({ uiScale: "100" }).uiScale).toBe(DEFAULT_SETTINGS.uiScale);
    expect(sanitize({ uiScale: 90 }).uiScale).toBe(90);
  });

  it("khóa lạ chen vào không làm gì, không rơi vào kết quả", () => {
    const s = sanitize({ locale: "en", trom: "vao", __proto__: { evil: true } });
    expect(s).toEqual({ ...DEFAULT_SETTINGS, locale: "en" });
    expect(Object.keys(s).sort()).toEqual(Object.keys(DEFAULT_SETTINGS).sort());
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// loadSettings / saveSettings — localStorage có thể ném lỗi bất kỳ lúc nào
// ─────────────────────────────────────────────────────────────────────────────

describe("loadSettings / saveSettings", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("chưa lưu gì thì dùng mặc định", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: () => {},
    });
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("đọc lại đúng thứ đã lưu, đã làm sạch", () => {
    const store = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
    });
    const mine: Settings = { ...DEFAULT_SETTINGS, uiScale: 115, locale: "en" };
    saveSettings(mine);
    expect(loadSettings()).toEqual(mine);
  });

  it("JSON hỏng thì về mặc định, không ném lỗi ra ngoài", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => "{ khong phai json hop le",
      setItem: () => {},
    });
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("getItem ném lỗi (ví dụ hạn mức lưu trữ) thì về mặc định", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {},
    });
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it("chỉ ĐỌC thuộc tính localStorage đã ném lỗi vẫn về mặc định", () => {
    // Mô phỏng đúng ca duyệt ẩn danh nêu trong tài liệu: bản thân getter của
    // thuộc tính `localStorage` ném lỗi, chưa tới lượt gọi `getItem`.
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get(): never {
        throw new DOMException("blocked in private mode");
      },
    });
    try {
      expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
    } finally {
      delete (globalThis as { localStorage?: unknown }).localStorage;
    }
  });

  it("setItem ném lỗi thì saveSettings nuốt lỗi, không văng ra ngoài", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: () => {
        throw new Error("quota exceeded");
      },
    });
    expect(() => saveSettings(DEFAULT_SETTINGS)).not.toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// nextScreen — Esc ở đâu thì về đó, đúng bảng chuyển trong menu.ts
// ─────────────────────────────────────────────────────────────────────────────

describe("nextScreen", () => {
  it("title mở world/settings/codex", () => {
    expect(nextScreen("title", "play")).toBe("world");
    expect(nextScreen("title", "settings")).toBe("settings");
    expect(nextScreen("title", "codex")).toBe("codex");
  });

  it("esc ở world mở paused", () => {
    expect(nextScreen("world", "esc")).toBe("paused");
  });

  it("esc hoặc resume ở paused quay lại world", () => {
    expect(nextScreen("paused", "esc")).toBe("world");
    expect(nextScreen("paused", "resume")).toBe("world");
  });

  it("esc ở settings/codex quay về đúng nơi đã mở nó (from)", () => {
    // Mở từ title rồi esc: quay về title.
    expect(nextScreen("settings", "esc", "title")).toBe("title");
    expect(nextScreen("codex", "esc", "title")).toBe("title");
    // Mở từ paused rồi esc: quay về paused, không phải title.
    expect(nextScreen("settings", "esc", "paused")).toBe("paused");
    expect(nextScreen("codex", "esc", "paused")).toBe("paused");
  });

  it("không truyền from thì mặc định về title", () => {
    expect(nextScreen("settings", "esc")).toBe("title");
  });

  it("vòng khép kín đầy đủ: title → settings → title, paused → codex → paused", () => {
    let screen: Screen = "title";
    screen = nextScreen(screen, "settings");
    expect(screen).toBe("settings");
    screen = nextScreen(screen, "esc", "title");
    expect(screen).toBe("title");

    screen = nextScreen(screen, "play");
    expect(screen).toBe("world");
    screen = nextScreen(screen, "esc");
    expect(screen).toBe("paused");
    const opener = screen;
    screen = nextScreen(screen, "codex");
    expect(screen).toBe("codex");
    screen = nextScreen(screen, "esc", opener);
    expect(screen).toBe("paused");
  });

  it("hành động không có nghĩa ở màn hình đó thì không đổi gì", () => {
    expect(nextScreen("title", "esc")).toBe("title");
    expect(nextScreen("title", "resume")).toBe("title");
    expect(nextScreen("world", "play")).toBe("world");
    expect(nextScreen("world", "settings")).toBe("world");
    expect(nextScreen("paused", "play")).toBe("paused");
  });
});
