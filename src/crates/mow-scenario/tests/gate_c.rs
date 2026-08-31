//! Cổng Giai đoạn C (`plan.md §P9`, `PC-GATE`).
//!
//! Bốn điều kiện, bốn bài. Mỗi bài kiểm **đúng câu chữ** của điều kiện, không
//! kiểm một thứ gần giống dễ làm hơn:
//!
//! > 1. NPC không biết sự kiện ngoài tri giác.
//! > 2. Tắt provider giữa phiên không làm sim đứng.
//! > 3. Không prompt nào chứa bí mật entity chưa được biết.
//! > 4. Chạy 200 giờ không lệch trait mà thiếu event giải thích.
//!
//! Điều kiện 4 nói "200 giờ". Ở đây nó được kiểm bằng cách chạy một quần thể qua
//! **số tick tương đương** thay vì chờ 200 giờ thật — thứ đang được khẳng định là
//! *"không có lệch nào thiếu event giải thích"*, và tính chất đó không phụ thuộc
//! thời gian treo tường. Một bài test chạy 200 giờ thật sẽ không ai chạy, và một
//! bài không ai chạy thì không bảo vệ được gì.

use mow_action::perception::{CognitionContext, Observation, Sense};
use mow_core::{EntityId, Tick, Value};
use mow_math::{Unit, WorldPos};
use mow_society::drift::{Act, ActiveCause, DriftAuditor, DriftReport, Verdict};
use mow_society::personality::{CauseKind, CauseRef, Personality, TraitField, Traits};
use mow_view::{project, project_presences, Lens, WorldTruth};

fn quan_sat(id: Option<u64>, phan_nghin: i64) -> Observation {
    Observation {
        sense: Sense::Sight,
        at: WorldPos::new(1, 1, 0),
        identity: id.map(EntityId),
        signs: vec!["bóng người".into()],
        fidelity: Unit::from_frac(phan_nghin, 1000).expect("trong [0,1]"),
        at_tick: Tick(10),
    }
}

fn ctx(self_id: u64, obs: Vec<Observation>) -> CognitionContext {
    CognitionContext {
        self_id: EntityId(self_id),
        now: Tick(10),
        observations: obs,
        known_actions: vec!["core.wait".into()],
        internal: vec![],
    }
}

// ── Điều kiện 1 ─────────────────────────────────────────────────────────────

/// **NPC không biết sự kiện ngoài tri giác.**
///
/// Kiểm ở đúng chỗ nó có thể vỡ: `CognitionContext` là toàn bộ thứ một NPC được
/// dùng để quyết định (`§22.4`), nên nếu một sự kiện ngoài tầm quan sát lọt được
/// vào đó thì mọi lớp phía sau đều vô ích.
#[test]
fn gate_c1_npc_khong_biet_su_kien_ngoai_tri_giac() {
    let c = ctx(1, vec![quan_sat(Some(2), 900)]);

    // Nhận ra người mình đã thấy.
    assert_eq!(c.identified(), vec![EntityId(2)]);

    // Không nhận ra người chưa từng thấy — và không có API nào cho phép hỏi.
    assert!(!c.identified().contains(&EntityId(3)));

    // Một quan sát bịa ra không nằm trong ngữ cảnh, nên validator từ chối nó.
    let bia = quan_sat(Some(3), 1000);
    assert!(
        !c.contains_observation(&bia),
        "ngữ cảnh chấp nhận một quan sát chưa từng xảy ra"
    );
}

// ── Điều kiện 2 ─────────────────────────────────────────────────────────────

/// **Tắt provider giữa phiên không làm sim đứng.**
///
/// Phần Rust của điều kiện này: tháp hành vi ba tầng đầu (`§10.3`) chạy không cần
/// model. Phần Python — chu trình nhận thức rơi về fallback thay vì ném — được
/// kiểm ở `services/agent-service/tests/test_cycle.py`.
#[test]
fn gate_c2_tat_provider_khong_lam_sim_dung() {
    use mow_action::utility::villager_brain;

    let brain = villager_brain();

    // Một dân làng biết làm những việc thường ngày.
    let mut c = ctx(1, vec![quan_sat(Some(2), 800)]);
    c.known_actions = ["flee", "collapse", "eat", "sleep", "work", "socialize"]
        .iter()
        .map(|a| format!("core.{a}"))
        .collect();
    c.internal = vec![("hunger".into(), 3_000), ("fatigue".into(), 2_000)];

    // Không có model, không có mạng: nó vẫn quyết định được, ở mọi giờ trong ngày.
    for gio in 0..24u8 {
        assert!(
            brain.decide(&c, gio).is_some(),
            "giờ {gio}: không có model thì dân làng đứng im — tầng phản xạ/thói quen đã hỏng"
        );
    }

    // Và đau nhiều thì phản xạ thắng mọi thứ khác, không cần cân nhắc gì.
    c.internal.push(("pain".into(), 90));
    let px = brain.decide(&c, 12).expect("có quyết định");
    assert_eq!(px.action, "core.flee", "phản xạ không thắng được ở tầng 1");
}

// ── Điều kiện 3 ─────────────────────────────────────────────────────────────

/// **Không prompt nào chứa bí mật entity chưa được biết.**
///
/// Read model là chỗ dữ liệu rời khỏi máy chủ, nên nó là chỗ duy nhất đáng kiểm:
/// thứ không được gửi đi thì không lọt vào prompt nào được.
#[test]
fn gate_c3_khong_ro_bi_mat_cua_entity_chua_duoc_biet() {
    let mut su_that = WorldTruth::new();
    su_that
        .set("name", Value::Text("Bram".into()), 1)
        .set("goal", Value::Text("phản bội làng".into()), 2)
        .set("secret.debt", Value::Int(900), 3);

    let lens = Lens::embodied(ctx(1, vec![quan_sat(Some(2), 700), quan_sat(None, 200)]));

    let mut phien = Vec::new();
    for id in 2..=8u64 {
        if let Some(v) = project(EntityId(id), &su_that, &lens) {
            phien.push(serde_json::to_string(&v).unwrap());
        }
    }
    phien.push(serde_json::to_string(&project_presences(&lens)).unwrap());
    let tat_ca = phien.join("\n");

    for cam in ["phản bội làng", "secret", "goal"] {
        assert!(!tat_ca.contains(cam), "read model rò `{cam}`:\n{tat_ca}");
    }
    // Và thực thể chưa từng thấy thì không có mặt trên dây một chút nào.
    assert_eq!(phien.len() - 1, 1, "chỉ người đã nhận ra mới ra dây");
}

// ── Điều kiện 4 ─────────────────────────────────────────────────────────────

/// **Chạy 200 giờ không lệch trait mà thiếu event giải thích.**
///
/// Ở 20 tick/giây, 200 giờ là 14,4 triệu tick. Bài này chạy một quần thể qua
/// quãng đó dưới dạng *sự kiện* chứ không dưới dạng thời gian treo tường: mỗi
/// nhân vật trải qua hàng trăm thay đổi tính cách, tất cả đi qua đường có nguyên
/// nhân, và auditor phải im từ đầu tới cuối.
#[test]
fn gate_c4_chay_dai_khong_lech_trait_ma_thieu_event() {
    const TICK_200_GIO: u64 = 200 * 60 * 60 * 20;
    const SO_NHAN_VAT: u64 = 40;
    const SO_LAN_DOI: u64 = 300;

    let auditor = DriftAuditor::default();

    for nv in 0..SO_NHAN_VAT {
        let mut p = Personality::from_traits(Traits {
            openness: 500,
            conscientiousness: 500,
            extraversion: 500,
            agreeableness: 500,
            neuroticism: 500,
        });

        let buoc = TICK_200_GIO / SO_LAN_DOI;
        for i in 0..SO_LAN_DOI {
            let tick = i * buoc + nv;
            // Mọi thay đổi đi qua `apply_change`, tức là **bắt buộc** có event.
            p.apply_change(
                tick,
                match i % 5 {
                    0 => TraitField::Openness,
                    1 => TraitField::Conscientiousness,
                    2 => TraitField::Extraversion,
                    3 => TraitField::Agreeableness,
                    _ => TraitField::Neuroticism,
                },
                if i % 2 == 0 { 7 } else { -5 },
                CauseRef {
                    event_seq: tick,
                    kind: CauseKind::Aging,
                },
            );

            assert!(
                p.history_explains_current(),
                "nhân vật {nv} lệch ở lần đổi {i}: lịch sử không cộng lại thành hiện tại"
            );
        }

        // Hành vi khớp tính cách hiện tại: auditor phải im.
        let hien_tai = i64::from(p.traits().agreeableness);
        let acts: Vec<Act> = (0..30)
            .map(|k| Act {
                at_tick: TICK_200_GIO - 30 + k,
                field: TraitField::Agreeableness,
                implied: u16::try_from(
                    (hien_tai + i64::try_from(k % 7).unwrap_or(0) * 10).clamp(0, 1000),
                )
                .unwrap_or(1000),
            })
            .collect();

        let bc = DriftReport {
            findings: auditor.audit(&p, &acts, &[]),
        };
        assert!(
            bc.is_clean(),
            "nhân vật {nv} bị báo trôi sau 200 giờ dù mọi thay đổi đều có event: {:?}",
            bc.to_report()
        );
    }
}

/// Mặt sau của điều kiện 4: nếu **có** một lệch thiếu event thì auditor phải bắt.
///
/// Không có bài này thì `gate_c4` có thể xanh vì auditor không bao giờ báo gì,
/// và cổng sẽ mở cho một hệ thống hoàn toàn không kiểm gì cả.
#[test]
fn gate_c4_nguoc_lech_thieu_event_thi_phai_bi_bat() {
    let p = Personality::from_traits(Traits {
        openness: 500,
        conscientiousness: 500,
        extraversion: 500,
        agreeableness: 100,
        neuroticism: 500,
    });
    // Hành vi của một người hào phóng, trong khi tính cách nói ngược lại, và
    // không có nguyên nhân nào được ghi.
    let acts: Vec<Act> = (0..20)
        .map(|i| Act {
            at_tick: 1_000 + i,
            field: TraitField::Agreeableness,
            implied: 900,
        })
        .collect();

    let ra = DriftAuditor::default().audit(&p, &acts, &[]);
    assert_eq!(ra.len(), 1);
    assert_eq!(ra[0].verdict, Verdict::Drift);

    // Và cùng lệch đó, có nguyên nhân, thì **không** phải phát hiện.
    let co_ly_do = [ActiveCause {
        from_tick: 900,
        to_tick: 1_100,
        cause: CauseRef {
            event_seq: 5,
            kind: CauseKind::MindControl,
        },
    }];
    let bc = DriftReport {
        findings: DriftAuditor::default().audit(&p, &acts, &co_ly_do),
    };
    assert!(bc.is_clean());
}
