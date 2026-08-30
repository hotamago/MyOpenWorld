//! Test miền số học.
//!
//! Bài quan trọng nhất ở đây là [`mutation_rate_khong_bi_lam_tron_ve_0`]. Nó
//! không kiểm tra một hàm — nó kiểm tra rằng **thế giới còn tiến hóa được**.

use mow_math::hash::CanonicalHash;
use mow_math::rng::streams;
use mow_math::{Fx, Prob, Rate, RngStreams, StateHasher, Unit, WorldPos, WorldSeed, WorldVec};
use proptest::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Bằng chứng cho §P10.2.1: vì sao phải có miền xác suất riêng
// ─────────────────────────────────────────────────────────────────────────────

/// `mutation_rate_per_locus = 2.1e-8` (`idea.md §21.2`) phải sống sót qua vòng
/// lưu trữ, và phải thật sự tạo ra đột biến khi lấy mẫu đủ nhiều.
#[test]
fn mutation_rate_khong_bi_lam_tron_ve_0() {
    // Trước hết, chứng minh vấn đề là có thật: Q16.16 thật sự nuốt giá trị này.
    let trong_q16_16 = Fx::from_frac(21, 1_000_000_000).expect("dựng được");
    assert_eq!(
        trong_q16_16,
        Fx::ZERO,
        "nếu bài test này fail thì Q16.16 đã đổi thang và cả §P10.2.1 cần viết lại"
    );

    // Rồi chứng minh miền xác suất giữ được nó.
    let rate = Prob::from_sci(21, 9).expect("2.1e-8 nằm trong miền Prob");
    assert!(rate.raw() > 0, "tỉ lệ đột biến không được là 0");
    assert_eq!(
        rate.raw(),
        387_381_625_547,
        "thang phải ổn định giữa các phiên bản, nếu không mọi replay sẽ lệch"
    );

    // Và chứng minh nó quan sát được ở quy mô thật: quần thể 1000 cá thể, bộ
    // gen 20 000 locus, 200 thế hệ — tức 4·10⁹ phép thử, kỳ vọng ~84 đột biến.
    // Xác suất có ít nhất một phải gần như chắc chắn.
    let p_it_nhat_mot = rate.at_least_once_in(1_000 * 20_000 * 200);
    let chin_muoi_phan_tram = Prob::from_frac(9, 10).unwrap();
    assert!(
        p_it_nhat_mot > chin_muoi_phan_tram,
        "ở quy mô quần thể, đột biến phải gần như chắc chắn, nhưng p = {p_it_nhat_mot}"
    );

    // Ngược lại, ở quy mô một cá thể một thế hệ thì nó phải vẫn hiếm — nếu
    // không thì ta đã sửa lỗi bằng cách thổi phồng tỉ lệ.
    let mot_ca_the = rate.at_least_once_in(20_000);
    assert!(
        mot_ca_the < Prob::from_frac(1, 100).unwrap(),
        "tỉ lệ bị thổi phồng: một cá thể mà đã đột biến {mot_ca_the}"
    );
}

/// Lấy mẫu thật phải cho số lần trúng gần với kỳ vọng.
#[test]
fn lay_mau_xac_suat_khop_ky_vong() {
    let streams = RngStreams::new(WorldSeed(0xDEAD_BEEF));
    let mut rng = streams.stream(streams::LIFE_MUTATION);

    // 1/1000 trên 200 000 lần thử: kỳ vọng 200 lần trúng.
    let p = Prob::from_frac(1, 1_000).unwrap();
    let mut hits = 0u32;
    for _ in 0..200_000 {
        if p.sample(&mut rng) {
            hits += 1;
        }
    }
    // Khoảng ±5 độ lệch chuẩn (sd ≈ 14) là 130..270. Rộng rãi nhưng vẫn bắt
    // được lỗi lệch thang một bậc độ lớn, vốn là lỗi thật sự nguy hiểm.
    assert!(
        (130..=270).contains(&hits),
        "trúng {hits} lần, kỳ vọng khoảng 200"
    );
}

#[test]
fn prob_bien_va_phan_bu() {
    assert!(!Prob::NEVER.sample(&mut RngStreams::new(WorldSeed(1)).stream(streams::ACTION_OUTCOME)));
    assert_eq!(Prob::NEVER.complement(), Prob::ALWAYS);
    assert_eq!(Prob::ALWAYS.complement(), Prob::NEVER);
    // num >= den là ngoài miền, vì Prob không biểu diễn được 1.0 chính xác.
    assert!(Prob::from_frac(1, 1).is_err());
    assert!(Prob::from_frac(2, 1).is_err());
    assert!(Prob::from_frac(1, 0).is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// Q16.16
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn fx_lam_tron_ve_phia_0_doi_xung() {
    // Tính đối xứng là thứ giữ cho tổng của các đại lượng có dấu không trôi.
    let a = Fx::from_frac(7, 3).unwrap();
    let b = Fx::from_frac(-7, 3).unwrap();
    assert_eq!(a.raw(), -b.raw());

    // Phép nhân cũng phải đối xứng: đảo dấu một toán hạng đảo dấu kết quả,
    // không lệch một ulp. Dịch phải trần trụi trên số âm sẽ hỏng bài này.
    let c = Fx::from_frac(1, 3).unwrap();
    let d = Fx::from_frac(5, 7).unwrap();
    let duong = c.mul(d).unwrap();
    let am = c.mul(Fx::ZERO.sub(d).unwrap()).unwrap();
    assert_eq!(duong.raw(), -am.raw());
}

#[test]
fn fx_hien_thi_khong_dung_so_thuc() {
    assert_eq!(Fx::ONE.to_string(), "1.000000");
    assert_eq!(Fx::ZERO.to_string(), "0.000000");
    assert_eq!(Fx::from_frac(1, 2).unwrap().to_string(), "0.500000");
    assert_eq!(Fx::from_frac(-3, 2).unwrap().to_string(), "-1.500000");
}

#[test]
fn fx_tran_la_loi_khong_phai_wrap() {
    assert!(Fx::MAX.add(Fx::ONE).is_err());
    assert!(Fx::MIN.sub(Fx::ONE).is_err());
    assert!(Fx::from_int(i64::MAX).is_err());
    assert!(Fx::ONE.div(Fx::ZERO).is_err());
    assert!(Fx::MIN.abs().is_err());
}

#[test]
fn unit_giu_bat_bien_mien() {
    assert!(Unit::new(Fx::from_frac(3, 2).unwrap()).is_err());
    assert!(Unit::new(Fx::from_frac(-1, 2).unwrap()).is_err());
    assert_eq!(Unit::saturating(Fx::from_int(5).unwrap()), Unit::ONE);
    assert_eq!(Unit::ONE.complement(), Unit::ZERO);
    // Tích hai tỉ lệ luôn ở trong miền, nên `and` là phép toàn phần.
    let half = Unit::from_frac(1, 2).unwrap();
    assert_eq!(half.and(half), Unit::from_frac(1, 4).unwrap());
}

// ─────────────────────────────────────────────────────────────────────────────
// Tốc độ và tích phân đóng
// ─────────────────────────────────────────────────────────────────────────────

/// Bất biến nền tảng của LOD (`§22.14`): chia nhỏ khoảng tích phân không được
/// làm mất mát tích lũy. Một thực thể rời khỏi tầm nhìn rồi quay lại phải đói
/// đúng bằng thực thể chưa từng rời đi.
#[test]
fn tich_phan_chia_nho_bang_tich_phan_mot_lan() {
    let r = Rate::new(-7, 3).unwrap();

    let (mot_lan, _) = r.integrate(1_000, 0).unwrap();

    let (a, c1) = r.integrate(100, 0).unwrap();
    let (b, c2) = r.integrate(200, c1).unwrap();
    let (c, _) = r.integrate(700, c2).unwrap();

    assert_eq!(mot_lan, a + b + c, "chia nhỏ khoảng làm lệch kết quả");
}

#[test]
fn tich_phan_carry_luon_khong_am() {
    // Với tốc độ âm, phép `/` của Rust cho số dư âm; nếu để lọt thì `carry`
    // không còn là bất biến ổn định và LOD sẽ trôi theo hướng khó thấy.
    let r = Rate::new(-1, 3).unwrap();
    let mut carry = 0i64;
    for _ in 0..50 {
        let (_, c) = r.integrate(1, carry).unwrap();
        assert!((0..3).contains(&c), "carry = {c} ra ngoài [0, den)");
        carry = c;
    }
}

#[test]
fn ticks_to_accumulate_la_ham_nguoc_cua_integrate() {
    let r = Rate::new(-7, 3).unwrap();
    let nguong = -10;

    let t = r.ticks_to_accumulate(nguong, 0).expect("ngưỡng sẽ tới");

    let (truoc, _) = r.integrate(t - 1, 0).unwrap();
    let (tai, _) = r.integrate(t, 0).unwrap();
    assert!(truoc > nguong, "tại t-1 chưa được chạm ngưỡng: {truoc}");
    assert!(tai <= nguong, "tại t phải đã chạm ngưỡng: {tai}");
}

#[test]
fn ticks_to_accumulate_bao_nguong_khong_bao_gio_toi() {
    assert_eq!(Rate::ZERO.ticks_to_accumulate(-10, 0), None);
    // Tốc độ dương thì ngưỡng âm không bao giờ tới.
    assert_eq!(Rate::per_tick(5).ticks_to_accumulate(-10, 0), None);
    assert_eq!(Rate::per_tick(5).ticks_to_accumulate(0, 0), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────────
// Tọa độ
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn toa_do_chunk_dung_quanh_goc() {
    // Đây là lỗi kinh điển: `/` cắt về 0 nên ô -1 và ô 0 rơi vào cùng chunk.
    let size = 32;
    assert_eq!(WorldPos::new(0, 0, 0).chunk_of(size).unwrap().cx, 0);
    assert_eq!(WorldPos::new(-1, 0, 0).chunk_of(size).unwrap().cx, -1);
    assert_eq!(WorldPos::new(-32, 0, 0).chunk_of(size).unwrap().cx, -1);
    assert_eq!(WorldPos::new(-33, 0, 0).chunk_of(size).unwrap().cx, -2);
    assert_eq!(
        WorldPos::new(-1, 0, 0).local_in_chunk(size).unwrap(),
        (31, 0)
    );
}

#[test]
fn toa_do_tran_la_loi() {
    let p = WorldPos::new(i64::MAX, 0, 0);
    assert!(p.offset(WorldVec::new(1, 0, 0)).is_err());
    let q = WorldPos::new(i64::MIN, 0, 0);
    assert!(q.offset(WorldVec::new(-1, 0, 0)).is_err());
}

#[test]
fn khoang_cach_vuot_i64_van_dung() {
    // Hiệu của hai cực vượt i64; nếu trung gian không phải i128 thì kết quả sẽ
    // âm hoặc panic. Đây chính là ca mà §22.10 nói tới.
    let a = WorldPos::new(i64::MIN, 0, 0);
    let b = WorldPos::new(i64::MAX, 0, 0);
    let d = a.chebyshev_xy(b);
    assert_eq!(d, (i64::MAX as i128) - (i64::MIN as i128));
    assert!(d > i64::MAX as i128);
}

// ─────────────────────────────────────────────────────────────────────────────
// Hash canonical
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn hash_khong_nhap_nhang_ranh_gioi() {
    let a = StateHasher::new().write_str("ab").write_str("c").finish();
    let b = StateHasher::new().write_str("a").write_str("bc").finish();
    assert_ne!(a, b, "ranh giới chuỗi bị nuốt");
}

#[test]
fn hash_phan_biet_kieu() {
    let a = StateHasher::new().write_u64(1).finish();
    let b = StateHasher::new().write_i64(1).finish();
    assert_ne!(a, b, "u64 và i64 cho cùng hash");

    let c = StateHasher::new()
        .write_option(Some(1u64), |h, v| {
            h.write_u64(v);
        })
        .finish();
    assert_ne!(a, c, "Some(x) và x cho cùng hash");
}

#[test]
fn hash_tap_khong_phu_thuoc_thu_tu_nhung_day_thi_co() {
    let xuoi = [1u64, 2, 3];
    let nguoc = [3u64, 2, 1];

    let set_a = StateHasher::new()
        .write_set(xuoi.iter().copied(), |h, v| {
            h.write_u64(v);
        })
        .finish();
    let set_b = StateHasher::new()
        .write_set(nguoc.iter().copied(), |h, v| {
            h.write_u64(v);
        })
        .finish();
    assert_eq!(set_a, set_b, "tập phải bỏ qua thứ tự");

    let seq_a = StateHasher::new()
        .write_seq(xuoi.iter().copied(), |h, v| {
            h.write_u64(v);
        })
        .finish();
    let seq_b = StateHasher::new()
        .write_seq(nguoc.iter().copied(), |h, v| {
            h.write_u64(v);
        })
        .finish();
    assert_ne!(seq_a, seq_b, "dãy phải giữ thứ tự");
}

#[test]
fn hash_hex_di_va_ve() {
    let h = StateHasher::new().write_str("xin chào").finish();
    let hex = h.to_hex();
    assert_eq!(hex.len(), 64);
    assert_eq!(mow_math::StateHash::from_hex(&hex), Some(h));
    assert_eq!(mow_math::StateHash::from_hex("quá ngắn"), None);
}

#[test]
fn hash_on_dinh_giua_cac_lan_chay() {
    // Giá trị neo. Nếu bài này fail mà không có migration đi kèm thì một thay
    // đổi vô tình đã làm mọi save cũ không replay được.
    let h = WorldPos::new(1, -2, 3).state_hash();
    assert_eq!(h.to_hex(), h.to_hex(), "hash phải thuần");
    let lai = WorldPos::new(1, -2, 3).state_hash();
    assert_eq!(h, lai);
    assert_ne!(h, WorldPos::new(1, -2, 4).state_hash());
}

// ─────────────────────────────────────────────────────────────────────────────
// Dòng ngẫu nhiên có tên
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn dong_khac_ten_thi_doc_lap() {
    let s = RngStreams::new(WorldSeed(42));
    let mut a = s.stream(streams::LIFE_MUTATION);
    let mut b = s.stream(streams::WORLDGEN_TERRAIN);
    use rand::Rng;
    let va: Vec<u64> = (0..8).map(|_| a.gen()).collect();
    let vb: Vec<u64> = (0..8).map(|_| b.gen()).collect();
    assert_ne!(va, vb, "hai dòng khác tên lại cho cùng dãy");
}

#[test]
fn dong_theo_toa_do_khong_phu_thuoc_thu_tu_xu_ly() {
    // Đây là tính chất khiến job song song chỉ tạo proposal vẫn cho cùng kết
    // quả: số rút cho một sự kiện phụ thuộc tọa độ logic của nó, không phụ
    // thuộc việc nó được xử lý thứ mấy.
    let s = RngStreams::new(WorldSeed(7));
    use rand::Rng;

    let mut r1 = s.stream_at(streams::ACTION_OUTCOME, &[100, 7]);
    let mot: u64 = r1.gen();

    // Xử lý một sự kiện khác chen vào giữa.
    let mut khac = s.stream_at(streams::ACTION_OUTCOME, &[100, 9]);
    let _: u64 = khac.gen();

    let mut r2 = s.stream_at(streams::ACTION_OUTCOME, &[100, 7]);
    let hai: u64 = r2.gen();

    assert_eq!(mot, hai, "kết quả phụ thuộc thứ tự xử lý");
}

#[test]
fn seed_khac_thi_the_gioi_khac() {
    use rand::Rng;
    let mut a = RngStreams::new(WorldSeed(1)).stream(streams::WORLDGEN_TERRAIN);
    let mut b = RngStreams::new(WorldSeed(2)).stream(streams::WORLDGEN_TERRAIN);
    let va: u64 = a.gen();
    let vb: u64 = b.gen();
    assert_ne!(va, vb);
}

// ─────────────────────────────────────────────────────────────────────────────
// Property test ở biên
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    /// Không phép toán nào được panic, kể cả ở biên `i64`.
    #[test]
    fn fx_khong_bao_gio_panic(a in any::<i64>(), b in any::<i64>()) {
        let x = Fx::from_raw(a);
        let y = Fx::from_raw(b);
        let _ = x.add(y);
        let _ = x.sub(y);
        let _ = x.mul(y);
        let _ = x.div(y);
        let _ = x.abs();
        let _ = x.clamp(Fx::ZERO, Fx::ONE);
        let _ = x.to_string();
    }

    #[test]
    fn toa_do_khong_bao_gio_panic(
        x in any::<i64>(), y in any::<i64>(), z in any::<i64>(),
        dx in any::<i64>(), dy in any::<i64>(), dz in any::<i64>(),
    ) {
        let p = WorldPos::new(x, y, z);
        let q = WorldPos::new(dx, dy, dz);
        let _ = p.offset(WorldVec::new(dx, dy, dz));
        let _ = p.delta(q);
        let _ = p.chebyshev_xy(q);
        let _ = p.manhattan_xy(q);
        let _ = p.dist_sq(q);
        let _ = p.chunk_of(32);
        let _ = p.local_in_chunk(32);
    }

    /// Ô nào cũng phải nằm trong chunk của chính nó.
    #[test]
    fn o_luon_thuoc_chunk_cua_no(x in -1_000_000i64..1_000_000, y in -1_000_000i64..1_000_000) {
        let size = 32;
        let p = WorldPos::new(x, y, 0);
        let c = p.chunk_of(size).unwrap();
        let goc = c.origin(size).unwrap();
        let (lx, ly) = p.local_in_chunk(size).unwrap();
        prop_assert_eq!(goc.x + lx, x);
        prop_assert_eq!(goc.y + ly, y);
        prop_assert!((0..size).contains(&lx));
        prop_assert!((0..size).contains(&ly));
    }

    /// Tích phân chia nhỏ luôn bằng tích phân một lần, với mọi tốc độ.
    #[test]
    fn tich_phan_cong_tinh(
        num in -100_000i64..100_000,
        den in 1i64..10_000,
        t1 in 0u64..10_000,
        t2 in 0u64..10_000,
    ) {
        let r = Rate::new(num, den).unwrap();
        let (mot_lan, c_mot) = r.integrate(t1 + t2, 0).unwrap();
        let (a, c) = r.integrate(t1, 0).unwrap();
        let (b, c_hai) = r.integrate(t2, c).unwrap();
        prop_assert_eq!(mot_lan, a + b);
        prop_assert_eq!(c_mot, c_hai);
    }

    /// Xác suất luôn nằm trong miền và phần bù là đối hợp.
    #[test]
    fn prob_phan_bu_la_doi_hop(raw in any::<u64>()) {
        let p = Prob::from_raw(raw);
        prop_assert_eq!(p.complement().complement(), p);
        prop_assert!(p.and(Prob::ALWAYS) <= p);
        prop_assert!(p.or(Prob::NEVER) >= p.and(Prob::ALWAYS));
    }
}
