//! Test cổng ngân sách hiệu năng (`PF-11`, `§P8.1`).

use mow_devtool::budget::{active_at, check, Failure, Measurement, Metric, Phase, BANG_NGAN_SACH};

fn du_moi_chi_so() -> Vec<Measurement> {
    vec![
        Measurement {
            metric: Metric::TickP99Ms,
            value_ms: 30,
            scale: 1_200,
        },
        Measurement {
            metric: Metric::ChunkGenMs,
            value_ms: 5,
            scale: 32 * 32 * 16,
        },
        Measurement {
            metric: Metric::CommandAckP95Ms,
            value_ms: 40,
            scale: 100,
        },
        Measurement {
            metric: Metric::TextureRebuildMs,
            value_ms: 3,
            scale: 32 * 32,
        },
    ]
}

/// Đủ và trong trần thì xanh.
#[test]
fn du_va_trong_tran_thi_xanh() {
    let r = check(Phase::F, &du_moi_chi_so());
    assert!(r.passed(), "{:?}", r.failures);
    assert_eq!(r.passed.len(), BANG_NGAN_SACH.len());
}

/// Vượt trần thì đỏ, và nói rõ vượt bao nhiêu.
#[test]
fn vuot_tran_thi_do_va_noi_ro_vuot_bao_nhieu() {
    let mut m = du_moi_chi_so();
    m[0].value_ms = 41;
    let r = check(Phase::F, &m);
    assert!(!r.passed());
    assert_eq!(
        r.failures,
        vec![Failure::OverBudget {
            metric: "tick_duration_ms_p99",
            measured_ms: 41,
            limit_ms: 40,
        }]
    );
}

/// Đúng bằng trần thì đạt — trần là "không vượt", không phải "phải dưới".
#[test]
fn dung_bang_tran_thi_dat() {
    let mut m = du_moi_chi_so();
    m[0].value_ms = 40;
    assert!(check(Phase::F, &m).passed());
}

/// **Đo ở quy mô nhỏ không phải là "đạt".**
///
/// Đây là bài chống việc đạt ngân sách bằng cách làm ít đi: một tick 2 ms với
/// 3 thực thể không chứng minh gì về một tick với 1200 thực thể.
#[test]
fn do_o_quy_mo_nho_khong_phai_la_dat() {
    let mut m = du_moi_chi_so();
    m[0] = Measurement {
        metric: Metric::TickP99Ms,
        value_ms: 2,
        scale: 3,
    };
    let r = check(Phase::F, &m);
    assert!(!r.passed(), "2 ms mà chỉ có 3 thực thể thì không nói gì");
    assert!(matches!(
        r.failures[0],
        Failure::ScaleTooSmall {
            metric: "tick_duration_ms_p99",
            ..
        }
    ));
}

/// **Không đo cũng không phải là "đạt"** — một chỉ số không đo là một chỉ số đã trôi.
#[test]
fn khong_do_cung_khong_phai_la_dat() {
    let r = check(Phase::F, &[]);
    assert!(!r.passed());
    assert_eq!(r.failures.len(), BANG_NGAN_SACH.len());
    assert!(r
        .failures
        .iter()
        .all(|f| matches!(f, Failure::NotMeasured { .. })));
}

/// **Ngân sách theo phase, không phải một mốc ở cuối.**
///
/// Giai đoạn A chỉ áp hai dòng có hiệu lực từ A. Nếu áp cả bảng ngay từ A thì
/// CI luôn đỏ, và câu *"vượt ngân sách là CI fail"* thành giả.
#[test]
fn ngan_sach_theo_phase_khong_phai_mot_moc_o_cuoi() {
    let chi_hai_dong = vec![
        Measurement {
            metric: Metric::TickP99Ms,
            value_ms: 30,
            scale: 1_200,
        },
        Measurement {
            metric: Metric::ChunkGenMs,
            value_ms: 5,
            scale: 32 * 32 * 16,
        },
    ];
    assert!(
        check(Phase::A, &chi_hai_dong).passed(),
        "Giai đoạn A chưa có gateway lẫn frontend để đo"
    );
    assert!(
        !check(Phase::F, &chi_hai_dong).passed(),
        "nhưng Giai đoạn F thì phải đo đủ"
    );
}

/// Ngân sách **siết dần**: số chỉ số có hiệu lực không giảm theo phase.
#[test]
fn ngan_sach_siet_dan_khong_noi_ra() {
    let mut truoc = 0;
    for p in [Phase::A, Phase::B, Phase::C, Phase::D, Phase::E, Phase::F] {
        let n = active_at(p).len();
        assert!(n >= truoc, "{p:?} có ít chỉ số hơn phase trước");
        truoc = n;
    }
    assert_eq!(active_at(Phase::F).len(), BANG_NGAN_SACH.len());
    assert!(active_at(Phase::A).len() < BANG_NGAN_SACH.len());
}

/// Mỗi chỉ số có quy mô tối thiểu **có nghĩa**, không phải 0.
#[test]
fn moi_chi_so_co_quy_mo_toi_thieu_co_nghia() {
    for b in BANG_NGAN_SACH {
        assert!(
            b.metric.min_scale() > 0,
            "{} không có quy mô tối thiểu — cổng tự mở",
            b.metric.as_str()
        );
        assert!(!b.metric.scale_unit().is_empty());
    }
    // Chunk phải đo đủ `32×32×16`, đúng như `§P8.1` mô tả.
    assert_eq!(Metric::ChunkGenMs.min_scale(), 32 * 32 * 16);
}

/// Mỗi chỉ số có tên ổn định và không trùng.
#[test]
fn moi_chi_so_co_ten_on_dinh_va_khong_trung() {
    let ten: std::collections::BTreeSet<&str> =
        BANG_NGAN_SACH.iter().map(|b| b.metric.as_str()).collect();
    assert_eq!(ten.len(), BANG_NGAN_SACH.len());
}

/// Báo cáo nói được **từng chỗ trượt**, không gộp thành "trượt".
#[test]
fn bao_cao_noi_duoc_tung_cho_truot() {
    let hong = vec![
        Measurement {
            metric: Metric::TickP99Ms,
            value_ms: 200,
            scale: 1_200,
        },
        Measurement {
            metric: Metric::ChunkGenMs,
            value_ms: 1,
            scale: 4,
        },
    ];
    let r = check(Phase::F, &hong);
    assert_eq!(
        r.failures.len(),
        4,
        "2 trượt + 2 không đo: {:?}",
        r.failures
    );
    for f in &r.failures {
        assert!(f.to_string().len() > 15, "{f}");
    }
}
