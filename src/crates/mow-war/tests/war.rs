//! Test chiến tranh (`PD-23`, `§12.4`).

use mow_core::{EntityId, Tick};
use mow_war::{battle, Army, Campaign, Enforcement, Terrain, Treaty};

fn quan(id: &str, troops: u32) -> Army {
    Army {
        id: id.into(),
        troops,
        morale: 700,
        command: 700,
        equipment: 700,
        supplies: 10_000,
        consumption_per_tick: 100,
        disease: 0,
    }
}

/// **Thắng bại không do tổng điểm**: quân đông gấp ba vẫn thua nếu morale bằng 0.
#[test]
fn quan_dong_gap_ba_van_thua_neu_morale_bang_khong() {
    let dong_ma_tan = Army {
        troops: 30_000,
        morale: 0,
        ..quan("horde", 30_000)
    };
    let it_ma_vung = quan("legion", 10_000);

    let ra = battle(&dong_ma_tan, &it_ma_vung, Terrain::Open);
    assert_eq!(ra.winner, "legion");
}

/// Nhân chứ không cộng: **bất kỳ thừa số nào bằng 0 làm cả tích bằng 0**.
#[test]
fn cac_yeu_to_nhan_voi_nhau_khong_cong_vao() {
    let khong_trang_bi = Army {
        equipment: 0,
        ..quan("a", 10_000)
    };
    assert_eq!(khong_trang_bi.effective_strength(1_000), 0);

    let khong_chi_huy = Army {
        command: 0,
        ..quan("a", 10_000)
    };
    assert_eq!(khong_chi_huy.effective_strength(1_000), 0);
}

/// **Địa hình quyết định**: cùng hai đạo quân, đổi chỗ đánh là đổi kết quả.
#[test]
fn dia_hinh_doi_ket_qua_cua_cung_hai_dao_quan() {
    let cong = quan("attacker", 14_000);
    let thu = quan("defender", 10_000);

    assert_eq!(battle(&cong, &thu, Terrain::Open).winner, "attacker");
    assert_eq!(
        battle(&cong, &thu, Terrain::Fortified).winner,
        "defender",
        "đánh vào thành lũy mà vẫn thắng bằng quân số"
    );
}

/// Hòa thì **bên phòng thủ giữ được đất** — đó là định nghĩa của phòng thủ.
#[test]
fn hoa_thi_ben_phong_thu_giu_duoc_dat() {
    let a = quan("attacker", 10_000);
    let b = quan("defender", 10_000);
    assert_eq!(battle(&a, &b, Terrain::Open).winner, "defender");
}

/// **Bệnh tật giết lính** — bỏ nó đi thì mọi chiến dịch dài đều khả thi.
#[test]
fn benh_tat_lam_yeu_dao_quan() {
    let khoe = quan("a", 10_000);
    let dich_ta = Army {
        disease: 800,
        ..quan("a", 10_000)
    };
    assert!(dich_ta.effective_strength(1_000) < khoe.effective_strength(1_000) / 3);
}

/// **Một trận thắng sạch sẽ không tồn tại**: bên thắng cũng mất quân.
///
/// Và bên thua mất một **tỉ lệ** lớn hơn — không nhất thiết một số tuyệt đối lớn
/// hơn. Với quân số 2:1, hai bên mất xấp xỉ bằng nhau về số người trong khi bên
/// nhỏ mất gấp đôi về tỉ lệ, và đó mới là thứ quyết định ai còn đứng được.
#[test]
fn ben_thang_cung_mat_quan_nhung_ben_thua_mat_ti_le_lon_hon() {
    let cong = quan("a", 20_000);
    let thu = quan("b", 10_000);
    let ra = battle(&cong, &thu, Terrain::Open);

    assert_eq!(ra.winner, "a");
    assert!(ra.attacker_losses > 0, "thắng mà không mất ai");

    let ti_le_cong = ra.attacker_losses * 1_000 / cong.troops;
    let ti_le_thu = ra.defender_losses * 1_000 / thu.troops;
    assert!(
        ti_le_thu > ti_le_cong,
        "bên thua phải mất tỉ lệ lớn hơn: {ti_le_thu} vs {ti_le_cong}"
    );
}

/// Trận đấu **giải thích được**: "sao quân tôi đông gấp đôi mà vẫn thua".
#[test]
fn tran_danh_giai_thich_duoc() {
    let ra = battle(&quan("a", 20_000), &quan("b", 10_000), Terrain::Fortified);
    assert!(!ra.factors.is_empty());
    for can in ["morale", "địa hình", "sức mạnh thực tế"] {
        assert!(
            ra.factors.iter().any(|(n, _)| n.contains(can)),
            "thiếu phần `{can}`"
        );
    }
}

/// Trận đánh **xác định**.
#[test]
fn tran_danh_xac_dinh() {
    let a = battle(&quan("a", 12_000), &quan("b", 10_000), Terrain::River);
    let b = battle(&quan("a", 12_000), &quan("b", 10_000), Terrain::River);
    assert_eq!(a, b);
}

// ───────────────────────── hậu cần ─────────────────────────

fn chien_dich(supply_open: bool) -> Campaign {
    Campaign {
        army: Army {
            supplies: 500,
            ..quan("besieger", 10_000)
        },
        supply_line_open: supply_open,
        resupply_per_tick: 100,
    }
}

/// **Đạo quân bị cắt đường tiếp tế tự tan** mà không cần đánh.
///
/// Đó là cách phần lớn các cuộc vây hãm trong lịch sử kết thúc.
#[test]
fn cat_duong_tiep_te_thi_dao_quan_tu_tan() {
    let mut c = chien_dich(false);
    let mut tan_o_tick = None;
    for t in 0..200 {
        let r = c.step();
        if r.broke {
            tan_o_tick = Some(t);
            break;
        }
    }
    assert!(
        tan_o_tick.is_some(),
        "cắt tiếp tế mà đạo quân vẫn đứng nguyên sau 200 tick"
    );
}

/// Còn đường tiếp tế thì trụ được.
#[test]
fn con_duong_tiep_te_thi_tru_duoc() {
    let mut c = chien_dich(true);
    for _ in 0..200 {
        let r = c.step();
        assert!(!r.broke, "tiếp tế đủ mà vẫn tan");
    }
    assert!(c.army.morale > 0);
}

/// **Đói làm bệnh nặng hơn** — thứ tự trong `step` có ý nghĩa.
#[test]
fn doi_lam_benh_nang_hon() {
    let mut c = chien_dich(false);
    let benh_dau = c.army.disease;
    for _ in 0..20 {
        c.step();
    }
    assert!(c.army.disease > benh_dau, "hết lương mà mức bệnh không đổi");
}

/// Morale tụt nhanh hơn quân số **khi đã thật sự hết lương**.
///
/// Năm tick đầu chưa đói — kho còn 500 khẩu phần cho mức tiêu 100/tick — nên
/// morale còn **tăng**. Bài này phải chạy qua khỏi mốc đó, nếu không nó đang đo
/// một đạo quân đang ăn no.
#[test]
fn morale_tut_nhanh_hon_quan_so_khi_het_luong() {
    let mut c = chien_dich(false);
    // Ăn hết kho trước.
    for _ in 0..5 {
        c.step();
    }
    assert_eq!(
        c.army.supplies, 0,
        "phải hết lương thì bài này mới có nghĩa"
    );

    let quan_dau = c.army.troops;
    let morale_dau = c.army.morale;
    for _ in 0..10 {
        c.step();
    }

    let mat_quan = quan_dau.saturating_sub(c.army.troops) * 1_000 / quan_dau.max(1);
    let mat_morale =
        u32::from(morale_dau.saturating_sub(c.army.morale)) * 1_000 / u32::from(morale_dau.max(1));
    assert!(
        mat_morale > mat_quan,
        "morale mất {mat_morale}‰, quân mất {mat_quan}‰"
    );
}

/// Còn ăn được bao nhiêu tick nữa — con số mà một tướng cần biết.
#[test]
fn tinh_duoc_con_an_duoc_bao_nhieu_tick() {
    let a = Army {
        supplies: 1_000,
        consumption_per_tick: 100,
        ..quan("a", 10_000)
    };
    assert_eq!(a.supply_ticks(), 10);
}

// ───────────────────────── hiệp ước ─────────────────────────

/// **Một biến `at_war = false` là chưa đủ**: hiệp ước không có cơ chế thực thi
/// nào thì bằng 0.
#[test]
fn hiep_uoc_khong_co_co_che_thuc_thi_thi_bang_khong() {
    let to_giay = Treaty {
        id: "treaty.paper".into(),
        parties: vec!["a".into(), "b".into()],
        signed: Tick(0),
        enforcement: vec![],
    };
    assert_eq!(to_giay.binding_strength(), 0);
    assert!(
        to_giay.worth_breaking(1),
        "một tờ giấy phải bị phá ngay khi có lợi dù nhỏ"
    );
}

/// Có cơ chế thì giữ được — và mỗi loại cơ chế răn đe khác nhau.
#[test]
fn moi_co_che_thuc_thi_ran_de_khac_nhau() {
    let co_con_tin = Treaty {
        id: "t".into(),
        parties: vec![],
        signed: Tick(0),
        enforcement: vec![Enforcement::Hostage { who: EntityId(1) }],
    };
    let co_giam_sat = Treaty {
        enforcement: vec![Enforcement::Inspection { detection: 400 }],
        ..co_con_tin.clone()
    };

    assert!(co_con_tin.binding_strength() > co_giam_sat.binding_strength());
    assert!(co_con_tin.binding_strength() > 0);
}

/// Nhiều cơ chế **cộng dồn**: một hòa ước có con tin, thương mại và bảo chứng
/// bền hơn hẳn.
#[test]
fn nhieu_co_che_cong_don_thanh_hoa_binh_ben() {
    let ben = Treaty {
        id: "t".into(),
        parties: vec![],
        signed: Tick(0),
        enforcement: vec![
            Enforcement::Hostage { who: EntityId(1) },
            Enforcement::Trade {
                value_per_period: 3_000,
            },
            Enforcement::Guarantor {
                who: EntityId(9),
                strength: 5_000,
            },
        ],
    };
    assert_eq!(ben.binding_strength(), 1_000);
    assert!(!ben.worth_breaking(900));
}

/// Đủ lợi thì vẫn phá — và đó là **đúng**, không phải lỗ hổng.
#[test]
fn du_loi_thi_van_pha_va_do_la_dung() {
    let vua_phai = Treaty {
        id: "t".into(),
        parties: vec![],
        signed: Tick(0),
        enforcement: vec![Enforcement::Inspection { detection: 600 }],
    };
    assert!(!vua_phai.worth_breaking(100));
    assert!(vua_phai.worth_breaking(10_000));
}
