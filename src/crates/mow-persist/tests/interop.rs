//! Save cua ban desktop va ban server phai doc duoc cua nhau (`§P9` Giai doan A).
//!
//! Day khong phai mot chi tiet nho. Neu hai hinh thai trien khai co hai dinh
//! dang save, thi:
//!
//! - Nguoi choi khong the mang the gioi tu may minh len may chu, hoac nguoc lai.
//! - Bug tai hien duoc o mot ban va khong o ban kia, va repro bundle mat mot
//!   nua gia tri.
//! - Moi tinh nang lien quan toi save phai duoc viet va kiem hai lan.
//!
//! Cach ngan: **mot dinh dang duy nhat**, va `deployment mode` chi doi *noi*
//! luu, khong doi *cach* luu. Bai test nay khoa dieu do lai.

use mow_core::{BranchId, EventSeq, Tick, WorldId};
use mow_math::StateHash;
use mow_persist::{BranchRecord, EventRecord, Snapshot, SqliteStore, Store};

const B: BranchId = BranchId(1);
const W: WorldId = WorldId(1);

fn su_kien(seq: u64, tick: u64) -> EventRecord {
    EventRecord {
        seq: EventSeq(seq),
        branch: B,
        world: W,
        tick: Tick(tick),
        kind: "core.entity.spawned".into(),
        actor: 0,
        subject: seq + 1,
        payload: format!("payload-{seq}").into_bytes(),
        cause: if seq > 0 {
            Some(EventSeq(seq - 1))
        } else {
            None
        },
        law_version: Some(1),
        norm_set_version: Some(4),
    }
}

/// Ghi mot the gioi day du: nhanh, su kien, anh chup.
fn ghi_the_gioi(s: &mut SqliteStore) -> StateHash {
    s.create_branch(&BranchRecord {
        id: B,
        parent: None,
        fork_tick: Tick(0),
        label: "goc".into(),
    })
    .unwrap();

    let evs: Vec<_> = (0..64).map(|i| su_kien(i, i * 10)).collect();
    s.append_events(&evs).unwrap();

    let hash = StateHash([0xab; 32]);
    s.put_snapshot(&Snapshot {
        branch: B,
        world: W,
        tick: Tick(630),
        event_count: 64,
        state_hash: hash,
        blob: b"trang thai da tuan tu hoa".to_vec(),
    })
    .unwrap();
    s.flush().unwrap();
    hash
}

#[test]
fn save_ban_server_mo_duoc_bang_ban_desktop_va_nguoc_lai() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("world.sqlite");

    // Ban "server" ghi.
    let hash = {
        let mut s = SqliteStore::open(&path).unwrap();
        ghi_the_gioi(&mut s)
    };

    // Ban "desktop" mo cung file do.
    {
        let s = SqliteStore::open(&path).unwrap();

        let nhanh = s.get_branch(B).unwrap().expect("co nhanh");
        assert_eq!(nhanh.label, "goc");

        let evs = s.read_events(B, EventSeq(0), EventSeq(1_000)).unwrap();
        assert_eq!(evs.len(), 64);
        assert_eq!(evs[0], su_kien(0, 0));
        assert_eq!(evs[63], su_kien(63, 630));

        let snap = s
            .latest_snapshot(B, Tick(10_000))
            .unwrap()
            .expect("co anh chup");
        assert_eq!(snap.state_hash, hash, "hash phai di va ve nguyen ven");
        assert_eq!(snap.blob, b"trang thai da tuan tu hoa");

        assert_eq!(s.next_seq(B).unwrap(), EventSeq(64));
    }

    // Ban desktop ghi tiep, ban server doc lai.
    {
        let mut s = SqliteStore::open(&path).unwrap();
        s.append_events(&[su_kien(64, 640)]).unwrap();
        s.flush().unwrap();
    }
    {
        let s = SqliteStore::open(&path).unwrap();
        assert_eq!(s.next_seq(B).unwrap(), EventSeq(65));
        let evs = s.read_events(B, EventSeq(64), EventSeq(65)).unwrap();
        assert_eq!(evs[0].tick, Tick(640));
    }
}

#[test]
fn nhat_ky_van_chi_ghi_them_sau_khi_mo_lai() {
    // Trigger cam UPDATE/DELETE phai con hieu luc o file da ton tai, khong chi
    // o file vua tao. Neu no chi duoc cai luc tao, thi mot save mo lai se mat
    // hang rao — va no se mat mot cach im lang.
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("w.sqlite");
    {
        let mut s = SqliteStore::open(&path).unwrap();
        s.append_events(&[su_kien(0, 0)]).unwrap();
    }
    {
        let mut s = SqliteStore::open(&path).unwrap();
        // Ghi trung khoa chinh phai that bai; day la duong duy nhat de cham toi
        // mot su kien da ton tai qua API cong khai.
        assert!(s.append_events(&[su_kien(0, 999)]).is_err());
        let evs = s.read_events(B, EventSeq(0), EventSeq(10)).unwrap();
        assert_eq!(evs[0].tick, Tick(0), "su kien cu bi ghi de");
    }
}

#[test]
fn khong_co_cot_so_thuc_o_ca_hai_hinh_thai() {
    // §P10.2.1 ap cho moi backend, khong chi cho backend dang phat trien.
    let d = tempfile::tempdir().unwrap();
    let s = SqliteStore::open(d.path().join("w.sqlite")).unwrap();
    assert!(s.kiem_tra_khong_co_cot_thuc().unwrap().is_empty());
}
