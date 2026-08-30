use mow_config::{load, AppConfig, LlmMode};
use std::fs;

fn viet(dir: &std::path::Path, ten: &str, noi_dung: &str) {
    fs::write(dir.join(ten), noi_dung).unwrap();
}

#[test]
fn nap_config_that_cua_du_an() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config");
    let c = load(&root, "dev").expect("config that cua du an phai hop le");
    assert_eq!(c.env, "dev");
    assert_eq!(c.llm.mode, LlmMode::Stub);
    // dev.yaml ghi de log_level cua base.yaml
    assert_eq!(c.observability.log_level, "debug");
    // nhung khong dung toi sim.tick_rate, nen gia tri cua base.yaml con nguyen
    assert_eq!(c.sim.tick_rate, 20);
}

#[test]
fn moi_truong_test_cung_hop_le() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config");
    let c = load(&root, "test").expect("test.yaml phai hop le");
    assert_eq!(c.env, "test");
    assert_eq!(c.sim.snapshot_interval, 0);
}

#[test]
fn thieu_base_thi_bao_ro() {
    let d = tempfile::tempdir().unwrap();
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("base.yaml"), "{e}");
}

#[test]
fn env_khong_ton_tai_thi_van_chay_bang_base() {
    let d = tempfile::tempdir().unwrap();
    viet(d.path(), "base.yaml", "env: dev\n");
    let c = load(d.path(), "khong-co-file-nay").expect("file theo env la tuy chon");
    assert_eq!(c.sim.tick_rate, 20);
}

#[test]
fn field_la_thi_bao_loi_chu_khong_bo_qua() {
    // `deny_unknown_fields`: mot loi chinh ta trong ten field phai la loi, khong
    // duoc tro thanh mot field moi im lang.
    let d = tempfile::tempdir().unwrap();
    viet(d.path(), "base.yaml", "env: dev\nsim:\n  tick_rat: 20\n");
    assert!(load(d.path(), "dev").is_err(), "loi chinh ta phai bi bat");
}

#[test]
fn chunk_size_khong_phai_luy_thua_2_thi_tu_choi() {
    let d = tempfile::tempdir().unwrap();
    viet(d.path(), "base.yaml", "env: dev\nsim:\n  chunk_size: 30\n");
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("sim.chunk_size"), "{e}");
}

#[test]
fn kho_ky_uc_trung_file_save_thi_tu_choi() {
    // P0-09: hai thu nay phai o hai file khac nhau.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\npersistence:\n  url: \"a.sqlite\"\nvector:\n  url: \"a.sqlite\"\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("vector.url"), "{e}");
}

#[test]
fn do_tre_nhan_thuc_bang_0_o_che_do_that_thi_tu_choi() {
    // §20.2.2: bang 0 nghia la the gioi phu thuoc toc do duong truyen.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\nllm:\n  mode: LIVE\n  provider: anthropic\n  cognitive_latency_ticks: 0\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("cognitive_latency_ticks"), "{e}");
}

#[test]
fn bi_mat_lot_vao_yaml_thi_tu_choi_khoi_dong() {
    // §P10.6: file config duoc commit, nen DSN co mat khau khong duoc o day.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\npersistence:\n  url: \"postgres://mow:matkhauthat@db:5432/mow\"\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    assert!(e.to_string().contains("persistence.url"), "{e}");
    assert!(
        e.to_string().contains(".env"),
        "loi phai chi ra cho dat dung: {e}"
    );
}

#[test]
fn endpoint_khong_co_thong_tin_dang_nhap_thi_khong_bi_bao_nham() {
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\nobservability:\n  otel_endpoint: \"http://jaeger:4317\"\n",
    );
    load(d.path(), "dev").expect("endpoint binh thuong khong phai bi mat");
}

#[test]
fn bao_het_moi_loi_mot_lan() {
    // Sua tung loi mot roi khoi dong lai la vong lap cham.
    let d = tempfile::tempdir().unwrap();
    viet(
        d.path(),
        "base.yaml",
        "env: dev\nsim:\n  tick_rate: 0\n  chunk_size: 30\nvector:\n  dimension: 0\n",
    );
    let e = load(d.path(), "dev").expect_err("phai loi");
    let s = e.to_string();
    assert!(s.contains("sim.tick_rate"), "{s}");
    assert!(s.contains("sim.chunk_size"), "{s}");
    assert!(s.contains("vector.dimension"), "{s}");
}

#[test]
fn sinh_duoc_json_schema() {
    let s = AppConfig::json_schema_string();
    assert!(s.contains("cognitive_latency_ticks"), "{s}");
    assert!(s.contains("AppConfig"));
}
