//! Test ngân sách nhận thức (`PC-08`).
//!
//! Mỗi test đặt tên theo **hỏng cái gì nếu nó sai**, không theo hàm nào nó gọi.

use mow_core::{CognitionScheduler, EntityId, Pending, StableKey, Tick, Weights};
use mow_math::CanonicalHash;
use std::collections::BTreeSet;

fn cho(e: u64, urgency: u32, in_focus: bool, since: u64) -> Pending {
    let id = EntityId(e);
    Pending {
        entity: id,
        urgency,
        in_focus,
        waiting_since: Tick(since),
        key: StableKey::plain(id),
    }
}

/// Không có test này thì `starvation` có thể tụt về 0 trong một lần chỉnh
/// trọng số, và một nhân vật ít cấp bách sẽ đứng ngây suốt đời.
///
/// Và "cuối cùng thì cũng tới lượt" là một lời hứa vô nghĩa nếu "cuối cùng" là
/// hai tiếng đồng hồ. Nên test kiểm **trần đã công bố**, chứ không kiểm "rốt
/// cuộc có xảy ra hay không".
#[test]
fn nguoi_it_cap_bach_toi_luot_trong_tran_da_cong_bo() {
    let mut s = CognitionScheduler::new(1);
    let tran = s
        .weights
        .max_wait(1000)
        .expect("trọng số mặc định phải có trần");

    // Trường hợp xấu nhất: A ở đỉnh thang cấp bách và vừa được phục vụ ở tick
    // trước, mỗi tick; B ở đáy thang và chờ từ tick 0.
    let mut cho_bao_lau = None;
    for t in 0..=tran + 10 {
        let pending = vec![
            cho(1, 1000, false, t.saturating_sub(1)),
            cho(2, 0, false, 0),
        ];
        if s.select(&pending, Tick(t))[0].entity == EntityId(2) {
            cho_bao_lau = Some(t);
            break;
        }
    }
    let cho_bao_lau = cho_bao_lau.expect("B không bao giờ được nghĩ");
    assert!(
        cho_bao_lau <= tran,
        "chờ {cho_bao_lau} tick, vượt trần đã công bố {tran}"
    );
}

/// Trần phải là một con số người ta chịu được. 502 tick ở 20 Hz là 25 giây —
/// lâu, nhưng người chơi sẽ không kết luận rằng nhân vật bị hỏng.
#[test]
fn tran_cho_o_muc_nguoi_choi_chiu_duoc() {
    let w = Weights::default();
    assert_eq!(w.max_wait(1000), Some(502));
    // Còn `starvation = 0` thì **không có trần** — và đó là toàn bộ vấn đề.
    let khong = Weights { starvation: 0, ..w };
    assert_eq!(khong.max_wait(1000), None);
}
#[test]
fn starvation_bang_khong_thi_chet_doi_that() {
    let mut s = CognitionScheduler::new(1);
    s.weights.starvation = 0;
    for t in 0..5_000u64 {
        let pending = vec![
            cho(1, 900, false, t.saturating_sub(1)),
            cho(2, 10, false, 0),
        ];
        let ra = s.select(&pending, Tick(t));
        assert_eq!(
            ra[0].entity,
            EntityId(1),
            "với starvation=0, B lẽ ra không bao giờ tới lượt"
        );
    }
}

/// Cùng đầu vào ⇒ cùng đầu ra, bất kể gọi bao nhiêu lần và theo thứ tự nào.
/// Đây là điều kiện để `§22.9` giữ được.
#[test]
fn chon_la_ham_thuan_khong_phu_thuoc_thu_tu_dau_vao() {
    let a = vec![
        cho(1, 500, false, 0),
        cho(2, 500, true, 0),
        cho(3, 700, false, 0),
    ];
    let mut b = a.clone();
    b.reverse();

    let mut s1 = CognitionScheduler::new(2);
    let mut s2 = CognitionScheduler::new(2);
    let r1: Vec<_> = s1.select(&a, Tick(10)).iter().map(|p| p.entity).collect();
    let r2: Vec<_> = s2.select(&b, Tick(10)).iter().map(|p| p.entity).collect();
    assert_eq!(r1, r2);
}

/// Điểm bằng nhau phải phá hòa bằng khóa ổn định, không bằng thứ tự vector.
#[test]
fn diem_bang_nhau_pha_hoa_on_dinh() {
    let mut s = CognitionScheduler::new(1);
    let pending = vec![cho(7, 100, false, 0), cho(3, 100, false, 0)];
    let mut nguoc = pending.clone();
    nguoc.reverse();

    let a = s.select(&pending, Tick(5))[0].entity;
    let mut s2 = CognitionScheduler::new(1);
    let b = s2.select(&nguoc, Tick(5))[0].entity;
    assert_eq!(a, b);
    assert_eq!(a, EntityId(3), "khóa nhỏ hơn phải thắng");
}

/// Ngân sách là trần cứng.
#[test]
fn khong_bao_gio_vuot_ngan_sach() {
    let mut s = CognitionScheduler::new(3);
    let pending: Vec<_> = (1..=50).map(|i| cho(i, 100, false, 0)).collect();
    assert_eq!(s.select(&pending, Tick(1)).len(), 3);
}

/// Đang trong tầm nhìn phải thắng cấp bách vừa phải — nếu không, nhân vật ngay
/// trước mặt người chơi sẽ đứng im trong khi ai đó ở nửa bên kia bản đồ suy nghĩ.
#[test]
fn nguoi_choi_dang_nhin_duoc_uu_tien() {
    let mut s = CognitionScheduler::new(1);
    let pending = vec![cho(1, 250, false, 0), cho(2, 100, true, 0)];
    assert_eq!(s.select(&pending, Tick(1))[0].entity, EntityId(2));
}

/// Nhưng chú ý của người chơi là **ngón tay đè cân, không phải công tắc**. Một
/// người sắp chết ở nửa bên kia bản đồ vẫn phải thắng một người đang no ngay
/// trước mặt — nếu không, thế giới chỉ diễn kịch cho camera, và mọi thứ ngoài
/// khung hình đứng yên chờ được nhìn.
#[test]
fn chu_y_khong_lan_at_cap_bach_that_su() {
    let mut s = CognitionScheduler::new(1);
    let pending = vec![cho(1, 900, false, 0), cho(2, 100, true, 0)];
    assert_eq!(s.select(&pending, Tick(1))[0].entity, EntityId(1));
}

/// `last_served` nằm trong state hash. Để nó lớn vô hạn là vừa rò rỉ bộ nhớ vừa
/// làm hash phình theo lịch sử thay vì theo trạng thái.
#[test]
fn prune_don_thuc_the_da_chet() {
    let mut s = CognitionScheduler::new(10);
    let pending: Vec<_> = (1..=5).map(|i| cho(i, 100, false, 0)).collect();
    s.select(&pending, Tick(1));

    let alive: BTreeSet<EntityId> = [EntityId(1), EntityId(2)].into_iter().collect();
    assert_eq!(s.prune(&alive), 3);
    assert!(s.last_served(EntityId(5)).is_none());
    assert!(s.last_served(EntityId(1)).is_some());
}

/// Hash phải đổi khi trọng số đổi — nếu không, hai thế giới với luật khác nhau
/// sẽ trông giống hệt nhau ở checkpoint.
#[test]
fn hash_phan_anh_trong_so() {
    let mut a = CognitionScheduler::new(4);
    let mut b = CognitionScheduler::new(4);
    b.weights.urgency = 11;
    assert_ne!(a.state_hash(), b.state_hash());

    // Và phải đổi khi có người được phục vụ.
    let truoc = a.state_hash();
    a.select(&[cho(1, 100, false, 0)], Tick(3));
    assert_ne!(truoc, a.state_hash());
}

/// Công cụ chẩn đoán phải chỉ đúng người chờ lâu nhất.
#[test]
fn longest_wait_chi_dung_nguoi() {
    let s = CognitionScheduler::new(1);
    let pending = vec![cho(1, 0, false, 90), cho(2, 0, false, 10)];
    let (e, w) = s.longest_wait(&pending, Tick(100)).unwrap();
    assert_eq!(e, EntityId(2));
    assert_eq!(w, 90);
}

/// Bộ lập lịch **không được** có chỗ để nhét hạn mức API vào. Test này là một
/// lời nhắc bằng mã: nếu ai đó thêm trường đó, serde sẽ khác và test gãy.
#[test]
fn khong_co_truong_nao_ve_han_muc_api() {
    let s = CognitionScheduler::new(4);
    let j = serde_json::to_string(&s).unwrap();
    for cam in [
        "quota",
        "rate_limit",
        "latency",
        "cost",
        "provider",
        "token",
    ] {
        assert!(
            !j.contains(cam),
            "trường `{cam}` không được nằm trong bộ lập lịch — đó là việc của gateway"
        );
    }
}
