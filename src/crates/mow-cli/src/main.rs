//! `mow-cli` — công cụ dòng lệnh của dự án.
//!
//! Nó gộp ba việc mà `plan.md §P7` và `§P12` yêu cầu:
//!
//! - `scenario run <đường dẫn>` — chạy kịch bản `given/when/then`, báo cáo JSON.
//! - `determinism --runs N` — chạy lại nhiều lần rồi so state hash, bisect nếu lệch.
//! - `debug-session` — phiên NDJSON cho `mow-mcp` (`§P7.2`).
//! - `pack validate <thư mục>` — kiểm content pack.
//!
//! Binary này **không có trong bản phát hành**. Xem `deploy/docker/server.Dockerfile`.

use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::process::ExitCode;

mod debug_session;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();

    match refs.as_slice() {
        [] | ["help"] | ["--help"] | ["-h"] => {
            in_tro_giup();
            ExitCode::SUCCESS
        }
        ["debug-session"] => debug_session(),
        ["scenario", "run", duong_dan] => scenario_run(Path::new(duong_dan)),
        ["scenario", "list", duong_dan] => scenario_list(Path::new(duong_dan)),
        ["pack", "validate", duong_dan] => pack_validate(Path::new(duong_dan)),
        ["determinism", rest @ ..] => determinism(rest),
        khac => {
            eprintln!("không hiểu lệnh: {khac:?}");
            in_tro_giup();
            ExitCode::FAILURE
        }
    }
}

fn in_tro_giup() {
    println!(
        "mow-cli — công cụ phát triển My Open World\n\
         \n\
         mow-cli debug-session              phiên NDJSON cho mow-mcp (§P7.2)\n\
         mow-cli scenario run <đường dẫn>   chạy kịch bản, in báo cáo JSON\n\
         mow-cli scenario list <đường dẫn>  liệt kê kịch bản và kiểm cấu trúc\n\
         mow-cli determinism --runs N       chạy lại N lần, so state hash, bisect\n\
         mow-cli pack validate <thư mục>    kiểm một content pack\n"
    );
}

fn debug_session() -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut r = BufReader::new(stdin.lock());
    let mut w = BufWriter::new(stdout.lock());
    let mut s = debug_session::Session::new();
    match s.serve(&mut r, &mut w) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("phiên gỡ lỗi lỗi: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Gom mọi file `.yaml` dưới một đường dẫn, đã sắp xếp.
fn thu_thap(root: &Path) -> Vec<std::path::PathBuf> {
    let mut ra = Vec::new();
    if root.is_file() {
        ra.push(root.to_path_buf());
        return ra;
    }
    let mut hang_doi = vec![root.to_path_buf()];
    while let Some(d) = hang_doi.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                hang_doi.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some("yaml") {
                ra.push(p);
            }
        }
    }
    // Sắp xếp để thứ tự chạy và thứ tự báo cáo ổn định giữa các hệ điều hành.
    ra.sort();
    ra
}

fn scenario_run(root: &Path) -> ExitCode {
    use mow_scenario::testing::TestWorldFactory;

    let mut dat = 0;
    let mut truot = 0;
    let mut bao_cao = Vec::new();

    for p in thu_thap(root) {
        let text = match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}: không đọc được: {e}", p.display());
                truot += 1;
                continue;
            }
        };
        let sc = match mow_scenario::Scenario::from_yaml(&text) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}: YAML hỏng: {e}", p.display());
                truot += 1;
                continue;
            }
        };
        match mow_scenario::run(&sc, &TestWorldFactory) {
            Ok(rep) => {
                if rep.passed() {
                    dat += 1;
                } else {
                    truot += 1;
                }
                eprint!("{rep}");
                bao_cao.push(serde_json::json!({
                    "scenario": rep.scenario,
                    "passed": rep.passed(),
                    "ticks": rep.ticks,
                    "state_hash": rep.state_hash.to_hex(),
                    "bindings": rep.bindings.iter()
                        .map(|(a, id)| (a.clone(), serde_json::json!(id.get())))
                        .collect::<serde_json::Map<String, serde_json::Value>>(),
                    "assertions": rep.assertions.iter().map(|a| serde_json::json!({
                        "name": a.name, "passed": a.passed, "detail": a.detail
                    })).collect::<Vec<_>>(),
                }));
            }
            Err(e) => {
                truot += 1;
                eprintln!("{}: {e}", p.display());
                bao_cao.push(serde_json::json!({
                    "scenario": sc.scenario, "passed": false, "error": e.to_string()
                }));
            }
        }
    }

    // Báo cáo máy đọc ra stdout, báo cáo người đọc ra stderr — để
    // `mow-cli scenario run ... > report.json` vẫn hiện tiến trình trên màn hình.
    println!(
        "{}",
        serde_json::json!({ "passed": dat, "failed": truot, "scenarios": bao_cao })
    );
    eprintln!("\n{dat} đạt, {truot} trượt");

    if truot == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn scenario_list(root: &Path) -> ExitCode {
    let mut loi = 0;
    for p in thu_thap(root) {
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        match mow_scenario::Scenario::from_yaml(&text) {
            Ok(sc) => match sc.validate() {
                Ok(()) => println!("  ✓ {:<28} {}", sc.scenario, p.display()),
                Err(e) => {
                    println!("  ✗ {:<28} {}", sc.scenario, p.display());
                    for m in e {
                        println!("      {m}");
                    }
                    loi += 1;
                }
            },
            Err(e) => {
                println!("  ✗ {} — YAML hỏng: {e}", p.display());
                loi += 1;
            }
        }
    }
    if loi == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn pack_validate(dir: &Path) -> ExitCode {
    let mut r = mow_plugin::Registry::new();
    match r.add_from_dir(dir) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    }
    match r.resolve_order() {
        Ok(order) => {
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "load_order": order.0,
                    "packs": r.pack_set().entries.iter().map(|(id, v, h)| serde_json::json!({
                        "id": id, "version": v, "content_hash": h.to_hex()
                    })).collect::<Vec<_>>(),
                })
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn determinism(args: &[&str]) -> ExitCode {
    use mow_devtool::determinism::{checkpoints_upto, compare, Runnable, Verdict};
    use mow_scenario::testing::TestWorldFactory;
    use mow_scenario::WorldFactory;

    let mut runs = 2usize;
    let mut ticks = 1_000u64;
    let mut seed = "test:tiny_village".to_owned();
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--runs" => {
                runs = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(2);
                i += 2;
            }
            "--ticks" => {
                ticks = args
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1_000);
                i += 2;
            }
            "--worldseed" => {
                seed = args.get(i + 1).map_or(seed.clone(), |v| (*v).to_owned());
                i += 2;
            }
            khac => {
                eprintln!("tham số lạ: {khac}");
                return ExitCode::FAILURE;
            }
        }
    }

    /// Một lần chạy: dựng lại thế giới từ đầu rồi tiến tới `tick`.
    ///
    /// **Dựng lại từ đầu mỗi lần** là bắt buộc. Nếu tiếp tục từ lần trước, ta
    /// đang so hai điểm trên cùng một dòng thời gian và bisect sẽ luôn nói
    /// "không lệch".
    struct LanChay {
        nhan: String,
        worldseed: String,
    }
    impl Runnable for LanChay {
        fn label(&self) -> String {
            self.nhan.clone()
        }
        fn hash_at(&mut self, tick: u64) -> mow_math::StateHash {
            let mut sim = TestWorldFactory
                .build(&self.worldseed, &std::collections::BTreeMap::new())
                .expect("worldseed hợp lệ");
            if tick > 0 {
                sim.advance(tick).expect("tiến được");
            }
            sim.state_hash()
        }
    }

    let mut ds: Vec<Box<dyn Runnable>> = (0..runs.max(2))
        .map(|k| {
            Box::new(LanChay {
                nhan: format!("run#{k}"),
                worldseed: seed.clone(),
            }) as Box<dyn Runnable>
        })
        .collect();

    match compare(&mut ds, &checkpoints_upto(ticks)) {
        Verdict::Identical { hash, checkpoints } => {
            println!(
                "{}",
                serde_json::json!({
                    "deterministic": true,
                    "runs": ds.len(),
                    "checkpoints": checkpoints,
                    "state_hash": hash.to_hex(),
                })
            );
            eprintln!("✓ {} lần chạy khớp nhau tại {checkpoints} mốc", ds.len());
            ExitCode::SUCCESS
        }
        Verdict::Diverged(d) => {
            eprint!("{d}");
            println!(
                "{}",
                serde_json::json!({
                    "deterministic": false,
                    "first_bad_tick": d.first_bad_tick,
                    "last_good_tick": d.last_good_tick,
                    "hashes": d.hashes.iter()
                        .map(|(k, v)| (k.clone(), serde_json::json!(v.to_hex())))
                        .collect::<serde_json::Map<String, serde_json::Value>>(),
                })
            );
            ExitCode::FAILURE
        }
    }
}
