//! **Lát cắt chơi được** (`PB-25`).
//!
//! Bài test này là thứ chứng minh Giai đoạn B tạo ra một **trò chơi**, không chỉ
//! một simulator. Nếu nó fail, thì mọi hệ thống khác có thể đúng mà vòng lặp
//! chơi vẫn không khép.

use mow_core::{invariant::Cost, val, Command, EntityId, InvariantRunner};
use mow_scenario::slice::{act, build_slice_world, now, observe_as, preview, WORLD};

const BIET_LAM: &[&str] = &["core.walk", "core.take", "core.eat", "core.speak"];

fn tim(sim: &mow_core::Sim, ten: &str) -> EntityId {
    sim.store()
        .ids()
        .find(|id| sim.store().attr_text(*id, "core.name") == Some(ten))
        .unwrap_or_else(|| panic!("không tìm thấy `{ten}`"))
}

/// **Vòng lặp chơi khép kín**: thấy → quyết định → hành động → thế giới đổi.
#[test]
fn vong_lap_choi_khep_kin() {
    let mut sim = build_slice_world(2026);
    let toi = tim(&sim, "Nguoi Choi");
    let banh = tim(&sim, "O Banh");

    // ── 1. THẤY ──────────────────────────────────────────────────────────────
    // Người chơi không nhìn thấy state; họ nhìn thấy cái nhân vật của họ thấy.
    let ctx = observe_as(&sim, toi, BIET_LAM);
    assert!(
        ctx.observations
            .iter()
            .any(|o| o.signs.iter().any(|s| s == "food")),
        "avatar phải thấy được ổ bánh cách một ô"
    );
    let doi_ban_dau = ctx.internal.iter().find(|(k, _)| k == "hunger").unwrap().1;
    assert_eq!(doi_ban_dau, 3_000, "bắt đầu ở trạng thái đói");

    // ── 2. ĐI TỚI ────────────────────────────────────────────────────────────
    act(
        &mut sim,
        &Command::new(
            "core.walk",
            WORLD,
            val! { "who" => toi.get(), "dx" => 1i64, "dy" => 0i64 },
        ),
    )
    .expect("đi được một bước");
    assert_eq!(sim.store().attr_int(toi, "core.pos.x"), Some(1));

    // ── 3. NHẶT ──────────────────────────────────────────────────────────────
    act(
        &mut sim,
        &Command::new(
            "core.take",
            WORLD,
            val! { "who" => toi.get(), "what" => banh.get() },
        ),
    )
    .expect("nhặt được");
    assert_eq!(sim.store().attr_entity(banh, "loc.inventory"), Some(toi));
    assert!(
        sim.store().attr(banh, "loc.cell").is_none(),
        "§22.33: vật phẩm phải ở đúng MỘT nơi"
    );

    // ── 4. ĂN ────────────────────────────────────────────────────────────────
    act(
        &mut sim,
        &Command::new(
            "core.eat",
            WORLD,
            val! { "who" => toi.get(), "what" => banh.get() },
        ),
    )
    .expect("ăn được");
    assert_eq!(sim.store().attr_int(toi, "need.hunger"), Some(7_000));
    assert!(
        !sim.store().contains(banh),
        "ăn xong thì bánh phải biến mất"
    );

    // ── 5. NÓI ───────────────────────────────────────────────────────────────
    act(
        &mut sim,
        &Command::new(
            "core.speak",
            WORLD,
            val! { "who" => toi.get(), "text" => "Chao Aren" },
        ),
    )
    .expect("nói được");
    assert!(
        sim.log().iter().any(|e| e.kind.0 == "core.speech.uttered"),
        "lời nói phải là một sự kiện trong thế giới"
    );

    // ── 6. THẾ GIỚI ĐÃ ĐỔI, VÀ ĐỔI THEO CÁCH TRUY ĐƯỢC ───────────────────────
    let cac_loai: Vec<&str> = sim.log().iter().map(|e| e.kind.0.as_str()).collect();
    for can in [
        "core.entity.moved",
        "core.item.taken",
        "core.item.eaten",
        "core.speech.uttered",
    ] {
        assert!(
            cac_loai.contains(&can),
            "thiếu sự kiện `{can}` trong nhật ký"
        );
    }

    // Và thế giới vẫn nhất quán sau tất cả.
    let rep = sim.check(&InvariantRunner::standard(Cost::Expensive));
    assert!(rep.is_clean(), "{rep}");
}

#[test]
fn khong_nhat_duoc_thu_ngoai_tam_voi() {
    // Nếu nhặt được từ xa thì việc đi lại mất hết ý nghĩa.
    let mut sim = build_slice_world(1);
    let toi = tim(&sim, "Nguoi Choi");
    let aren = tim(&sim, "Aren");

    // Aren ở cách 3 ô.
    let e = act(
        &mut sim,
        &Command::new(
            "core.take",
            WORLD,
            val! { "who" => toi.get(), "what" => aren.get() },
        ),
    )
    .expect_err("phải ngoài tầm");
    assert!(e.to_string().contains("ngoài tầm"), "{e}");
}

#[test]
fn khong_di_qua_nhieu_o_mot_luc() {
    // Trần vật lý: không có bước dài, và đó là thứ khiến khoảng cách có nghĩa.
    let mut sim = build_slice_world(1);
    let toi = tim(&sim, "Nguoi Choi");
    assert!(act(
        &mut sim,
        &Command::new(
            "core.walk",
            WORLD,
            val! { "who" => toi.get(), "dx" => 50i64, "dy" => 0i64 }
        )
    )
    .is_err());
}

#[test]
fn chi_an_duoc_thu_dang_cam() {
    let mut sim = build_slice_world(1);
    let toi = tim(&sim, "Nguoi Choi");
    let banh = tim(&sim, "O Banh");
    // Chưa nhặt.
    let e = act(
        &mut sim,
        &Command::new(
            "core.eat",
            WORLD,
            val! { "who" => toi.get(), "what" => banh.get() },
        ),
    )
    .expect_err("chưa cầm thì không ăn được");
    assert!(e.to_string().contains("đang cầm"), "{e}");
}

#[test]
fn khong_an_duoc_thu_khong_an_duoc() {
    let mut sim = build_slice_world(1);
    let toi = tim(&sim, "Nguoi Choi");
    let aren = tim(&sim, "Aren");
    // Đưa Aren vào túi bằng đường True God để bỏ qua tầm với, rồi thử ăn.
    act(
        &mut sim,
        &Command::new(
            "truegod.set_attr",
            WORLD,
            val! { "entity" => aren.get(), "key" => "loc.inventory", "value" => toi.get() },
        ),
    )
    .expect("True God đặt được");
    let e = act(
        &mut sim,
        &Command::new(
            "core.eat",
            WORLD,
            val! { "who" => toi.get(), "what" => aren.get() },
        ),
    )
    .expect_err("người không phải thức ăn");
    assert!(e.to_string().contains("không ăn được"), "{e}");
}

// ─────────────────────────────────────────────────────────────────────────────
// §15.5 — True God: preview trước commit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn preview_khong_doi_the_gioi() {
    // Nếu xem trước mà đổi thế giới thì nó không còn là xem trước.
    let sim = build_slice_world(7);
    let toi = tim(&sim, "Nguoi Choi");
    let truoc = sim.state_hash();

    let cmd = Command::new(
        "truegod.set_attr",
        WORLD,
        val! { "entity" => toi.get(), "key" => "core.name", "value" => "Ten Moi" },
    );
    let p = preview(&sim, &cmd);

    assert_eq!(sim.state_hash(), truoc, "preview đã đổi thế giới thật");
    assert_eq!(p.before, truoc);
}

#[test]
fn preview_noi_dung_hau_qua() {
    let mut sim = build_slice_world(7);
    let toi = tim(&sim, "Nguoi Choi");
    let cmd = Command::new(
        "truegod.set_attr",
        WORLD,
        val! { "entity" => toi.get(), "key" => "core.name", "value" => "Ten Moi" },
    );

    let p = preview(&sim, &cmd);
    assert!(p.changes_anything());
    assert!(p.error.is_none());
    assert_eq!(p.events, 1);
    assert!(p.entities_affected > 0);

    // Commit rồi thì hash thật phải khớp cái preview đã hứa.
    act(&mut sim, &cmd).expect("commit được");
    assert_eq!(
        sim.state_hash(),
        p.after,
        "preview hứa một hash và commit cho ra hash khác — preview nói dối"
    );
}

#[test]
fn preview_bao_truoc_lenh_se_that_bai() {
    // Người dùng phải biết trước, thay vì bấm rồi mới thấy lỗi.
    let sim = build_slice_world(7);
    let cmd = Command::new(
        "truegod.set_attr",
        WORLD,
        val! { "entity" => 9_999u64, "key" => "x", "value" => 1i64 },
    );
    let p = preview(&sim, &cmd);
    assert!(p.error.is_some());
    assert!(!p.changes_anything());
}

#[test]
fn moi_can_thiep_co_provenance() {
    // `§16.4`: không có nó, một thế giới kỳ lạ sáu tháng sau sẽ không ai biết
    // là do luật hay do có người sửa.
    let mut sim = build_slice_world(7);
    let toi = tim(&sim, "Nguoi Choi");
    act(
        &mut sim,
        &Command::new(
            "truegod.set_attr",
            WORLD,
            val! { "entity" => toi.get(), "key" => "core.blessed", "value" => true },
        ),
    )
    .unwrap();

    let ev = sim
        .log()
        .iter()
        .find(|e| e.kind.0 == "truegod.intervened")
        .expect("có sự kiện");
    assert_eq!(ev.payload.get_text("provenance"), Some("true_god"));
    assert_eq!(ev.subject, Some(toi));
}

// ─────────────────────────────────────────────────────────────────────────────
// Determinism của cả lát cắt
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cung_chuoi_thao_tac_cho_cung_the_gioi() {
    let chay = || {
        let mut sim = build_slice_world(99);
        let toi = tim(&sim, "Nguoi Choi");
        let banh = tim(&sim, "O Banh");
        act(
            &mut sim,
            &Command::new(
                "core.walk",
                WORLD,
                val! { "who" => toi.get(), "dx" => 1i64, "dy" => 0i64 },
            ),
        )
        .unwrap();
        act(
            &mut sim,
            &Command::new(
                "core.take",
                WORLD,
                val! { "who" => toi.get(), "what" => banh.get() },
            ),
        )
        .unwrap();
        act(
            &mut sim,
            &Command::new(
                "core.eat",
                WORLD,
                val! { "who" => toi.get(), "what" => banh.get() },
            ),
        )
        .unwrap();
        sim.advance(500).unwrap();
        sim.state_hash()
    };
    assert_eq!(chay(), chay());
}

#[test]
fn quan_sat_theo_tri_giac_khong_ro_ri_state() {
    // Avatar không được thấy thứ ở xa chỉ vì nó tồn tại trong `Store`.
    let mut sim = build_slice_world(5);
    let toi = tim(&sim, "Nguoi Choi");

    // Đẩy Aren đi thật xa bằng True God.
    let aren = tim(&sim, "Aren");
    act(
        &mut sim,
        &Command::new(
            "truegod.set_attr",
            WORLD,
            val! { "entity" => aren.get(), "key" => "core.pos.x", "value" => 5_000i64 },
        ),
    )
    .unwrap();

    let ctx = observe_as(&sim, toi, BIET_LAM);
    assert!(
        !ctx.identified().contains(&aren),
        "nhìn thấy người cách 5000 ô"
    );
    assert!(
        sim.store().contains(aren),
        "nhưng Aren vẫn tồn tại trong thế giới"
    );
}

#[test]
fn dong_ho_chay_va_lat_cat_van_nhat_quan() {
    let mut sim = build_slice_world(11);
    assert_eq!(now(&sim).0, 0);
    sim.advance(10_000).unwrap();
    assert_eq!(now(&sim).0, 10_000);
    let rep = sim.check(&InvariantRunner::standard(Cost::Expensive));
    assert!(rep.is_clean(), "{rep}");
}
