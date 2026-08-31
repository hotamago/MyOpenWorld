//! Test tiền sử (`PF-05`, `§7.6.4`, `§22.46`).

use mow_core::Tick;
use mow_scenario::prehistory::{
    detail_chunk, run_prehistory, ChunkError, MacroDelta, MacroEvent, MacroKind, PrehistoryConfig,
    TICK_MOI_NAM,
};

fn cau_hinh(years: u32) -> PrehistoryConfig {
    PrehistoryConfig {
        years,
        initial_polities: vec![
            "veskar".to_owned(),
            "tolm".to_owned(),
            "arren".to_owned(),
            "kesh".to_owned(),
        ],
        seed: 4_242,
    }
}

// ───────────── quy tắc 1 · tiến qua thời gian thật ─────────────

/// **Không phải mọi thứ đều ở tick 0.**
///
/// Đây là quy tắc mà `§7.6.4` in đậm, và vi phạm nó làm cả thế giới *"trông
/// như vừa được tạo ra cùng một lúc"*.
#[test]
fn khong_phai_moi_thu_deu_o_tick_0() {
    let d = run_prehistory(&cau_hinh(300));
    let sau_tick_0 = d.events.iter().filter(|e| e.at_tick.0 > 0).count();
    assert!(
        sau_tick_0 > 0,
        "300 năm mà mọi event đều ở tick 0 thì tiền sử chỉ là trang trí"
    );
    // Và những cái ở tick 0 đúng là những cái phải ở đó: việc lập quốc ban đầu.
    for e in d.events.iter().filter(|e| e.at_tick.0 == 0) {
        assert!(matches!(e.kind, MacroKind::PolityFounded { .. }));
    }
}

/// **Tick suy đúng từ năm** — không phải một con số bịa.
#[test]
fn tick_suy_dung_tu_nam() {
    let d = run_prehistory(&cau_hinh(200));
    for e in &d.events {
        assert_eq!(
            e.at_tick,
            Tick(u64::from(e.at_year) * TICK_MOI_NAM),
            "event năm {} có tick sai",
            e.at_year
        );
    }
}

/// Event **sắp theo thời gian**, không theo thứ tự sinh.
#[test]
fn event_sap_theo_thoi_gian() {
    let d = run_prehistory(&cau_hinh(300));
    let mut truoc = 0;
    for e in &d.events {
        assert!(e.at_year >= truoc, "{} sau {truoc}", e.at_year);
        truoc = e.at_year;
    }
}

/// Đồng hồ world bắt đầu từ chỗ tiền sử kết thúc.
#[test]
fn dong_ho_world_bat_dau_tu_cho_tien_su_ket_thuc() {
    let d = run_prehistory(&cau_hinh(300));
    assert_eq!(d.ends_at, Tick(300 * TICK_MOI_NAM));
    assert!(d.events.iter().all(|e| e.at_tick <= d.ends_at));
}

// ───────────── quy tắc 2 · chốt trước khi mở chunk ─────────────

/// **Mở chunk chỉ tra macro-delta, không quyết định nó.**
///
/// Chữ ký của [`detail_chunk`] nhận `&MacroDelta`, không `&mut` — nên không có
/// đường nào để việc mở chunk viết thêm vào lịch sử. Bài này khẳng định điều
/// đó bằng cách mở đủ mọi chunk và kiểm rằng delta không đổi.
#[test]
fn mo_chunk_khong_quyet_dinh_lich_su() {
    let d = run_prehistory(&cau_hinh(400));
    let truoc = d.clone();
    for r in 0..5_000 {
        let _ = detail_chunk(&d, r).unwrap();
    }
    assert_eq!(d, truoc, "mở chunk mà lịch sử đổi thì camera viết lịch sử");
}

/// **Lịch sử không phụ thuộc đường đi của camera.**
///
/// Hai người chơi đi hai thứ tự khác nhau phải thấy cùng một lịch sử.
#[test]
fn lich_su_khong_phu_thuoc_duong_di_cua_camera() {
    let d = run_prehistory(&cau_hinh(400));

    let mut nguoi_a = Vec::new();
    for r in 0..200 {
        nguoi_a.push(detail_chunk(&d, r).unwrap());
    }
    let mut nguoi_b = Vec::new();
    for r in (0..200).rev() {
        nguoi_b.push(detail_chunk(&d, r).unwrap());
    }
    nguoi_b.reverse();
    assert_eq!(nguoi_a, nguoi_b);
}

/// **Mở chunk trước khi chốt là lỗi**, không phải một trường hợp im lặng.
#[test]
fn mo_chunk_truoc_khi_chot_la_loi() {
    let chua_chot = MacroDelta {
        events: vec![MacroEvent {
            at_year: 5,
            at_tick: Tick(5 * TICK_MOI_NAM),
            kind: MacroKind::PolityFounded {
                polity: "x".to_owned(),
            },
            caused_by: None,
        }],
        ruins: Default::default(),
        borders: Default::default(),
        feuds: Default::default(),
        lineages: Default::default(),
        trade_routes: Default::default(),
        ends_at: Tick(0),
        sealed: false, // chưa chốt
    };
    assert!(!chua_chot.is_sealed());
    assert_eq!(
        detail_chunk(&chua_chot, 1).unwrap_err(),
        ChunkError::PrehistoryNotSealed
    );
}

/// **Tàn tích ở đúng nơi từng có thành phố**, tra được từ delta.
#[test]
fn tan_tich_o_dung_noi_tung_co_thanh_pho() {
    let d = run_prehistory(&cau_hinh(600));
    assert!(!d.ruins.is_empty(), "600 năm phải có ít nhất một tàn tích");

    for (vung, ten) in &d.ruins {
        // Vùng có tàn tích thì chunk ở đó thấy nó.
        assert_eq!(
            detail_chunk(&d, *vung).unwrap().ruin.as_deref(),
            Some(ten.as_str())
        );
        // Và có một event thật sinh ra nó.
        assert!(d.events.iter().any(|e| matches!(
            &e.kind,
            MacroKind::SettlementAbandoned { region, .. } if region == vung
        )));
    }
}

/// Vùng không có tàn tích thì chunk ở đó trống — và trống là câu trả lời, không
/// phải một lời mời sinh ra thứ gì đó.
#[test]
fn vung_khong_co_tan_tich_thi_trong() {
    let d = run_prehistory(&cau_hinh(300));
    let trong = (0..10_000u64).find(|r| d.ruin_at(*r).is_none()).unwrap();
    assert_eq!(detail_chunk(&d, trong).unwrap().ruin, None);
}

// ───────────── §22.17 · mọi thứ có event thật đằng sau ─────────────

/// **Không có trường văn bản tự do nào** trong event vĩ mô.
///
/// Một trường `narrative` sẽ được điền bằng văn model sinh, và từ đó biên niên
/// sử có những câu không truy được về dữ liệu nào.
#[test]
fn khong_co_truong_van_ban_tu_do_nao() {
    let d = run_prehistory(&cau_hinh(100));
    let j = serde_json::to_string(&d.events[0]).unwrap();
    for cam in ["narrative", "description", "story", "text", "summary"] {
        assert!(!j.contains(cam), "event mang văn bản tự do: {j}");
    }
}

/// **Thù hằn có nguyên nhân truy ngược được.**
#[test]
fn thu_han_co_nguyen_nhan_truy_nguoc_duoc() {
    let d = run_prehistory(&cau_hinh(600));
    assert!(!d.feuds.is_empty(), "600 năm phải có mối thù");

    // Mỗi mối thù có ít nhất một cuộc chiến thật giữa đúng hai bên đó.
    for (a, b) in &d.feuds {
        assert!(
            d.events.iter().any(|e| matches!(
                &e.kind,
                MacroKind::War { a: x, b: y, .. }
                    if (x == a && y == b) || (x == b && y == a)
            )),
            "thù giữa {a} và {b} không có cuộc chiến nào đằng sau"
        );
    }
}

/// **Biên giới dịch chuyển vì một cuộc chiến cụ thể**, truy ngược được.
#[test]
fn bien_gioi_dich_chuyen_vi_mot_cuoc_chien_cu_the() {
    let d = run_prehistory(&cau_hinh(600));
    let idx = d
        .events
        .iter()
        .position(|e| matches!(e.kind, MacroKind::BorderShifted { .. }))
        .expect("600 năm phải có biên giới dịch chuyển");

    let nguyen_nhan = d.causes_of(idx);
    assert!(!nguyen_nhan.is_empty(), "biên giới đổi mà không vì gì cả");
    assert!(matches!(nguyen_nhan[0].kind, MacroKind::War { .. }));
}

/// Tàn tích cũng truy được về cuộc chiến làm nó bị bỏ.
#[test]
fn tan_tich_truy_duoc_ve_cuoc_chien() {
    let d = run_prehistory(&cau_hinh(600));
    let idx = d
        .events
        .iter()
        .position(|e| matches!(e.kind, MacroKind::SettlementAbandoned { .. }))
        .expect("600 năm phải có tàn tích");
    assert!(matches!(
        d.causes_of(idx).first().map(|e| &e.kind),
        Some(MacroKind::War { .. })
    ));
}

// ───────────── xác định ─────────────

/// **Cùng seed cho cùng lịch sử** — hai người tải cùng worldseed nhận cùng thứ.
#[test]
fn cung_seed_cho_cung_lich_su() {
    assert_eq!(
        run_prehistory(&cau_hinh(500)),
        run_prehistory(&cau_hinh(500))
    );
}

/// Seed khác cho lịch sử khác.
#[test]
fn seed_khac_cho_lich_su_khac() {
    let mut khac = cau_hinh(500);
    khac.seed = 9_999;
    assert_ne!(run_prehistory(&cau_hinh(500)), run_prehistory(&khac));
}

/// Chạy dài hơn thì lịch sử **nối tiếp**, không phải viết lại.
#[test]
fn chay_dai_hon_thi_lich_su_noi_tiep() {
    let ngan = run_prehistory(&cau_hinh(200));
    let dai = run_prehistory(&cau_hinh(400));
    assert_eq!(
        dai.events[..ngan.events.len()],
        ngan.events[..],
        "200 năm đầu của một lịch sử 400 năm phải giống hệt lịch sử 200 năm"
    );
}

/// Tiền sử 0 năm là hợp lệ và đã chốt ngay.
#[test]
fn tien_su_khong_nam_la_hop_le() {
    let d = run_prehistory(&cau_hinh(0));
    assert_eq!(d.ends_at, Tick(0));
    assert!(
        d.is_sealed(),
        "0 năm thì chốt xong ngay, không phải chưa chốt"
    );
    assert!(detail_chunk(&d, 1).is_ok());
}

/// **Không gọi LLM** — hàm không nhận tham số nào cho phép nó.
///
/// Test này khẳng định một quyết định API: `run_prehistory` chỉ nhận
/// `&PrehistoryConfig`, và cấu hình đó không có đường nào cắm một model vào.
/// Một tiền sử gọi LLM thì không tái lập được, nên hai người tải cùng worldseed
/// sẽ nhận hai lịch sử khác nhau.
#[test]
fn khong_goi_llm() {
    let j = serde_json::to_string(&cau_hinh(10)).unwrap();
    for cam in ["llm", "model", "prompt", "persona"] {
        assert!(!j.contains(cam), "cấu hình tiền sử có đường cắm LLM: {j}");
    }
}
