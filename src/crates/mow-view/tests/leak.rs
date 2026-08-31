//! Test rò rỉ read model (`PC-15`, `§18.9`, `§P6.9.4`).
//!
//! Đây là **phiên bản frontend của prompt leak guard** ở `§P6.2`: nó chụp đúng
//! những byte sẽ đi ra dây và khẳng định không có gì trong đó thuộc về thứ
//! avatar chưa biết.
//!
//! Vì sao kiểm trên chuỗi JSON chứ không trên struct: một trường có thể lọt ra
//! qua `Serialize` mà không có getter nào, và một test đọc struct qua API sẽ
//! không bao giờ thấy nó. Client thì thấy.

use mow_action::perception::{CognitionContext, Observation, Sense};
use mow_core::{EntityId, Tick, Value};
use mow_math::{Unit, WorldPos};
use mow_view::{project, project_presences, Certainty, Lens, Mode, WorldTruth};

fn quan_sat(id: Option<u64>, fidelity_phan_nghin: i64) -> Observation {
    Observation {
        sense: Sense::Sight,
        at: WorldPos::new(10, 20, 0),
        identity: id.map(EntityId),
        signs: vec!["đang đi".into()],
        fidelity: Unit::from_frac(fidelity_phan_nghin, 1000).expect("trong [0,1]"),
        at_tick: Tick(100),
    }
}

fn ctx(self_id: u64, obs: Vec<Observation>) -> CognitionContext {
    CognitionContext {
        self_id: EntityId(self_id),
        now: Tick(100),
        observations: obs,
        known_actions: vec!["core.wait".into()],
        internal: vec![("hunger".into(), 400)],
    }
}

fn su_that() -> WorldTruth {
    let mut t = WorldTruth::new();
    t.set("name", Value::Text("Bram".into()), 1)
        .set("health", Value::Int(80), 2)
        .set("goal", Value::Text("giết Aren".into()), 3)
        .set("secret.debt", Value::Int(500), 4)
        .set("money", Value::Int(120), 5);
    t
}

/// Cốt lõi của `§18.9`: người chơi hóa thân **không** đọc được ý định của NPC.
#[test]
fn hoa_than_khong_doc_duoc_noi_tam_cua_nguoi_khac() {
    let lens = Lens::embodied(ctx(1, vec![quan_sat(Some(2), 900)]));
    let v = project(EntityId(2), &su_that(), &lens).expect("nhận ra Bram");

    let json = serde_json::to_string(&v).unwrap();
    for cam in ["giết Aren", "goal", "secret", "money"] {
        assert!(!json.contains(cam), "payload rò `{cam}`:\n{json}");
    }
    // Nhưng cái nhìn thấy được thì vẫn thấy.
    assert!(v.field("name").is_some());
    assert!(v.field("health").is_some());
}

/// Chỉ số của người khác là **ước đoán có sai số**, không bao giờ là sự thật.
#[test]
fn chi_so_nguoi_khac_luon_mang_nhan_phong_doan() {
    let lens = Lens::embodied(ctx(1, vec![quan_sat(Some(2), 900)]));
    let v = project(EntityId(2), &su_that(), &lens).unwrap();

    for (k, f) in v.fields() {
        assert!(
            matches!(f.certainty, Certainty::Belief { .. }),
            "trường `{k}` của người khác mang nhãn sự thật"
        );
    }
}

/// Nhìn rõ hơn thì tin chắc hơn. Không có điều này thì `fidelity` chỉ là trang trí.
#[test]
fn nhin_ro_hon_thi_tin_chac_hon() {
    let ro = Lens::embodied(ctx(1, vec![quan_sat(Some(2), 950)]));
    let mo = Lens::embodied(ctx(1, vec![quan_sat(Some(2), 200)]));

    let c_ro = project(EntityId(2), &su_that(), &ro).unwrap();
    let c_mo = project(EntityId(2), &su_that(), &mo).unwrap();

    let lay = |v: &mow_view::EntityView| match v.field("health").unwrap().certainty {
        Certainty::Belief { confidence } => confidence,
        Certainty::Truth => panic!("phải là phỏng đoán"),
    };
    assert!(lay(&c_ro) > lay(&c_mo));
}

/// Nhìn tận mặt lúc sáng rồi thoáng thấy lúc tối: cái biết vẫn là cái biết lúc sáng.
#[test]
fn quan_sat_ro_nhat_quyet_dinh_khong_phai_moi_nhat() {
    let lens = Lens::embodied(ctx(1, vec![quan_sat(Some(2), 950), quan_sat(Some(2), 100)]));
    let v = project(EntityId(2), &su_that(), &lens).unwrap();
    match v.field("health").unwrap().certainty {
        Certainty::Belief { confidence } => assert!(confidence > 900),
        Certainty::Truth => panic!("phải là phỏng đoán"),
    }
}

/// Nội tâm **của chính mình** thì đọc được, và là sự thật.
#[test]
fn noi_tam_cua_chinh_minh_thi_doc_duoc() {
    let lens = Lens::embodied(ctx(2, vec![]));
    let v = project(EntityId(2), &su_that(), &lens).unwrap();
    assert!(v.field("goal").is_some());
    assert_eq!(v.field("goal").unwrap().certainty, Certainty::Truth);
}

/// Không nhận ra thì trả `None`, **không** phải một view rỗng.
///
/// View rỗng vẫn nói cho client biết "thực thể này tồn tại và có id đó", và đó
/// đã là rò rỉ.
#[test]
fn khong_nhan_ra_thi_khong_co_view_nao_ca() {
    let lens = Lens::embodied(ctx(1, vec![quan_sat(None, 500)]));
    assert!(project(EntityId(2), &su_that(), &lens).is_none());
}

/// Bóng người trong sương ra dây **không mang id**.
///
/// Trả id sẽ là chính cái rò rỉ mà `§18.9` cấm: người chơi mở devtool, đọc id,
/// và biết bóng người là ai trong khi nhân vật của họ thì không.
#[test]
fn bong_nguoi_trong_suong_khong_mang_danh_tinh() {
    let lens = Lens::embodied(ctx(1, vec![quan_sat(None, 400)]));
    let ps = project_presences(&lens);
    assert_eq!(ps.len(), 1);

    let json = serde_json::to_string(&ps).unwrap();
    assert!(!json.contains("\"id\""), "bóng người mang id:\n{json}");
    assert!(!json.contains("Bram"));
    assert_eq!(lens.anonymous_sightings(), 1);
}

/// Nhà quan sát thấy sự thật của vùng đang xem — nhưng **không** đọc nội tâm.
///
/// Nếu đọc được thì `§10.2` chỉ còn là trang trí, và toàn bộ kịch tính của việc
/// *đoán* biến mất.
#[test]
fn quan_sat_thay_su_that_nhung_khong_doc_noi_tam() {
    let v = project(EntityId(2), &su_that(), &Lens::observer()).unwrap();
    assert_eq!(v.field("health").unwrap().certainty, Certainty::Truth);
    assert!(v.field("goal").is_none(), "chế độ quan sát đọc được ý định");
    assert!(v.field("secret.debt").is_none());
}

/// True God thấy mọi thứ, cộng provenance.
#[test]
fn true_god_thay_moi_thu_va_ca_nguon_goc() {
    let v = project(EntityId(2), &su_that(), &Lens::true_god()).unwrap();
    assert!(v.field("goal").is_some());
    assert_eq!(v.field("goal").unwrap().provenance, Some(3));
}

/// Provenance **chỉ** True God. Ở chế độ khác nó phải biến mất khỏi payload,
/// không phải gửi rồi để client ẩn.
#[test]
fn provenance_khong_ro_ra_o_che_do_khac() {
    for lens in [
        Lens::observer(),
        Lens::embodied(ctx(1, vec![quan_sat(Some(2), 900)])),
    ] {
        let v = project(EntityId(2), &su_that(), &lens).unwrap();
        for (k, f) in v.fields() {
            assert!(f.provenance.is_none(), "`{k}` rò provenance");
        }
        let json = serde_json::to_string(&v).unwrap();
        assert!(
            !json.contains("provenance"),
            "payload có khóa provenance:\n{json}"
        );
    }
}

/// Thuộc tính mới **mặc định là riêng tư**.
///
/// Chiều này quan trọng: nếu mặc định là công khai, thì mỗi thuộc tính mới là
/// một rò rỉ tiềm năng cho tới khi có ai đó nhớ ra phải cấm nó.
#[test]
fn thuoc_tinh_noi_tam_theo_tien_to_cung_bi_chan() {
    let mut t = WorldTruth::new();
    t.set("secret.affair", Value::Text("với Cai".into()), 9)
        .set("belief.about.aren", Value::Int(-800), 10)
        .set("plan.step", Value::Text("đợi trời tối".into()), 11);

    let lens = Lens::embodied(ctx(1, vec![quan_sat(Some(2), 900)]));
    let v = project(EntityId(2), &t, &lens).unwrap();
    assert!(v.fields().is_empty(), "rò: {:?}", v.fields().keys());
}

/// Ba chế độ có tên ổn định trên dây — client và server phải khớp.
#[test]
fn ten_che_do_on_dinh() {
    assert_eq!(Mode::Embodied.as_str(), "embodied");
    assert_eq!(Mode::Observer.as_str(), "observer");
    assert_eq!(Mode::TrueGod.as_str(), "true_god");
}

/// Chụp **toàn bộ payload một phiên** và khẳng định không rò gì (`§P6.9.4`).
#[test]
fn chup_toan_bo_payload_mot_phien_hoa_than_va_khang_dinh_khong_ro() {
    let lens = Lens::embodied(ctx(
        1,
        vec![
            quan_sat(Some(2), 800),
            quan_sat(None, 300),
            quan_sat(Some(4), 500),
        ],
    ));

    // Mỗi người một sự thật riêng. Dùng chung một bảng cho cả làng sẽ làm test
    // này vô nghĩa: avatar chiếu **chính nó** thì thấy nội tâm của nó, và nếu
    // nội tâm đó lại là của Bram thì chuỗi cấm xuất hiện một cách hợp lệ.
    let cua_avatar = {
        let mut t = WorldTruth::new();
        t.set("name", Value::Text("Aren".into()), 1)
            .set("health", Value::Int(95), 2)
            .set("goal", Value::Text("tìm nước".into()), 3);
        t
    };

    // Cả làng có sáu người; avatar chỉ nhận ra hai, cộng chính mình.
    let mut phien: Vec<String> = Vec::new();
    for id in 1..=6u64 {
        let truth = if id == 1 { &cua_avatar } else { &su_that() };
        if let Some(v) = project(EntityId(id), truth, &lens) {
            phien.push(serde_json::to_string(&v).unwrap());
        }
    }
    phien.push(serde_json::to_string(&project_presences(&lens)).unwrap());

    let tat_ca = phien.join(
        "
",
    );

    // Không một byte nào của nội tâm người khác được xuất hiện.
    for cam in ["giết Aren", "secret", "\"money\"", "provenance"] {
        assert!(
            !tat_ca.contains(cam),
            "phiên rò `{cam}`:
{tat_ca}"
        );
    }
    // `goal` chỉ được xuất hiện đúng một lần: của chính avatar.
    assert_eq!(
        tat_ca.matches("\"goal\"").count(),
        1,
        "rò goal:
{tat_ca}"
    );
    assert!(tat_ca.contains("tìm nước"));

    // Và đúng ba thực thể ra dây: chính mình, và hai người nhận ra được.
    assert_eq!(
        phien.len() - 1,
        3,
        "số thực thể ra dây sai:
{tat_ca}"
    );
}
