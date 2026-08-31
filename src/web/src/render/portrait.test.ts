import { describe, expect, it } from "vitest";
import {
  buildPortrait,
  kinshipResemblance,
  LAYER_SOURCE,
  PORTRAIT_LAYERS,
  strangerPortrait,
  type Phenotype,
} from "./portrait";

function pheno(over: Partial<Phenotype> = {}): Phenotype {
  return {
    species: "core.human",
    ageYears: 30,
    nutrition: 700,
    illness: 0,
    scars: 0,
    mood: 0,
    ...over,
  };
}

describe("xác định", () => {
  it("cùng seed và cùng kiểu hình luôn cho cùng chân dung", () => {
    const a = buildPortrait(0x1234_5678_9abc_def0n, pheno());
    const b = buildPortrait(0x1234_5678_9abc_def0n, pheno());
    expect(a.key).toBe(b.key);
  });

  it("seed khác thì mặt khác", () => {
    const a = buildPortrait(1n, pheno());
    const b = buildPortrait(2n, pheno());
    expect(a.key).not.toBe(b.key);
  });

  it("mọi lớp đều có mặt, đúng thứ tự", () => {
    const p = buildPortrait(7n, pheno());
    expect(p.layers.map((l) => l.layer)).toEqual([...PORTRAIT_LAYERS]);
  });

  it("dùng BigInt nên hai seed cách nhau 1 ở trên 2^53 vẫn khác nhau", () => {
    // Với `number`, cả hai đều làm tròn về cùng một giá trị và hai người xa lạ
    // sẽ có khuôn mặt giống hệt nhau (`§22.10`).
    const a = buildPortrait(2n ** 53n + 1n, pheno());
    const b = buildPortrait(2n ** 53n + 2n, pheno());
    expect(a.key).not.toBe(b.key);
  });
});

describe("con giống cha mẹ", () => {
  it("cùng bộ gen thì phần di truyền giống hệt, dù trạng thái khác hẳn", () => {
    const khoe = buildPortrait(42n, pheno({ nutrition: 900, mood: 800 }));
    const om = buildPortrait(42n, pheno({ nutrition: 100, illness: 900, scars: 5, mood: -900 }));

    expect(kinshipResemblance(khoe, om)).toBe(1);
    // Nhưng chân dung tổng thể thì khác — nhân vật mang lịch sử trên mặt.
    expect(khoe.key).not.toBe(om.key);
  });

  it("bộ gen khác thì phần di truyền thường khác", () => {
    const a = buildPortrait(1n, pheno());
    const b = buildPortrait(999_999n, pheno());
    expect(kinshipResemblance(a, b)).toBeLessThan(1);
  });

  it("chỉ tính lớp di truyền, không tính lớp trạng thái", () => {
    // Hai người **xa lạ** cùng đang ốm không được tính là giống nhau hơn.
    const la_1 = buildPortrait(11n, pheno({ illness: 900 }));
    const la_2 = buildPortrait(22n, pheno({ illness: 900 }));
    const anh = buildPortrait(11n, pheno({ illness: 0 }));
    const em = buildPortrait(11n, pheno({ illness: 0, mood: 900 }));

    expect(kinshipResemblance(anh, em)).toBeGreaterThan(kinshipResemblance(la_1, la_2));
  });

  it("bảng nguồn của lớp là cơ chế di truyền, nên nó phải đúng", () => {
    // Chuyển một lớp hình thái sang `phenotype` sẽ làm nó thôi di truyền, và
    // không có gì báo lỗi — các dòng họ chỉ mất dần nét chung.
    expect(LAYER_SOURCE.skin).toBe("genotype");
    expect(LAYER_SOURCE.hair).toBe("genotype");
    expect(LAYER_SOURCE.eyes).toBe("genotype");
    expect(LAYER_SOURCE.species).toBe("genotype");
    expect(LAYER_SOURCE.build).toBe("phenotype");
    expect(LAYER_SOURCE.condition).toBe("phenotype");
  });
});

describe("chân dung mang lịch sử", () => {
  it("đói lâu thì gầy đi", () => {
    const bt = buildPortrait(5n, pheno({ nutrition: 700 }));
    const doi = buildPortrait(5n, pheno({ nutrition: 100 }));
    expect(bt.layers.find((l) => l.layer === "build")?.variant).toBe("normal");
    expect(doi.layers.find((l) => l.layer === "build")?.variant).toBe("emaciated");
  });

  it("bệnh nặng hiện ra", () => {
    const p = buildPortrait(5n, pheno({ illness: 900 }));
    expect(p.layers.find((l) => l.layer === "condition")?.variant).toBe("gravely_ill");
  });

  it("già đi theo tuổi", () => {
    const be = buildPortrait(5n, pheno({ ageYears: 5 }));
    const gia = buildPortrait(5n, pheno({ ageYears: 70 }));
    expect(be.layers.find((l) => l.layer === "age")?.variant).toBe("child");
    expect(gia.layers.find((l) => l.layer === "age")?.variant).toBe("elder");
  });

  it("chỉ hiện một dấu hiệu trạng thái, cái nặng nhất", () => {
    // Chồng cả bệnh lẫn sẹo lẫn đói lên một khuôn mặt 32px là không đọc được.
    const p = buildPortrait(5n, pheno({ illness: 900, scars: 5, nutrition: 50 }));
    const cond = p.layers.filter((l) => l.layer === "condition");
    expect(cond).toHaveLength(1);
    expect(cond[0]!.variant).toBe("gravely_ill");
  });

  it("tâm trạng đổi biểu cảm", () => {
    const vui = buildPortrait(5n, pheno({ mood: 800 }));
    const buon = buildPortrait(5n, pheno({ mood: -800 }));
    expect(vui.layers.find((l) => l.layer === "expression")?.variant).toBe("elated");
    expect(buon.layers.find((l) => l.layer === "expression")?.variant).toBe("anguished");
  });
});

describe("người chưa từng gặp", () => {
  it("hiện bóng chung, không phải chân dung đầy đủ", () => {
    const p = strangerPortrait({ species: "core.human", build: "stout" });
    expect(p.layers.find((l) => l.layer === "species")?.variant).toBe("core.human");
    expect(p.layers.find((l) => l.layer === "skin")?.variant).toBe("unknown");
    expect(p.layers.find((l) => l.layer === "hair")?.variant).toBe("unknown");
  });

  it("không nhận seed nên không thể lỡ tay vẽ ra mặt thật", () => {
    const la = strangerPortrait({ species: "core.human" });
    const that = buildPortrait(42n, pheno());
    expect(la.key).not.toBe(that.key);
    // Và hai người lạ khác nhau trông giống nhau — đúng như "bóng chung".
    expect(la.key).toBe(strangerPortrait({ species: "core.human" }).key);
  });
});

// ═══════════════ PF-19 · bộ mười lăm lớp đầy đủ (§18.14.4) ═══════════════

describe("chân dung đầy đủ 15 lớp (PF-19, §18.14.4)", () => {
  const nen: Phenotype = {
    species: "human",
    ageYears: 30,
    nutrition: 600,
    illness: 0,
    scars: 0,
    mood: 0,
  };

  it("có đúng mười lăm lớp", () => {
    expect(PORTRAIT_LAYERS).toHaveLength(15);
    expect(buildPortrait(1n, nen).layers).toHaveLength(15);
  });

  it("thứ tự lớp là thứ tự vẽ: biểu cảm nằm trên cùng", () => {
    const l = [...PORTRAIT_LAYERS];
    expect(l[0]).toBe("species");
    expect(l[l.length - 1]).toBe("expression");
    // Trang bị đè lên trang phục.
    expect(l.indexOf("equipment")).toBeGreaterThan(l.indexOf("dress_culture"));
    expect(l.indexOf("equipment")).toBeGreaterThan(l.indexOf("dress_status"));
  });

  it("nét mặt di truyền — hai anh em cùng seed có cùng nét", () => {
    const a = buildPortrait(4242n, nen);
    const b = buildPortrait(4242n, { ...nen, ageYears: 60, mood: -800 });
    const net = (p: typeof a) =>
      p.layers.find((l) => l.layer === "features")?.variant;
    expect(net(a)).toBe(net(b));
  });

  it("sẹo KHÔNG di truyền — con không thừa hưởng vết sẹo của cha", () => {
    expect(LAYER_SOURCE.injuries).toBe("phenotype");
    const cha = buildPortrait(4242n, { ...nen, scars: 5 });
    const con = buildPortrait(4242n, nen);
    const seo = (p: typeof cha) =>
      p.layers.find((l) => l.layer === "injuries")?.variant;
    expect(seo(cha)).not.toBe(seo(con));
  });

  it("trẻ con không mọc râu dù bộ gen nói có", () => {
    const rau = (tuoi: number) =>
      buildPortrait(4242n, { ...nen, ageYears: tuoi }).layers.find(
        (l) => l.layer === "facial_hair",
      )?.variant;
    expect(rau(8)).toBe("none");
    expect(rau(30)).not.toBe("none");
    // Nhưng vẫn là cùng một phương án ở mọi tuổi trưởng thành — nó là gen.
    expect(rau(30)).toBe(rau(70));
  });

  it("bộ phận mất đi thắng số sẹo", () => {
    const p = buildPortrait(1n, {
      ...nen,
      scars: 9,
      missingParts: ["left_hand"],
    });
    expect(p.layers.find((l) => l.layer === "injuries")?.variant).toBe(
      "missing_left_hand",
    );
  });

  it("cùng tập bộ phận cho cùng khóa bất kể thứ tự truyền vào", () => {
    const a = buildPortrait(1n, { ...nen, missingParts: ["eye", "left_hand"] });
    const b = buildPortrait(1n, { ...nen, missingParts: ["left_hand", "eye"] });
    expect(a.key).toBe(b.key);
  });

  it("bệnh và sẹo là hai lớp riêng, chồng lên nhau được", () => {
    const p = buildPortrait(1n, { ...nen, illness: 800, scars: 2 });
    expect(p.layers.find((l) => l.layer === "condition")?.variant).toBe(
      "gravely_ill",
    );
    expect(p.layers.find((l) => l.layer === "injuries")?.variant).toBe(
      "scarred",
    );
  });

  it("không khai văn hóa thì mặc đồ chung, không phải để trống", () => {
    expect(
      buildPortrait(1n, nen).layers.find((l) => l.layer === "dress_culture")
        ?.variant,
    ).toBe("common");
  });

  it("địa vị thành năm bậc rời rạc, phân biệt được", () => {
    const bac = (s: number) =>
      buildPortrait(1n, { ...nen, status: s }).layers.find(
        (l) => l.layer === "dress_status",
      )?.variant;
    expect(bac(50)).toBe("destitute");
    expect(bac(300)).toBe("common");
    expect(bac(500)).toBe("prosperous");
    expect(bac(800)).toBe("notable");
    expect(bac(950)).toBe("exalted");
    // Không khai địa vị khác với địa vị thấp.
    expect(
      buildPortrait(1n, nen).layers.find((l) => l.layer === "dress_status")
        ?.variant,
    ).toBe("unmarked");
  });

  it("chỉ hiện món trang bị ngoài cùng", () => {
    const p = buildPortrait(1n, {
      ...nen,
      equipped: ["plate_armour", "cloak", "pouch"],
    });
    expect(p.layers.find((l) => l.layer === "equipment")?.variant).toBe(
      "plate_armour",
    );
  });

  it("trần hai dấu hiệu effect — vượt quá thì thành nhiễu", () => {
    const p = buildPortrait(1n, {
      ...nen,
      visibleEffects: ["burning", "frozen", "cursed", "blessed"],
    });
    const dau = p.layers.find((l) => l.layer === "effect_marks")?.variant ?? "";
    expect(dau.split("+")).toHaveLength(2);
  });

  it("dấu hiệu effect xác định bất kể thứ tự truyền vào", () => {
    const a = buildPortrait(1n, { ...nen, visibleEffects: ["frozen", "burning"] });
    const b = buildPortrait(1n, { ...nen, visibleEffects: ["burning", "frozen"] });
    expect(a.key).toBe(b.key);
  });

  it("vẫn thuần: cùng đầu vào cho cùng chân dung", () => {
    const day: Phenotype = {
      ...nen,
      missingParts: ["left_hand"],
      culture: "veskar",
      status: 700,
      equipped: ["mail"],
      visibleEffects: ["blessed"],
    };
    expect(buildPortrait(9n, day)).toEqual(buildPortrait(9n, day));
  });

  it("phần di truyền vẫn là bảy lớp, đủ để đo họ hàng", () => {
    const gen = PORTRAIT_LAYERS.filter((l) => LAYER_SOURCE[l] === "genotype");
    expect(gen).toEqual([
      "species",
      "skin",
      "hair",
      "eyes",
      "features",
      "facial_hair",
    ]);
  });

  it("người lạ vẫn chỉ hiện những gì quan sát được, đủ mười lăm lớp", () => {
    const la = strangerPortrait({ build: "gaunt", equipment: "hood" });
    expect(la.layers).toHaveLength(15);
    expect(la.layers.filter((l) => l.variant === "unknown")).toHaveLength(13);
  });
});
