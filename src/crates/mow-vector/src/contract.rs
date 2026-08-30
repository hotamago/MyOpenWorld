//! Hợp đồng của chỉ mục — dùng lại nguyên vẹn cho Qdrant ở `PC-20`.

use crate::{Hit, LineageStep, MemoryId, MemoryPoint, Query, VectorIndex};
use mow_core::{BranchId, Tick};

const DIM: usize = 4;

/// Số chiều mà mọi hiện thực phải dùng trong bộ test này.
pub const DIMENSION: usize = DIM;

fn diem(id: u64, ns: &str, branch: u64, tick: u64, v: [i16; DIM]) -> MemoryPoint {
    MemoryPoint {
        id: MemoryId(id),
        namespace: ns.to_owned(),
        persona_version: 1,
        created_branch: BranchId(branch),
        created_tick: Tick(tick),
        vector: v.to_vec(),
        payload: format!("m{id}").into_bytes(),
    }
}

fn truy_van(v: [i16; DIM], ns: &[&str], lineage: Vec<LineageStep>) -> Query {
    Query {
        vector: v.to_vec(),
        namespaces: ns.iter().map(|s| (*s).to_owned()).collect(),
        lineage,
        limit: 10,
    }
}

/// Dòng dõi chỉ có một nhánh gốc.
fn chi_nhanh_goc(b: u64) -> Vec<LineageStep> {
    vec![LineageStep {
        branch: BranchId(b),
        cutoff: Tick(u64::MAX),
    }]
}

fn ids(h: &[Hit]) -> Vec<u64> {
    h.iter().map(|x| x.point.id.0).collect()
}

/// Chạy toàn bộ hợp đồng.
pub fn run_all<V: VectorIndex, F: Fn() -> V>(factory: F) {
    upsert_va_tim_lai(&factory);
    xep_hang_theo_do_gan(&factory);
    hoa_diem_pha_bang_id(&factory);
    namespace_rong_thi_khong_thay_gi(&factory);
    khong_thay_namespace_khac(&factory);
    tombstone_chi_anh_huong_nhanh_do(&factory);
    thay_ky_uc_cua_cha_truoc_diem_fork(&factory);
    khong_thay_ky_uc_cha_tao_sau_diem_fork(&factory);
    khong_thay_nhanh_ngoai_dong_doi(&factory);
    clear_roi_dung_lai_duoc(&factory);
    ket_qua_khong_phu_thuoc_thu_tu_chen(&factory);
    sai_so_chieu_la_loi(&factory);
}

/// Thêm rồi tìm lại được.
pub fn upsert_va_tim_lai<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 1, 0, [100, 0, 0, 0])).unwrap();
    let r = v
        .search(&truy_van([100, 0, 0, 0], &["lan"], chi_nhanh_goc(1)))
        .unwrap();
    assert_eq!(ids(&r), vec![1]);
    assert_eq!(r[0].point.payload, b"m1");
}

/// Gần hơn thì xếp trước.
pub fn xep_hang_theo_do_gan<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 1, 0, [100, 0, 0, 0])).unwrap();
    v.upsert(&diem(2, "lan", 1, 0, [0, 100, 0, 0])).unwrap();
    v.upsert(&diem(3, "lan", 1, 0, [90, 10, 0, 0])).unwrap();

    let r = v
        .search(&truy_van([100, 0, 0, 0], &["lan"], chi_nhanh_goc(1)))
        .unwrap();
    assert_eq!(ids(&r), vec![1, 3, 2], "hợp đồng: xếp theo độ gần giảm dần");
}

/// Hai điểm bằng nhau thì phá hòa bằng `id`, không phải bằng thứ tự chèn.
pub fn hoa_diem_pha_bang_id<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    // Chèn ngược thứ tự id để phân biệt "phá hòa bằng id" với "giữ thứ tự chèn".
    v.upsert(&diem(9, "lan", 1, 0, [50, 0, 0, 0])).unwrap();
    v.upsert(&diem(3, "lan", 1, 0, [50, 0, 0, 0])).unwrap();
    v.upsert(&diem(7, "lan", 1, 0, [50, 0, 0, 0])).unwrap();

    let r = v
        .search(&truy_van([1, 0, 0, 0], &["lan"], chi_nhanh_goc(1)))
        .unwrap();
    assert_eq!(
        ids(&r),
        vec![3, 7, 9],
        "hợp đồng: hòa điểm phải phá bằng id tăng dần, không phải thứ tự chèn"
    );
}

/// Không truyền namespace thì **không thấy gì**, không phải thấy tất cả.
pub fn namespace_rong_thi_khong_thay_gi<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 1, 0, [100, 0, 0, 0])).unwrap();
    let r = v
        .search(&truy_van([100, 0, 0, 0], &[], chi_nhanh_goc(1)))
        .unwrap();
    assert!(
        r.is_empty(),
        "hợp đồng: quên truyền namespace phải cho kết quả rỗng, không phải rò toàn bộ ký ức"
    );
}

/// Không đọc được ký ức của người khác (`§22.16`).
pub fn khong_thay_namespace_khac<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 1, 0, [100, 0, 0, 0])).unwrap();
    v.upsert(&diem(2, "binh", 1, 0, [100, 0, 0, 0])).unwrap();
    let r = v
        .search(&truy_van([100, 0, 0, 0], &["lan"], chi_nhanh_goc(1)))
        .unwrap();
    assert_eq!(
        ids(&r),
        vec![1],
        "hợp đồng: không rò ký ức sang namespace khác"
    );
}

/// Quên ở nhánh này không làm nhánh chị em quên theo.
pub fn tombstone_chi_anh_huong_nhanh_do<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 1, 0, [100, 0, 0, 0])).unwrap();
    v.tombstone(MemoryId(1), BranchId(2)).unwrap();

    // Nhánh 2 (con của 1) không thấy nữa.
    let nhanh2 = vec![
        LineageStep {
            branch: BranchId(2),
            cutoff: Tick(u64::MAX),
        },
        LineageStep {
            branch: BranchId(1),
            cutoff: Tick(100),
        },
    ];
    assert!(
        v.search(&truy_van([100, 0, 0, 0], &["lan"], nhanh2))
            .unwrap()
            .is_empty(),
        "hợp đồng: bia mộ phải có hiệu lực trên nhánh đã đặt"
    );

    // Nhánh gốc vẫn thấy.
    assert_eq!(
        ids(&v
            .search(&truy_van([100, 0, 0, 0], &["lan"], chi_nhanh_goc(1)))
            .unwrap()),
        vec![1],
        "hợp đồng: quên ở nhánh con không được xóa ký ức ở nhánh cha"
    );
}

/// **Vế một của lọc dòng dõi**: thấy ký ức cha tạo trước điểm fork.
pub fn thay_ky_uc_cua_cha_truoc_diem_fork<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 1, 50, [100, 0, 0, 0])).unwrap();
    let con = vec![
        LineageStep {
            branch: BranchId(2),
            cutoff: Tick(u64::MAX),
        },
        LineageStep {
            branch: BranchId(1),
            cutoff: Tick(100),
        },
    ];
    assert_eq!(
        ids(&v.search(&truy_van([100, 0, 0, 0], &["lan"], con)).unwrap()),
        vec![1],
        "hợp đồng: nhánh con phải kế thừa ký ức của cha tới điểm fork"
    );
}

/// **Vế hai, vế mà lọc phẳng theo `branch_id` bỏ sót**: không thấy ký ức cha
/// tạo ra *sau* khi đã tách.
pub fn khong_thay_ky_uc_cha_tao_sau_diem_fork<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    // Cha tạo ký ức này ở tick 150, sau khi con đã tách ra ở tick 100.
    v.upsert(&diem(1, "lan", 1, 150, [100, 0, 0, 0])).unwrap();
    let con = vec![
        LineageStep {
            branch: BranchId(2),
            cutoff: Tick(u64::MAX),
        },
        LineageStep {
            branch: BranchId(1),
            cutoff: Tick(100),
        },
    ];
    assert!(
        v.search(&truy_van([100, 0, 0, 0], &["lan"], con))
            .unwrap()
            .is_empty(),
        "hợp đồng: nhánh con đọc được tương lai của thế giới song song"
    );
}

/// Nhánh ngoài dòng dõi thì hoàn toàn không thấy.
pub fn khong_thay_nhanh_ngoai_dong_doi<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 99, 0, [100, 0, 0, 0])).unwrap();
    assert!(
        v.search(&truy_van([100, 0, 0, 0], &["lan"], chi_nhanh_goc(1)))
            .unwrap()
            .is_empty(),
        "hợp đồng: nhánh không nằm trong dòng dõi phải vô hình"
    );
}

/// Xóa sạch rồi dựng lại — `PC-06`.
pub fn clear_roi_dung_lai_duoc<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    v.upsert(&diem(1, "lan", 1, 0, [100, 0, 0, 0])).unwrap();
    v.tombstone(MemoryId(1), BranchId(1)).unwrap();
    v.clear().unwrap();
    assert_eq!(v.len().unwrap(), 0);

    v.upsert(&diem(1, "lan", 1, 0, [100, 0, 0, 0])).unwrap();
    assert_eq!(
        ids(&v
            .search(&truy_van([100, 0, 0, 0], &["lan"], chi_nhanh_goc(1)))
            .unwrap()),
        vec![1],
        "hợp đồng: clear phải xóa cả bia mộ, nếu không rebuild sẽ mất dữ liệu"
    );
}

/// Kết quả không phụ thuộc thứ tự chèn — điều kiện để rebuild chỉ mục không
/// làm đổi thế giới.
pub fn ket_qua_khong_phu_thuoc_thu_tu_chen<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let du_lieu = [
        (5u64, [10i16, 20, 0, 0]),
        (2, [20, 10, 0, 0]),
        (8, [15, 15, 0, 0]),
        (1, [30, 0, 0, 0]),
    ];

    let mut xuoi = f();
    for (id, v) in du_lieu {
        xuoi.upsert(&diem(id, "lan", 1, 0, v)).unwrap();
    }
    let mut nguoc = f();
    for (id, v) in du_lieu.iter().rev() {
        nguoc.upsert(&diem(*id, "lan", 1, 0, *v)).unwrap();
    }

    let q = truy_van([25, 5, 0, 0], &["lan"], chi_nhanh_goc(1));
    assert_eq!(
        ids(&xuoi.search(&q).unwrap()),
        ids(&nguoc.search(&q).unwrap()),
        "hợp đồng: rebuild chỉ mục không được đổi thứ tự truy xuất"
    );
}

/// Sai số chiều là lỗi, không phải im lặng cắt bớt.
pub fn sai_so_chieu_la_loi<V: VectorIndex, F: Fn() -> V>(f: &F) {
    let mut v = f();
    let mut p = diem(1, "lan", 1, 0, [1, 0, 0, 0]);
    p.vector.push(7);
    assert!(v.upsert(&p).is_err(), "hợp đồng: sai số chiều phải báo lỗi");
}
