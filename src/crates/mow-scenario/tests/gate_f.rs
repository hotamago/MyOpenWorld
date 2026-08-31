//! Cổng Giai đoạn F (`plan.md §9`, `PF-GATE`).
//!
//! Ba điều kiện, nguyên văn:
//!
//! > 1. Content pack bên thứ ba **nạp được và không đổi hash** của world không
//! >    dùng nó.
//! > 2. **Rewind tạo branch an toàn.**
//! > 3. **Biên niên sử chỉ dùng event có thật.**
//!
//! Điểm chung: cả ba là những lời hứa mà người dùng **đặt cược vào** trước khi
//! biết chúng đúng hay sai.
//!
//! - Người chơi cài mod **rồi** mới biết save cũ còn mở được không.
//! - Người chơi bấm rewind **rồi** mới biết nhánh cũ còn nguyên không.
//! - Người chơi đọc biên niên sử và **tin** rằng nó kể chuyện đã xảy ra.
//!
//! Vi phạm cái nào cũng không sập ngay. Nó sập ở lần sau, và lúc đó dữ liệu đã
//! mất hoặc niềm tin đã mất.

use mow_core::{BranchId, EntityId, EventSeq, Tick, WorldId};
use mow_persist::sqlite::SqliteStore;
use mow_persist::store::{BranchRecord, EventRecord, Store};
use mow_plugin::{PackSet, Registry};
use mow_yuu::audit::{Chronicle, ChronicleError, Line};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

// ═══════════════════════ Điều kiện 1 ═══════════════════════

fn goc_content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content")
}

fn chi_core() -> Registry {
    let mut r = Registry::new();
    r.add_from_dir(goc_content().join("core"))
        .expect("core nạp được");
    r.resolve_order().expect("giải được thứ tự");
    r
}

fn co_them_mod() -> Registry {
    let mut r = Registry::new();
    r.add_from_dir(goc_content().join("core"))
        .expect("core nạp được");
    r.add_from_dir(goc_content().join("example-thirdparty"))
        .expect("pack bên thứ ba nạp được");
    r.resolve_order().expect("giải được thứ tự");
    r
}

/// **Content pack bên thứ ba nạp được và không đổi hash của world không dùng nó.**
///
/// Vế thứ hai là vế quyết định việc có ai dám cài mod hay không: nếu chỉ cần
/// **có mặt** trong thư mục là hash đổi, thì cài một mod là một quyết định
/// không hoàn tác được.
#[test]
fn gate_f1_pack_ben_thu_ba_khong_doi_hash_cua_world_khong_dung_no() {
    // ── Nạp được ──
    let ca_hai = co_them_mod();
    assert_eq!(ca_hai.len(), 2);
    assert!(ca_hai.manifest("example_thirdparty").is_some());

    // ── Và không đổi hash của core ──
    assert_eq!(
        chi_core().hash_of("core"),
        ca_hai.hash_of("core"),
        "content hash của một pack chỉ được phụ thuộc vào nội dung của chính nó"
    );

    // ── Save cũ mở lại được sau khi cài mod ──
    let save_cu: PackSet = chi_core().pack_set();
    assert_eq!(save_cu.entries.len(), 1);
    co_them_mod()
        .verify_against(&save_cu)
        .expect("save chỉ dùng core phải mở được khi máy có thêm pack khác");

    // ── Vế đối chứng: cơ chế không phải lúc nào cũng nói "được" ──
    //
    // Không có nó thì một cài đặt luôn trả `Ok` cũng qua bài trên, và cả
    // `§22.30` thành trang trí.
    let save_co_mod = co_them_mod().pack_set();
    assert!(
        chi_core().verify_against(&save_co_mod).is_err(),
        "save dùng mod mà máy không có mod thì phải từ chối, không nạp một nửa"
    );
}

/// Pack bên thứ ba **xin quyền** cho những gì nó khai (`§19.7`).
#[test]
fn gate_f1_pack_ben_thu_ba_khong_co_quyen_no_khong_xin() {
    let r = co_them_mod();
    let q = r.grants_of("example_thirdparty").expect("đã nạp");
    assert!(q.has(mow_plugin::Capability::DefineLaw), "nó có `laws/`");
    assert!(
        !q.has(mow_plugin::Capability::OverrideForeign),
        "nó không ghi đè của ai"
    );

    // Và không đăng ký được id của core.
    let mut r = co_them_mod();
    assert!(r.define("example_thirdparty", "core.apple").is_err());
}

// ═══════════════════════ Điều kiện 2 ═══════════════════════

/// World duy nhất dùng trong bài này.
const W: WorldId = WorldId(1);

fn kho() -> SqliteStore {
    SqliteStore::in_memory().expect("mở được kho trong bộ nhớ")
}

fn su_kien(branch: BranchId, seq: u64, tick: u64, kind: &str) -> EventRecord {
    EventRecord {
        seq: EventSeq(seq),
        branch,
        world: W,
        tick: Tick(tick),
        kind: kind.to_owned(),
        actor: 0,
        subject: 0,
        payload: Vec::new(),
        cause: None,
        law_version: None,
        norm_set_version: None,
    }
}

/// **Rewind tạo branch an toàn.**
///
/// "An toàn" nghĩa là ba chuyện cùng lúc, và bỏ chuyện nào cũng làm rewind
/// thành một thao tác người ta sợ bấm:
///
/// 1. Nhánh cũ **còn nguyên** — mọi event sau điểm tách vẫn đọc được.
/// 2. Nhánh mới **thấy quá khứ chung**, không phải một thế giới trống.
/// 3. Nhánh mới **không thấy tương lai** của nhánh cũ.
#[test]
fn gate_f2_rewind_tao_branch_an_toan() {
    let mut s = kho();

    // Nhánh gốc chạy tới tick 100.
    s.create_branch(&BranchRecord {
        id: BranchId(1),
        parent: None,
        fork_tick: Tick(0),
        label: "gốc".into(),
    })
    .unwrap();
    s.append_events(&[
        su_kien(BranchId(1), 1, 10, "core.founded"),
        su_kien(BranchId(1), 2, 40, "core.war"),
        su_kien(BranchId(1), 3, 90, "core.collapse"),
    ])
    .unwrap();

    // Người chơi rewind về tick 50 và tạo nhánh mới.
    s.create_branch(&BranchRecord {
        id: BranchId(2),
        parent: Some(BranchId(1)),
        fork_tick: Tick(50),
        label: "thử lại từ 50".into(),
    })
    .unwrap();
    s.append_events(&[su_kien(BranchId(2), 1, 60, "core.treaty")])
        .unwrap();

    // ── 1. Nhánh cũ còn nguyên ──
    let cu = s
        .read_events(BranchId(1), EventSeq(0), EventSeq(u64::MAX))
        .unwrap();
    assert_eq!(cu.len(), 3, "rewind không được xóa gì của nhánh cũ");
    assert!(
        cu.iter().any(|e| e.kind == "core.collapse"),
        "event sau điểm tách vẫn phải đọc được — nếu không thì rewind là xóa"
    );

    // ── 2. Nhánh mới thấy quá khứ chung ──
    let dong_doi = s.ancestry(BranchId(2)).unwrap();
    assert_eq!(dong_doi.len(), 2, "nhánh mới phải biết cha nó");
    assert_eq!(dong_doi[0].id, BranchId(2));
    assert_eq!(dong_doi[1].id, BranchId(1));
    assert_eq!(
        dong_doi[0].fork_tick,
        Tick(50),
        "điểm tách phải ghi lại — không có nó thì không lọc được quá khứ chung"
    );

    // ── 3. Nhánh mới không thấy tương lai của nhánh cũ ──
    let moi = s
        .read_events(BranchId(2), EventSeq(0), EventSeq(u64::MAX))
        .unwrap();
    assert!(
        !moi.iter().any(|e| e.kind == "core.collapse"),
        "nhánh tách ở tick 50 mà thấy được sự sụp đổ ở tick 90 là thấy tương lai"
    );
    assert_eq!(moi.len(), 1);
}

/// Rewind **nhiều lần** vẫn an toàn, và dòng dõi vẫn truy được.
#[test]
fn gate_f2_rewind_nhieu_lan_van_truy_duoc_dong_doi() {
    let mut s = kho();
    s.create_branch(&BranchRecord {
        id: BranchId(1),
        parent: None,
        fork_tick: Tick(0),
        label: "gốc".into(),
    })
    .unwrap();
    for i in 2..=5u64 {
        s.create_branch(&BranchRecord {
            id: BranchId(i),
            parent: Some(BranchId(i - 1)),
            fork_tick: Tick(i * 10),
            label: format!("lần {i}"),
        })
        .unwrap();
    }

    let dong_doi = s.ancestry(BranchId(5)).unwrap();
    assert_eq!(dong_doi.len(), 5, "năm đời nhánh phải truy được hết");
    // Từ chính nó ngược về gốc.
    assert_eq!(dong_doi[0].id, BranchId(5));
    assert_eq!(dong_doi[4].id, BranchId(1));
    // Điểm tách giảm dần khi lùi về gốc — một nhánh không thể tách sau con nó.
    for w in dong_doi.windows(2) {
        assert!(w[0].fork_tick >= w[1].fork_tick);
    }
}

/// **Ảnh chụp của nhánh cũ vẫn dùng được** sau khi rewind.
///
/// Đây là điều kiện để rewind **hoàn tác được**: quay lại nhánh cũ phải là một
/// thao tác, không phải một cuộc phục hồi.
#[test]
fn gate_f2_anh_chup_cua_nhanh_cu_van_dung_duoc_sau_rewind() {
    let mut s = kho();
    s.create_branch(&BranchRecord {
        id: BranchId(1),
        parent: None,
        fork_tick: Tick(0),
        label: "gốc".into(),
    })
    .unwrap();
    s.put_snapshot(&mow_persist::store::Snapshot {
        branch: BranchId(1),
        world: W,
        tick: Tick(80),
        event_count: 3,
        state_hash: mow_math::StateHash([7u8; 32]),
        blob: vec![1, 2, 3],
    })
    .unwrap();

    s.create_branch(&BranchRecord {
        id: BranchId(2),
        parent: Some(BranchId(1)),
        fork_tick: Tick(50),
        label: "thử lại".into(),
    })
    .unwrap();

    let anh = s
        .latest_snapshot(BranchId(1), Tick(100))
        .unwrap()
        .expect("ảnh chụp của nhánh cũ phải còn");
    assert_eq!(anh.tick, Tick(80));
    assert_eq!(anh.blob, vec![1, 2, 3]);
}

// ═══════════════════════ Điều kiện 3 ═══════════════════════

fn nhat_ky_that() -> BTreeSet<EventSeq> {
    (1..=20).map(EventSeq).collect()
}

/// **Biên niên sử chỉ dùng event có thật.**
///
/// `§22.17`, và nó là bất biến khó giữ nhất trong cả tài liệu vì vi phạm nó
/// làm ra thứ **đọc hay hơn**.
#[test]
fn gate_f3_bien_nien_su_chi_dung_event_co_that() {
    // ── Câu có nguồn thì vào được ──
    let that = Chronicle::compose(
        &nhat_ky_that(),
        vec![
            Line {
                text: "Veskar và Tolm khai chiến vào mùa thu.".to_owned(),
                sources: vec![EventSeq(3), EventSeq(4)],
            },
            Line {
                text: "Biên giới phía nam dịch về Tolm ba vùng.".to_owned(),
                sources: vec![EventSeq(5)],
            },
        ],
    )
    .expect("câu có nguồn thì dựng được");
    assert_eq!(that.lines.len(), 2);
    assert_eq!(that.sources().len(), 3);

    // ── Và mỗi câu truy ngược được ──
    assert_eq!(that.why(0), Some(&[EventSeq(3), EventSeq(4)][..]));

    // ── Câu không nguồn thì KHÔNG vào được, dù nó đọc hay hơn ──
    let hay_ma_bia = Chronicle::compose(
        &nhat_ky_that(),
        vec![Line {
            text: "Người ta nói đó là một thời đại vàng son đã mất.".to_owned(),
            sources: vec![],
        }],
    );
    assert!(matches!(
        hay_ma_bia,
        Err(ChronicleError::UnsourcedClaim { .. })
    ));

    // ── Câu trỏ tới event không tồn tại cũng bị chặn ──
    assert!(matches!(
        Chronicle::compose(
            &nhat_ky_that(),
            vec![Line {
                text: "Một trận đánh chưa từng xảy ra.".to_owned(),
                sources: vec![EventSeq(9_999)],
            }],
        ),
        Err(ChronicleError::DanglingSource { .. })
    ));
}

/// **Một câu bịa lẫn giữa mười câu thật vẫn làm hỏng cả biên niên sử.**
///
/// Không có "phần lớn là thật". Một biên niên sử mà người đọc phải đoán câu nào
/// tin được là một biên niên sử không dùng được.
#[test]
fn gate_f3_mot_cau_bia_lam_hong_ca_bien_nien_su() {
    let mut cac: Vec<Line> = (1..=10)
        .map(|i| Line {
            text: format!("chuyện thứ {i}"),
            sources: vec![EventSeq(i)],
        })
        .collect();
    assert!(Chronicle::compose(&nhat_ky_that(), cac.clone()).is_ok());

    cac.push(Line {
        text: "và rồi mọi người sống hạnh phúc mãi mãi".to_owned(),
        sources: vec![],
    });
    assert!(Chronicle::compose(&nhat_ky_that(), cac).is_err());
}

/// Historian **không nhận một model** — nó nhận nhật ký và những câu đã dựng.
///
/// Bài này khẳng định một quyết định API. Nếu `compose` nhận được một hàm sinh
/// văn bản thì `§22.17` chỉ còn là một lời khuyên, và lời khuyên đó sẽ thua lần
/// đầu có ai muốn biên niên sử đọc hay hơn.
#[test]
fn gate_f3_historian_khong_nhan_mot_model() {
    // Cùng nhật ký và cùng câu ⇒ cùng biên niên sử, mọi lần.
    let cac = vec![Line {
        text: "x".to_owned(),
        sources: vec![EventSeq(1)],
    }];
    let a = Chronicle::compose(&nhat_ky_that(), cac.clone()).unwrap();
    let b = Chronicle::compose(&nhat_ky_that(), cac).unwrap();
    assert_eq!(a, b);
}

// ═══════════════ Ba điều kiện cùng đứng ═══════════════

/// Cả ba điều kiện cùng đúng trong một lần chạy.
///
/// Không phải trang trí: hai trong ba điều kiện đụng tới cùng một thứ — cái mà
/// người dùng **đặt cược vào trước khi biết kết quả**. Chạy chúng cạnh nhau là
/// cách rẻ nhất để thấy chúng không mâu thuẫn.
#[test]
fn gate_f_ba_dieu_kien_cung_dung() {
    // 1. Mod không làm hỏng save cũ.
    let save_cu = chi_core().pack_set();
    assert!(co_them_mod().verify_against(&save_cu).is_ok());

    // 2. Rewind giữ nguyên nhánh cũ.
    let mut s = kho();
    s.create_branch(&BranchRecord {
        id: BranchId(1),
        parent: None,
        fork_tick: Tick(0),
        label: "gốc".into(),
    })
    .unwrap();
    s.append_events(&[su_kien(BranchId(1), 1, 90, "core.collapse")])
        .unwrap();
    s.create_branch(&BranchRecord {
        id: BranchId(2),
        parent: Some(BranchId(1)),
        fork_tick: Tick(50),
        label: "thử lại".into(),
    })
    .unwrap();
    assert_eq!(
        s.read_events(BranchId(1), EventSeq(0), EventSeq(u64::MAX))
            .unwrap()
            .len(),
        1
    );

    // 3. Biên niên sử chỉ dùng event có thật.
    assert!(Chronicle::compose(
        &nhat_ky_that(),
        vec![Line {
            text: "một chuyện có thật".to_owned(),
            sources: vec![EventSeq(1)],
        }],
    )
    .is_ok());

    // Và một chi tiết dễ quên: `EntityId` vẫn là kiểu định danh duy nhất đi
    // qua cả ba — không có chỗ nào dùng một `u64` trần.
    let _ = EntityId(1);
}
