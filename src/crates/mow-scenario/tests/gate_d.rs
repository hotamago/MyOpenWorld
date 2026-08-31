//! Cổng Giai đoạn D (`plan.md §P9`, `PD-GATE`).
//!
//! Ba điều kiện, và cả ba đều là **truy ngược được**:
//!
//! > 1. Một bản án truy được về hành vi / nhân chứng / chứng cứ.
//! > 2. Lạm phát có nguyên nhân truy được.
//! > 3. Audit view chỉ đúng storylet đã kích hoạt.
//!
//! Điểm chung của ba điều kiện là chúng không hỏi *"hệ thống có chạy không"* mà
//! hỏi *"khi nó chạy xong, người chơi có hiểu vì sao không"*. Một hệ thống chạy
//! đúng mà không giải thích được thì trượt cổng này — và đó là chủ đích.

use mow_core::{EntityId, Tick};
use mow_director::storylet::{Director, Precondition, Storylet, WorldFacts};
use mow_econ::money::{EconomyProfile, Faucet, MonetaryStage, MoneyDiagnosis, Sink};
use mow_law::crime::Witness;
use mow_law::norms::{
    judge, Deed, Enforcement, LegalOrder, NormSet, ProofMode, ProofRequirement, Rule, Sanction,
    SanctionKind, Scope,
};
use mow_law::trial::{try_case, Evidence, Procedure, TrialContext};

// ─────────────────────────── Điều kiện 1 ───────────────────────────

/// **Một bản án truy được về hành vi, nhân chứng và chứng cứ.**
///
/// Bài này đi hết chuỗi `§12.5.2`: hành vi → cáo buộc theo bộ luật đang hiệu lực
/// → chứng cứ → phán quyết. Mỗi mắt phải chỉ ngược về mắt trước bằng dữ liệu,
/// không bằng một câu tường thuật.
#[test]
fn gate_d1_ban_an_truy_duoc_ve_hanh_vi_nhan_chung_chung_cu() {
    // ── Bộ luật đang hiệu lực, có version ──
    let mut co_luat = LegalOrder::new();
    co_luat.add(NormSet {
        id: "veskar.criminal_code".into(),
        version: 3,
        precedence: 0,
        scope: Scope {
            jurisdiction: "organization:veskar".into(),
            territorial: true,
            districts: vec!["docks".into()],
            members: vec![],
        },
        rules: vec![Rule {
            act: "theft".into(),
            value_above: Some(50),
            sanction: Sanction {
                kind: SanctionKind::Corporal,
                severity: 400,
            },
            proof_required: vec![ProofRequirement::WitnessCount(2)],
            proof_mode: ProofMode::AnyOf,
            enforced_against: vec![],
        }],
        enforcement: Enforcement::default(),
    });

    // ── Hành vi ──
    let hanh_vi = Deed {
        actor: EntityId(1),
        act: "theft".into(),
        value: 300,
        district: "docks".into(),
        actor_class: "commoner".into(),
        actor_groups: vec![],
    };

    // ── Cáo buộc, mang theo version luật LÚC HÀNH VI ──
    let cac = judge(&co_luat, &hanh_vi);
    assert_eq!(cac.len(), 1, "phải có đúng một cáo buộc");
    let cao_buoc = &cac[0];
    assert_eq!(cao_buoc.norm_set, "veskar.criminal_code");
    assert_eq!(
        cao_buoc.norm_set_version, 3,
        "bản án phải truy về đúng phiên bản luật lúc hành vi xảy ra"
    );
    assert_eq!(cao_buoc.act, hanh_vi.act, "cáo buộc phải trỏ về hành vi");

    // ── Nhân chứng, mỗi người là một belief có động cơ ──
    let nhan_chung: Vec<Evidence> = (10..12)
        .map(|i| {
            Evidence::Testimony(Witness {
                who: EntityId(i),
                believes_actor: Some(EntityId(1)),
                confidence: 800,
                motive_to_testify: 500,
            })
        })
        .collect();

    // ── Phán quyết ──
    let phan_quyet = try_case(
        EntityId(1),
        cao_buoc,
        &nhan_chung,
        Procedure::Evidentiary,
        Tick(1_000),
        &TrialContext::default(),
    );

    assert!(phan_quyet.guilty);
    assert_eq!(
        phan_quyet.evidence_accepted, 2,
        "bản án phải nói nó dựa trên bao nhiêu chứng cứ"
    );
    assert!(
        phan_quyet.reasons.iter().any(|r| r.contains("veskar")),
        "lý do phải trỏ về bộ luật: {:?}",
        phan_quyet.reasons
    );

    // ── Và truy được cả về từng nhân chứng ──
    for e in &nhan_chung {
        if let Evidence::Testimony(w) = e {
            assert!(w.will_testify());
            assert!(w.is_truthful(EntityId(1)));
        }
    }
}

/// Mặt sau: **chứng cứ không đủ thì không có bản án**, và lý do nói rõ.
///
/// Không có bài này thì `gate_d1` có thể xanh với một tòa kết tội mọi người.
#[test]
fn gate_d1_nguoc_chung_cu_khong_du_thi_khong_ket_toi() {
    let mut co_luat = LegalOrder::new();
    co_luat.add(NormSet {
        id: "veskar.criminal_code".into(),
        version: 3,
        precedence: 0,
        scope: Scope {
            jurisdiction: "organization:veskar".into(),
            territorial: true,
            districts: vec!["docks".into()],
            members: vec![],
        },
        rules: vec![Rule {
            act: "theft".into(),
            value_above: None,
            sanction: Sanction {
                kind: SanctionKind::Corporal,
                severity: 400,
            },
            proof_required: vec![ProofRequirement::WitnessCount(2)],
            proof_mode: ProofMode::AnyOf,
            enforced_against: vec![],
        }],
        enforcement: Enforcement::default(),
    });
    let hanh_vi = Deed {
        actor: EntityId(1),
        act: "theft".into(),
        value: 300,
        district: "docks".into(),
        actor_class: "commoner".into(),
        actor_groups: vec![],
    };
    let cao_buoc = &judge(&co_luat, &hanh_vi)[0];

    // Chỉ một nhân chứng, và người đó sợ nên không dám khai.
    let so_hai = vec![Evidence::Testimony(Witness {
        who: EntityId(10),
        believes_actor: Some(EntityId(1)),
        confidence: 900,
        motive_to_testify: -500,
    })];

    let v = try_case(
        EntityId(1),
        cao_buoc,
        &so_hai,
        Procedure::Evidentiary,
        Tick(1_000),
        &TrialContext::default(),
    );
    assert!(!v.guilty);
    assert_eq!(v.evidence_accepted, 0, "lời khai không được đưa ra");
    assert!(v.reasons.iter().any(|r| r.contains("không đủ")));
}

// ─────────────────────────── Điều kiện 2 ───────────────────────────

/// **Lạm phát có nguyên nhân truy được.**
///
/// Không phải "giá tăng 3%". Phải chỉ ra **cái vòi nào** đang bơm quá tay.
#[test]
fn gate_d2_lam_phat_co_nguyen_nhan_truy_duoc() {
    let nen_kinh_te = EconomyProfile {
        stage: MonetaryStage::Coinage,
        faucets: vec![
            Faucet {
                id: "state_minting".into(),
                rate: 1_200,
            },
            Faucet {
                id: "mining".into(),
                rate: 150,
            },
            Faucet {
                id: "foreign_trade".into(),
                rate: 80,
            },
        ],
        sinks: vec![
            Sink {
                id: "wear".into(),
                rate: 300,
                physical: true,
                voluntary: false,
            },
            Sink {
                id: "temple_building".into(),
                rate: 200,
                physical: true,
                voluntary: true,
            },
        ],
        money_supply: 100_000,
        goods_supply: 4_000,
    };

    match nen_kinh_te.audit() {
        MoneyDiagnosis::Inflation { cause, surplus } => {
            assert_eq!(surplus, 1_430 - 500);
            assert!(
                cause.contains("state_minting"),
                "phải chỉ đúng vòi lớn nhất, không phải một câu chung chung: {cause}"
            );
        }
        khac => panic!("phải chẩn đoán ra lạm phát, nhận được {khac:?}"),
    }

    // Và mức giá là một tỉ số quan sát được, không phải một hệ số bị chỉnh.
    assert_eq!(nen_kinh_te.price_level(), 100_000 * 1_000 / 4_000);
}

/// Giảm phát cũng phải truy được, và **kiểu giảm phát đặc trưng** phải gọi đúng tên.
#[test]
fn gate_d2_giam_phat_vi_thieu_cong_vat_chat_cung_truy_duoc() {
    let p = EconomyProfile {
        stage: MonetaryStage::Coinage,
        faucets: vec![Faucet {
            id: "mining".into(),
            rate: 50,
        }],
        sinks: vec![Sink {
            id: "tax".into(),
            rate: 800,
            physical: false,
            voluntary: false,
        }],
        money_supply: 1_000,
        goods_supply: 1_000,
    };
    match p.audit() {
        MoneyDiagnosis::Deflation { cause, .. } => {
            assert!(cause.contains("cống vật chất"), "{cause}");
        }
        khac => panic!("phải chẩn đoán ra giảm phát, nhận được {khac:?}"),
    }
}

// ─────────────────────────── Điều kiện 3 ───────────────────────────

fn kho_storylet() -> Vec<Storylet> {
    vec![
        Storylet {
            id: "storylet.mine_flooding".into(),
            preconditions: vec![Precondition::InfrastructureExists {
                kind: "mine".into(),
            }],
            base_salience: 600,
            boosts: vec![],
            perturbation: vec![],
            budget_cost: 2,
            cooldown: 900,
            provenance: "core".into(),
        },
        Storylet {
            id: "storylet.volcano".into(),
            preconditions: vec![Precondition::InfrastructureExists {
                kind: "volcano".into(),
            }],
            base_salience: 900,
            boosts: vec![],
            perturbation: vec![],
            budget_cost: 2,
            cooldown: 900,
            provenance: "core".into(),
        },
    ]
}

/// **Audit view chỉ đúng storylet đã kích hoạt** — và đúng cái không.
///
/// Cả hai vế đều cần. Một audit chỉ liệt kê cái đã nổ không trả lời được câu hỏi
/// hay gặp nhất: *"sao chuyện kia không xảy ra?"*
#[test]
fn gate_d3_audit_chi_dung_storylet_da_kich_hoat() {
    let w = WorldFacts {
        infrastructure: vec!["mine".into()],
        pressures: vec![],
        last_fired: vec![],
        flags: vec![],
        now: 10_000,
        player_focus: None,
    };

    let audit = Director { budget: 10 }.select(&kho_storylet(), &w);
    assert_eq!(audit.len(), 2, "audit phải nói về mọi storylet trong kho");

    let da_no: Vec<&str> = audit
        .iter()
        .filter(|a| a.fired)
        .map(|a| a.storylet.as_str())
        .collect();
    assert_eq!(da_no, vec!["storylet.mine_flooding"]);

    // Cái salience cao hơn **không** nổ, và audit nói rõ vì sao.
    let nui_lua = audit
        .iter()
        .find(|a| a.storylet == "storylet.volcano")
        .unwrap();
    assert!(!nui_lua.fired);
    assert_eq!(nui_lua.salience, 900, "salience cao hơn mà vẫn không nổ");
    assert_eq!(
        nui_lua.rejected_because.as_deref(),
        Some("thế giới chưa có nguyên nhân")
    );
    // Và chỉ đúng vị từ nào không thỏa.
    assert!(nui_lua
        .preconditions
        .iter()
        .any(|(mo_ta, ok)| { !ok && mo_ta.contains("volcano") }));
}

/// Storylet đã nổ phải nói được **vì precondition nào** và **salience bao nhiêu**.
#[test]
fn gate_d3_storylet_da_no_noi_duoc_vi_sao_va_bao_nhieu() {
    let w = WorldFacts {
        infrastructure: vec!["mine".into()],
        pressures: vec![],
        last_fired: vec![],
        flags: vec![],
        now: 10_000,
        player_focus: None,
    };
    let audit = Director { budget: 10 }.select(&kho_storylet(), &w);
    let no = audit.iter().find(|a| a.fired).unwrap();

    assert!(no.rejected_because.is_none());
    assert_eq!(no.salience, 600);
    assert!(!no.salience_parts.is_empty(), "salience phải có phân rã");
    assert!(
        no.preconditions.iter().all(|(_, ok)| *ok),
        "storylet đã nổ thì mọi vị từ phải thỏa"
    );
}

/// **Không có storylet nào nổ khi thế giới chưa có nguyên nhân** — và audit vẫn
/// nói đầy đủ.
#[test]
fn gate_d3_the_gioi_trong_rong_thi_khong_gi_no_nhung_audit_van_giai_thich() {
    let trong = WorldFacts {
        infrastructure: vec![],
        pressures: vec![],
        last_fired: vec![],
        flags: vec![],
        now: 10_000,
        player_focus: None,
    };
    let audit = Director { budget: 100 }.select(&kho_storylet(), &trong);
    assert!(audit.iter().all(|a| !a.fired));
    assert!(audit.iter().all(|a| a.rejected_because.is_some()));
}
