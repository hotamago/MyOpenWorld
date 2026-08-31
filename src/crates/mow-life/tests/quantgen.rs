//! Test di truyền định lượng (`PD-22`, `§9.5.1`).

use mow_life::quantgen::{
    breeding_value, express, inbreeding_depression, Environment, Population, Trait,
};

fn chieu_cao() -> Trait {
    Trait {
        id: "height".into(),
        heritability: 800, // rất di truyền
        fitness_linked: true,
        gxe: 400,
    }
}

fn tinh_khi() -> Trait {
    Trait {
        id: "temperament".into(),
        heritability: 150, // hầu như do hoàn cảnh
        fitness_linked: false,
        gxe: 200,
    }
}

fn tru_phu() -> Environment {
    Environment {
        nutrition: 900,
        disease_load: 100,
        climate: 800,
        mana: 500,
    }
}

fn doi_kem() -> Environment {
    Environment {
        nutrition: 150,
        disease_load: 700,
        climate: 300,
        mana: 200,
    }
}

/// **`h²` khác nhau cho từng trait** — cách rẻ nhất để "con nhà nòi" đúng với
/// vài thứ và sai với nhiều thứ khác.
#[test]
fn h2_khac_nhau_lam_dong_doi_co_nghia_voi_vai_trait_thoi() {
    let gen_tot = 900;
    let gen_te = 100;
    let e = tru_phu();

    let cao_tot = express(&chieu_cao(), gen_tot, &e, 0, 0, 0);
    let cao_te = express(&chieu_cao(), gen_te, &e, 0, 0, 0);
    let khi_tot = express(&tinh_khi(), gen_tot, &e, 0, 0, 0);
    let khi_te = express(&tinh_khi(), gen_te, &e, 0, 0, 0);

    let anh_huong_gen_len_chieu_cao = cao_tot.abs_diff(cao_te);
    let anh_huong_gen_len_tinh_khi = khi_tot.abs_diff(khi_te);

    assert!(
        anh_huong_gen_len_chieu_cao > anh_huong_gen_len_tinh_khi * 2,
        "dòng dõi phải quyết định chiều cao mạnh hơn hẳn tính khí: {anh_huong_gen_len_chieu_cao} vs {anh_huong_gen_len_tinh_khi}"
    );
}

/// **Tương tác gen×môi trường**: chọn giống ở một nơi rồi mang sang nơi khác có
/// thể thất bại.
///
/// Với mô hình cộng thuần, một giống tốt hơn ở đâu cũng tốt hơn **đúng bằng ấy**
/// — nên không có gì để thất bại.
#[test]
fn giong_tot_o_vung_tru_phu_co_the_that_bai_o_vung_doi_kem() {
    let t = chieu_cao();
    let gen_tot = 950;
    let gen_thuong = 500;

    let loi_the_o_tru_phu = i32::from(express(&t, gen_tot, &tru_phu(), 0, 0, 0))
        - i32::from(express(&t, gen_thuong, &tru_phu(), 0, 0, 0));
    let loi_the_o_doi_kem = i32::from(express(&t, gen_tot, &doi_kem(), 0, 0, 0))
        - i32::from(express(&t, gen_thuong, &doi_kem(), 0, 0, 0));

    assert!(
        loi_the_o_tru_phu > loi_the_o_doi_kem,
        "lợi thế di truyền phải co lại ở vùng đói kém: {loi_the_o_tru_phu} vs {loi_the_o_doi_kem}"
    );
}

/// Không có `gxe` thì hai vùng cho cùng lợi thế — đúng như mô hình cộng thuần,
/// và đó là lý do phải có số hạng tương tác.
#[test]
fn khong_co_gxe_thi_loi_the_di_truyen_khong_doi_theo_vung() {
    let cong_thuan = Trait {
        gxe: 0,
        ..chieu_cao()
    };

    let a = i32::from(express(&cong_thuan, 950, &tru_phu(), 0, 0, 0))
        - i32::from(express(&cong_thuan, 500, &tru_phu(), 0, 0, 0));
    let b = i32::from(express(&cong_thuan, 950, &doi_kem(), 0, 0, 0))
        - i32::from(express(&cong_thuan, 500, &doi_kem(), 0, 0, 0));
    assert_eq!(a, b);
}

/// **Cận huyết không phải một hình phạt cố định**: cùng `F`, hai quần thể, hai
/// hậu quả.
#[test]
fn cung_he_so_can_huyet_hai_quan_the_hai_hau_qua() {
    let f = 250;
    let quan_the_khoe = inbreeding_depression(f, 100);
    let quan_the_da_qua_nut_that = inbreeding_depression(f, 900);

    assert!(
        quan_the_da_qua_nut_that > quan_the_khoe * 5,
        "gánh nặng alen lặn phải quyết định mức hại: {quan_the_da_qua_nut_that} vs {quan_the_khoe}"
    );
}

/// **Chỉ trait gắn sức sống mới chịu cận huyết.**
///
/// Gộp tất cả lại làm cận huyết thành một hình phạt chung chung thay vì một cơ
/// chế sinh học.
#[test]
fn chi_trait_gan_suc_song_moi_chiu_can_huyet() {
    let e = tru_phu();
    let cao_sach = express(&chieu_cao(), 700, &e, 0, 0, 0);
    let cao_can_huyet = express(&chieu_cao(), 700, &e, 800, 800, 0);
    assert!(cao_can_huyet < cao_sach);

    let khi_sach = express(&tinh_khi(), 700, &e, 0, 0, 0);
    let khi_can_huyet = express(&tinh_khi(), 700, &e, 800, 800, 0);
    assert_eq!(khi_sach, khi_can_huyet, "tính khí không gắn sức sống");
}

/// Con **không phải bản sao trung bình của cha mẹ** — phân ly làm chọn giống mất
/// nhiều thế hệ.
#[test]
fn con_khong_phai_ban_sao_trung_binh_cua_cha_me() {
    let tb = breeding_value(800, 600, 0);
    assert_eq!(tb, 700);

    assert!(breeding_value(800, 600, 150) > tb);
    assert!(breeding_value(800, 600, -150) < tb);
}

/// **Nút thắt di truyền**: quần thể nhỏ tích cận huyết nhanh hơn hẳn.
#[test]
fn quan_the_nho_tich_can_huyet_nhanh_hon_han() {
    let lon = Population {
        effective_size: 500,
        deleterious_load: 100,
    };
    let nho = Population {
        effective_size: 10,
        deleterious_load: 100,
    };
    assert!(
        nho.inbreeding_per_generation() > lon.inbreeding_per_generation() * 10,
        "{} vs {}",
        nho.inbreeding_per_generation(),
        lon.inbreeding_per_generation()
    );
}

/// **Một dòng họ quý tộc khép kín tự suy yếu qua vài thế hệ.**
///
/// Không cần viết riêng — nó rơi ra từ việc cận huyết đẩy gánh nặng lên, và
/// gánh nặng làm cận huyết hại hơn.
#[test]
fn dong_ho_khep_kin_tu_suy_yeu_qua_vai_the_he() {
    let mut dong_ho = Population {
        effective_size: 8,
        deleterious_load: 50,
    };
    let t = chieu_cao();
    let e = tru_phu();

    let doi_dau = express(&t, 800, &e, 0, dong_ho.deleterious_load, 0);

    for _ in 0..6 {
        dong_ho.advance_closed();
    }
    let doi_sau = express(&t, 800, &e, 400, dong_ho.deleterious_load, 0);

    assert!(
        doi_sau < doi_dau,
        "dòng họ khép kín mà không suy yếu: {doi_sau} vs {doi_dau}"
    );
    assert!(dong_ho.deleterious_load > 50);
}

/// **Quần thể rồng bị săn xuống dưới ngưỡng mắc kẹt** — thêm con không cứu được,
/// vì cái mất là đa dạng chứ không phải số lượng.
#[test]
fn quan_the_qua_nut_that_thi_mac_ket() {
    let con_nhieu = Population {
        effective_size: 200,
        deleterious_load: 100,
    };
    let bi_san_can = Population {
        effective_size: 12,
        deleterious_load: 100,
    };
    assert!(!con_nhieu.bottlenecked(50));
    assert!(bi_san_can.bottlenecked(50));
}

/// Biểu hiện **xác định**: nhiễu là tham số, không phải xúc xắc bên trong.
#[test]
fn bieu_hien_xac_dinh() {
    let t = chieu_cao();
    let e = tru_phu();
    assert_eq!(
        express(&t, 700, &e, 100, 200, 30),
        express(&t, 700, &e, 100, 200, 30)
    );
}

/// Kết quả luôn nằm trong thang `0`–`1000`, kể cả với đầu vào cực đoan.
#[test]
fn ket_qua_luon_trong_thang() {
    let t = chieu_cao();
    for gen in [0u16, 500, 1_000] {
        for e in [&tru_phu(), &doi_kem()] {
            for f in [0u16, 1_000] {
                let v = express(&t, gen, e, f, 1_000, i16::MAX);
                assert!(v <= 1_000);
                let v = express(&t, gen, e, f, 1_000, i16::MIN);
                assert!(v <= 1_000);
            }
        }
    }
}
