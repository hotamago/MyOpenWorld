//! Test hiệu ứng.

use mow_core::{Clock, ClockDomain, Tick};
use mow_effect::disease::Stage;
use mow_effect::{
    mitigate, resolve, Compartments, Effect, EffectError, EffectProposal, Infection, Modifier, Op,
    Pathogen, Perceptible, Stacking, Ward,
};
use mow_math::{Fx, Prob, Unit};

fn m(stat: &str, op: Op, v: i64, source: &str, stacking: Stacking) -> Modifier {
    Modifier {
        stat: stat.to_owned(),
        op,
        value: Fx::from_int(v).unwrap(),
        source: source.to_owned(),
        stacking,
    }
}

fn thay_duoc(sign: &str) -> Perceptible {
    Perceptible {
        sense: "sight".to_owned(),
        sign: sign.to_owned(),
        difficulty: Unit::ZERO,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.20 — không bao giờ ghi base stat
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn base_stat_khong_bao_gio_doi() {
    let base = Fx::from_int(10).unwrap();
    let r = resolve(
        base,
        &[m("core.strength", Op::Add, -5, "curse", Stacking::Additive)],
    );
    assert_eq!(r.base, base, "base bị sửa — §22.20 vi phạm");
    assert_eq!(r.value, Fx::from_int(5).unwrap());
}

#[test]
fn go_effect_thi_gia_tri_tro_ve_dung_ke_ca_khi_base_da_doi() {
    // Đây là lý do pipeline tồn tại. Với cách "trừ thẳng vào chỉ số", gỡ lời
    // nguyền sau khi nhân vật lên cấp sẽ cộng lại sai.
    let curse = m("core.strength", Op::Add, -5, "curse", Stacking::Additive);

    let truoc = resolve(Fx::from_int(10).unwrap(), std::slice::from_ref(&curse));
    assert_eq!(truoc.value, Fx::from_int(5).unwrap());

    // Nhân vật lên cấp: base đổi từ 10 lên 20.
    let sau_len_cap = resolve(Fx::from_int(20).unwrap(), &[curse]);
    assert_eq!(sau_len_cap.value, Fx::from_int(15).unwrap());

    // Gỡ lời nguyền.
    let da_go = resolve(Fx::from_int(20).unwrap(), &[]);
    assert_eq!(
        da_go.value,
        Fx::from_int(20).unwrap(),
        "gỡ effect phải trả về đúng base"
    );
}

#[test]
fn moi_gia_tri_suy_ra_bam_duoc_ve_nguon() {
    // `§18.13`: mọi giá trị suy ra phải bấm được về nguồn.
    let r = resolve(
        Fx::from_int(10).unwrap(),
        &[
            m("core.strength", Op::Add, 5, "blessing", Stacking::Additive),
            m("core.strength", Op::Multiply, 2, "rage", Stacking::Additive),
        ],
    );
    assert_eq!(r.steps.len(), 2);
    assert_eq!(r.steps[0].source, "blessing");
    assert_eq!(r.steps[0].before, Fx::from_int(10).unwrap());
    assert_eq!(r.steps[0].after, Fx::from_int(15).unwrap());
    assert_eq!(r.steps[1].source, "rage");
    assert_eq!(r.steps[1].after, Fx::from_int(30).unwrap());
}

#[test]
fn thu_tu_ap_dung_on_dinh_khong_theo_thu_tu_chen() {
    // `+5` rồi `×2` cho 30; `×2` rồi `+5` cho 25. Nếu thứ tự phụ thuộc lịch sử
    // thì hai nhân vật giống hệt nhau sẽ có chỉ số khác nhau.
    let a = m("s", Op::Add, 5, "x", Stacking::Additive);
    let b = m("s", Op::Multiply, 2, "y", Stacking::Additive);
    let base = Fx::from_int(10).unwrap();

    let xuoi = resolve(base, &[a.clone(), b.clone()]);
    let nguoc = resolve(base, &[b, a]);
    assert_eq!(xuoi.value, nguoc.value);
    assert_eq!(
        xuoi.value,
        Fx::from_int(30).unwrap(),
        "Add phải chạy trước Multiply"
    );
}

#[test]
fn hai_modifier_cung_op_pha_hoa_bang_nguon() {
    let base = Fx::from_int(0).unwrap();
    let mk = |dao: bool| {
        let ds = [
            m("s", Op::Set, 1, "aaa", Stacking::Additive),
            m("s", Op::Set, 2, "bbb", Stacking::Additive),
        ];
        let v: Vec<_> = if dao {
            ds.iter().rev().cloned().collect()
        } else {
            ds.to_vec()
        };
        resolve(base, &v).value
    };
    assert_eq!(mk(false), mk(true));
}

// ─────────────────────────────────────────────────────────────────────────────
// Năm chính sách chồng chập
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn additive_cong_don_het() {
    let r = resolve(
        Fx::ZERO,
        &[
            m("s", Op::Add, 3, "a", Stacking::Additive),
            m("s", Op::Add, 3, "b", Stacking::Additive),
            m("s", Op::Add, 3, "c", Stacking::Additive),
        ],
    );
    assert_eq!(r.value, Fx::from_int(9).unwrap());
}

#[test]
fn highest_only_chi_cai_manh_nhat() {
    // Ba nguồn ánh sáng thì sáng bằng cái mạnh nhất.
    let r = resolve(
        Fx::ZERO,
        &[
            m("s", Op::Add, 3, "a", Stacking::HighestOnly),
            m("s", Op::Add, 9, "b", Stacking::HighestOnly),
            m("s", Op::Add, 5, "c", Stacking::HighestOnly),
        ],
    );
    assert_eq!(r.value, Fx::from_int(9).unwrap());
}

#[test]
fn diminishing_returns_ngan_chong_hai_muoi_la_bua_nho() {
    // Cái thứ hai một nửa, cái thứ ba một phần tư. Không cần trần cứng nào.
    let r = resolve(
        Fx::ZERO,
        &[
            m("s", Op::Add, 8, "a", Stacking::DiminishingReturns),
            m("s", Op::Add, 8, "b", Stacking::DiminishingReturns),
            m("s", Op::Add, 8, "c", Stacking::DiminishingReturns),
        ],
    );
    // 8 + 4 + 2 = 14, chứ không phải 24.
    assert_eq!(r.value, Fx::from_int(14).unwrap());
}

#[test]
fn diminishing_returns_hoi_tu_khong_tang_vo_han() {
    let ds: Vec<_> = (0..30)
        .map(|i| {
            m(
                "s",
                Op::Add,
                8,
                &format!("s{i:02}"),
                Stacking::DiminishingReturns,
            )
        })
        .collect();
    let r = resolve(Fx::ZERO, &ds);
    assert!(
        r.value < Fx::from_int(17).unwrap(),
        "30 lá bùa cho {} — chuỗi phải hội tụ dưới 16",
        r.value
    );
}

#[test]
fn replace_va_exclusive_chon_xac_dinh() {
    let r = resolve(
        Fx::ZERO,
        &[
            m("s", Op::Add, 3, "aaa", Stacking::Replace),
            m("s", Op::Add, 7, "zzz", Stacking::Replace),
        ],
    );
    assert_eq!(
        r.value,
        Fx::from_int(7).unwrap(),
        "`Replace` lấy nguồn mới nhất"
    );
}

#[test]
fn chinh_sach_khac_nhau_khong_canh_tranh_nhau() {
    // Một buff cộng và một buff nhân không cạnh tranh.
    let r = resolve(
        Fx::from_int(10).unwrap(),
        &[
            m("s", Op::Add, 5, "a", Stacking::Additive),
            m("s", Op::Multiply, 2, "b", Stacking::HighestOnly),
        ],
    );
    assert_eq!(r.value, Fx::from_int(30).unwrap());
}

#[test]
fn cap_va_floor_ep_vao_khoang() {
    let r = resolve(
        Fx::from_int(100).unwrap(),
        &[m("s", Op::Cap, 50, "limit", Stacking::Additive)],
    );
    assert_eq!(r.value, Fx::from_int(50).unwrap());

    let r2 = resolve(
        Fx::from_int(1).unwrap(),
        &[m("s", Op::Floor, 10, "min", Stacking::Additive)],
    );
    assert_eq!(r2.value, Fx::from_int(10).unwrap());
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.22 — không có effect vô hình
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn effect_khong_khai_bao_perceptible_as_bi_tu_choi() {
    let e = Effect::new("core.curse", "arcane", "witch", vec![], vec![]);
    assert!(matches!(e, Err(EffectError::Imperceptible(_))));
    let msg = e.unwrap_err().to_string();
    assert!(
        msg.contains("chẩn đoán sai"),
        "lỗi phải nói vì sao bất biến này tồn tại: {msg}"
    );
}

#[test]
fn dau_hieu_la_trieu_chung_khong_phai_ten_effect() {
    // Người quan sát thấy "da tái"; kết luận đó là bệnh hay bùa là suy luận của
    // họ, và có thể sai.
    let e = Effect::new(
        "core.curse_of_wasting",
        "arcane",
        "witch",
        vec![],
        vec![thay_duoc("da tái"), thay_duoc("gầy nhanh")],
    )
    .unwrap();

    let thay = e.signs_for(&["sight".to_owned()], Unit::ONE);
    assert_eq!(thay, vec!["da tái", "gầy nhanh"]);
    assert!(
        !thay.iter().any(|s| s.contains("curse")),
        "tên effect rò ra ngoài"
    );
}

#[test]
fn chan_doan_sai_la_ket_qua_hop_le() {
    // Hai nguyên nhân khác nhau cho cùng một triệu chứng. Đây là tính năng.
    let benh = Effect::new(
        "core.fever",
        "disease",
        "pathogen",
        vec![],
        vec![thay_duoc("sốt cao")],
    )
    .unwrap();
    let bua = Effect::new(
        "core.hex",
        "arcane",
        "witch",
        vec![],
        vec![thay_duoc("sốt cao")],
    )
    .unwrap();

    let a = benh.signs_for(&["sight".to_owned()], Unit::ONE);
    let b = bua.signs_for(&["sight".to_owned()], Unit::ONE);
    assert_eq!(
        a, b,
        "hai nguyên nhân phải cho cùng triệu chứng quan sát được"
    );
}

#[test]
fn thieu_giac_quan_thi_khong_thay() {
    let e = Effect::new(
        "core.stink",
        "physical",
        "x",
        vec![],
        vec![Perceptible {
            sense: "smell".to_owned(),
            sign: "mùi hôi".to_owned(),
            difficulty: Unit::ZERO,
        }],
    )
    .unwrap();
    assert!(e.signs_for(&["sight".to_owned()], Unit::ONE).is_empty());
    assert_eq!(
        e.signs_for(&["smell".to_owned()], Unit::ONE),
        vec!["mùi hôi"]
    );
}

#[test]
fn dau_hieu_kho_thi_can_ky_nang() {
    let e = Effect::new(
        "core.subtle",
        "arcane",
        "x",
        vec![],
        vec![Perceptible {
            sense: "sight".to_owned(),
            sign: "đồng tử hơi giãn".to_owned(),
            difficulty: Unit::from_frac(8, 10).unwrap(),
        }],
    )
    .unwrap();
    assert!(e
        .signs_for(&["sight".to_owned()], Unit::from_frac(5, 10).unwrap())
        .is_empty());
    assert_eq!(e.signs_for(&["sight".to_owned()], Unit::ONE).len(), 1);
}

#[test]
fn effect_het_han_theo_mien_dong_ho_da_khai_bao() {
    let e = Effect::new(
        "core.buff",
        "arcane",
        "x",
        vec![],
        vec![thay_duoc("phát sáng")],
    )
    .unwrap()
    .expiring(Tick(100), ClockDomain::Proper);

    let mut c = Clock::synchronous();
    assert!(!e.is_expired(&c));
    c.advance_divine(100).unwrap();
    assert!(e.is_expired(&c));
}

// ─────────────────────────────────────────────────────────────────────────────
// §22.21 — chuỗi giảm thiểu
// ─────────────────────────────────────────────────────────────────────────────

fn de_xuat(category: &str, mag: i64) -> EffectProposal {
    EffectProposal {
        effect: Effect::new(
            "core.harm",
            category,
            "x",
            vec![],
            vec![thay_duoc("vết thương")],
        )
        .unwrap(),
        magnitude: Unit::from_frac(mag, 10).unwrap(),
    }
}

fn ward(id: &str, blocks: &[&str], giam: i64, order: i32) -> Ward {
    Ward {
        id: id.to_owned(),
        blocks: blocks.iter().map(|s| (*s).to_owned()).collect(),
        reduction: Unit::from_frac(giam, 10).unwrap(),
        order,
    }
}

#[test]
fn chuoi_chay_theo_thu_tu_ward_vat_lieu_khang() {
    let r = mitigate(
        &de_xuat("physical", 10),
        &[
            ward("resist", &[], 2, 30),
            ward("armor", &[], 5, 20),
            ward("shield", &[], 5, 10),
        ],
    );
    assert_eq!(
        r.steps.iter().map(|s| s.ward.as_str()).collect::<Vec<_>>(),
        vec!["shield", "armor", "resist"],
        "thứ tự vật lý: lá chắn ngoài, rồi giáp, rồi cơ thể"
    );
}

#[test]
fn giam_theo_ti_le_khong_phai_tru_tuyet_doi() {
    // Trừ tuyệt đối sẽ khiến hai lá bùa yếu chặn được đòn mạnh hơn một lá bùa
    // mạnh, và người chơi sẽ khai thác ngay.
    let hai_yeu = mitigate(
        &de_xuat("physical", 10),
        &[ward("a", &[], 3, 1), ward("b", &[], 3, 2)],
    );
    let mot_manh = mitigate(&de_xuat("physical", 10), &[ward("c", &[], 6, 1)]);
    assert!(
        hai_yeu.magnitude > mot_manh.magnitude,
        "hai lá bùa 30% ({}) không được mạnh bằng một lá 60% ({})",
        hai_yeu.magnitude,
        mot_manh.magnitude
    );
}

#[test]
fn ward_chi_can_dung_loai_no_khai_bao() {
    let r = mitigate(
        &de_xuat("arcane", 10),
        &[ward("armor", &["physical"], 9, 1)],
    );
    assert!(r.steps.is_empty(), "giáp vật lý không được cản phép");
    assert_eq!(r.magnitude, Unit::ONE);
}

#[test]
fn chan_hoan_toan_thi_dung_som() {
    let r = mitigate(
        &de_xuat("arcane", 10),
        &[ward("absolute", &[], 10, 1), ward("thua", &[], 5, 2)],
    );
    assert!(r.blocked);
    assert_eq!(r.steps.len(), 1, "đã chặn hết thì không chạy tiếp");
}

#[test]
fn ward_cung_thu_tu_pha_hoa_bang_id() {
    let mk = |dao: bool| {
        let ds = [ward("aaa", &[], 3, 5), ward("bbb", &[], 5, 5)];
        let v: Vec<_> = if dao {
            ds.iter().rev().cloned().collect()
        } else {
            ds.to_vec()
        };
        mitigate(&de_xuat("physical", 10), &v).magnitude
    };
    assert_eq!(mk(false), mk(true));
}

// ─────────────────────────────────────────────────────────────────────────────
// §9.8.5 — bệnh và dịch
// ─────────────────────────────────────────────────────────────────────────────

fn cum() -> Pathogen {
    Pathogen {
        id: "core.flu".to_owned(),
        transmission_per_contact: Prob::from_ppm(80_000).unwrap(),
        incubation_ticks: 2_000,
        infectious_ticks: 5_000,
        lethality: Prob::from_ppm(20_000).unwrap(),
        symptom_effect: "core.fever".to_owned(),
    }
}

#[test]
fn ti_le_lay_hiem_khong_bi_lam_tron_ve_0() {
    // Cùng bài học với tỉ lệ đột biến: Q16.16 sẽ nuốt một tỉ lệ như thế này.
    let hiem = Prob::from_sci(5, 7).unwrap(); // 5e-7
    assert!(hiem.raw() > 0);
    assert_eq!(Fx::from_frac(5, 10_000_000).unwrap(), Fx::ZERO);
}

#[test]
fn u_benh_thi_chua_lay_duoc() {
    // Đây là giai đoạn khiến cách ly luôn muộn: không triệu chứng, vẫn đi lại.
    let p = cum();
    let i = Infection {
        pathogen: p.id.clone(),
        stage: Stage::Exposed,
        since_tick: 0,
        infected_by: Some(7),
    };
    assert!(!i.is_infectious_at(&p, 1_999));
    assert!(i.is_infectious_at(&p, 2_000));
}

#[test]
fn giai_doan_la_ham_cua_thoi_gian_nen_lod_khong_lam_lech() {
    let p = cum();
    let i = Infection {
        pathogen: p.id.clone(),
        stage: Stage::Infectious,
        since_tick: 0,
        infected_by: None,
    };
    // Không ai đụng vào suốt 5000 tick vì thực thể ở mức `Far`.
    assert_eq!(i.stage_at(&p, 5_000), Stage::Recovered);
}

#[test]
fn truy_nguoc_duoc_chuoi_lay() {
    let i = Infection {
        pathogen: "core.flu".to_owned(),
        stage: Stage::Exposed,
        since_tick: 100,
        infected_by: Some(42),
    };
    assert_eq!(i.infected_by, Some(42));
}

#[test]
fn chuyen_lod_bao_toan_dan_so() {
    // `§22.14`.
    let ca_the = vec![
        Stage::Susceptible,
        Stage::Susceptible,
        Stage::Exposed,
        Stage::Infectious,
        Stage::Recovered,
        Stage::Dead,
    ];
    let c = Compartments::from_individuals(&ca_the);
    assert_eq!(c.total(), 6, "gộp lên mức khu định cư làm mất người");
    assert_eq!(c.alive(), 5);
}

#[test]
fn dich_bung_roi_tat_o_muc_khu_dinh_cu() {
    let p = cum();
    let mut c = Compartments {
        susceptible: 999,
        exposed: 0,
        infectious: 1,
        ..Default::default()
    };
    let dan_so_ban_dau = c.total();

    let mut dinh = 0;
    for _ in 0..200_000 {
        c = c.step(&p, 3);
        dinh = dinh.max(c.infectious);
        if c.is_over() {
            break;
        }
    }

    assert!(c.is_over(), "dịch không bao giờ tắt");
    assert!(dinh > 10, "dịch không bao giờ bùng: đỉnh chỉ {dinh} ca");
    assert_eq!(c.total(), dan_so_ban_dau, "dân số không được bảo toàn");
    assert!(c.dead > 0, "một dịch có tỉ lệ tử vong 2% mà không ai chết");
}

#[test]
fn dich_tat_ngay_o_lang_nho_van_bao_toan_dan_so() {
    // Với dân số nhỏ, `n / q * p` và `n * p / q` cho kết quả khác nhau, và cái
    // sau làm dịch tắt ngóm trong một ngôi làng ba mươi người.
    let p = cum();
    let mut c = Compartments {
        susceptible: 29,
        exposed: 0,
        infectious: 1,
        ..Default::default()
    };
    for _ in 0..100_000 {
        c = c.step(&p, 5);
        assert_eq!(c.total(), 30, "dân số lệch giữa chừng");
        if c.is_over() {
            break;
        }
    }
    assert!(
        c.recovered + c.dead > 1,
        "dịch không lan được trong làng nhỏ"
    );
}
