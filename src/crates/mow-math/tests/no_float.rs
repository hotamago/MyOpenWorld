//! Test kiến trúc: **không có số thực trên đường commit** (`plan.md §P10.2`).
//!
//! Lint `clippy::float_arithmetic` bắt được *phép toán* trên số thực, nhưng
//! không bắt được một trường `f64` nằm im trong struct, một `as f32` để hiển
//! thị, hay một `parse::<f64>()` khi đọc content. Những thứ đó đủ để phá
//! determinism: một `f64` trong state là một giá trị có thể khác nhau giữa hai
//! máy, và nó sẽ đi thẳng vào state hash.
//!
//! Bài test này quét mã nguồn. Nó thô, nhưng nó chạy trong mọi CI và không
//! phụ thuộc vào việc ai đó có bật clippy hay không.

use std::path::Path;

/// Những mẫu tuyệt đối không được xuất hiện.
const CAM: &[&str] = &[
    "f32",
    "f64",
    "as f32",
    "as f64",
    "float",
    "powf",
    "sqrt()",
    "::consts::PI",
];

/// Dòng có đánh dấu này được miễn, và **phải** kèm lý do ngay sau đó.
const MIEN_TRU: &str = "allow-float:";

#[test]
fn khong_co_so_thuc_trong_mow_math() {
    let goc = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut vi_pham = Vec::new();
    quet(&goc, &mut vi_pham);

    assert!(
        vi_pham.is_empty(),
        "tìm thấy số thực trong đường commit:\n{}",
        vi_pham.join("\n")
    );
}

fn quet(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    // Sắp xếp để thông báo lỗi ổn định giữa các lần chạy.
    paths.sort();

    for p in paths {
        if p.is_dir() {
            quet(&p, out);
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Ok(noi_dung) = std::fs::read_to_string(&p) else {
            continue;
        };
        for (i, dong) in noi_dung.lines().enumerate() {
            if dong.contains(MIEN_TRU) {
                continue;
            }
            // Bỏ qua tài liệu: các mục doc giải thích *vì sao* không dùng số
            // thực thì đương nhiên phải nhắc tới chúng.
            let cat = dong.trim_start();
            if cat.starts_with("//") || cat.starts_with("/*") || cat.starts_with('*') {
                continue;
            }
            for mau in CAM {
                if dong.contains(mau) {
                    out.push(format!(
                        "{}:{}: chứa `{}`\n    {}",
                        p.display(),
                        i + 1,
                        mau,
                        dong.trim()
                    ));
                }
            }
        }
    }
}
