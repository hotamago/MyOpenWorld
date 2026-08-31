import { describe, expect, it } from "vitest";
import { gate, naiveViewer, sameIcon, type Truth, type Viewer } from "./perception";

function thanhKiem(over: Partial<Truth> = {}): Truth {
  return {
    silhouette: "core.sword",
    material: "core.steel",
    quality: "quality_master",
    states: [],
    claimDisputed: false,
    ...over,
  };
}

function nguoiBiet(over: Partial<Viewer> = {}): Viewer {
  return {
    mode: "embodied",
    canAppraise: true,
    perceives: new Set(["magic.aura", "curse.mark"]),
    knowsClaimDispute: true,
    seesThroughDisguise: true,
    ...over,
  };
}

describe("chất lượng chưa thẩm định", () => {
  it("hiện dấu hỏi, không phải bỏ trống", () => {
    // Bỏ trống trông y hệt "đồ thường", và người chơi sẽ học rằng mọi thứ nhặt
    // được đều là đồ thường.
    const { spec, withheld } = gate(thanhKiem(), naiveViewer());
    expect(spec.annotation).toBe("unappraised");
    expect(withheld).toContain("quality:quality_master");
  });

  it("có kỹ năng thẩm định thì hiện thật", () => {
    const { spec, withheld } = gate(thanhKiem(), nguoiBiet());
    expect(spec.annotation).toBe("quality_master");
    expect(withheld).toEqual([]);
  });

  it("chỉ có đúng một chỗ sinh ra dấu hỏi", () => {
    // `unappraised` chỉ được mang một nghĩa. Lỗi hợp thành **không** dùng nó.
    const khong_co_chat_luong = gate(thanhKiem({ quality: undefined }), naiveViewer());
    expect(khong_co_chat_luong.spec.annotation).toBeUndefined();
  });
});

describe("phép ẩn và lời nguyền", () => {
  const bi_nguyen = thanhKiem({
    states: [
      { sign: "cursed", perceptibleAs: "curse.mark" },
      { sign: "hidden_enchant", perceptibleAs: "magic.aura" },
      { sign: "burnt", perceptibleAs: null },
    ],
  });

  it("chỉ hiện huy hiệu nếu người xem nhận biết được", () => {
    const { spec, withheld } = gate(bi_nguyen, naiveViewer());
    expect(spec.states).toEqual(["burnt"]);
    expect(withheld).toContain("state:cursed");
    expect(withheld).toContain("state:hidden_enchant");
  });

  it("người nhận biết được thì thấy hết", () => {
    const { spec } = gate(bi_nguyen, nguoiBiet());
    expect(spec.states).toEqual(["cursed", "hidden_enchant", "burnt"]);
  });

  it("nhận biết một phần thì thấy một phần", () => {
    const chi_thay_nguyen = nguoiBiet({ perceives: new Set(["curse.mark"]) });
    const { spec } = gate(bi_nguyen, chi_thay_nguyen);
    expect(spec.states).toEqual(["cursed", "burnt"]);
  });

  it("ướt, cháy, gãy thì ai cũng thấy", () => {
    const { spec } = gate(thanhKiem({ states: [{ sign: "wet", perceptibleAs: null }] }), naiveViewer());
    expect(spec.states).toEqual(["wet"]);
  });
});

describe("cải trang", () => {
  const gian_diep = thanhKiem({ silhouette: "core.noble", disguisedAs: "core.beggar" });

  it("hiện bóng của lớp cải trang cho tới khi có ai nhìn ra", () => {
    const { spec, withheld } = gate(gian_diep, naiveViewer());
    expect(spec.silhouette).toBe("core.beggar");
    expect(withheld).toContain("silhouette:core.noble");
  });

  it("nhìn ra thì hiện bóng thật", () => {
    expect(gate(gian_diep, nguoiBiet()).spec.silhouette).toBe("core.noble");
  });
});

describe("dấu đồ gian", () => {
  const do_gian = thanhKiem({ claimDisputed: true });

  it("chỉ hiện với người biết có tranh chấp claim", () => {
    const { spec, withheld } = gate(do_gian, naiveViewer());
    expect(spec.marker).toBeUndefined();
    expect(withheld).toContain("marker:stolen");
  });

  it("người biết thì thấy", () => {
    expect(gate(do_gian, nguoiBiet()).spec.marker).toBe("stolen");
  });
});

describe("chế độ quan sát và True God", () => {
  const day_du = thanhKiem({
    silhouette: "core.noble",
    disguisedAs: "core.beggar",
    claimDisputed: true,
    states: [{ sign: "cursed", perceptibleAs: "curse.mark" }],
  });

  it("hiện đầy đủ dù người xem không biết gì", () => {
    for (const mode of ["observer", "true_god"] as const) {
      const mu = { ...naiveViewer(), mode };
      const { spec, withheld } = gate(day_du, mu);
      expect(spec.silhouette).toBe("core.noble");
      expect(spec.annotation).toBe("quality_master");
      expect(spec.states).toEqual(["cursed"]);
      expect(spec.marker).toBe("stolen");
      expect(withheld).toEqual([]);
    }
  });
});

describe("bộ lọc thật sự lọc", () => {
  it("người biết và người không biết thấy icon khác nhau", () => {
    // Một bộ lọc không giấu gì bao giờ trông y hệt một bộ lọc đúng.
    const bi_mat = thanhKiem({
      claimDisputed: true,
      states: [{ sign: "cursed", perceptibleAs: "curse.mark" }],
    });
    expect(sameIcon(bi_mat, naiveViewer(), nguoiBiet())).toBe(false);
  });

  it("không có bí mật nào thì hai người thấy giống nhau", () => {
    const thuong = thanhKiem({ quality: undefined, states: [] });
    expect(sameIcon(thuong, naiveViewer(), nguoiBiet())).toBe(true);
  });

  it("cùng người xem, cùng sự thật thì luôn cùng icon", () => {
    const t = thanhKiem({ claimDisputed: true });
    expect(sameIcon(t, naiveViewer(), naiveViewer())).toBe(true);
  });
});

describe("không có mặc định nào", () => {
  it("Viewer đòi khai báo đủ, vì mặc định nào cũng sai theo một hướng", () => {
    // `naiveViewer()` là mốc "không biết gì" tường minh, không phải một mặc định
    // ngầm. Nếu quên khai `knowsClaimDispute`, `false` thì icon thiếu thông tin,
    // `true` thì icon rò thông tin — nên không bên nào được làm mặc định.
    const v = naiveViewer();
    expect(v.canAppraise).toBe(false);
    expect(v.knowsClaimDispute).toBe(false);
    expect(v.seesThroughDisguise).toBe(false);
    expect(v.perceives.size).toBe(0);
  });
});
