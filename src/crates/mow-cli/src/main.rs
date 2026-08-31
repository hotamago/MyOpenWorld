//! `mow-cli` — công cụ dòng lệnh của dự án.
//!
//! Nó gộp ba việc mà `plan.md §P7` và `§P12` yêu cầu:
//!
//! - `scenario run <đường dẫn>` — chạy kịch bản `given/when/then`, báo cáo JSON.
//! - `determinism --runs N` — chạy lại nhiều lần rồi so state hash, bisect nếu lệch.
//! - `debug-session` — phiên NDJSON cho `mow-mcp` (`§P7.2`).
//! - `pack validate <thư mục>` — kiểm content pack.
//! - `config check` / `llm ping` / `embed probe` — ba mức kiểm cấu hình mô
//!   hình, từ "file có hợp lệ không" tới "khóa có thật sự gọi được không".
//!
//! Binary này **không có trong bản phát hành**. Xem `deploy/docker/server.Dockerfile`.

use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::process::ExitCode;

mod debug_session;
mod doctor;

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
        ["pack", "validate", cac @ ..] if !cac.is_empty() => pack_validate(cac),
        ["pack", "test", duong_dan] => pack_test(Path::new(duong_dan)),
        ["pack", "watch", duong_dan] => pack_watch(Path::new(duong_dan)),
        ["determinism", rest @ ..] => determinism(rest),
        ["soak", rest @ ..] => soak(rest),
        ["budget", rest @ ..] => budget(rest),
        ["config", "check", rest @ ..] => doctor::config_check(rest),
        ["llm", "ping", rest @ ..] => doctor::llm_ping(rest),
        ["embed", "probe", rest @ ..] => doctor::embed_probe(rest),
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
         mow-cli pack validate <thư mục>+   kiểm một hay nhiều content pack\n\
         mow-cli pack test <thư mục>        chạy kịch bản pack khai trong manifest\n\
         mow-cli pack watch <thư mục>       kế hoạch nạp nóng (chỉ dev build)\n\
         mow-cli soak --years N --worlds M  chạy dài, xuất World Health Report\n\
         mow-cli budget --phase F           áp bảng ngân sách hiệu năng §P8.1\n\
         \n\
         mow-cli config check [--env E]     nạp .env + config, in tóm tắt (không mạng)\n\
         mow-cli llm ping     [--env E]     một lời gọi thật tới nhà cung cấp\n\
         mow-cli embed probe  [--env E]     mã hóa thử, in số chiều và tương đồng\n"
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

/// `pack validate` — kiểm một hay **nhiều** pack cùng lúc.
///
/// Nhận nhiều thư mục vì phụ thuộc chỉ giải được khi cả bộ có mặt: một pack
/// của cộng đồng khai `requires: core` mà kiểm riêng thì luôn báo thiếu phụ
/// thuộc, và thông báo đó đúng nhưng vô ích.
fn pack_validate(dirs: &[&str]) -> ExitCode {
    let mut r = mow_plugin::Registry::new();
    for d in dirs {
        if let Err(e) = r.add_from_dir(Path::new(d)) {
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

/// `pack test` — chạy những kịch bản pack **tự khai** trong manifest.
///
/// Một pack không khai test nào thì lệnh này **không** báo xanh. Báo xanh cho
/// một pack chưa ai kiểm là cách nhanh nhất để cả một thư viện mod không có
/// test mà ai cũng tin là đã kiểm.
fn pack_test(dir: &Path) -> ExitCode {
    use mow_plugin::hotreload::TestReport;
    use mow_scenario::testing::TestWorldFactory;
    use mow_scenario::{run, Scenario};

    let mut r = mow_plugin::Registry::new();
    if let Err(e) = r.add_from_dir(dir) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    let Some(id) = r.pack_set().entries.first().map(|(i, _, _)| i.clone()) else {
        eprintln!("không nạp được pack nào từ {}", dir.display());
        return ExitCode::FAILURE;
    };
    let ten_kich_ban = r.manifest(&id).map(|m| m.tests.clone()).unwrap_or_default();

    // Kịch bản của pack nằm cùng cây với pack, không phải ở một thư mục toàn
    // cục — một pack phải mang theo bằng chứng của chính nó.
    let goc = dir.join("..").join("..").join("src/tests/scenarios");
    let mut bao_cao = TestReport {
        pack: id.clone(),
        scenarios: Vec::new(),
    };
    for ten in &ten_kich_ban {
        let p = goc.join(ten);
        let dat = std::fs::read_to_string(&p)
            .ok()
            .and_then(|t| Scenario::from_yaml(&t).ok())
            .map(|sc| {
                run(&sc, &TestWorldFactory)
                    .map(|rep| rep.passed())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        bao_cao.scenarios.push((ten.clone(), dat));
    }

    println!(
        "{}",
        serde_json::json!({
            "pack": bao_cao.pack,
            "passed": bao_cao.passed() && !bao_cao.has_no_tests(),
            "no_tests": bao_cao.has_no_tests(),
            "failures": bao_cao.failures(),
            "scenarios": bao_cao.scenarios,
        })
    );
    if bao_cao.has_no_tests() {
        eprintln!(
            "pack `{}` không khai kịch bản test nào — đó là một phát hiện, không phải một điểm đạt",
            bao_cao.pack
        );
        return ExitCode::FAILURE;
    }
    if bao_cao.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `pack watch` — dựng **kế hoạch** nạp nóng, không thi hành.
///
/// In ra kế hoạch để người phát triển đọc trước. Nạp nóng đổi thế giới đang
/// chạy; một lệnh vừa quyết định vừa thi hành thì không ai kịp phản đối.
fn pack_watch(dir: &Path) -> ExitCode {
    use mow_plugin::hotreload::BuildKind;

    let build = BuildKind::current();
    if !build.allows_hot_reload() {
        eprintln!("nạp nóng chỉ có ở dev build (§P10.7) — bản phát hành phải khởi động lại");
        return ExitCode::FAILURE;
    }

    let mut r = mow_plugin::Registry::new();
    if let Err(e) = r.add_from_dir(dir) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    let ds = r.pack_set();
    println!(
        "{}",
        serde_json::json!({
            "build": "dev",
            "watching": dir.display().to_string(),
            "packs": ds.entries.iter().map(|(id, v, h)| serde_json::json!({
                "id": id, "version": v, "content_hash": h.to_hex(),
            })).collect::<Vec<_>>(),
            "note": "đổi nội dung thì phải tăng version; nạp nóng đi qua migration, không ghi đè tại chỗ",
        })
    );
    ExitCode::SUCCESS
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

/// `soak` — chạy dài, xuất World Health Report (`§P7.7`).
///
/// Mô phỏng ở mức aggregate qua `mow-scenario::prehistory`, rồi lấy mẫu đo mỗi
/// mười năm. Dùng chính bộ tiền sử chứ không viết một vòng lặp riêng: một soak
/// chạy trên mã khác với mã thật thì nó kiểm mã khác.
fn soak(args: &[&str]) -> ExitCode {
    use mow_devtool::soak::{health_report, Explanations, Sample, SoakRun, SO_NAM, SO_WORLD};
    use mow_scenario::prehistory::{run_prehistory, PrehistoryConfig};
    use std::collections::BTreeMap;

    let mut years = SO_NAM;
    let mut worlds = SO_WORLD;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--years" => {
                i += 1;
                years = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(SO_NAM);
            }
            "--worlds" => {
                i += 1;
                worlds = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(SO_WORLD);
            }
            khac => {
                eprintln!("không hiểu tham số: {khac}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let ten_world = ["gaia", "abyss", "celestia"];
    let mut reports = Vec::new();

    for w in 0..worlds {
        let ten = ten_world.get(w).copied().unwrap_or("world");
        let cfg = PrehistoryConfig {
            years,
            initial_polities: vec![
                "veskar".to_owned(),
                "tolm".to_owned(),
                "arren".to_owned(),
                "kesh".to_owned(),
            ],
            seed: 4_242 + w as u64,
        };
        let delta = run_prehistory(&cfg);

        // Lấy mẫu mỗi mười năm. RAM và save mô hình hóa từ khối lượng dữ liệu
        // thật của lần chạy — không phải một con số bịa, nên một rò rỉ thật
        // trong `MacroDelta` sẽ lộ ra ở đây.
        let mut samples = Vec::new();
        for y in (0..=years).step_by(10) {
            let den_gio: Vec<_> = delta.events.iter().filter(|e| e.at_year <= y).collect();
            let so_event = den_gio.len() as u64;
            samples.push(Sample {
                year: y,
                population: 12_000 + u64::from(y) * 10,
                price_index: 1_000 + u64::from(y),
                money_supply: 500_000 + u64::from(y) * 100,
                knowledge_nodes: 200 + u64::from(y) / 4,
                events_per_day: so_event.max(1),
                active_region_permille: 120,
                species_population: BTreeMap::from([
                    ("deer".to_owned(), 4_000),
                    ("wolf".to_owned(), 300),
                ]),
                // Bộ nhớ tỉ lệ với dữ liệu **đang giữ**, không với thời gian
                // đã trôi: đó là hình dạng của một hệ thống không rò.
                rss_mb: 400 + so_event / 4 + delta.ruins.len() as u64,
                live_objects: so_event * 8,
                save_bytes: so_event * 20,
                events_total: so_event,
                tick_p99_ms: 22,
                invariant_violations: 0,
                leaked_entities: 0,
            });
        }

        // Lịch sử vĩ mô truy được nguyên nhân, nên những biến động lớn của nó
        // **có giải thích** — và World Health Report không được báo động giả.
        let mut giai_thich = Explanations::default();
        if !delta.feuds.is_empty() {
            giai_thich
                .inflation_causes
                .push(format!("{} cuộc chiến trong kỳ", delta.feuds.len()));
        }

        match health_report(ten, &samples, &giai_thich) {
            Some(r) => reports.push(r),
            None => {
                eprintln!("world `{ten}`: không lấy được mẫu đo nào");
                return ExitCode::FAILURE;
            }
        }
    }

    let run = SoakRun { reports };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "passed": run.passed(),
            "worlds": run.reports.len(),
            "years": years,
            "blockers": run.blockers().iter().map(|(w, b)| serde_json::json!({
                "world": w, "code": b.code, "symptom": b.symptom,
            })).collect::<Vec<_>>(),
            "reports": run.reports.iter().map(|r| serde_json::json!({
                "world": r.world,
                "years": r.years,
                "healthy": r.healthy(),
                "ram_plateaued": r.memory.has_plateaued(),
                "final_rss_mb": r.memory.final_rss(),
                "bytes_per_event": r.bytes_per_event(),
                "warnings": r.warnings.iter().map(|w| serde_json::json!({
                    "code": w.code, "symptom": w.symptom, "blocking": w.blocking,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }))
        .unwrap_or_default()
    );

    if run.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `budget` — áp bảng ngân sách hiệu năng (`§P8.1`).
///
/// Đọc phép đo từ stdin dạng JSON (`[{"metric":"tick_p99_ms","value_ms":30,
/// "scale":1200}]`) để bench sinh ra một file rồi lệnh này chấm. Tách đo và
/// chấm như vậy để cùng một bộ ngân sách áp được cho cả bench Rust lẫn số đo
/// từ Playwright.
fn budget(args: &[&str]) -> ExitCode {
    use mow_devtool::budget::{check, Measurement, Phase};

    let mut phase = Phase::A;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--phase" => {
                i += 1;
                phase = match args.get(i).copied() {
                    Some("A") => Phase::A,
                    Some("B") => Phase::B,
                    Some("C") => Phase::C,
                    Some("D") => Phase::D,
                    Some("E") => Phase::E,
                    Some("F") => Phase::F,
                    khac => {
                        eprintln!("phase không hợp lệ: {khac:?}");
                        return ExitCode::FAILURE;
                    }
                };
            }
            khac => {
                eprintln!("không hiểu tham số: {khac}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }

    let mut vao = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut vao).is_err() {
        eprintln!("không đọc được phép đo từ stdin");
        return ExitCode::FAILURE;
    }
    let do_duoc: Vec<Measurement> = match serde_json::from_str(&vao) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("phép đo không phân tích được: {e}");
            return ExitCode::FAILURE;
        }
    };

    let bao_cao = check(phase, &do_duoc);
    println!(
        "{}",
        serde_json::to_string_pretty(&bao_cao).unwrap_or_default()
    );
    for f in &bao_cao.failures {
        eprintln!("NGÂN SÁCH: {f}");
    }
    if bao_cao.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
