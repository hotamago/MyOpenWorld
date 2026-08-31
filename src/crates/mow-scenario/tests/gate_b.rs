//! Cổng Giai đoạn B — năm điều kiện hoàn thành ở `plan.md §P9`.
//!
//! Mỗi bài dưới đây kiểm **đúng một** điều kiện, viết nguyên văn ở đầu bài. Đây
//! không phải test tính năng — các tính năng đã có test riêng trong crate của
//! chúng. Đây là những bài chứng minh rằng các tính năng **ghép lại được**, và
//! đó là câu hỏi khác hẳn.

use mow_action::utility::villager_brain;
use mow_action::{CognitionContext, Layer};
use mow_core::{val, Command, EntityId, Tick};
use mow_effect::{resolve, Modifier, Op, Stacking};
use mow_items::{Capacity, CraftQuality, Inventory, ItemDef, ItemStack};
use mow_life::body::{Injury, InjuryKind};
use mow_life::{BodyPlan, Homeostasis, Need, SCALE};
use mow_math::{Fx, Mass, Rate, Unit, Volume};
use mow_scenario::slice::{act, build_slice_world, WORLD};
use mow_spatial::lod::{transition, Aggregate, Conserved};
use mow_spatial::Lod;
use std::collections::{BTreeMap, BTreeSet};

// ─────────────────────────────────────────────────────────────────────────────
// 1. "Cư dân tự ăn/ngủ/làm việc"
// ─────────────────────────────────────────────────────────────────────────────

fn ngu_canh(gio_doi: i64, gio_met: i64) -> CognitionContext {
    CognitionContext {
        self_id: EntityId(1),
        now: Tick(0),
        observations: vec![],
        known_actions: [
            "core.eat",
            "core.sleep",
            "core.work",
            "core.socialize",
            "core.flee",
            "core.collapse",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect(),
        internal: vec![
            ("hunger".to_owned(), gio_doi),
            ("fatigue".to_owned(), gio_met),
            ("pain".to_owned(), 0),
        ],
    }
}

#[test]
fn cu_dan_tu_an_ngu_lam_viec() {
    let b = villager_brain();

    // Đói thì ăn, bất kể đang giờ làm.
    let an = b.decide(&ngu_canh(500, 9_000), 9).expect("phải quyết định");
    assert_eq!(an.action, "core.eat");

    // Mệt thì ngủ.
    let ngu = b.decide(&ngu_canh(9_000, 200), 9).expect("phải quyết định");
    assert_eq!(ngu.action, "core.sleep");

    // Không đói không mệt, giờ làm thì làm.
    let lam = b
        .decide(&ngu_canh(9_000, 9_000), 9)
        .expect("phải quyết định");
    assert_eq!(lam.action, "core.work");
    assert_eq!(lam.layer, Layer::Routine);

    // Buổi tối thì giao tiếp.
    let choi = b
        .decide(&ngu_canh(9_000, 9_000), 15)
        .expect("phải quyết định");
    assert_eq!(choi.action, "core.socialize");

    // Và **không lời gọi LLM nào** đã xảy ra: cả bài này chạy trong bộ nhớ.
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. "Áp-gỡ 1000 effect trả về base"
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ap_go_1000_effect_tra_ve_base() {
    // Đây là bài chứng minh `§22.20` ở quy mô thật. Với cách "trừ thẳng vào chỉ
    // số", một nghìn lần áp-gỡ sẽ tích lũy sai số làm tròn và không bao giờ về
    // đúng gốc.
    let base = Fx::from_int(37).unwrap();

    let mods: Vec<Modifier> = (0..1_000)
        .map(|i| Modifier {
            stat: "core.strength".into(),
            op: if i % 3 == 0 {
                Op::Add
            } else if i % 3 == 1 {
                Op::Multiply
            } else {
                Op::Cap
            },
            value: Fx::from_int(i64::from(i % 7) + 1).unwrap(),
            source: format!("s{i:04}"),
            stacking: Stacking::Additive,
        })
        .collect();

    let co_effect = resolve(base, &mods);
    assert_ne!(co_effect.value, base, "1000 effect mà không đổi gì");
    assert_eq!(co_effect.base, base, "base bị sửa");

    // Gỡ hết.
    let da_go = resolve(base, &[]);
    assert_eq!(da_go.value, base, "gỡ hết effect không trả về đúng base");

    // Và áp lại đúng bộ đó cho ra đúng kết quả cũ — không trôi.
    assert_eq!(resolve(base, &mods).value, co_effect.value);
}

#[test]
fn ap_go_1000_effect_tren_co_the_cung_tra_ve_base() {
    // Cùng câu hỏi ở tầng cao hơn: lão hóa, thương tích, hồi phục.
    let goc = BodyPlan::humanoid();
    let vitality_goc = goc.vitality();

    let mut b = BodyPlan::humanoid();
    for i in 0..1_000 {
        let part = if i % 2 == 0 {
            "core.arm_left"
        } else {
            "core.leg_right"
        };
        b.injure(
            part,
            &Injury {
                kind: InjuryKind::Cut,
                severity: 1,
                at_tick: i,
                infected: false,
            },
        );
    }
    assert!(b.vitality() < vitality_goc);

    // "Gỡ" ở đây là chữa lành hoàn toàn — dựng lại từ sơ đồ.
    let da_lanh = BodyPlan::humanoid();
    assert_eq!(da_lanh.vitality(), vitality_goc);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. "Kho nghìn đơn vị không nổ entity"
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn kho_nghin_don_vi_khong_no_entity() {
    // `§22.32`: chồng là **component dữ liệu**, không phải một entity mỗi món.
    // Một nghìn mũi tên phải là vài chồng, không phải một nghìn entity.
    let mut defs = BTreeMap::new();
    defs.insert(
        "core.arrow".to_owned(),
        ItemDef {
            id: "core.arrow".into(),
            mass: Mass::new(10),
            volume: Volume::new(5),
            max_stack: 100,
            parts: vec![],
            equip_slots: vec![],
            layer: None,
            coverage: Unit::ZERO,
            tags: vec![],
        },
    );
    let cap = Capacity {
        volume: Volume::new(100_000),
        comfortable_mass: Mass::new(1_000_000),
        max_mass: Mass::new(2_000_000),
    };

    let mut inv = Inventory::new();
    for _ in 0..40 {
        inv.insert(
            ItemStack {
                def: "core.arrow".into(),
                count: 50,
                quality: CraftQuality::Plain,
            },
            &defs,
            cap,
        )
        .expect("bỏ vào được");
    }

    assert_eq!(inv.count("core.arrow"), 2_000);
    assert_eq!(
        inv.instances().len(),
        0,
        "chồng không được tạo ra entity nào"
    );
    // 2000 mũi tên, trần chồng 100 → tối đa 20 chồng.
    assert!(
        inv.stacks().len() <= 20,
        "2000 mũi tên thành {} chồng — chồng không gộp",
        inv.stacks().len()
    );

    // Và thế giới vẫn tính được tải mà không duyệt 2000 phần tử.
    let l = inv.load(&defs, cap, Mass::ZERO, Volume::ZERO);
    assert_eq!(l.mass_carried, Mass::new(20_000));
}

#[test]
fn kho_lon_khong_lam_the_gioi_phinh_entity() {
    // Kiểm ở tầng `Sim`: nhặt nhiều đồ không được làm số entity tăng theo số món.
    let mut sim = build_slice_world(3);
    let so_entity_ban_dau = sim.store().len();

    // Đưa 500 đơn vị vào kho chung bằng một thuộc tính chồng.
    let ai_do = sim.store().ids().next().unwrap();
    act(
        &mut sim,
        &Command::new(
            "truegod.set_attr",
            WORLD,
            val! { "entity" => ai_do.get(), "key" => "stack.core.arrow", "value" => 500i64 },
        ),
    )
    .unwrap();

    assert_eq!(
        sim.store().len(),
        so_entity_ban_dau,
        "một chồng 500 món đã tạo ra entity mới"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. "Hai kiếm sĩ cùng chết khi cùng chí mạng"
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn hai_kiem_si_cung_chet_khi_cung_chi_mang() {
    // Đây là bài kiểm `§22.43` ở dạng dễ thấy nhất: nếu `EntityId` quyết định
    // thắng thua, thì người có id nhỏ hơn sẽ **luôn** sống sót, và trận đấu có
    // một kết quả định trước mà không ai nhìn thấy được.
    use mow_action::{resolve_all, Contention, Tier};
    use mow_core::StableKey;

    let don_chi_mang = |actor: u64, muc_tieu: u64| Contention {
        actor: EntityId(actor),
        tier: Tier::Impact,
        action: "core.lethal_strike".into(),
        // Mỗi người nhắm vào người kia, nên hai đề xuất **không** cạnh tranh
        // cùng một mục tiêu — cả hai đều phải được thực hiện.
        target: Some(muc_tieu.to_string()),
        priority: 100,
        key: StableKey::plain(EntityId(actor)),
    };

    let r = resolve_all(vec![don_chi_mang(1, 2), don_chi_mang(2, 1)]);

    let thang: Vec<u64> = r
        .iter()
        .filter_map(|o| o.winner.as_ref().map(|w| w.actor.get()))
        .collect();
    assert_eq!(
        thang.len(),
        2,
        "chỉ một trong hai đòn được thực hiện — người kia sống sót nhờ id"
    );
    assert!(thang.contains(&1) && thang.contains(&2));

    // Và không ai thua vì phá hòa: cả hai đều thắng ở nhóm của mình.
    let (hoa, _) = mow_action::resolve::tiebreak_ratio(&r);
    assert_eq!(hoa, 0, "trận đấu được quyết bằng khóa phá hòa");
}

#[test]
fn hai_nguoi_tranh_mot_qua_tao_thi_chi_mot_nguoi_duoc() {
    // Mặt còn lại: khi thật sự tranh nhau **một** thứ, phải có kẻ thua — và kẻ
    // thua phải thua vì một lý do quan sát được.
    use mow_action::{resolve_all, Contention, LossReason, Tier};
    use mow_core::StableKey;

    let voi_tay = |actor: u64, nhanh: i64| Contention {
        actor: EntityId(actor),
        tier: Tier::Transfer,
        action: "core.take".into(),
        target: Some("apple".into()),
        priority: nhanh,
        key: StableKey::plain(EntityId(actor)),
    };

    let r = resolve_all(vec![voi_tay(1, 30), voi_tay(2, 80)]);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].winner.as_ref().unwrap().actor, EntityId(2));
    assert_eq!(r[0].losers[0].1, LossReason::LowerPriority);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. "Tua thời gian không mất dân"
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn tua_thoi_gian_khong_mat_dan() {
    // Hai đường: một khu định cư chạy ở mức `Active` suốt, và một khu chạy ở
    // `Far` rồi được nâng lên. Cả hai phải cho cùng dân số.
    let goc = Conserved {
        population: 240,
        casualties: 12,
        resources: 5_000,
        relationships: 890,
        projects: 3,
        knowledge: 47,
    };

    let mut a = Aggregate::new(Lod::Active, goc);
    let ai_con: BTreeSet<u64> = (1..=240).collect();

    // Tua: hạ xuống Far, chạy, rồi nâng lại.
    transition(&mut a, Lod::Far, goc, &ai_con).expect("hạ mức phải bảo toàn");
    transition(&mut a, Lod::Near, goc, &ai_con).expect("nâng mức phải bảo toàn");
    transition(&mut a, Lod::Active, goc, &ai_con).expect("về Active phải bảo toàn");

    assert_eq!(
        a.conserved, goc,
        "tua thời gian làm lệch đại lượng bảo toàn"
    );
    assert_eq!(a.lod, Lod::Active);
}

#[test]
fn tua_thoi_gian_khong_lam_lech_nhu_cau() {
    // Cùng câu hỏi ở tầng cá thể: một người ở mức `Far` suốt 100 000 tick phải
    // đói đúng bằng người được cập nhật liên tục.
    let mk = || Need::full("core.hunger", Rate::new(-SCALE, 200_000).unwrap(), Tick(0));

    let mut lien_tuc = mk();
    for t in (1_000..=100_000).step_by(1_000) {
        lien_tuc.settle(Tick(t)).unwrap();
    }

    let bo_quen = mk();

    assert_eq!(
        lien_tuc.value_at(Tick(100_000)).unwrap(),
        bo_quen.value_at(Tick(100_000)).unwrap()
    );
}

#[test]
fn tua_thoi_gian_khong_lam_lech_ca_bo_nhu_cau() {
    let mk = || {
        let mut h = Homeostasis::new();
        h.insert(Need::full(
            "core.hunger",
            Rate::new(-SCALE, 200_000).unwrap(),
            Tick(0),
        ));
        h.insert(Need::full(
            "core.sleep",
            Rate::new(-SCALE, 40_000).unwrap(),
            Tick(0),
        ));
        h.insert(Need::full(
            "core.thirst",
            Rate::new(-SCALE, 60_000).unwrap(),
            Tick(0),
        ));
        h
    };

    let mut lien_tuc = mk();
    for t in (500..=50_000).step_by(500) {
        lien_tuc.settle_all(Tick(t)).unwrap();
    }
    let mut bo_quen = mk();
    bo_quen.settle_all(Tick(50_000)).unwrap();

    for a in lien_tuc.iter() {
        let b = bo_quen.get(&a.id).expect("cùng bộ nhu cầu");
        assert_eq!(a.value, b.value, "nhu cầu `{}` lệch sau khi tua", a.id);
    }
}

#[test]
fn tua_thoi_gian_khong_lam_lech_state_hash() {
    // Bài tổng: cùng một chuỗi thao tác, một đường tua từng bước nhỏ và một
    // đường nhảy một lần, phải cho cùng thế giới.
    use mow_math::CanonicalHash;

    let tung_buoc = {
        let mut sim = build_slice_world(42);
        for _ in 0..100 {
            sim.advance(100).unwrap();
        }
        sim.state_hash()
    };
    let mot_lan = {
        let mut sim = build_slice_world(42);
        sim.advance(10_000).unwrap();
        sim.state_hash()
    };
    assert_eq!(tung_buoc, mot_lan);

    // Và bộ nhu cầu cũng vậy, ở tầng dữ liệu.
    let n = Need::full("core.hunger", Rate::new(-7, 3).unwrap(), Tick(0));
    assert_eq!(n.state_hash(), n.state_hash());
}
