//! Test vật phẩm huyền thoại và di sản (`PE-16`, `§8.9`).

use mow_core::{EntityId, EventSeq};
use mow_items::legend::{
    Claim, Deed, Fate, Legend, Path, Provenance, SapientItem, SocialPower, NGUONG_TAY_NGHE,
};

fn viec(seq: u64, kind: &str, who: Option<u64>) -> Deed {
    Deed {
        seq: EventSeq(seq),
        kind: kind.to_owned(),
        who: who.map(EntityId),
        detail: format!("{kind} @ {seq}"),
    }
}

/// Một thanh kiếm tầm thường, rèn xoàng, chưa làm gì.
fn kiem_tam_thuong() -> Provenance {
    Provenance {
        item: 1,
        forged_at: EventSeq(10),
        maker: Some(EntityId(3)),
        craft_percentile: 500,
        deeds: vec![],
        binding: None,
        anomalous_origin: None,
        fate: Fate::Extant,
    }
}

// ───────────────── §8.9.1 · bốn con đường ─────────────────

/// **Không có "tỉ lệ rơi đồ huyền thoại"** — không đường nào thì không huyền thoại.
#[test]
fn khong_di_duong_nao_thi_khong_huyen_thoai() {
    let p = kiem_tam_thuong();
    assert!(p.why().is_empty());
    assert!(!p.is_legendary());
}

/// **Một thanh kiếm tầm thường thành huyền thoại bằng lịch sử.**
///
/// Con đường thú vị nhất, vì nó không cần gì đặc biệt lúc rèn — giá trị nằm ở
/// provenance, không ở vật liệu.
#[test]
fn kiem_tam_thuong_thanh_huyen_thoai_bang_lich_su() {
    let mut p = kiem_tam_thuong();
    p.deeds = vec![
        viec(50, "wielded_at_battle", Some(7)),
        viec(80, "wielded_at_battle", Some(7)),
        viec(120, "slew", Some(7)),
    ];
    assert_eq!(p.why(), vec![Path::AccumulatedHistory]);
    assert_eq!(p.craft_percentile, 500, "vẫn là một thanh kiếm rèn xoàng");
}

/// Đổi chủ mười lần **không** làm nên huyền thoại.
#[test]
fn doi_chu_muoi_lan_khong_lam_nen_huyen_thoai() {
    let mut p = kiem_tam_thuong();
    p.deeds = (0..10)
        .map(|i| viec(50 + i, "changed_hands", Some(i)))
        .collect();
    assert!(
        !p.is_legendary(),
        "đi qua mười tiệm cầm đồ vẫn là kiếm thường"
    );
}

/// Bốn con đường độc lập, và một món có thể đi nhiều đường.
#[test]
fn bon_con_duong_doc_lap_va_cong_don_duoc() {
    let p = Provenance {
        craft_percentile: NGUONG_TAY_NGHE,
        deeds: vec![
            viec(50, "wielded_at_battle", None),
            viec(60, "slew", None),
            viec(70, "sealed", None),
        ],
        binding: Some("soul.anchored".into()),
        anomalous_origin: Some("rift.shard".into()),
        ..kiem_tam_thuong()
    };
    assert_eq!(
        p.why(),
        vec![
            Path::Masterwork,
            Path::AccumulatedHistory,
            Path::MagicalBinding,
            Path::DivineOrAnomalous
        ]
    );
}

/// Tay nghề phải ở **đuôi trên** mới tính.
#[test]
fn tay_nghe_phai_o_duoi_tren_moi_tinh() {
    let mut p = kiem_tam_thuong();
    p.craft_percentile = NGUONG_TAY_NGHE - 1;
    assert!(!p.is_legendary());
    p.craft_percentile = NGUONG_TAY_NGHE;
    assert_eq!(p.why(), vec![Path::Masterwork]);
}

/// Chuỗi provenance **truy ngược được về event thật**.
#[test]
fn chuoi_provenance_truy_nguoc_ve_event_that() {
    let mut p = kiem_tam_thuong();
    p.deeds = vec![
        viec(50, "changed_hands", Some(7)),
        viec(90, "changed_hands", Some(9)),
    ];
    assert_eq!(p.current_holder(), Some(EntityId(9)));
    assert!(p.deeds.iter().all(|d| d.seq.0 > 0));
}

// ───────────────── §8.9.2 · truyền thuyết ≠ lịch sử ─────────────────

/// **Hai lớp cạnh nhau**: khoảng cách giữa sự thật và niềm tin là nội dung chơi được.
#[test]
fn khoang_cach_giua_su_that_va_niem_tin_do_duoc() {
    let truyen_thuyet = Legend {
        item: 1,
        called: "Quốc Bảo Kiếm".into(),
        claims: vec![
            Claim {
                about: "forged_at".into(),
                believed: "năm lập quốc".into(),
                held_by_permille: 900,
            },
            Claim {
                about: "maker".into(),
                believed: "thợ rèn hoàng gia đầu tiên".into(),
                held_by_permille: 850,
            },
        ],
    };
    let su_that = vec![
        (
            "forged_at".to_owned(),
            "một trăm năm sau lập quốc".to_owned(),
        ),
        ("maker".to_owned(), "thợ rèn hoàng gia đầu tiên".to_owned()),
    ];

    let lech = truyen_thuyet.discrepancies(&su_that);
    assert_eq!(lech.len(), 1, "chỉ một tuyên bố sai");
    assert_eq!(lech[0].about, "forged_at");
    assert_eq!(lech[0].held_by_permille, 900);
}

/// Truyền thuyết **không mang `EventSeq` nào** — nó không cần có thật để lan.
#[test]
fn truyen_thuyet_khong_tro_vao_event_nao() {
    let l = Legend {
        item: 1,
        called: "Quốc Bảo Kiếm".into(),
        claims: vec![Claim {
            about: "slew".into(),
            believed: "giết rồng".into(),
            held_by_permille: 700,
        }],
    };
    let j = serde_json::to_string(&l).unwrap();
    assert!(
        !j.contains("seq"),
        "belief mà trỏ event thì nó là lịch sử: {j}"
    );
}

/// Không lệch chỗ nào thì danh sách rỗng — truyền thuyết đúng cũng có thật.
#[test]
fn truyen_thuyet_dung_thi_khong_lech_cho_nao() {
    let l = Legend {
        item: 1,
        called: "x".into(),
        claims: vec![Claim {
            about: "maker".into(),
            believed: "A".into(),
            held_by_permille: 500,
        }],
    };
    assert!(l
        .discrepancies(&[("maker".to_owned(), "A".to_owned())])
        .is_empty());
}

// ───────────────── §8.9.3 · vật phẩm là đối tượng xã hội ─────────────────

/// **Quyền uy chỉ thật đúng bằng mức người ta tin.**
#[test]
fn vuong_mien_khong_ai_cong_nhan_thi_quyen_uy_bang_khong() {
    let khong_ai_tin = SocialPower {
        item: 2,
        role: "crown".into(),
        nominal: 1_000,
        believed_authentic_permille: 0,
    };
    assert_eq!(khong_ai_tin.authority(), 0, "bằng không, không phải 'thấp'");
}

/// **Một bản sao được tin là thật có quyền uy đúng bằng bản thật.**
#[test]
fn ban_sao_duoc_tin_la_that_co_quyen_uy_bang_ban_that() {
    let that = SocialPower {
        item: 2,
        role: "crown".into(),
        nominal: 1_000,
        believed_authentic_permille: 800,
    };
    let sao = SocialPower {
        item: 3,
        ..that.clone()
    };
    assert!(that.same_authority_as(&sao));
    assert_eq!(sao.authority(), 800);
}

// ───────────────── §8.9.4 · vật phẩm có tri giác ─────────────────

/// **Không phải trường hợp đặc biệt** — chiếm ngân sách nhận thức như mọi `Sapient`.
#[test]
fn vat_pham_co_tri_giac_chiem_ngan_sach_nhu_moi_sapient() {
    let k = SapientItem {
        item: 1,
        as_entity: EntityId(4_242),
        memory_namespace: "item.1".into(),
    };
    assert!(k.consumes_cognition_budget());
    assert_eq!(k.as_entity, EntityId(4_242), "nó **là** một entity");
}

/// Kiểu này cố tình nghèo nàn — không có đường vòng quanh `INV-22-3`.
#[test]
fn vat_pham_co_tri_giac_khong_co_truong_rieng_nao() {
    let j = serde_json::to_string(&SapientItem {
        item: 1,
        as_entity: EntityId(1),
        memory_namespace: "item.1".into(),
    })
    .unwrap();
    for cam in ["persona", "prompt", "fallback", "acl", "llm"] {
        assert!(
            !j.contains(cam),
            "có đường vòng quanh cognition contract: {j}"
        );
    }
}

// ───────────────── hủy diệt là thật ─────────────────

/// **Hủy là hủy.** Không hoàn tác, không tìm lại.
#[test]
fn huy_diet_la_that() {
    let huy = Fate::Destroyed {
        at: EventSeq(9_000),
        how: "nấu chảy trong lò thánh".into(),
    };
    assert!(!huy.usable());
    assert!(!huy.recoverable());
}

/// **Mất tích khác bị hủy** — gộp hai cái là xóa mất một thể loại nhiệm vụ.
#[test]
fn mat_tich_khac_bi_huy() {
    let mat = Fate::Lost {
        since: EventSeq(500),
    };
    assert!(!mat.usable(), "không dùng được vì không ai biết nó ở đâu");
    assert!(mat.recoverable(), "nhưng vẫn tìm lại được");
}

/// **Truyền thuyết sống tiếp sau khi vật bị hủy** — và đó mới là phần đáng chơi.
#[test]
fn truyen_thuyet_song_tiep_sau_khi_vat_bi_huy() {
    let mut p = kiem_tam_thuong();
    p.deeds = vec![
        viec(50, "wielded_at_battle", None),
        viec(60, "slew", None),
        viec(70, "founded", None),
    ];
    p.fate = Fate::Destroyed {
        at: EventSeq(9_000),
        how: "nấu chảy".into(),
    };
    let l = Legend {
        item: p.item,
        called: "Quốc Bảo Kiếm".into(),
        claims: vec![],
    };
    assert!(!p.fate.recoverable());
    assert!(l.survives_destruction(), "mất chuông vẫn còn tiếng");
    assert!(p.is_legendary(), "và chuỗi provenance vẫn đứng đó");
}
