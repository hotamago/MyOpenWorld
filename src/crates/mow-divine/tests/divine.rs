//! Test domain authority, linh hồn và thăng thần (`PE-13`).

use mow_core::{EntityId, EventSeq};
use mow_divine::authority::{DivineError, Domain, DomainAct, God, GodKind, Grant};
use mow_divine::soul::{Ascension, AscensionPath, Soul, SoulError, SoulPolicy, SoulState};

fn than_bao() -> God {
    God {
        who: EntityId(900),
        kind: GodKind::Ascended,
        domains: vec![Domain {
            name: "storm".into(),
            fields: vec!["weather.wind".into(), "weather.rain".into()],
            counters: vec!["calm".into()],
        }],
        energy: 10_000,
        followers: 40_000,
        anchored_regions: vec![1, 2],
    }
}

// ───────────────────── §14.2 · domain authority ─────────────────────

/// **Không có biến thể nào đặt kết quả.**
///
/// Test này đọc chính bề mặt API: nếu ai đó thêm một `SetState` vào
/// [`DomainAct`] thì `INV-22-1` mất hiệu lực cho toàn bộ hệ thần linh, nên nó
/// phải làm test này đỏ.
#[test]
fn khong_co_tac_dong_nao_dat_thang_ket_qua() {
    let cac = [
        DomainAct::Amplify {
            field: "weather.wind".into(),
            region: 1,
            permille: 500,
        },
        DomainAct::Manifest {
            field: "weather.rain".into(),
            region: 1,
            magnitude: 300,
        },
        DomainAct::Oppose {
            rival: EntityId(901),
            region: 1,
        },
    ];
    for a in &cac {
        let j = serde_json::to_string(a).unwrap();
        for cam in ["destroyed", "set_state", "kill", "health"] {
            assert!(!j.contains(cam), "tác động chạm thẳng state: {j}");
        }
    }
}

/// **Thần bão không đặt `city.destroyed = true`** — chỉ đề xuất lên trường.
#[test]
fn than_bao_chi_de_xuat_len_truong_thoi_tiet() {
    let mut t = than_bao();
    let ket = t
        .act(&DomainAct::Manifest {
            field: "weather.wind".into(),
            region: 1,
            magnitude: 800,
        })
        .unwrap();
    let de_xuat = ket.proposal.unwrap();
    assert_eq!(de_xuat.field, "weather.wind");
    assert_eq!(de_xuat.from, EntityId(900));
    // Kết quả cuối vẫn phải đi qua weather → vật liệu → cảnh báo → cư dân.
    assert_eq!(de_xuat.delta, 800);
}

/// **Một thần bão mạnh tới đâu cũng không chạm được độ phì của đất.**
#[test]
fn than_bao_khong_cham_duoc_truong_ngoai_domain() {
    let mut t = than_bao();
    t.energy = u64::MAX;
    let loi = t
        .act(&DomainAct::Amplify {
            field: "soil.fertility".into(),
            region: 1,
            permille: 100,
        })
        .unwrap_err();
    assert!(matches!(loi, DivineError::OutsideDomain { .. }));
}

/// Không có liên kết với vùng thì **không với tới**.
#[test]
fn khong_co_lien_ket_thi_khong_voi_toi() {
    let mut t = than_bao();
    let loi = t
        .act(&DomainAct::Manifest {
            field: "weather.wind".into(),
            region: 99,
            magnitude: 100,
        })
        .unwrap_err();
    assert!(matches!(loi, DivineError::NoAnchor { region: 99 }));
}

/// Can thiệp **tốn năng lượng**, và hết thì thôi.
#[test]
fn can_thiep_ton_nang_luong_va_het_thi_thoi() {
    let mut t = God {
        energy: 400,
        ..than_bao()
    };
    assert!(t
        .act(&DomainAct::Manifest {
            field: "weather.rain".into(),
            region: 1,
            magnitude: 100,
        })
        .is_ok());
    assert_eq!(t.energy, 200);
    assert!(matches!(
        t.act(&DomainAct::Manifest {
            field: "weather.rain".into(),
            region: 1,
            magnitude: 500,
        })
        .unwrap_err(),
        DivineError::NotEnoughEnergy { .. }
    ));
}

/// Can thiệp **luôn có giá xã hội** (`§14.2` mục 5).
#[test]
fn can_thiep_luon_co_gia_xa_hoi() {
    let mut t = than_bao();
    let ket = t
        .act(&DomainAct::Manifest {
            field: "weather.wind".into(),
            region: 1,
            magnitude: 800,
        })
        .unwrap();
    assert!(ket.follower_reaction > 0);
}

/// **Hai thần ngang sức thì triệt tiêu**, không phải một bên "thắng".
#[test]
fn hai_than_ngang_suc_thi_triet_tieu() {
    let a = than_bao();
    let b = than_bao();
    assert_eq!(a.contest(&b, "weather.wind"), 0);

    let manh = God {
        energy: 30_000,
        ..than_bao()
    };
    assert!(manh.contest(&b, "weather.wind") > 0);
    // Domain không chạm tới thì không tham chiến được.
    assert_eq!(a.contest(&b, "soil.fertility"), 0);
}

/// **True God không đi qua đường này** — quyền ở tầng ngoài simulation.
#[test]
fn true_god_khong_di_qua_domain_authority() {
    let mut tg = God {
        kind: GodKind::True,
        ..than_bao()
    };
    assert!(matches!(
        tg.act(&DomainAct::Manifest {
            field: "weather.wind".into(),
            region: 1,
            magnitude: 1
        })
        .unwrap_err(),
        DivineError::TrueGodUsesAnotherPath
    ));
}

/// Ba loại thần khác nhau ở **nằm trong law** và **thu hồi được**.
#[test]
fn ba_loai_than_khac_nhau_o_hai_truc() {
    assert!(GodKind::Ascended.bound_by_law());
    assert!(GodKind::Administrator.bound_by_law());
    assert!(!GodKind::True.bound_by_law());

    assert!(!GodKind::Ascended.revocable());
    assert!(GodKind::Administrator.revocable());
    assert!(!GodKind::True.revocable());
}

/// **Quyền mượn được thì lấy lại được.**
#[test]
fn quyen_cua_administrator_thu_hoi_duoc() {
    let mut g = Grant {
        to: EntityId(5),
        capability: "world.reshape_terrain".into(),
        scope: vec![1, 2],
        revoked: false,
    };
    assert!(g.active_at(1));
    assert!(!g.active_at(9), "ngoài phạm vi thì không dùng được");
    g.revoke();
    assert!(!g.active_at(1));
}

// ───────────────────── §14.3 · linh hồn và thăng thần ─────────────────────

fn linh_hon_co_the() -> Soul {
    Soul {
        id: EntityId(500),
        state: SoulState::Unbound,
        unfulfilled_vows: vec![EventSeq(101), EventSeq(202)],
    }
}

fn co_sieu_hinh() -> SoulPolicy {
    SoulPolicy {
        persists_after_death: true,
        reincarnates: true,
        bindable_to_items: true,
        summonable: true,
        memory_persists: true,
    }
}

/// World duy vật thì **mọi thao tác siêu hình đều bị từ chối**.
#[test]
fn world_duy_vat_tu_choi_moi_thao_tac_sieu_hinh() {
    let p = SoulPolicy::materialist();
    let mut s = linh_hon_co_the();
    assert!(matches!(
        s.bind_to_item(1, &p).unwrap_err(),
        SoulError::PolicyForbids { .. }
    ));
    assert!(matches!(
        s.summon(EntityId(1), 100, &p).unwrap_err(),
        SoulError::PolicyForbids { .. }
    ));
    assert!(matches!(
        s.reincarnate(EntityId(2), &p).unwrap_err(),
        SoulError::PolicyForbids { .. }
    ));
}

/// Neo vào vật tạo ra một vật phẩm có tri giác (`§8.9.4`).
#[test]
fn neo_vao_vat_tao_ra_vat_pham_co_tri_giac() {
    let mut s = linh_hon_co_the();
    s.bind_to_item(4_242, &co_sieu_hinh()).unwrap();
    assert_eq!(s.state, SoulState::BoundToItem { item: 4_242 });
}

/// **Triệu hồi có hạn** — vĩnh viễn là sức mạnh miễn phí.
#[test]
fn trieu_hoi_co_han() {
    let mut s = linh_hon_co_the();
    s.summon(EntityId(9), 1_500, &co_sieu_hinh()).unwrap();
    match s.state {
        SoulState::Summoned { until, .. } => assert_eq!(until, 1_500),
        khac => panic!("{khac:?}"),
    }
}

/// **Lời thề đi theo qua luân hồi** — không thì cách trả nợ rẻ nhất là chết.
#[test]
fn loi_the_di_theo_qua_luan_hoi() {
    let mut s = linh_hon_co_the();
    s.reincarnate(EntityId(777), &co_sieu_hinh()).unwrap();
    assert_eq!(s.unfulfilled_vows, vec![EventSeq(101), EventSeq(202)]);
    assert_eq!(s.id, EntityId(500), "định danh giữ nguyên");
}

/// **Thăng thần không xóa lịch sử cũ** — và không có đường nào để xóa.
#[test]
fn thang_than_khong_xoa_lich_su_cu() {
    let s = linh_hon_co_the();
    let a = Ascension::new(
        &s,
        vec![AscensionPath::AnchoredByFaith],
        EventSeq(9_000),
        1_400,
        58,
    );
    assert!(!a.erases_history());
    assert_eq!(a.who, EntityId(500), "EntityId **không đổi**");
    assert_eq!(a.carried.identity, EntityId(500));
    assert_eq!(
        a.carried.vows,
        vec![EventSeq(101), EventSeq(202)],
        "thành thần không phải cách xù nợ"
    );
    assert_eq!(a.carried.memories, 1_400);
    assert_eq!(a.carried.relationships, 58);
}

/// **Bốn trong năm đường đi từ dưới lên.**
#[test]
fn bon_trong_nam_duong_di_tu_duoi_len() {
    let cac = [
        AscensionPath::SoulCultivation,
        AscensionPath::InheritedDomain,
        AscensionPath::CollectiveRitual,
        AscensionPath::AnchoredByFaith,
        AscensionPath::DivineGrant,
    ];
    assert_eq!(cac.iter().filter(|p| p.bottom_up()).count(), 4);
}

/// Một pantheon toàn thần do người chơi chỉ định thì phân biệt được.
#[test]
fn phan_biet_duoc_than_tu_len_va_than_duoc_chi_dinh() {
    let s = linh_hon_co_the();
    let tu_len = Ascension::new(&s, vec![AscensionPath::CollectiveRitual], EventSeq(1), 0, 0);
    let chi_dinh = Ascension::new(&s, vec![AscensionPath::DivineGrant], EventSeq(1), 0, 0);
    assert!(tu_len.self_made());
    assert!(!chi_dinh.self_made());
}

/// Nhiều đường cùng lúc là hợp lệ.
#[test]
fn thang_than_qua_nhieu_duong_cung_luc() {
    let s = linh_hon_co_the();
    let a = Ascension::new(
        &s,
        vec![
            AscensionPath::SoulCultivation,
            AscensionPath::InheritedDomain,
        ],
        EventSeq(1),
        0,
        0,
    );
    assert_eq!(a.paths.len(), 2);
    assert!(a.self_made());
}
