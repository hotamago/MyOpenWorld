//! Test gateway và thời điểm áp.
//!
//! Bài quan trọng nhất là [`toc_do_mang_khong_lot_vao_the_gioi`]. Nó kiểm
//! chứng đúng cái lỗi mà `§20.2.2` được viết ra để chữa.

use mow_core::{EntityId, Tick};
use mow_llm::admission::{AdmissionError, AdmissionLedger, CallState};
use mow_llm::client::{Gateway, LlmError, Mode, ModelClient, Request, Response};
use mow_math::{CanonicalHash, StateHash};

fn yeu_cau(prompt: &str, noi_dung: &str) -> Request {
    Request {
        prompt_id: prompt.to_owned(),
        prompt_version: 1,
        model: "claude-opus-5".to_owned(),
        rendered: noi_dung.to_owned(),
        max_output_tokens: 800,
    }
}

fn so() -> AdmissionLedger {
    AdmissionLedger::new()
}

fn len_lich(s: &mut AdmissionLedger, id: u64, t: u64, d: u64) {
    s.schedule(
        id,
        EntityId(1),
        Tick(t),
        d,
        "cognition.plan",
        2,
        StateHash::ZERO,
        "claude-opus-5",
    )
    .expect("lên lịch được");
    s.mark_sent(id).expect("gửi được");
}

// ─────────────────────────────────────────────────────────────────────────────
// §20.2.2 — thời điểm áp kết quả
// ─────────────────────────────────────────────────────────────────────────────

/// **Bài quan trọng nhất.** Mô hình nhanh và mô hình chậm phải cho cùng một
/// thế giới.
#[test]
fn toc_do_mang_khong_lot_vao_the_gioi() {
    let d = 10;

    // Thế giới A: kết quả về ngay tick 1.
    let mut a = so();
    len_lich(&mut a, 1, 100, d);
    a.record_response(1, "claude-opus-5", "đi về phía đông".into())
        .unwrap();
    let ap_a: Vec<_> = (100..=115).flat_map(|t| a.admit_due(Tick(t))).collect();

    // Thế giới B: kết quả về muộn, ở tick 109 — vẫn kịp trước T+D = 110.
    let mut b = so();
    len_lich(&mut b, 1, 100, d);
    let mut ap_b = Vec::new();
    for t in 100..=115 {
        if t == 109 {
            b.record_response(1, "claude-opus-5", "đi về phía đông".into())
                .unwrap();
        }
        ap_b.extend(b.admit_due(Tick(t)));
    }

    assert_eq!(ap_a.len(), 1);
    assert_eq!(ap_b.len(), 1);
    assert_eq!(
        ap_a[0].call.admission_tick, ap_b[0].call.admission_tick,
        "thời điểm áp phải giống nhau bất kể kết quả về lúc nào"
    );
    assert_eq!(ap_a[0].call.admission_tick, Tick(110));
    assert_eq!(
        ap_a[0].call.state_hash(),
        ap_b[0].call.state_hash(),
        "hai thế giới phải giống hệt nhau"
    );
}

#[test]
fn ket_qua_ve_som_van_phai_cho_toi_t_cong_d() {
    let mut s = so();
    len_lich(&mut s, 1, 100, 10);
    s.record_response(1, "claude-opus-5", "xong".into())
        .unwrap();

    // Trước T+D thì chưa áp gì cả.
    for t in 100..110 {
        assert!(
            s.admit_due(Tick(t)).is_empty(),
            "áp sớm tại tick {t} — mô hình nhanh đang làm nhân vật nhanh theo"
        );
    }
    let ap = s.admit_due(Tick(110));
    assert_eq!(ap.len(), 1);
    assert!(ap[0].used_response);
    assert_eq!(ap[0].call.state, CallState::Accepted);
}

#[test]
fn khong_kip_thi_fallback_dung_tai_t_cong_d() {
    let mut s = so();
    len_lich(&mut s, 1, 100, 10);
    // Không có kết quả.
    let ap = s.admit_due(Tick(110));
    assert_eq!(ap.len(), 1);
    assert!(!ap[0].used_response, "phải dùng hành vi dự phòng");
    assert_eq!(ap[0].call.state, CallState::Fallback);
}

#[test]
fn ket_qua_ve_muon_sau_khi_da_fallback_bi_bo_qua() {
    // Nếu không bỏ qua, nhân vật sẽ hành động hai lần cho một lần suy nghĩ.
    let mut s = so();
    len_lich(&mut s, 1, 100, 10);
    s.admit_due(Tick(110));
    assert_eq!(s.get(1).unwrap().state, CallState::Fallback);

    let da_nhan = s
        .record_response(1, "claude-opus-5", "muộn quá rồi".into())
        .unwrap();
    assert!(!da_nhan, "kết quả về muộn phải bị bỏ qua");
    assert_eq!(s.get(1).unwrap().state, CallState::Fallback);
    assert!(s.admit_due(Tick(200)).is_empty(), "không được áp lần hai");
}

#[test]
fn dieu_kien_tien_de_mat_thi_huy() {
    let mut s = so();
    len_lich(&mut s, 1, 100, 10);
    s.cancel(1).unwrap();
    assert_eq!(s.get(1).unwrap().state, CallState::Cancelled);
    assert!(s.admit_due(Tick(110)).is_empty(), "đã hủy thì không áp");
    assert!(s.cancel(1).is_err(), "hủy hai lần là lỗi");
}

#[test]
fn do_tre_bang_0_bi_tu_choi() {
    let mut s = so();
    let e = s
        .schedule(1, EntityId(1), Tick(0), 0, "p", 1, StateHash::ZERO, "m")
        .expect_err("phải bị từ chối");
    assert_eq!(e, AdmissionError::ZeroLatency);
}

#[test]
fn trung_request_id_bi_tu_choi() {
    let mut s = so();
    len_lich(&mut s, 1, 100, 10);
    let e = s
        .schedule(1, EntityId(2), Tick(100), 10, "p", 1, StateHash::ZERO, "m")
        .expect_err("phải bị từ chối");
    assert_eq!(e, AdmissionError::Duplicate(1));
}

#[test]
fn ap_theo_thu_tu_request_id_khong_theo_thu_tu_ket_qua_ve() {
    let mut s = so();
    for id in [3u64, 1, 2] {
        len_lich(&mut s, id, 100, 10);
    }
    // Kết quả về theo thứ tự lộn xộn.
    for id in [2u64, 3, 1] {
        s.record_response(id, "m", format!("kq{id}")).unwrap();
    }
    let ap = s.admit_due(Tick(110));
    assert_eq!(
        ap.iter().map(|a| a.call.request_id).collect::<Vec<_>>(),
        vec![1, 2, 3],
        "thứ tự áp phải xác định, không theo thứ tự kết quả về"
    );
}

#[test]
fn ghi_model_that_su_da_dung_khong_phai_model_da_yeu_cau() {
    // §20.10: khi gateway hạ cấp mô hình, log phải nói cái gì đã thật sự sinh
    // ra câu trả lời — nếu không, đọc lại log sẽ dẫn tới kết luận sai.
    let mut s = so();
    len_lich(&mut s, 1, 100, 10);
    s.record_response(1, "claude-haiku-4-5", "trả lời rẻ tiền".into())
        .unwrap();
    let ap = s.admit_due(Tick(110));
    assert_eq!(ap[0].call.model, "claude-haiku-4-5");
}

#[test]
fn ket_qua_chua_ap_khong_anh_huong_state_hash() {
    // Nếu có, tốc độ mạng lại lọt vào thế giới qua cửa sau.
    let mut a = so();
    len_lich(&mut a, 1, 100, 10);
    let h_truoc = a.state_hash();

    a.record_response(1, "claude-opus-5", "đã về nhưng chưa tới hạn".into())
        .unwrap();
    assert_eq!(
        a.state_hash(),
        h_truoc,
        "kết quả đã về nhưng chưa áp không được đổi state hash"
    );

    a.admit_due(Tick(110));
    assert_ne!(a.state_hash(), h_truoc, "sau khi áp thì phải đổi");
}

#[test]
fn loi_goi_treo_vinh_vien_bi_don_di() {
    // Sổ nằm trong state hash; một lời gọi treo mãi là rò rỉ bộ nhớ **và** một
    // state hash lớn dần vô hạn.
    let mut s = so();
    len_lich(&mut s, 1, 100, 10);
    s.cancel(1).unwrap();
    len_lich(&mut s, 2, 100, 10);

    assert_eq!(s.expire_older_than(Tick(10_000), 100), 1);
    assert_eq!(s.get(2).unwrap().state, CallState::Expired);
    assert_eq!(s.in_flight(), 0);
    assert_eq!(s.prune_terminal(), 2);
    assert!(s.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Bốn chế độ
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn stub_khong_can_mang() {
    let mut g = Gateway::stub().with_stub("cognition.plan", "đi kiếm ăn");
    let r = g.call(&yeu_cau("cognition.plan", "bạn đói")).unwrap();
    assert_eq!(r.text, "đi kiếm ăn");
    assert_eq!(r.model, "stub");
}

#[test]
fn stub_thieu_cau_tra_loi_thi_bao_loi_khong_tra_chuoi_rong() {
    let mut g = Gateway::stub();
    let e = g.call(&yeu_cau("khong.co", "x")).expect_err("phải lỗi");
    assert!(matches!(e, LlmError::NoStub(_)), "{e}");
}

#[test]
fn replay_thieu_ban_ghi_la_loi_khong_phai_ly_do_goi_that() {
    // Cả hai lối thoát — gọi thật, hay trả lời tạm — đều biến một bài test
    // xanh thành một lời nói dối.
    let mut g = Gateway::stub().with_stub("p", "có stub đây");
    g.set_mode(Mode::Replay);
    let e = g.call(&yeu_cau("p", "x")).expect_err("phải lỗi");
    assert!(matches!(e, LlmError::NoCassette { .. }), "{e}");
}

#[test]
fn ghi_roi_phat_lai_duoc() {
    struct Provider;
    impl ModelClient for Provider {
        fn mode(&self) -> Mode {
            Mode::Live
        }
        fn call(&mut self, req: &Request) -> Result<Response, LlmError> {
            Ok(Response {
                text: format!("trả lời cho `{}`", req.rendered),
                model: req.model.clone(),
                input_tokens: 10,
                output_tokens: 5,
            })
        }
    }

    let d = tempfile::tempdir().unwrap();
    let mut g = Gateway::stub()
        .with_upstream(Box::new(Provider))
        .with_cassette_dir(d.path());
    g.set_mode(Mode::Record);

    let req = yeu_cau("cognition.plan", "bạn đang đói");
    let goc = g.call(&req).unwrap();

    // Phát lại từ file vừa ghi.
    let mut g2 = Gateway::stub();
    g2.set_mode(Mode::Replay);
    let n = g2
        .load_cassettes(d.path().join("cognition.plan.cassette.jsonl"))
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(g2.call(&req).unwrap(), goc);
}

#[test]
fn doi_model_thi_khong_trung_ban_ghi_cu() {
    // Thiếu `model` trong hash yêu cầu thì đổi mô hình vẫn trúng bản ghi cũ,
    // và bài test sẽ xanh trong khi chưa bao giờ chạy trên mô hình mới.
    let a = yeu_cau("p", "x");
    let mut b = a.clone();
    b.model = "claude-haiku-4-5".to_owned();
    assert_ne!(a.hash(), b.hash());
}

#[test]
fn doi_phien_ban_prompt_thi_khong_trung_ban_ghi_cu() {
    let a = yeu_cau("p", "x");
    let mut b = a.clone();
    b.prompt_version = 2;
    assert_ne!(a.hash(), b.hash());
}

#[test]
fn live_khong_co_provider_thi_bao_loi_ro_rang() {
    let mut g = Gateway::stub();
    g.set_mode(Mode::Live);
    let e = g.call(&yeu_cau("p", "x")).expect_err("phải lỗi");
    assert!(matches!(e, LlmError::NoProvider(Mode::Live)), "{e}");
}

#[test]
fn set_mode_tra_ve_che_do_cu() {
    let mut g = Gateway::stub();
    assert_eq!(g.mode(), Mode::Stub);
    assert_eq!(g.set_mode(Mode::Replay), Mode::Stub);
    assert_eq!(g.mode(), Mode::Replay);
}
