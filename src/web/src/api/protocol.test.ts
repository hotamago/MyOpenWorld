import { describe, expect, it } from "vitest";
import {
  assertReadOnly,
  decodeServer,
  encodeClient,
  isCommand,
  type ClientMessage,
} from "./protocol";

const VUNG = {
  min: { x: 0n, y: 0n, z: 0 },
  max: { x: 100n, y: 100n, z: 0 },
};

const XEM: ClientMessage = {
  type: "view_subscription",
  ...VUNG,
  zoom: 16,
  overlays: [],
  mode: "observer",
};

const TIEU_DIEM: ClientMessage = {
  type: "set_simulation_focus",
  center: { x: 50n, y: 50n, z: 0 },
  radius_chunks: 3,
  lod: "active",
};

describe("PA-07 — nhìn không phải là một hành động trong thế giới", () => {
  it("ViewSubscription không phải command", () => {
    expect(isCommand(XEM)).toBe(false);
  });

  it("SetSimulationFocus LÀ command", () => {
    // Nó đổi cách thế giới được mô phỏng, nên nó ghi event.
    expect(isCommand(TIEU_DIEM)).toBe(true);
  });

  it("điều khiển thời gian là command", () => {
    expect(isCommand({ type: "time_control", action: "step", ticks: 10 })).toBe(true);
  });

  it("gửi command từ vòng lặp vẽ là lỗi nổ ngay tại chỗ", () => {
    // Không có bảo vệ này, kéo bản đồ sẽ ghi vào nhật ký sự kiện: replay phụ
    // thuộc vào việc người xem kéo chuột thế nào, và determinism harness đỏ ở
    // mọi lần chạy.
    expect(() => assertReadOnly(XEM)).not.toThrow();
    expect(() => assertReadOnly(TIEU_DIEM)).toThrow(/command/);
    expect(() => assertReadOnly(TIEU_DIEM)).toThrow(/vòng lặp vẽ/);
  });
});

describe("§22.10 — tọa độ ra dây dưới dạng chuỗi", () => {
  it("mã hóa tọa độ thành chuỗi, không phải số", () => {
    const j = JSON.parse(encodeClient(XEM)) as { min: { x: unknown } };
    expect(typeof j.min.x).toBe("string");
  });

  it("tọa độ vượt 2^53 đi qua nguyên vẹn", () => {
    const xa = (1n << 55n) + 1n;
    const m: ClientMessage = {
      ...XEM,
      min: { x: xa, y: 0n, z: 0 },
    };
    const j = JSON.parse(encodeClient(m)) as { min: { x: string } };
    expect(BigInt(j.min.x)).toBe(xa);
  });

  it("giải mã thực thể trả về bigint", () => {
    const m = decodeServer(
      JSON.stringify({
        type: "entity",
        id: "e1",
        at: { x: "36028797018963969", y: "-1", z: 2 },
      }),
    );
    expect(m.type).toBe("entity");
    if (m.type === "entity") {
      expect(m.at?.x).toBe(36028797018963969n);
      expect(m.at?.y).toBe(-1n);
    }
  });

  it("thực thể rời tầm nhìn là `at: null`, không phải đã chết", () => {
    // Phân biệt này quan trọng: client không được suy ra "đã chết" từ "không
    // còn thấy". Suy sai sẽ làm giao diện báo tang cho những người đang sống.
    const m = decodeServer(JSON.stringify({ type: "entity", id: "e1", at: null }));
    if (m.type === "entity") expect(m.at).toBeNull();
  });
});

describe("§18.6 — overlay không đơn vị bị từ chối ngay ở biên", () => {
  it("thiếu đơn vị là lỗi giải mã", () => {
    expect(() =>
      decodeServer(
        JSON.stringify({
          type: "overlay",
          channel: "temp",
          cx: "0",
          cy: "0",
          data: [1, 2, 3],
          domain: [0, 1],
        }),
      ),
    ).toThrow(/đơn vị/);
  });

  it("có đơn vị thì đọc được", () => {
    const m = decodeServer(
      JSON.stringify({
        type: "overlay",
        channel: "temp",
        cx: "0",
        cy: "0",
        data: [1, 2, 3],
        unit: "°C",
        domain: [-40, 50],
      }),
    );
    expect(m.type).toBe("overlay");
    if (m.type === "overlay") {
      expect(m.unit).toBe("°C");
      expect(m.data).toBeInstanceOf(Uint8Array);
    }
  });
});

describe("giải mã", () => {
  it("thông điệp lạ bị từ chối, không im lặng bỏ qua", () => {
    // Một thông điệp không đọc được nghĩa là giao thức đã lệch giữa hai bên.
    expect(() => decodeServer(JSON.stringify({ type: "khong_biet" }))).toThrow(/lạ/);
  });

  it("tick giữ nguyên dạng chuỗi", () => {
    const m = decodeServer(
      JSON.stringify({
        type: "tick",
        tick: "18446744073709551615",
        divine_tick: "1",
        state_hash: "ab",
      }),
    );
    if (m.type === "tick") expect(m.tick).toBe("18446744073709551615");
  });
});
