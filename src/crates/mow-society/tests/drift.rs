//! Test chống trôi persona (`PC-13`, `§20.11`).

use mow_society::drift::{Act, ActiveCause, DriftAuditor, DriftReport, Verdict};
use mow_society::personality::{CauseKind, CauseRef, Personality, TraitField, Traits};

fn nguoi_keo_kiet() -> Personality {
    Personality::from_traits(Traits {
        openness: 500,
        conscientiousness: 500,
        extraversion: 500,
        // Rất khó chịu, rất ít chia sẻ.
        agreeableness: 120,
        neuroticism: 500,
    })
}

/// Hành động của một người hào phóng.
fn hao_phong(tick: u64) -> Act {
    Act {
        at_tick: tick,
        field: TraitField::Agreeableness,
        implied: 850,
    }
}

fn nguyen_nhan(seq: u64, kind: CauseKind) -> CauseRef {
    CauseRef {
        event_seq: seq,
        kind,
    }
}

/// Lệch không có nguyên nhân là **bug**, và phải được báo. Đây là toàn bộ lý do
/// auditor tồn tại: không có nó, một nhân vật dần trở thành người khác mà không
/// có một dòng log nào.
#[test]
fn lech_khong_nguyen_nhan_la_bug_va_phai_bao() {
    let p = nguoi_keo_kiet();
    let acts: Vec<Act> = (0..20).map(|i| hao_phong(1_000 + i * 10)).collect();

    let ra = DriftAuditor::default().audit(&p, &acts, &[]);
    assert_eq!(ra.len(), 1, "phải phát hiện đúng một lệch");
    assert_eq!(ra[0].verdict, Verdict::Drift);
    assert_eq!(ra[0].field, TraitField::Agreeableness);
    assert!(ra[0].gap() > 300);

    let bc = DriftReport { findings: ra };
    assert!(!bc.is_clean(), "trôi không được coi là sạch");
    assert_eq!(bc.to_report().len(), 1);
}

/// Cùng hành vi đó, nhưng có một bùa điều khiển tâm trí đang tác động, là **cốt
/// truyện** — và cốt truyện thì không phải lỗi cần sửa.
#[test]
fn lech_co_nguyen_nhan_la_cot_truyen_khong_phai_loi() {
    let p = nguoi_keo_kiet();
    let acts: Vec<Act> = (0..20).map(|i| hao_phong(1_000 + i * 10)).collect();
    let causes = [ActiveCause {
        from_tick: 900,
        to_tick: 1_500,
        cause: nguyen_nhan(42, CauseKind::MindControl),
    }];

    let ra = DriftAuditor::default().audit(&p, &acts, &causes);
    assert_eq!(ra.len(), 1);
    assert_eq!(
        ra[0].verdict,
        Verdict::Story(nguyen_nhan(42, CauseKind::MindControl))
    );

    let bc = DriftReport { findings: ra };
    assert!(bc.is_clean(), "có nguyên nhân thì không phải thứ cần báo");
    assert_eq!(bc.story_beats().len(), 1);
    assert_eq!(bc.story_beats()[0].1, CauseKind::MindControl);
}

/// Một sang chấn đã đổi tính cách qua đúng đường cũng giải thích được hành vi —
/// không cần một `ActiveCause` riêng.
#[test]
fn tinh_cach_da_doi_qua_dung_duong_thi_giai_thich_duoc_hanh_vi() {
    let mut p = nguoi_keo_kiet();
    p.apply_change(
        950,
        TraitField::Agreeableness,
        400,
        nguyen_nhan(7, CauseKind::Conversion),
    );
    // Vẫn còn lệch: 520 so với hành vi 850.
    let acts: Vec<Act> = (0..20).map(|i| hao_phong(1_000 + i * 10)).collect();

    let ra = DriftAuditor::default().audit(&p, &acts, &[]);
    assert_eq!(ra.len(), 1);
    assert_eq!(
        ra[0].verdict,
        Verdict::Story(nguyen_nhan(7, CauseKind::Conversion))
    );
}

/// **Ghi tắt là phát hiện nặng nhất.** Nhân vật vẫn nhất quán với chính nó ở
/// mọi thời điểm, nên không có gì trông sai — chỉ có lịch sử là không cộng lại
/// thành hiện tại.
///
/// Ở đây mô phỏng một bản save đã bị sửa: `traits` khác `birth` mà `history`
/// rỗng. Trong mã bình thường điều này không viết ra được, vì `traits` là
/// trường riêng và `apply_change` bắt buộc có `CauseRef`.
#[test]
fn ghi_tat_bi_bat_du_hanh_vi_van_nhat_quan() {
    let goc = nguoi_keo_kiet();
    let mut json: serde_json::Value = serde_json::to_value(&goc).unwrap();
    json["traits"]["agreeableness"] = serde_json::json!(900);
    let sua: Personality = serde_json::from_value(json).unwrap();

    assert!(
        !sua.history_explains_current(),
        "lịch sử rỗng không giải thích được đặc điểm đã đổi"
    );

    // Hành vi **khớp hoàn toàn** với đặc điểm hiện tại — không có gì trông sai.
    let acts: Vec<Act> = (0..20)
        .map(|i| Act {
            at_tick: 1_000 + i * 10,
            field: TraitField::Agreeableness,
            implied: 900,
        })
        .collect();

    let ra = DriftAuditor::default().audit(&sua, &acts, &[]);
    assert_eq!(ra.len(), 1, "vẫn phải bị bắt");
    assert_eq!(ra[0].verdict, Verdict::Tampered);
    assert_eq!(ra[0].expected, 120, "phải chỉ ra đặc điểm lúc sinh");
    assert_eq!(ra[0].observed, 900, "và đặc điểm hiện tại");
}

/// Một nhân vật thay đổi qua đúng đường thì lịch sử luôn cộng lại đúng.
#[test]
fn doi_qua_dung_duong_thi_lich_su_luon_cong_lai_dung() {
    let mut p = nguoi_keo_kiet();
    for i in 0..25 {
        p.apply_change(
            i * 100,
            TraitField::Neuroticism,
            if i % 2 == 0 { 40 } else { -25 },
            nguyen_nhan(i, CauseKind::Trauma),
        );
        assert!(p.history_explains_current(), "lệch ở bước {i}");
    }
    assert_eq!(p.history().len(), 25);
}

/// Chặn dưới phải giữ được ở cả hai đầu, và lịch sử vẫn phải khớp sau khi chặn.
/// Nếu `apply_change` chặn còn `history_explains_current` thì không, mọi nhân
/// vật chạm trần sẽ bị báo là ghi tắt.
#[test]
fn chan_tran_va_san_khong_lam_lech_lich_su() {
    let mut p = nguoi_keo_kiet();
    for i in 0..40 {
        p.apply_change(
            i,
            TraitField::Agreeableness,
            -500,
            nguyen_nhan(i, CauseKind::Aging),
        );
        assert!(p.history_explains_current(), "lệch khi chạm sàn, bước {i}");
    }
    assert_eq!(p.traits().agreeableness, 0);

    for i in 0..40 {
        p.apply_change(
            100 + i,
            TraitField::Agreeableness,
            500,
            nguyen_nhan(i, CauseKind::Oath),
        );
        assert!(p.history_explains_current(), "lệch khi chạm trần, bước {i}");
    }
    assert_eq!(p.traits().agreeableness, 1000);
}

/// **Một hành động không kết luận được gì.** Một người hào phóng vẫn có thể keo
/// kiệt một lần; kết luận từ một mẫu là biến mọi nhân vật có chiều sâu thành
/// một báo cáo lỗi.
#[test]
fn mot_hanh_dong_le_khong_du_de_ket_luan() {
    let p = nguoi_keo_kiet();
    let it: Vec<Act> = (0..3).map(|i| hao_phong(1_000 + i)).collect();
    assert!(DriftAuditor::default().audit(&p, &it, &[]).is_empty());

    // Đủ mẫu thì mới kết luận.
    let du: Vec<Act> = (0..8).map(|i| hao_phong(1_000 + i)).collect();
    assert_eq!(DriftAuditor::default().audit(&p, &du, &[]).len(), 1);
}

/// Hành vi khớp tính cách thì auditor phải im. Một auditor kêu suốt ngày là một
/// auditor không ai đọc, và lúc đó nó tệ hơn là không có.
#[test]
fn hanh_vi_khop_tinh_cach_thi_khong_bao_gi() {
    let p = nguoi_keo_kiet();
    let acts: Vec<Act> = (0..30)
        .map(|i| Act {
            at_tick: i,
            field: TraitField::Agreeableness,
            // Dao động quanh 120, như người thật.
            implied: 120 + u16::try_from(i % 7).unwrap() * 20,
        })
        .collect();
    assert!(DriftAuditor::default().audit(&p, &acts, &[]).is_empty());
}

/// Lệch ở một đặc điểm không được che lệch ở đặc điểm khác.
#[test]
fn moi_dac_diem_duoc_soi_rieng() {
    let p = nguoi_keo_kiet();
    let mut acts: Vec<Act> = (0..10).map(hao_phong).collect();
    acts.extend((0..10).map(|i| Act {
        at_tick: i,
        field: TraitField::Extraversion,
        implied: 30,
    }));

    let ra = DriftAuditor::default().audit(&p, &acts, &[]);
    assert_eq!(ra.len(), 2);
    assert!(ra.iter().all(|f| f.verdict == Verdict::Drift));
}

/// Nguyên nhân ở quá xa **không** giải thích được gì — nếu không, một sang chấn
/// từ mười năm trước sẽ hợp thức hóa mọi hành vi về sau, và auditor thành vô dụng.
#[test]
fn nguyen_nhan_qua_xa_khong_giai_thich_duoc() {
    let mut p = nguoi_keo_kiet();
    p.apply_change(
        10,
        TraitField::Agreeableness,
        50,
        nguyen_nhan(1, CauseKind::Trauma),
    );
    // Hành vi lệch xảy ra rất lâu sau đó.
    let acts: Vec<Act> = (0..20).map(|i| hao_phong(500_000 + i)).collect();

    let ra = DriftAuditor::default().audit(&p, &acts, &[]);
    assert_eq!(ra.len(), 1);
    assert_eq!(
        ra[0].verdict,
        Verdict::Drift,
        "nguyên nhân cách đó nửa triệu tick không giải thích được gì"
    );
}

/// Auditor phải là **hàm thuần**: soi hai lần cho cùng kết quả, và không đổi gì.
#[test]
fn auditor_la_ham_thuan() {
    let p = nguoi_keo_kiet();
    let acts: Vec<Act> = (0..20).map(|i| hao_phong(1_000 + i)).collect();
    let a = DriftAuditor::default();
    assert_eq!(a.audit(&p, &acts, &[]), a.audit(&p, &acts, &[]));
    assert_eq!(p, nguoi_keo_kiet(), "auditor không được đổi gì");
}
