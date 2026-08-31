//! Test DSL luật, sandbox, vật phẩm mang hành vi, bí mật (`PE-01`–`PE-08`).

use mow_core::EntityId;
use mow_magic::artifact::{
    check_synthesis, Bearer, Behaviour, Charges, Gate, GateRequirement, Revelation, Synthesis,
    SynthesisError, GATES,
};
use mow_magic::dsl::{Expr, Quantity, Rule, RuleError, Unit, MAX_DEPTH};
use mow_magic::sandbox::{
    Capability, ContextKind, Invocation, LawHistory, LoadError, ModuleManifest, ModuleRegistry,
    Outcome, Sandbox,
};
use mow_magic::secrecy::{audit_prompt, audit_session, Secret, SecretRegistry};
use mow_math::Fx;
use std::collections::BTreeMap;

fn q(n: i64, unit: Unit) -> Quantity {
    Quantity {
        value: Fx::from_int(n).unwrap(),
        unit,
    }
}

fn hang(n: i64, unit: Unit) -> Expr {
    Expr::Const(q(n, unit))
}

// ─────────────────────────── PE-01 · DSL Tier 0 ───────────────────────────

/// **Lỗi đơn vị không sai cú pháp, không sai kiểu, và im lặng.** Bộ kiểm phải bắt.
#[test]
fn cong_mana_vao_nhiet_luong_bi_bat_luc_kiem_tinh() {
    let sai = Expr::Add(
        Box::new(hang(12_000, Unit::Mmu)),
        Box::new(hang(8, Unit::Kilojoule)),
    );
    let loi = sai.typecheck(&BTreeMap::new()).unwrap_err();
    assert!(matches!(loi, RuleError::UnitMismatch { .. }), "{loi:?}");
}

/// Cùng đơn vị thì cộng được.
#[test]
fn cung_don_vi_thi_cong_duoc() {
    let dung = Expr::Add(
        Box::new(hang(100, Unit::Joule)),
        Box::new(hang(50, Unit::Joule)),
    );
    assert_eq!(dung.typecheck(&BTreeMap::new()).unwrap(), Unit::Joule);
}

/// **Nhân hai đại lượng có thứ nguyên bị cấm** thay vì bịa ra một đơn vị.
#[test]
fn nhan_hai_dai_luong_co_thu_nguyen_bi_cam() {
    let sai = Expr::Mul(
        Box::new(hang(10, Unit::Joule)),
        Box::new(hang(3, Unit::Metre)),
    );
    assert!(matches!(
        sai.typecheck(&BTreeMap::new()).unwrap_err(),
        RuleError::DimensionalProduct { .. }
    ));

    // Nhưng nhân với một hệ số không thứ nguyên thì được, và giữ nguyên đơn vị.
    let dung = Expr::Mul(
        Box::new(hang(10, Unit::Joule)),
        Box::new(hang(3, Unit::Ratio)),
    );
    assert_eq!(dung.typecheck(&BTreeMap::new()).unwrap(), Unit::Joule);
}

/// **Không có đổi đơn vị ngầm** — một hệ số 1000 lặng lẽ là chỗ lỗi trốn vào.
#[test]
fn doi_don_vi_phai_tuong_minh() {
    let doi = Expr::Convert {
        inner: Box::new(hang(8, Unit::Kilojoule)),
        to: Unit::Joule,
        factor: Fx::from_int(1_000).unwrap(),
    };
    assert_eq!(doi.typecheck(&BTreeMap::new()).unwrap(), Unit::Joule);
    let ra = doi.eval(&BTreeMap::new()).unwrap();
    assert_eq!(ra.unit, Unit::Joule);
    assert_eq!(ra.value, Fx::from_int(8_000).unwrap());
}

/// **Đảm bảo dừng**: cây quá sâu bị từ chối **lúc nạp**, không lúc chạy.
#[test]
fn bieu_thuc_qua_sau_bi_tu_choi_luc_nap() {
    let mut e = hang(1, Unit::Ratio);
    for _ in 0..MAX_DEPTH + 5 {
        e = Expr::Add(Box::new(e), Box::new(hang(1, Unit::Ratio)));
    }
    assert!(matches!(
        e.typecheck(&BTreeMap::new()).unwrap_err(),
        RuleError::TooDeep { .. }
    ));
}

/// Biến không khai báo bị bắt, không im lặng thành 0.
#[test]
fn bien_khong_khai_bao_bi_bat() {
    let e = Expr::Var("caster.focus".into());
    assert!(matches!(
        e.typecheck(&BTreeMap::new()).unwrap_err(),
        RuleError::UnknownVar(_)
    ));
}

/// Không có biến thể số thực nào trong `Quantity` — `§P10.2.1`.
#[test]
fn khong_co_so_thuc_trong_duong_commit() {
    let j = serde_json::to_string(&q(42, Unit::Joule)).unwrap();
    assert!(!j.contains('.'), "giá trị có dấu chấm thập phân: {j}");
}

/// Một luật đầy đủ: kiểm tĩnh trả về **mọi** lỗi, không dừng ở lỗi đầu.
#[test]
fn kiem_tinh_tra_ve_moi_loi_khong_dung_o_loi_dau() {
    let luat = Rule {
        rule_id: "magic.firebolt".into(),
        version: 1,
        trigger: "action.cast_spell".into(),
        inputs: BTreeMap::from([("focus".to_owned(), Unit::Ratio)]),
        compute: BTreeMap::from([
            (
                "a".to_owned(),
                Expr::Add(Box::new(hang(1, Unit::Joule)), Box::new(hang(1, Unit::Mmu))),
            ),
            ("b".to_owned(), Expr::Var("khong_ton_tai".into())),
        ]),
        output_units: BTreeMap::new(),
    };
    assert_eq!(luat.validate().len(), 2, "phải báo cả hai lỗi cùng lúc");
}

/// Đơn vị đầu ra không khớp khai báo cũng là lỗi.
#[test]
fn don_vi_dau_ra_khong_khop_khai_bao_la_loi() {
    let luat = Rule {
        rule_id: "magic.firebolt".into(),
        version: 1,
        trigger: "x".into(),
        inputs: BTreeMap::new(),
        compute: BTreeMap::from([("energy".to_owned(), hang(100, Unit::Mmu))]),
        output_units: BTreeMap::from([("energy".to_owned(), Unit::Joule)]),
    };
    assert_eq!(luat.validate().len(), 1);
}

/// Luật hợp lệ chạy được, và **luôn dừng**.
#[test]
fn luat_hop_le_chay_duoc_va_luon_dung() {
    let luat = Rule {
        rule_id: "magic.firebolt".into(),
        version: 1,
        trigger: "action.cast_spell".into(),
        inputs: BTreeMap::from([("focus".to_owned(), Unit::Ratio)]),
        compute: BTreeMap::from([(
            "projectile_energy".to_owned(),
            Expr::Clamp {
                value: Box::new(Expr::Mul(
                    Box::new(Expr::Var("focus".into())),
                    Box::new(hang(180, Unit::Joule)),
                )),
                lo: Box::new(hang(500, Unit::Joule)),
                hi: Box::new(hang(6_000, Unit::Joule)),
            },
        )]),
        output_units: BTreeMap::from([("projectile_energy".to_owned(), Unit::Joule)]),
    };
    assert!(luat.validate().is_empty(), "{:?}", luat.validate());

    let ctx = BTreeMap::from([("focus".to_owned(), q(10, Unit::Ratio))]);
    let ra = luat.run(&ctx).unwrap();
    let e = ra["projectile_energy"];
    assert_eq!(e.unit, Unit::Joule);
    assert_eq!(e.value, Fx::from_int(1_800).unwrap());
}

/// Chạy **xác định**: cùng đầu vào, cùng kết quả.
#[test]
fn chay_luat_xac_dinh() {
    let e = Expr::Mul(
        Box::new(hang(7, Unit::Ratio)),
        Box::new(hang(6, Unit::Joule)),
    );
    let c = BTreeMap::new();
    assert_eq!(e.eval(&c).unwrap(), e.eval(&c).unwrap());
}

// ───────────────────── PE-02, PE-03 · sandbox và context ─────────────────────

fn module(context: ContextKind, caps: Vec<Capability>) -> ModuleManifest {
    ModuleManifest {
        id: "core.spell.firebolt".into(),
        version: 3,
        context,
        capabilities: caps,
        fuel_limit: 100_000,
        memory_limit: 1 << 20,
        imports: vec!["mow.read_observation".into(), "mow.emit_proposal".into()],
    }
}

/// **Con đường ngắn nhất tới lỗ hổng toàn tri** phải bị chặn ở cửa nạp.
#[test]
fn module_agent_xin_doc_authoritative_thi_bi_tu_choi_nap() {
    let mut r = ModuleRegistry::new();
    let loi = r
        .load(module(
            ContextKind::Agent,
            vec![Capability::ReadAuthoritative("epidemiology".into())],
        ))
        .unwrap_err();

    assert!(matches!(loi, LoadError::AgentWantsAuthoritative { .. }));
    assert!(
        !r.has("core.spell.firebolt"),
        "module bị từ chối mà vẫn nằm trong registry"
    );
}

/// `SystemResolver` thì được — nhưng chỉ đúng miền đã khai.
#[test]
fn system_resolver_chi_doc_duoc_dung_mien_da_khai() {
    let m = module(
        ContextKind::SystemResolver,
        vec![Capability::ReadAuthoritative("epidemiology".into())],
    );
    let mut r = ModuleRegistry::new();
    assert!(r.load(m.clone()).is_ok());

    let s = Sandbox::new(m);
    assert!(s.may_read("epidemiology"));
    assert!(!s.may_read("economy"), "đọc được miền chưa khai");
}

/// Module `Agent` **không đọc được gì** authoritative, kể cả khi lọt qua nạp.
#[test]
fn module_agent_khong_doc_duoc_authoritative_du_the_nao() {
    let s = Sandbox::new(module(
        ContextKind::Agent,
        vec![Capability::ReadOwnObservations],
    ));
    for mien in ["epidemiology", "economy", "terrain", ""] {
        assert!(!s.may_read(mien));
    }
}

/// **Không WASI**: không tệp, không mạng, và không hỏi được giờ.
#[test]
fn khong_wasi() {
    let mut m = module(ContextKind::Agent, vec![]);
    m.imports
        .push("wasi_snapshot_preview1.clock_time_get".into());
    assert!(matches!(
        ModuleRegistry::new().load(m).unwrap_err(),
        LoadError::WantsWasi { .. }
    ));
}

/// Danh sách **trắng**, không phải đen.
#[test]
fn import_ngoai_danh_sach_trang_bi_tu_choi() {
    let mut m = module(ContextKind::Agent, vec![]);
    m.imports.push("host.write_state".into());
    assert!(matches!(
        ModuleRegistry::new().load(m).unwrap_err(),
        LoadError::ForbiddenImport { .. }
    ));
}

/// Module không khai trần fuel là module treo được.
#[test]
fn khong_khai_tran_fuel_thi_khong_nap() {
    let mut m = module(ContextKind::Agent, vec![]);
    m.fuel_limit = 0;
    assert!(matches!(
        ModuleRegistry::new().load(m).unwrap_err(),
        LoadError::NoFuelLimit { .. }
    ));
}

/// **Hết fuel là lỗi xác định**: hết ở đúng cùng một bước, mọi lần chạy.
#[test]
fn het_fuel_o_dung_cung_mot_buoc_moi_lan_chay() {
    let mut m = module(ContextKind::Agent, vec![]);
    m.fuel_limit = 100;
    let s = Sandbox::new(m);

    let a = s.run(1_000, 10, 0);
    let b = s.run(1_000, 10, 0);
    assert_eq!(a, b, "hai lần chạy phải hết fuel ở cùng một chỗ");
    assert_eq!(a.outcome, Outcome::OutOfFuel { at_step: 10 });
}

/// Đủ fuel thì chạy xong và chỉ trả **proposal**.
#[test]
fn du_fuel_thi_chay_xong_va_chi_tra_proposal() {
    let s = Sandbox::new(module(ContextKind::Agent, vec![]));
    match s.run(5, 10, 0).outcome {
        Outcome::Completed {
            fuel_used,
            proposals,
        } => {
            assert_eq!(fuel_used, 50);
            assert_eq!(proposals, 5);
        }
        khac => panic!("phải chạy xong, nhận được {khac:?}"),
    }
}

/// Trần bộ nhớ chặn được.
#[test]
fn tran_bo_nho_chan_duoc() {
    let s = Sandbox::new(module(ContextKind::Agent, vec![]));
    assert!(matches!(
        s.run(1, 1, 1 << 30).outcome,
        Outcome::OutOfMemory { .. }
    ));
}

// ───────────────────── PE-04 · version luật không hồi tố ─────────────────────

/// **Sửa luật không hồi tố lên lịch sử.**
#[test]
fn sua_luat_khong_hoi_to_len_lich_su() {
    let s = Sandbox::new(module(ContextKind::Agent, vec![]));
    let hom_qua: Invocation = s.run(3, 10, 0);
    assert_eq!(hom_qua.rule_version, 3);

    // Hôm nay Yuu chỉnh cân bằng, luật lên v4.
    let mut lich_su = LawHistory::new();
    lich_su.publish(3);
    lich_su.publish(4);
    assert_eq!(lich_su.current(), Some(4));

    // Lần gọi hôm qua **vẫn mang v3**.
    assert_eq!(hom_qua.rule_version, 3);
    assert!(
        !lich_su.is_retroactive(&hom_qua),
        "v3 vẫn còn trong lịch sử, nên lần gọi cũ vẫn diễn giải được"
    );
}

/// Một phiên bản bị **xóa khỏi lịch sử** thì lần gọi cũ không diễn giải được nữa
/// — và đó là điều phải báo, không phải im lặng.
#[test]
fn xoa_phien_ban_cu_lam_lan_goi_cu_khong_dien_giai_duoc() {
    let s = Sandbox::new(module(ContextKind::Agent, vec![]));
    let cu = s.run(1, 10, 0);

    let mut lich_su = LawHistory::new();
    lich_su.publish(9); // chỉ còn v9, v3 đã bị xóa
    assert!(lich_su.is_retroactive(&cu));
}

// ───────────────────── PE-05 · vật phẩm mang hành vi ─────────────────────

fn truong() -> Behaviour {
    Behaviour {
        module: "law.rune.frost_lance".into(),
        module_version: 3,
        bound_params: BTreeMap::from([("power".to_owned(), 4_200)]),
        gates: vec![
            GateRequirement {
                gate: Gate::Knowledge,
                detail: "spell.frost".into(),
                threshold: 3,
            },
            GateRequirement {
                gate: Gate::CommandWord,
                detail: "aer-thul-mor".into(),
                threshold: 0,
            },
            GateRequirement {
                gate: Gate::Cost,
                detail: "mana".into(),
                threshold: 12_000,
            },
        ],
        charges: Charges {
            max: 12,
            current: 7,
            recharge_per_day: 500,
        },
        fuel_budget: 250_000,
    }
}

fn phap_su_du_dieu_kien() -> Bearer {
    Bearer {
        who: Some(EntityId(1)),
        knowledge: BTreeMap::from([("spell.frost".to_owned(), 4)]),
        command_words: vec!["aer-thul-mor".into()],
        resources: BTreeMap::from([("mana".to_owned(), 50_000)]),
        ..Bearer::default()
    }
}

/// **Vật phẩm không chứa code** — chỉ tham chiếu module và tham số đóng băng.
#[test]
fn vat_pham_khong_chua_ma_nguon() {
    let j = serde_json::to_string(&truong()).unwrap();
    for cam in ["expr", "source", "script", "code", "eval"] {
        assert!(!j.contains(cam), "vật phẩm mang `{cam}`: {j}");
    }
    assert!(j.contains("law.rune.frost_lance"));
}

/// **Trượng cũ không đổi hành vi** vì hôm nay chỉnh cân bằng.
#[test]
fn truong_cu_khong_doi_hanh_vi_khi_luat_len_version() {
    let t = truong();
    assert_eq!(t.module_version, 3);
    let mut lich_su = LawHistory::new();
    lich_su.publish(3);
    lich_su.publish(7);
    // Vật phẩm vẫn trỏ v3 sau khi luật lên v7.
    assert_eq!(t.module_version, 3);
    assert_ne!(lich_su.current(), Some(t.module_version));
}

/// Đủ mọi cổng thì dùng được.
#[test]
fn du_moi_cong_thi_dung_duoc() {
    assert!(truong().usable_by(&phap_su_du_dieu_kien()));
}

/// **Mất khẩu quyết thì thành di vật không ai dùng được** — và nói rõ đường tìm lại.
#[test]
fn mat_khau_quyet_thanh_di_vat_nhung_van_co_duong_khoi_phuc() {
    let quen = Bearer {
        command_words: vec![],
        ..phap_su_du_dieu_kien()
    };
    let chan = truong().blocked_for(&quen);
    assert_eq!(chan.len(), 1);
    assert_eq!(chan[0].gate, Gate::CommandWord);
    assert!(chan[0].routes.contains(&"tra khảo chủ cũ"));
    assert!(chan[0].routes.contains(&"thử mò có rủi ro"));
}

/// **Mọi cổng phải có đường vượt** — một cổng không lối thoát là khóa tùy tiện.
#[test]
fn moi_cong_deu_co_duong_vuot() {
    for g in GATES {
        assert!(
            !g.escape_routes().is_empty(),
            "cổng {g:?} không có đường vượt — đó là một cái khóa tùy tiện"
        );
    }
    assert!(truong().arbitrary_locks().is_empty());
}

/// `Risk` không chặn ai — và đó là lý do nó nguy hiểm.
#[test]
fn cong_risk_khong_chan_ai() {
    let mut t = truong();
    t.gates.push(GateRequirement {
        gate: Gate::Risk,
        detail: "phản đòn".into(),
        threshold: 0,
    });
    assert!(t.usable_by(&phap_su_du_dieu_kien()), "risk không được chặn");
}

/// Hết lần dùng thì thôi, dù đủ mọi cổng.
#[test]
fn het_lan_dung_thi_thoi() {
    let mut t = truong();
    t.charges.current = 0;
    assert!(!t.usable_by(&phap_su_du_dieu_kien()));
}

/// **Vật phẩm không mở được cửa sau** mà spell thường không có.
#[test]
fn vat_pham_khong_mo_duoc_cua_sau() {
    let binh_thuong = 250_000;
    assert!(!truong().exceeds_spell_budget(binh_thuong));

    let gian_lan = Behaviour {
        fuel_budget: 100_000_000,
        ..truong()
    };
    assert!(
        gian_lan.exceeds_spell_budget(binh_thuong),
        "một vật phẩm xin fuel gấp trăm lần là một hệ thống luật song song"
    );
}

// ───────────────────── PE-06 · bí mật không vào prompt ─────────────────────

fn so_bi_mat() -> SecretRegistry {
    let mut r = SecretRegistry::new();
    r.add(Secret {
        item: 1,
        kind: "command_word".into(),
        content: "aer-thul-mor".into(),
    })
    .add(Secret {
        item: 1,
        kind: "curse".into(),
        content: "hút tuổi thọ người dùng".into(),
    });
    r
}

/// **Lớp 1**: view không có chỗ cho bí mật người xem chưa biết.
#[test]
fn view_khong_co_cho_cho_bi_mat_chua_biet() {
    let r = so_bi_mat();
    let mu_tit = r.view_for(99, 1);
    assert!(mu_tit.known_secrets.is_empty());

    let j = serde_json::to_string(&mu_tit).unwrap();
    assert!(!j.contains("aer-thul-mor"));
    assert!(!j.contains("hút tuổi thọ"));
}

/// Biết một bí mật thì thấy đúng một, không thấy cái kia.
#[test]
fn biet_mot_bi_mat_thi_chi_thay_mot() {
    let mut r = so_bi_mat();
    r.reveal_to(5, 1, "command_word");
    let v = r.view_for(5, 1);
    assert_eq!(v.known_secrets.len(), 1);
    assert_eq!(v.known_secrets[0].kind, "command_word");
}

/// **Lớp 2**: Auditor bắt được rò rỉ nguyên văn, và **ném** thay vì cảnh báo.
#[test]
fn auditor_bat_duoc_ro_ri_va_nem_thay_vi_canh_bao() {
    let r = so_bi_mat();
    let prompt = "Cây trượng phản ứng khi nghe aer-thul-mor.";
    let loi = audit_prompt(prompt, 99, &[1], &r).unwrap_err();
    assert_eq!(loi.len(), 1);
    assert_eq!(loi[0].kind, "command_word");
    assert!(format!("{}", loi[0]).contains("NGHIÊM TRỌNG"));
}

/// Người **đã biết** thì không phải rò rỉ.
#[test]
fn nguoi_da_biet_thi_khong_phai_ro_ri() {
    let mut r = so_bi_mat();
    r.reveal_to(5, 1, "command_word");
    assert!(audit_prompt("nói aer-thul-mor", 5, &[1], &r).is_ok());
}

/// Quét **cả một phiên**, không phải một prompt ngẫu nhiên.
///
/// Rò rỉ hiếm là rò rỉ khó tái hiện, và rò rỉ khó tái hiện sẽ sống sót tới bản
/// phát hành.
#[test]
fn quet_ca_mot_phien_khong_phai_mot_prompt_ngau_nhien() {
    let r = so_bi_mat();
    let phien = vec![
        (99, "trời hôm nay đẹp".to_owned(), vec![1]),
        (99, "cây trượng nặng".to_owned(), vec![1]),
        (99, "nó hút tuổi thọ người dùng".to_owned(), vec![1]),
    ];
    let loi = audit_session(&phien, &r).unwrap_err();
    assert_eq!(loi.len(), 1);
    assert_eq!(loi[0].kind, "curse");
}

/// **Auditor không thay thế lớp 1**: rò rỉ ngữ nghĩa lọt qua.
///
/// Test này khẳng định một **hạn chế**, không phải một tính năng. Nó tồn tại để
/// không ai kết luận rằng có Auditor là đủ.
#[test]
fn auditor_khong_bat_duoc_ro_ri_ngu_nghia() {
    let r = so_bi_mat();
    // Không chứa chuỗi bí mật nào, nhưng nói đúng nội dung lời nguyền.
    let vong_veo = "Ai cầm cây trượng này lâu thì tóc bạc và chết sớm.";
    assert!(
        audit_prompt(vong_veo, 99, &[1], &r).is_ok(),
        "Auditor mà bắt được cái này thì test đang khẳng định một điều sai"
    );
}

// ───────────────────── PE-07 · NPC tổng hợp module ─────────────────────

/// **Chỉ ghép từ node đã biết** — không thì NPC phát minh ra thứ chưa ai nghĩ tới.
#[test]
fn chi_ghep_duoc_tu_node_da_biet() {
    let biet = BTreeMap::from([("spell.frost".to_owned(), 4_i64)]);
    let s = Synthesis {
        author: EntityId(1),
        from_nodes: vec!["spell.frost".into(), "spell.forbidden".into()],
        complexity: 10,
    };
    let loi = check_synthesis(&s, &biet, 1_000, 100);
    assert!(loi.contains(&SynthesisError::UnknownNode("spell.forbidden".into())));
}

/// **Trần độ phức tạp theo skill** — không thì học trò tạo ra thứ đại sư không tạo nổi.
#[test]
fn tran_do_phuc_tap_theo_skill() {
    let biet = BTreeMap::from([("spell.frost".to_owned(), 4_i64)]);
    let s = Synthesis {
        author: EntityId(1),
        from_nodes: vec!["spell.frost".into()],
        complexity: 80,
    };
    // Học trò: skill 200 ⇒ trần 20.
    assert!(check_synthesis(&s, &biet, 200, 100)
        .iter()
        .any(|e| matches!(e, SynthesisError::TooComplex { .. })));
    // Đại sư: skill 1000 ⇒ trần 100.
    assert!(check_synthesis(&s, &biet, 1_000, 100).is_empty());
}

// ───────────────────── PE-08 · thiên phú và khải thị ─────────────────────

/// Khải thị phải có **provenance điều tra được**.
#[test]
fn khai_thi_phai_dieu_tra_duoc_nguon_goc() {
    let co_nguon = Revelation {
        grants: "spell.starfall".into(),
        source: "deity.the_wanderer".into(),
        event_seq: 4_812,
        reducible_to: Some("knowledge.astral_conduit".into()),
    };
    assert!(co_nguon.is_investigable());

    let tu_hu_khong = Revelation {
        source: String::new(),
        event_seq: 0,
        ..co_nguon.clone()
    };
    assert!(
        !tu_hu_khong.is_investigable(),
        "một món quà từ hư không thì không ai tìm hiểu được"
    );
}

/// **Tháo ngược trả về node tri thức** — nếu không thì nền văn minh không giàu thêm.
#[test]
fn khai_thi_khong_thao_nguoc_duoc_la_mot_ngo_cut() {
    let mo_duong = Revelation {
        grants: "spell.starfall".into(),
        source: "deity.the_wanderer".into(),
        event_seq: 1,
        reducible_to: Some("knowledge.astral_conduit".into()),
    };
    assert!(!mo_duong.is_dead_end());

    let ngo_cut = Revelation {
        reducible_to: None,
        ..mo_duong
    };
    assert!(
        ngo_cut.is_dead_end(),
        "người nhận dùng được mà không ai khác học lại được"
    );
}
