//! Test storylet và biên niên sử hai lớp (`PD-17`, `PD-18`).

use mow_core::{EntityId, Tick};
use mow_director::chronicle::{Chronicle, Fact, Legend, Retelling};
use mow_director::storylet::{Boost, Director, Perturbation, Precondition, Storylet, WorldFacts};

fn ngap_mo() -> Storylet {
    Storylet {
        id: "storylet.mine_flooding".into(),
        preconditions: vec![
            Precondition::InfrastructureExists {
                kind: "mine".into(),
            },
            Precondition::Pressure {
                name: "water_table_rising".into(),
                min: 500,
            },
        ],
        base_salience: 400,
        boosts: vec![Boost {
            when: Precondition::Flag {
                name: "settlement_depends_on_mine".into(),
            },
            by: 350,
        }],
        perturbation: vec![
            Perturbation::ApplyEffect {
                effect: "effect.region.flooding".into(),
                target: "mine_lower_levels".into(),
            },
            Perturbation::ResourceDelta {
                resource: "ore_output".into(),
                delta: -700,
            },
        ],
        budget_cost: 2,
        cooldown: 900,
        provenance: "core".into(),
    }
}

fn the_gioi() -> WorldFacts {
    WorldFacts {
        infrastructure: vec!["mine".into()],
        pressures: vec![("water_table_rising".into(), 700)],
        last_fired: vec![],
        flags: vec![],
        now: 10_000,
        player_focus: None,
    }
}

// ───────────────────────── PD-17 · storylet ─────────────────────────

/// **Quy tắc 1**: storylet không có trường `outcomes`, và không thể có.
#[test]
fn storylet_khong_co_truong_outcomes() {
    let j = serde_json::to_string(&ngap_mo()).unwrap();
    for cam in ["outcome", "result", "consequence", "resolution"] {
        assert!(
            !j.contains(cam),
            "storylet có trường `{cam}` — nó chỉ được đặt điều kiện"
        );
    }
    // Cái nó có là `perturbation`: đổi điều kiện thế giới.
    assert!(j.contains("perturbation"));
}

/// **Quy tắc 2**: thế giới chưa có nguyên nhân thì không kích hoạt được.
#[test]
fn the_gioi_chua_co_nguyen_nhan_thi_khong_kich_hoat_duoc() {
    let s = ngap_mo();
    assert!(s.eligible(&the_gioi()));

    let khong_co_mo = WorldFacts {
        infrastructure: vec![],
        ..the_gioi()
    };
    assert!(!s.eligible(&khong_co_mo));

    let nuoc_chua_len = WorldFacts {
        pressures: vec![("water_table_rising".into(), 100)],
        ..the_gioi()
    };
    assert!(!s.eligible(&nuoc_chua_len));
}

/// **Quy tắc 3**: cooldown ngăn cùng một tai họa giáng xuống mãi.
#[test]
fn cooldown_ngan_cung_mot_tai_hoa_giang_xuong_mai() {
    let s = ngap_mo();
    let vua_xay_ra = WorldFacts {
        last_fired: vec![("storylet.mine_flooding".into(), 9_500)],
        ..the_gioi()
    };
    assert!(s.on_cooldown(&vua_xay_ra));

    let da_lau = WorldFacts {
        last_fired: vec![("storylet.mine_flooding".into(), 1_000)],
        ..the_gioi()
    };
    assert!(!s.on_cooldown(&da_lau));
}

/// Ngân sách ngăn **mọi** tai họa cùng giáng xuống một lúc.
#[test]
fn ngan_sach_ngan_moi_tai_hoa_cung_giang_xuong_mot_luc() {
    let pool: Vec<Storylet> = (0..5)
        .map(|i| Storylet {
            id: format!("storylet.disaster_{i}"),
            ..ngap_mo()
        })
        .collect();

    let d = Director { budget: 5 };
    let ra = d.select(&pool, &the_gioi());
    let da_no = ra.iter().filter(|a| a.fired).count();
    assert_eq!(da_no, 2, "ngân sách 5, mỗi cái tốn 2 ⇒ đúng hai cái");

    // Và những cái trượt phải nói rõ vì hết ngân sách.
    assert!(ra.iter().any(|a| a
        .rejected_because
        .as_deref()
        .is_some_and(|r| r.contains("ngân sách"))));
}

/// Người chơi nhìn vào đâu thì chỗ đó **được cộng salience** — nhưng không được
/// bám lấy mãi.
#[test]
fn chu_y_cua_nguoi_choi_cong_salience_nhung_cooldown_van_chan() {
    let s = ngap_mo();
    let binh_thuong = s.salience(&the_gioi()).0;

    let quan_trong = WorldFacts {
        flags: vec!["settlement_depends_on_mine".into()],
        ..the_gioi()
    };
    assert!(s.salience(&quan_trong).0 > binh_thuong);

    // Nhưng dù salience cao tới đâu, cooldown vẫn chặn.
    let vua_xay_ra = WorldFacts {
        last_fired: vec![("storylet.mine_flooding".into(), 9_900)],
        ..quan_trong
    };
    let ra = Director { budget: 100 }.select(&[s], &vua_xay_ra);
    assert!(!ra[0].fired);
    assert!(ra[0]
        .rejected_because
        .as_deref()
        .is_some_and(|r| r.contains("nghỉ")));
}

/// **Audit trả cả những cái trượt** — câu hỏi hay gặp nhất là "vì sao chuyện kia
/// không xảy ra".
#[test]
fn audit_tra_ca_nhung_cai_truot_kem_ly_do() {
    let du = ngap_mo();
    let thieu = Storylet {
        id: "storylet.volcano".into(),
        preconditions: vec![Precondition::InfrastructureExists {
            kind: "volcano".into(),
        }],
        ..ngap_mo()
    };

    let ra = Director { budget: 10 }.select(&[du, thieu], &the_gioi());
    assert_eq!(ra.len(), 2, "audit phải nói về cả hai");

    let volcano = ra
        .iter()
        .find(|a| a.storylet == "storylet.volcano")
        .unwrap();
    assert!(!volcano.fired);
    assert_eq!(
        volcano.rejected_because.as_deref(),
        Some("thế giới chưa có nguyên nhân")
    );
    // Và nói rõ vị từ nào không thỏa.
    assert!(volcano.preconditions.iter().any(|(_, ok)| !ok));
}

/// Salience có **phân rã**, không phải một con số từ trên trời.
#[test]
fn salience_co_phan_ra() {
    let quan_trong = WorldFacts {
        flags: vec!["settlement_depends_on_mine".into()],
        ..the_gioi()
    };
    let (tong, parts) = ngap_mo().salience(&quan_trong);
    assert_eq!(parts.len(), 2);
    assert_eq!(tong, 750);
    assert_eq!(parts[0].1 + parts[1].1, tong);
}

/// Chọn **xác định**: cùng thế giới thì cùng kết quả, không phụ thuộc thứ tự pool.
#[test]
fn chon_storylet_xac_dinh() {
    let a = Storylet {
        id: "storylet.zeta".into(),
        ..ngap_mo()
    };
    let b = Storylet {
        id: "storylet.alpha".into(),
        ..ngap_mo()
    };

    let d = Director { budget: 2 };
    let xuoi: Vec<String> = d
        .select(&[a.clone(), b.clone()], &the_gioi())
        .iter()
        .filter(|x| x.fired)
        .map(|x| x.storylet.clone())
        .collect();
    let nguoc: Vec<String> = d
        .select(&[b, a], &the_gioi())
        .iter()
        .filter(|x| x.fired)
        .map(|x| x.storylet.clone())
        .collect();
    assert_eq!(xuoi, nguoc);
    assert_eq!(xuoi, vec!["storylet.alpha"]);
}

/// Storylet là **điểm mở rộng của plugin**: namespace riêng đi theo dữ liệu.
#[test]
fn storylet_mang_namespace_va_provenance() {
    let cua_pack = Storylet {
        id: "mypack.storylet.sandstorm".into(),
        provenance: "mypack".into(),
        ..ngap_mo()
    };
    let ra = Director { budget: 10 }.select(&[cua_pack], &the_gioi());
    assert!(ra[0].storylet.starts_with("mypack."));
}

// ───────────────────── PD-18 · biên niên sử hai lớp ─────────────────────

fn ke(teller: u64, gen: u32, motive: &str, says: &str) -> Retelling {
    Retelling {
        teller: EntityId(teller),
        at: Tick(1_000 * u64::from(gen)),
        generation: gen,
        motive: motive.into(),
        says: says.into(),
    }
}

fn bien_nien() -> Chronicle {
    Chronicle {
        facts: vec![Fact {
            event_seq: 1,
            at: Tick(0),
            actor: Some(EntityId(10)),
            what: "Aren mở cổng thành cho quân địch".into(),
        }],
        legends: vec![Legend {
            about_event: 1,
            believed_by: "culture.veskar".into(),
            chain: vec![
                ke(
                    10,
                    0,
                    "chứng kiến tận mắt",
                    "Aren mở cổng thành cho quân địch",
                ),
                ke(11, 1, "kể lại y nguyên", "Aren mở cổng thành cho quân địch"),
                // Đời thứ hai bẻ nó — vì con cháu Aren đã trả tiền.
                ke(
                    12,
                    2,
                    "được dòng họ Aren trả tiền",
                    "Aren tử thủ tới người cuối",
                ),
                ke(13, 3, "kể lại y nguyên", "Aren tử thủ tới người cuối"),
            ],
        }],
    }
}

/// **Hai lớp cạnh nhau**, và chỗ lệch được đánh dấu.
#[test]
fn danh_dau_duoc_cho_hai_lop_lech_nhau() {
    let c = bien_nien();
    let lech = c.divergences();
    assert_eq!(lech.len(), 1);
    assert_eq!(lech[0].truth, "Aren mở cổng thành cho quân địch");
    assert_eq!(lech[0].belief, "Aren tử thủ tới người cuối");
}

/// **Bấm vào là thấy lệch từ đâu**: ai kể sai, ở đời nào, vì động cơ gì.
#[test]
fn bam_vao_cho_lech_thay_du_ba_ve() {
    let lech = &bien_nien().divergences()[0];
    assert_eq!(lech.introduced_by, Some(EntityId(12)), "ai");
    assert_eq!(lech.generation, Some(2), "đời nào");
    assert_eq!(
        lech.motive.as_deref(),
        Some("được dòng họ Aren trả tiền"),
        "vì động cơ gì"
    );
}

/// Người kể **cuối cùng** có thể hoàn toàn trung thực — chỉ ra đúng người đã bẻ.
#[test]
fn chi_ra_nguoi_da_be_khong_phai_nguoi_ke_cuoi() {
    let c = bien_nien();
    let cuoi = c.legends[0].current().unwrap();
    assert_eq!(cuoi.teller, EntityId(13));
    assert_eq!(cuoi.motive, "kể lại y nguyên");

    // Nhưng chỗ lệch được quy cho người số 12.
    assert_eq!(c.divergences()[0].introduced_by, Some(EntityId(12)));
}

/// Truyền thuyết **trùng khớp sự thật không xuất hiện** ở danh sách lệch.
///
/// Đánh dấu chỗ lệch chỉ có nghĩa khi phần lớn không lệch.
#[test]
fn truyen_thuyet_dung_thi_khong_bi_danh_dau() {
    let c = Chronicle {
        facts: vec![Fact {
            event_seq: 1,
            at: Tick(0),
            actor: Some(EntityId(10)),
            what: "trận lụt năm ấy".into(),
        }],
        legends: vec![Legend {
            about_event: 1,
            believed_by: "culture.veskar".into(),
            chain: vec![ke(10, 0, "chứng kiến", "trận lụt năm ấy")],
        }],
    };
    assert!(c.divergences().is_empty());
}

/// **Hai văn hóa kể hai câu chuyện khác nhau về cùng một ngày.**
#[test]
fn hai_van_hoa_co_the_tin_khac_nhau_ve_cung_mot_su_kien() {
    let mut c = bien_nien();
    c.legends.push(Legend {
        about_event: 1,
        believed_by: "culture.invader".into(),
        chain: vec![ke(50, 0, "phe thắng kể", "cổng thành tự mở vì ý trời")],
    });

    assert!(c.contested(1));
    assert_eq!(c.divergences().len(), 2, "cả hai bản đều lệch sự thật");
}

/// Không có sự thật tương ứng thì không kết luận gì.
///
/// Một truyền thuyết về chuyện chưa từng xảy ra không phải là "lệch" — nó là một
/// chuyện khác hẳn, và engine không có tư cách phán.
#[test]
fn truyen_thuyet_ve_su_kien_khong_ton_tai_thi_khong_ket_luan() {
    let c = Chronicle {
        facts: vec![],
        legends: vec![Legend {
            about_event: 999,
            believed_by: "culture.veskar".into(),
            chain: vec![ke(1, 0, "nghe kể", "rồng đã bay qua")],
        }],
    };
    assert!(c.divergences().is_empty());
}

/// Danh sách lệch **xác định**, không phụ thuộc thứ tự nạp.
#[test]
fn danh_sach_lech_xac_dinh() {
    let mut c = bien_nien();
    c.legends.push(Legend {
        about_event: 1,
        believed_by: "culture.aaa".into(),
        chain: vec![ke(50, 0, "x", "khác hẳn")],
    });

    let a: Vec<String> = c
        .divergences()
        .iter()
        .map(|d| d.believed_by.clone())
        .collect();
    c.legends.reverse();
    let b: Vec<String> = c
        .divergences()
        .iter()
        .map(|d| d.believed_by.clone())
        .collect();
    assert_eq!(a, b);
    assert_eq!(a[0], "culture.aaa");
}
