/**
 * Máy trạng thái màn hình và thiết lập người chơi.
 *
 * ## Vì sao tách khỏi mọi file `.vue`
 *
 * Ba thứ ở đây — chuyển màn hình, làm sạch dữ liệu thiết lập, đọc/ghi
 * `localStorage` — không cần DOM và không cần Vue để đúng hay sai. Giữ chúng
 * thuần TypeScript nghĩa là `vitest` kiểm được toàn bộ logic mà không phải
 * dựng một trình duyệt giả, và các component `.vue` trong thư mục này chỉ
 * còn việc hiển thị — gọi vào đây, không tự quyết định gì.
 *
 * ## Vì sao `sanitize` xử lý từng trường, không phải cả object
 *
 * `localStorage` là dữ liệu người chơi có thể đã sửa tay (mở DevTools, sửa
 * JSON) — hỏng theo cách không đoán trước được: thiếu trường, sai kiểu, giá
 * trị ngoài khoảng, khóa lạ chen vào. Nếu một trường hỏng khiến cả object bị
 * vứt và trả về mặc định toàn bộ, thì mọi thiết lập khác người chơi đã chỉnh
 * tay — ngôn ngữ, cỡ chữ, tốc độ ưa thích — biến mất theo, dù chúng vẫn hợp
 * lệ. Đúng chỗ này, mỗi trường phải tự đứng hoặc tự ngã một mình.
 */

/** Màn hình đang hiện. */
export type Screen = "title" | "world" | "paused" | "settings" | "codex";

/** Hành động điều hướng màn hình mà một cú bấm nút hoặc phím `Esc` phát ra. */
export type ScreenAction = "esc" | "play" | "settings" | "codex" | "resume";

/** Thiết lập người chơi đổi được. Lưu ở `localStorage`, hỏng thì về mặc định. */
export interface Settings {
  locale: "vi" | "en";
  /** Nấc tốc độ mặc định khi vào thế giới. */
  speedIndex: number;
  /** Hiện nhãn tên trên bản đồ. */
  showLabels: boolean;
  /** Hiện lưới ô. */
  showGrid: boolean;
  /** Giảm chuyển động, cho người nhạy với hiệu ứng. */
  reduceMotion: boolean;
  /** Cỡ chữ giao diện, phần trăm: 90 | 100 | 115. */
  uiScale: number;
}

/**
 * Nấc tốc độ hợp lệ cao nhất.
 *
 * Khớp với số nấc của `SPEED_STEPS` ở `src/api/game.ts` (12 nấc, chỉ số
 * `0..11`) tại thời điểm viết file này. Chép hằng số thay vì `import` từ
 * `api/game.ts`: mục tiêu của `menu.ts` là làm phần thuần độc lập (lý do ở
 * đầu file), và mượn một con số từ module khác chỉ để có đúng một hằng số sẽ
 * biến nó thành một phụ thuộc runtime không cần thiết, giữa lúc file đó đang
 * được nơi khác sửa song song. Nếu hai nơi lệch nhau sau này (đổi số nấc tốc
 * độ mà quên sửa ở đây), hậu quả chỉ là `sanitize` từ chối một `speedIndex`
 * còn hợp lệ và âm thầm rơi về mặc định — không phải một lỗi nguy hiểm, nên
 * đánh đổi này chấp nhận được.
 */
export const MAX_SPEED_INDEX = 11;

/** Thiết lập mặc định — dùng khi chưa có gì lưu, hoặc khi dữ liệu lưu đã hỏng. */
export const DEFAULT_SETTINGS: Settings = {
  locale: "vi",
  speedIndex: 5,
  showLabels: true,
  showGrid: false,
  reduceMotion: false,
  uiScale: 100,
};

/** Khóa lưu trong `localStorage`. Có số phiên bản để đổi hình dạng sau này
 * không đọc nhầm dữ liệu cũ thành dữ liệu hỏng. */
const STORAGE_KEY = "mow.settings.v1";

function isLocale(v: unknown): v is Settings["locale"] {
  return v === "vi" || v === "en";
}

/** Nấc tốc độ hợp lệ: số nguyên, không âm, không vượt `MAX_SPEED_INDEX`. */
function isSpeedIndex(v: unknown): v is number {
  return typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= MAX_SPEED_INDEX;
}

function isBoolean(v: unknown): v is boolean {
  return typeof v === "boolean";
}

/** Cỡ chữ hợp lệ: đúng một trong ba nấc giao diện hỗ trợ. */
function isUiScale(v: unknown): v is number {
  return v === 90 || v === 100 || v === 115;
}

/**
 * Làm sạch dữ liệu thô thành một `Settings` hợp lệ.
 *
 * Nhận `unknown` vì nguồn thật sự là kết quả của `JSON.parse` trên một chuỗi
 * người chơi có thể đã sửa tay — không gì đảm bảo hình dạng của nó.
 *
 * `raw` không phải một object thuần (`null`, chuỗi, mảng, số, `undefined`)
 * thì coi như một object rỗng; quy tắc "từng trường tự đứng hoặc tự ngã" ở
 * đầu file vẫn áp dụng, nên kết quả khi đó là `DEFAULT_SETTINGS` nguyên vẹn —
 * không phải một ngoại lệ ném ra, vì dữ liệu bẩn không phải một tình huống
 * bất thường ở đây, nó là tình huống bình thường cần xử lý.
 */
export function sanitize(raw: unknown): Settings {
  const o: Record<string, unknown> =
    typeof raw === "object" && raw !== null && !Array.isArray(raw)
      ? (raw as Record<string, unknown>)
      : {};

  return {
    locale: isLocale(o.locale) ? o.locale : DEFAULT_SETTINGS.locale,
    speedIndex: isSpeedIndex(o.speedIndex) ? o.speedIndex : DEFAULT_SETTINGS.speedIndex,
    showLabels: isBoolean(o.showLabels) ? o.showLabels : DEFAULT_SETTINGS.showLabels,
    showGrid: isBoolean(o.showGrid) ? o.showGrid : DEFAULT_SETTINGS.showGrid,
    reduceMotion: isBoolean(o.reduceMotion) ? o.reduceMotion : DEFAULT_SETTINGS.reduceMotion,
    uiScale: isUiScale(o.uiScale) ? o.uiScale : DEFAULT_SETTINGS.uiScale,
  };
}

/**
 * Đọc thiết lập từ `localStorage`.
 *
 * Bọc toàn bộ trong `try/catch`, kể cả việc **đọc thuộc tính** `localStorage`
 * chứ không chỉ gọi `getItem`: một số trình duyệt ở chế độ duyệt ẩn danh ném
 * lỗi ngay khi thuộc tính đó được truy cập, trước khi bất kỳ phương thức nào
 * của nó được gọi. Một trò chơi không mở được vì không đọc nổi thiết lập là
 * một lỗi tệ hơn nhiều lần so với việc lặng lẽ dùng mặc định.
 */
export function loadSettings(): Settings {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_SETTINGS;
    return sanitize(JSON.parse(raw) as unknown);
  } catch {
    return DEFAULT_SETTINGS;
  }
}

/**
 * Ghi thiết lập vào `localStorage`.
 *
 * Cũng bọc `try/catch`, cùng lý do như `loadSettings`: dung lượng lưu trữ đầy
 * hoặc chế độ ẩn danh không được phép làm gián đoạn người chơi giữa lúc họ
 * đang chỉnh thiết lập.
 */
export function saveSettings(s: Settings): void {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(s));
  } catch {
    // Không lưu được thì thôi — thiết lập vẫn dùng tốt cho phiên đang chạy,
    // chỉ là không còn đó ở lần mở trò chơi sau.
  }
}

/**
 * Máy trạng thái màn hình. `Esc` ở đâu thì về đó.
 *
 * Năm màn hình, năm hành động — bảng chuyển đầy đủ:
 *
 * - `title` --`play`--> `world`; --`settings`--> `settings`; --`codex`--> `codex`
 * - `world` --`esc`--> `paused`
 * - `paused` --`esc` hoặc `resume`--> `world`; --`settings`--> `settings`; --`codex`--> `codex`
 * - `settings` / `codex` --`esc`--> `from` (xem bên dưới)
 *
 * Mọi cặp `(current, action)` không nằm trong bảng trên trả về nguyên
 * `current` — một hành động không có nghĩa ở màn hình đó thì không làm gì,
 * không phải một lỗi.
 *
 * ## Vì sao có tham số thứ ba `from`
 *
 * `settings` và `codex` là hai màn hình dùng chung, mở được từ cả `title` lẫn
 * `paused`. Một máy trạng thái thuần không tự nhớ "ai đã gọi mình" — nó chỉ
 * biết mỗi trạng thái hiện tại, và `settings` khi đứng một mình không phân
 * biệt được nó tới từ đâu. Nên nơi gọi (component) phải nhớ hộ: ngay lúc phát
 * hành động `"settings"`/`"codex"`, tự lưu lại `current` lúc đó; lúc `esc`,
 * truyền giá trị đã lưu làm `from`. Với những cặp `(current, action)` khác —
 * kể cả `esc` ở `world`/`paused`, nơi điểm đến đã cố định — `from` không được
 * dùng tới, nên gọi hàm ở đó không cần bận tâm và có thể bỏ qua tham số này
 * (mặc định `"title"`, an toàn vì `title` là màn hình gốc của toàn bộ máy).
 */
export function nextScreen(current: Screen, action: ScreenAction, from: Screen = "title"): Screen {
  switch (current) {
    case "title":
      if (action === "play") return "world";
      if (action === "settings") return "settings";
      if (action === "codex") return "codex";
      break;

    case "world":
      if (action === "esc") return "paused";
      break;

    case "paused":
      if (action === "esc" || action === "resume") return "world";
      if (action === "settings") return "settings";
      if (action === "codex") return "codex";
      break;

    case "settings":
    case "codex":
      if (action === "esc") return from;
      break;
  }
  return current;
}
