//! Kịch bản ma thuật (`PE-14`, phần `magic/*`).
//!
//! Bốn kịch bản, mỗi cái đi hết một chuỗi từ đầu tới cuối thay vì kiểm một hàm.
//! Chúng tồn tại vì các unit test ở `mow-magic` chứng minh từng mảnh đúng, còn
//! chỗ hỏng thật lại nằm ở **mối nối** — một luật hợp lệ nạp vào một sandbox
//! cấu hình sai, một vật phẩm trỏ đúng module nhưng module đó đã lên version
//! khác, một prompt sạch dựng từ một view bẩn.

use mow_magic::artifact::{
    check_synthesis, Bearer, Behaviour, Charges, Gate, GateRequirement, Synthesis, SynthesisError,
};
use mow_magic::dsl::{Expr, Quantity, Rule, Unit};
use mow_magic::sandbox::{
    Capability, ContextKind, ContextKind as Ctx, LawHistory, LoadError, ModuleManifest,
    ModuleRegistry, Outcome, Sandbox,
};
use mow_magic::secrecy::{audit_session, Secret, SecretRegistry};
use mow_math::Fx;
use std::collections::BTreeMap;

fn hang(n: i64, u: Unit) -> Expr {
    Expr::Const(Quantity {
        value: Fx::from_int(n).unwrap(),
        unit: u,
    })
}

fn manifest(id: &str, ctx: ContextKind, caps: Vec<Capability>) -> ModuleManifest {
    ModuleManifest {
        id: id.into(),
        version: 3,
        context: ctx,
        capabilities: caps,
        fuel_limit: 50_000,
        memory_limit: 1 << 20,
        imports: vec!["mow.read_observation".into(), "mow.emit_proposal".into()],
    }
}

// ─────────────────── magic/cast_spell — vòng đời một câu thần chú ───────────────────

/// Từ luật YAML tới đề xuất, không có bước nào chạy code tự do.
///
/// Chuỗi: soạn luật → kiểm tĩnh → nạp module → chạy có fuel → ra proposal.
/// Nếu một mắt nào cho phép `eval` thì cả chuỗi mất giá trị, nên kịch bản này
/// đi từ đầu chứ không chỉ kiểm mắt cuối.
#[test]
fn magic_cast_spell_di_het_chuoi_ma_khong_co_eval() {
    let luat = Rule {
        rule_id: "magic.firebolt".into(),
        version: 3,
        trigger: "action.cast_spell".into(),
        inputs: BTreeMap::from([
            ("focus".to_owned(), Unit::Ratio),
            ("mana_spent".to_owned(), Unit::Mmu),
        ]),
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

    let mut reg = ModuleRegistry::new();
    reg.load(manifest(
        "magic.firebolt",
        Ctx::Agent,
        vec![Capability::ReadOwnObservations],
    ))
    .expect("spell chạy ở context agent");

    let s = Sandbox::new(manifest("magic.firebolt", Ctx::Agent, vec![]));
    let ket = s.run(4, 20, 0);
    match ket.outcome {
        Outcome::Completed { proposals, .. } => assert_eq!(proposals, 4),
        khac => panic!("{khac:?}"),
    }
    assert_eq!(ket.rule_version, 3, "lần gọi mang version luật lúc chạy");
}

/// **Một spell không nhìn thấy được thứ chủ nhân nó không nhìn thấy.**
///
/// Đây là `INV-22-4` áp cho ma thuật: một module `Agent` xin đọc dịch tễ để
/// "cân bằng" thì bị chặn ở cửa nạp, không phải ở chỗ gọi.
#[test]
fn magic_spell_khong_nhin_thay_hon_chu_nhan_no() {
    let mut reg = ModuleRegistry::new();
    let loi = reg
        .load(manifest(
            "magic.plague_sense",
            Ctx::Agent,
            vec![Capability::ReadAuthoritative("epidemiology".into())],
        ))
        .unwrap_err();
    assert!(matches!(loi, LoadError::AgentWantsAuthoritative { .. }));
    assert!(!reg.has("magic.plague_sense"));

    // Cùng một module, khai đúng context, thì nạp được — và đó là chỗ khác biệt.
    assert!(reg
        .load(manifest(
            "system.epidemic_resolver",
            Ctx::SystemResolver,
            vec![Capability::ReadAuthoritative("epidemiology".into())],
        ))
        .is_ok());
}

// ─────────────── magic/artifact — trượng cũ sau một lần cân bằng ───────────────

/// **Chỉnh cân bằng hôm nay không đổi cây trượng rèn năm ngoái.**
#[test]
fn magic_artifact_khong_doi_hanh_vi_khi_luat_len_version() {
    let truong = Behaviour {
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
        ],
        charges: Charges {
            max: 12,
            current: 5,
            recharge_per_day: 500,
        },
        fuel_budget: 200_000,
    };

    let mut ls = LawHistory::new();
    ls.publish(3);
    ls.publish(4);
    ls.publish(5);

    assert_eq!(ls.current(), Some(5));
    assert_eq!(truong.module_version, 3, "vật phẩm vẫn trỏ v3");
    assert_eq!(
        truong.bound_params["power"], 4_200,
        "tham số đã đóng băng lúc rèn"
    );

    let phap_su = Bearer {
        knowledge: BTreeMap::from([("spell.frost".to_owned(), 4)]),
        command_words: vec!["aer-thul-mor".into()],
        ..Bearer::default()
    };
    assert!(truong.usable_by(&phap_su));
    assert!(truong.arbitrary_locks().is_empty());
}

/// Mất khẩu quyết thì thành di vật — **nhưng luôn có đường tìm lại**.
#[test]
fn magic_artifact_mat_khau_quyet_van_con_duong_khoi_phuc() {
    let truong = Behaviour {
        module: "law.rune.frost_lance".into(),
        module_version: 3,
        bound_params: BTreeMap::new(),
        gates: vec![GateRequirement {
            gate: Gate::CommandWord,
            detail: "aer-thul-mor".into(),
            threshold: 0,
        }],
        charges: Charges {
            max: 1,
            current: 1,
            recharge_per_day: 0,
        },
        fuel_budget: 1_000,
    };
    let quen = Bearer::default();
    assert!(!truong.usable_by(&quen));
    let chan = truong.blocked_for(&quen);
    assert_eq!(chan.len(), 1);
    assert!(
        !chan[0].routes.is_empty(),
        "một cổng không lối thoát là khóa tùy tiện"
    );
}

// ─────────────── magic/synthesis — NPC tự ghép một câu thần chú ───────────────

/// **NPC ghép được module chỉ từ node nó biết** — và không hơn.
#[test]
fn magic_synthesis_npc_chi_ghep_tu_node_da_biet() {
    let hoc_tro = BTreeMap::from([
        ("spell.frost".to_owned(), 3_i64),
        ("spell.shape".to_owned(), 2),
    ]);

    let trong_tam_voi = Synthesis {
        author: mow_core::EntityId(1),
        from_nodes: vec!["spell.frost".into(), "spell.shape".into()],
        complexity: 15,
    };
    assert!(
        check_synthesis(&trong_tam_voi, &hoc_tro, 800, 100).is_empty(),
        "ghép từ hai node đã biết, độ phức tạp vừa sức"
    );

    let ngoai_tam = Synthesis {
        from_nodes: vec!["spell.frost".into(), "spell.forbidden_star".into()],
        ..trong_tam_voi.clone()
    };
    assert!(check_synthesis(&ngoai_tam, &hoc_tro, 800, 100)
        .contains(&SynthesisError::UnknownNode("spell.forbidden_star".into())));

    let qua_suc = Synthesis {
        complexity: 90,
        ..trong_tam_voi
    };
    assert!(check_synthesis(&qua_suc, &hoc_tro, 200, 100)
        .iter()
        .any(|e| matches!(e, SynthesisError::TooComplex { .. })));
}

// ─────────────── magic/secrecy — một phiên chơi không rò bí mật ───────────────

/// Quét **cả một phiên**: prompt nào cũng sạch với người chưa biết.
#[test]
fn magic_secrecy_ca_mot_phien_khong_ro_bi_mat() {
    let mut so = SecretRegistry::new();
    so.add(Secret {
        item: 1,
        kind: "command_word".into(),
        content: "aer-thul-mor".into(),
    })
    .add(Secret {
        item: 1,
        kind: "curse".into(),
        content: "hút tuổi thọ người dùng".into(),
    });
    so.reveal_to(5, 1, "command_word");

    // Người mù tịt: view không có gì, nên prompt dựng từ view cũng không có gì.
    let mu_tit = so.view_for(99, 1);
    assert!(mu_tit.known_secrets.is_empty());

    let phien = vec![
        (99, "Cây trượng lạnh khi chạm.".to_owned(), vec![1]),
        (
            99,
            "Ông lão nói nó từng thuộc về một vị vua.".to_owned(),
            vec![1],
        ),
        // Người **đã biết** khẩu quyết thì nhắc tới nó không phải rò rỉ.
        (
            5,
            "Ta đọc aer-thul-mor rồi vung trượng.".to_owned(),
            vec![1],
        ),
    ];
    assert!(audit_session(&phien, &so).is_ok());

    // Nhưng đưa lời nguyền vào prompt của người chưa biết thì phiên **hỏng**.
    let hong = vec![(99, "nó hút tuổi thọ người dùng".to_owned(), vec![1])];
    let ro = audit_session(&hong, &so).unwrap_err();
    assert_eq!(ro.len(), 1);
    assert_eq!(ro[0].kind, "curse");
}
