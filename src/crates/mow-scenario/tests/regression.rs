//! Bộ regression: một bài cho **mỗi lỗi đã sửa** (`PF-14`).
//!
//! ## Vì sao một file riêng
//!
//! Mỗi bài ở đây đã từng là một lỗi thật trong quá trình phát triển. Chúng nằm
//! rải rác trong các crate được — và phần lớn *cũng* nằm ở đó, cạnh mã chúng
//! bảo vệ. File này là **danh mục**: nó gom lại những lỗi có chung một hình
//! dạng, để hình dạng đó nhìn thấy được.
//!
//! Hình dạng đó là:
//!
//! > **Công thức chạy trơn tru trong khi lặng lẽ không làm gì cả.**
//!
//! Không cái nào panic. Không cái nào sai kiểu. Mỗi cái cho ra một con số hợp
//! lý, và con số đó đi thẳng vào state. Đó là lý do chúng sống sót qua review
//! và chỉ lộ ra khi có người ngồi đọc kết quả và thấy nó *nhàm chán* — một
//! quần thể không bao giờ sụp, một dịch bệnh không bao giờ lan, một tên trộm
//! không bao giờ bị răn đe.
//!
//! Một bài regression cho mỗi cái là rẻ. Tìm lại chúng lần thứ hai thì không.

use mow_core::clock::{Clock, ClockDomain, Deadline, Tick};
use mow_core::EntityId;
use mow_culture::religion::{credibility, Observance, Rite};
use mow_culture::underworld::{recruitment_pool, Cohort};
use mow_eco::succession::{Event, Patch, Process, Stage};
use mow_econ::money::Coinage;
use mow_law::crime::Temptation;
use mow_life::speciation::{secondary_contact, IsolatedPopulation, SpeciationRoute};
use mow_math::Rate;
use mow_org::state::{District, StateCapacity};
use mow_portal::clock::{rebase_processes, Process as TimedProcess};

// ══════════ Lớp lỗi 1: chia hai lần, kết quả luôn bằng 0 ══════════

/// **`recruitment_pool` từng chia cho 1000 hai lần** và trả về 0 cho mọi dân số.
///
/// Hàm chạy, trả một `u64` hợp lệ, và thế giới có một hệ thống tội phạm ngầm
/// không bao giờ tuyển được ai. Không có gì báo — chỉ là không bao giờ có băng
/// đảng nào hình thành.
#[test]
fn regression_tuyen_mo_khong_bao_gio_bang_khong_o_dan_so_that() {
    // Một nhóm đông, ít gắn bó, ít cơ hội hợp pháp: phải tuyển được người.
    let n = recruitment_pool(&Cohort {
        size: 50_000,
        belonging: 200,
        lawful_opportunity: 200,
    });
    assert!(
        n > 0,
        "nhóm 50 000 người vừa lạc lõng vừa bí đường mà không tuyển được ai — \
         dấu hiệu của một phép chia thừa"
    );
}

/// Và nó phải **tăng theo dân số**, không dừng ở một hằng số.
#[test]
fn regression_tuyen_mo_tang_theo_dan_so() {
    let voi = |size| {
        recruitment_pool(&Cohort {
            size,
            belonging: 200,
            lawful_opportunity: 200,
        })
    };
    assert!(voi(100_000) > voi(10_000));
}

/// Và **hai điều kiện cùng lúc**: chỉ nghèo thôi thì không đủ.
#[test]
fn regression_chi_ngheo_thoi_khong_du_de_bi_tuyen() {
    let ngheo_ma_gan_bo = recruitment_pool(&Cohort {
        size: 50_000,
        belonging: 950,
        lawful_opportunity: 200,
    });
    let vua_lac_long_vua_bi_duong = recruitment_pool(&Cohort {
        size: 50_000,
        belonging: 200,
        lawful_opportunity: 200,
    });
    assert!(vua_lac_long_vua_bi_duong > ngheo_ma_gan_bo * 3);
}

// ══════════ Lớp lỗi 2: ngưỡng không bao giờ với tới ══════════

/// **Răn đe từng không thể răn đe được ai.**
///
/// Rủi ro bị chặn ở 1000 trong khi `need + gain` lên tới 2000, nên vế phải của
/// bất đẳng thức luôn thắng. Hệ thống hình phạt chạy đủ, ghi án đủ, và không
/// đổi hành vi của một ai.
#[test]
fn regression_ran_de_phai_ran_de_duoc() {
    let hinh_phat_nang = Temptation {
        actor: EntityId(1),
        act: "theft".into(),
        need: 500,
        gain: 500,
        exposure: 900,
        capability: 800,
        believed_coverage: 900,
        moral_cost: 200,
        believed_sanction: 1_000,
    };
    assert!(
        !hinh_phat_nang.deliberate().will_act,
        "giữa chợ ban ngày, cưỡng chế dày, hình phạt tối đa mà vẫn phạm tội — \
         răn đe không có tác dụng nào"
    );

    // Nhưng đói tới mức tuyệt vọng, ở chỗ khuất, thì vẫn phạm — răn đe
    // **không** được tuyệt đối, nếu không thì cả `§12.5.2` chỉ là một cái khóa.
    let tuyet_vong = Temptation {
        need: 1_000,
        gain: 900,
        exposure: 50,
        believed_coverage: 100,
        moral_cost: 0,
        believed_sanction: 300,
        ..hinh_phat_nang
    };
    assert!(tuyet_vong.deliberate().will_act);
}

/// **Debasement từng không phát hiện được ở ngưỡng 30.**
///
/// Không ai từng pha tiền tới mức đó trước khi hệ thống sụp vì lý do khác, nên
/// cơ chế phát hiện tồn tại mà không bao giờ chạy.
#[test]
fn regression_pha_tien_phat_hien_duoc_o_muc_thuc_te() {
    // Pha 20% — một mức có thật trong lịch sử.
    let da_pha = Coinage {
        id: "veskar.silver".into(),
        face_value: 100,
        fineness: 800,
        original_fineness: 1_000,
        weight: 4_000,
        original_weight: 4_000,
    };
    assert!(
        da_pha.detectable_by(1_000),
        "một chuyên gia không nhận ra tiền pha 20% thì cơ chế phát hiện \
         không bao giờ chạy"
    );
    // Nhưng người thường thì chưa — khoảng giữa hai mốc là chỗ đáng chơi.
    assert!(!da_pha.detectable_by(50));
}

/// **`StateCapacity::coverage` từng bão hòa ở 1000 cho mọi huyện.**
///
/// Mọi huyện đều "phủ hoàn toàn", nên khoảng cách giữa trung tâm và biên giới
/// biến mất — và cả một trục chính trị đi cùng nó.
#[test]
fn regression_phu_song_nha_nuoc_khac_nhau_giua_trung_tam_va_bien() {
    let nn = StateCapacity {
        revenue: 400_000,
        officials: 60,
        corruption: 150,
        delay_per_hop: 10,
        distortion_per_hop: 100,
        // Hai mốc quy chiếu. Không có chúng thì mọi khu đều trả về 1000 — mô
        // hình vẫn chạy, test "không panic" vẫn xanh, và cả một trục chính trị
        // biến mất.
        full_coverage_cost_per_capita: 40,
        full_coverage_officials_per_1000: 3,
        districts: vec![
            District {
                id: "capital".into(),
                admin_distance: 0,
                population: 10_000,
            },
            District {
                id: "frontier".into(),
                admin_distance: 6,
                population: 10_000,
            },
        ],
    };

    let trung_tam = nn.coverage("capital");
    let bien_gioi = nn.coverage("frontier");
    assert!(
        trung_tam > bien_gioi,
        "trung tâm {trung_tam} không hơn biên giới {bien_gioi} — phủ sóng đã bão hòa"
    );
    assert!(
        bien_gioi < 1_000,
        "biên giới phủ hoàn toàn là dấu hiệu của một cái trần vô định"
    );
}

// ══════════ Lớp lỗi 3: mô hình đúng, hằng số làm nó bất động ══════════

/// **Phân kỳ loài từng quá chậm để đo được** ở thang thời gian của trò chơi.
///
/// Công thức snowball đúng, hằng số sai: 600 đời cách ly cho con lai còn 98%
/// khả năng sinh sản, nên `§9.5.5` — "cỗ máy tạo loài tốt nhất" — không bao giờ
/// tạo ra loài nào.
#[test]
fn regression_phan_ky_loai_do_duoc_o_thang_thoi_gian_tro_choi() {
    let g = secondary_contact(
        &IsolatedPopulation {
            id: "x".into(),
            route: SpeciationRoute::IsolationThenDivergence,
            effective_size: 800,
            generations: 600,
            selection_differential: 60,
        },
        400,
    );
    assert!(
        g.decline_is_measurable(),
        "600 đời cách ly mà con lai vẫn sinh sản bình thường — \
         portal không còn là cỗ máy tạo loài"
    );
}

/// **Diễn thế từng bị chặn bởi đất mà không có lối gỡ.**
///
/// Đất mọc 1‰ mỗi năm là đúng; nhưng nếu hệ hình thành đất cũng mất thì mảnh
/// đất kẹt vĩnh viễn, và người chơi không có việc gì làm với nó.
#[test]
fn regression_dat_bi_xoi_mon_van_co_loi_go() {
    let mut p = Patch::mature_forest(1);
    p.apply(&Event::Cleared);
    p.apply(&Event::Erosion { permille: 600 });
    assert!(p.blocked_by().is_some(), "phải bị chặn");

    // Khôi phục hệ hình thành đất là lối gỡ, và nó phải thật sự gỡ được.
    p.apply(&Event::ProcessRestored(Process::SoilFormation));
    for _ in 0..900 {
        p.apply(&Event::Time { years: 1 });
    }
    assert_eq!(
        p.stage,
        Stage::MatureForest,
        "khôi phục đúng quá trình mà vẫn không hồi thì lối gỡ chỉ là trang trí"
    );
}

// ══════════ Lớp lỗi 4: "chữa" một vế làm hỏng vế kia ══════════

/// **Rebase deadline từng nhân đồng loạt.**
///
/// Chữa được bệnh ủ dở, và làm mọi hợp đồng đáo hạn tức thì. Hai vế phải cùng
/// đúng trong **một** lần đi qua cổng — kiểm riêng từng vế sẽ cho qua cách sai.
#[test]
fn regression_rebase_khong_nhan_dong_loat() {
    let gaia = Clock::synchronous();
    let nhanh = Clock::new(Rate::per_tick(10));

    let cac = vec![
        TimedProcess {
            id: "disease.incubation".into(),
            deadline: Deadline::new(Tick(300), ClockDomain::Proper),
        },
        TimedProcess {
            id: "contract.loan".into(),
            deadline: Deadline::new(Tick(900), ClockDomain::WorldLocal),
        },
    ];
    let (moi, audit) = rebase_processes(&cac, &gaia, &nhanh).unwrap();

    // Tra theo id chứ không theo chỉ số: một lần đổi thứ tự đầu vào sẽ làm
    // bài này kiểm nhầm vế và vẫn xanh.
    let han = |ten: &str| {
        moi.iter()
            .find(|x| x.id == ten)
            .expect("tiến trình phải còn trong danh sách")
            .deadline
            .at
    };
    assert_eq!(
        han("disease.incubation"),
        Tick(300 * 10),
        "proper phải quy đổi"
    );
    assert_eq!(
        han("contract.loan"),
        Tick(900),
        "world_local phải giữ nguyên — nhân đồng loạt sẽ làm nợ đáo hạn tức thì"
    );
    assert_eq!(audit.changed().count(), 1, "đúng một miền được đổi");
}

/// **Uy tín tôn giáo từng tính theo tỉ lệ trả/chi phí.**
///
/// Năm mươi buổi giảng rẻ tiền bằng một chuyến hành hương, nên tín ngưỡng đắt
/// đỏ không còn lợi thế nào — và toàn bộ cơ chế "phô trương tốn kém" của
/// Henrich mất tác dụng.
#[test]
fn regression_uy_tin_ton_giao_theo_chi_phi_tuyet_doi() {
    let giang_dao = Rite {
        id: "sermon".into(),
        cost: 1,
        hard_to_fake: false,
        public: true,
    };
    let hanh_huong = Rite {
        id: "pilgrimage".into(),
        cost: 900,
        hard_to_fake: true,
        public: true,
    };
    let du = |rite: &str, paid| Observance {
        who: EntityId(1),
        rite: rite.to_owned(),
        paid,
        witnesses: 50,
    };

    let re = credibility(&giang_dao, &du("sermon", 1));
    let dat = credibility(&hanh_huong, &du("pilgrimage", 900));
    assert!(
        dat > re * 10,
        "một chuyến hành hương ({dat}) phải đáng tin hơn hẳn một buổi giảng \
         rẻ tiền ({re}) — tính theo tỉ lệ trả/chi phí thì hai cái bằng nhau"
    );
}
