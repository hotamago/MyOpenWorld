//! Tiền sử tùy chọn (`idea.md §7.6.4`, `§22.46`, `§22.17`, `PF-05`).
//!
//! Hai quy tắc, in đậm trong `§7.6.4`, và cả hai đều dễ vi phạm mà không ai
//! nhận ra:
//!
//! > **Tiền sử phải tiến qua thời gian thật.**
//!
//! Cách sai: sinh ra 300 năm lịch sử rồi gắn tất cả vào tick 0. Kết quả vẫn có
//! chiến tranh, dòng họ, tàn tích — và **mọi thứ trong world trông như vừa
//! được tạo ra cùng một lúc**: một cụ già 80 tuổi sinh cùng tick với đứa cháu,
//! một tàn tích 200 năm tuổi có cùng timestamp với thành phố đang sống.
//!
//! > **Lịch sử vĩ mô phải được chốt trước khi mở chunk.**
//!
//! Cách sai: để việc mở chunk quyết định có tàn tích ở đó hay không. Khi ấy
//! lịch sử **phụ thuộc vào đường đi của camera** — cùng một thế giới, hai
//! người chơi đi hai hướng, hai lịch sử khác nhau. Đúng loại lỗi mà `§7.2` đã
//! cấm với địa hình nền, và nó tệ hơn ở đây vì lịch sử có chuỗi nhân quả:
//! một mối thù sinh ra vì một trận đánh mà trận đánh ấy chỉ tồn tại nếu ai đó
//! đi ngang qua.
//!
//! ## Không vi phạm `§22.17`
//!
//! `§7.6.4`: *"Tiền sử là cách rẻ nhất để có một thế giới **đã sống** ngay từ
//! giờ đầu tiên mà không vi phạm `§22.17` — mọi thứ trong biên niên sử đều có
//! event thật đằng sau"*.
//!
//! Nên [`MacroEvent`] không có trường `narrative` hay `description` do model
//! sinh. Nó có `at_year`, `kind`, và những id liên quan. Văn bản là việc của
//! Historian, dựng **từ** những trường này, và dựng lại được bất cứ lúc nào.

use mow_core::Tick;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Số tick trong một năm mô phỏng.
///
/// Ở mức aggregate, tiền sử không chạy từng tick — nó nhảy theo năm. Nhưng
/// timestamp ghi ra vẫn là **tick thật**, để một event tiền sử và một event
/// người chơi so sánh được trực tiếp mà không cần đổi đơn vị.
pub const TICK_MOI_NAM: u64 = 20 * 60 * 60 * 24 * 365;

/// Loại event vĩ mô mà tiền sử sinh ra (`§7.6.4`).
///
/// Tập **đóng**. Mỗi loại là một thứ có hệ quả về sau: một biên giới dịch
/// chuyển đổi ai đánh thuế ai, một tàn tích đổi cái gì đào được ở đó, một mối
/// thù đổi ai từ chối buôn bán với ai.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroKind {
    /// Một thế lực lập ra.
    PolityFounded {
        /// Thế lực nào.
        polity: String,
    },
    /// Biên giới dịch chuyển.
    BorderShifted {
        /// Bên được.
        gained_by: String,
        /// Bên mất.
        lost_by: String,
        /// Số vùng.
        regions: u32,
    },
    /// Chiến tranh.
    War {
        /// Bên A.
        a: String,
        /// Bên B.
        b: String,
        /// Ai thắng; `None` là hòa.
        winner: Option<String>,
    },
    /// Một khu định cư bị bỏ, thành tàn tích.
    ///
    /// **Ở đúng nơi từng có thành phố** — vị trí không sinh ngẫu nhiên lúc mở
    /// chunk mà kế thừa từ event lập thành phố trước đó.
    SettlementAbandoned {
        /// Khu định cư nào.
        settlement: String,
        /// Ở vùng nào.
        region: u64,
    },
    /// Một dòng họ phân nhánh.
    LineageSplit {
        /// Dòng gốc.
        parent: String,
        /// Nhánh mới.
        branch: String,
    },
    /// Tuyến thương mại mở.
    TradeRoute {
        /// Từ đâu.
        from: String,
        /// Tới đâu.
        to: String,
    },
}

/// Một event vĩ mô đã xảy ra trong tiền sử.
///
/// **Không có trường văn bản tự do.** Đó là chỗ `§22.17` được giữ: một trường
/// `narrative` sẽ được điền bằng văn model sinh, và từ đó biên niên sử có
/// những câu không truy được về dữ liệu nào.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MacroEvent {
    /// Xảy ra ở năm thứ mấy của tiền sử.
    pub at_year: u32,
    /// Tick thật — **không phải 0**.
    pub at_tick: Tick,
    /// Chuyện gì.
    pub kind: MacroKind,
    /// Event nào gây ra nó, nếu có.
    ///
    /// Chỉ số trong danh sách event của cùng một lần chạy. Đây là thứ làm
    /// *"thù hằn có nguyên nhân truy ngược được"* thành thật chứ không thành
    /// một câu trong tài liệu.
    pub caused_by: Option<usize>,
}

/// Cấu hình một giai đoạn tiền sử.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrehistoryConfig {
    /// Chạy bao nhiêu năm.
    pub years: u32,
    /// Thế lực khởi đầu.
    pub initial_polities: Vec<String>,
    /// Seed cho các quyết định vĩ mô.
    pub seed: u64,
}

/// Kết quả tiền sử: **macro-delta** chốt trước khi mở chunk (`§22.46`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroDelta {
    /// Mọi event, theo thứ tự thời gian.
    pub events: Vec<MacroEvent>,
    /// Tàn tích: vùng → tên khu định cư cũ.
    pub ruins: BTreeMap<u64, String>,
    /// Biên giới cuối: thế lực → số vùng.
    pub borders: BTreeMap<String, u32>,
    /// Mối thù: cặp `(a, b)` với `a < b` để không lưu hai chiều.
    pub feuds: BTreeSet<(String, String)>,
    /// Dòng họ: nhánh → dòng cha.
    pub lineages: BTreeMap<String, String>,
    /// Tuyến thương mại.
    pub trade_routes: BTreeSet<(String, String)>,
    /// Tick mà tiền sử kết thúc — đồng hồ world bắt đầu từ đây.
    pub ends_at: Tick,
    /// Đã chốt chưa (`§22.46`).
    ///
    /// Một **trường**, không phải một thứ suy ra. Suy từ `ends_at > 0` thì một
    /// tiền sử 0 năm — hợp lệ, và chốt xong ngay — sẽ bị đọc là chưa chốt; suy
    /// từ `events.is_empty()` thì một lần chạy dở dang có vài event sẽ bị đọc
    /// là đã chốt. Chốt là một **hành động** mà [`run_prehistory`] làm ở dòng
    /// cuối, nên nó phải được ghi lại chứ không đoán.
    pub sealed: bool,
}

impl MacroDelta {
    /// **Mọi thứ đã chốt chưa** — điều kiện để được phép mở chunk (`§22.46`).
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Truy ngược nguyên nhân của một event, gần nhất trước.
    ///
    /// Đây là hàm trả lời *"vì sao hai nhà này thù nhau"*, và câu trả lời là
    /// một chuỗi event có thật chứ không phải một đoạn văn.
    pub fn causes_of(&self, idx: usize) -> Vec<&MacroEvent> {
        let mut v = Vec::new();
        let mut cur = idx;
        let mut da_qua = BTreeSet::new();
        while let Some(e) = self.events.get(cur) {
            let Some(c) = e.caused_by else { break };
            if !da_qua.insert(c) {
                break;
            }
            let Some(nguyen_nhan) = self.events.get(c) else {
                break;
            };
            v.push(nguyen_nhan);
            cur = c;
        }
        v
    }

    /// Vùng này có tàn tích không — câu hỏi mà việc mở chunk **tra**, không
    /// **quyết định**.
    pub fn ruin_at(&self, region: u64) -> Option<&str> {
        self.ruins.get(&region).map(String::as_str)
    }
}

/// RNG xác định, cùng thuật toán với `mow-math` nhưng cục bộ ở mức năm.
fn quay(seed: u64, nam: u32, muc: &str) -> u64 {
    let mut h = mow_math::StateHasher::with_domain("mow.prehistory.v1");
    h.write_u64(seed);
    h.write_u64(u64::from(nam));
    h.write_str(muc);
    u64::from_le_bytes(h.finish().0[..8].try_into().expect("32 byte đủ 8"))
}

/// Chạy tiền sử ở mức aggregate (`§7.6.4`).
///
/// **Không gọi LLM.** `§7.6.4` nói rõ; và lý do sâu hơn là một tiền sử gọi LLM
/// thì không tái lập được, nên hai người tải cùng worldseed sẽ nhận hai lịch
/// sử khác nhau.
///
/// Mọi event mang tick thật, tính từ `at_year` — đó là toàn bộ nội dung của
/// quy tắc *"tiến qua thời gian thật"*.
/// Trạng thái tích lũy trong lúc chạy tiền sử.
///
/// Gom vào một struct để [`run_prehistory`] chia được thành từng giai đoạn mà
/// không phải chuyền tám tham số `&mut` qua lại.
struct DangChay {
    events: Vec<MacroEvent>,
    borders: BTreeMap<String, u32>,
    ruins: BTreeMap<u64, String>,
    feuds: BTreeSet<(String, String)>,
    lineages: BTreeMap<String, String>,
    trade_routes: BTreeSet<(String, String)>,
}

/// Một năm chiến tranh: cuộc chiến, biên giới dịch chuyển, tàn tích.
///
/// Tách ra vì ba việc này **dính vào nhau bằng nhân quả** — biên giới đổi *vì*
/// cuộc chiến, tàn tích có *vì* cuộc chiến — nên chúng phải cùng biết chỉ số
/// của event chiến tranh.
fn mot_nam_chien_tranh(st: &mut DangChay, cfg: &PrehistoryConfig, nam: u32, ds: &[String]) {
    let tick = Tick(u64::from(nam) * TICK_MOI_NAM);
    let a = &ds[(quay(cfg.seed, nam, "war.a") as usize) % ds.len()];
    let b = &ds[(quay(cfg.seed, nam, "war.b") as usize) % ds.len()];
    if a == b {
        return;
    }
    let thang = if quay(cfg.seed, nam, "war.win").is_multiple_of(2) {
        a
    } else {
        b
    };
    let thua = if thang == a { b } else { a };

    let idx_war = st.events.len();
    st.events.push(MacroEvent {
        at_year: nam,
        at_tick: tick,
        kind: MacroKind::War {
            a: a.clone(),
            b: b.clone(),
            winner: Some(thang.clone()),
        },
        caused_by: None,
    });

    // Biên giới dịch chuyển **vì** chiến tranh đó.
    let so_vung = 1 + (quay(cfg.seed, nam, "war.regions") % 3) as u32;
    let mat = st.borders.get(thua).copied().unwrap_or(0).min(so_vung);
    if mat > 0 {
        *st.borders.entry(thang.clone()).or_insert(0) += mat;
        *st.borders.entry(thua.clone()).or_insert(0) -= mat;
        st.events.push(MacroEvent {
            at_year: nam,
            at_tick: tick,
            kind: MacroKind::BorderShifted {
                gained_by: thang.clone(),
                lost_by: thua.clone(),
                regions: mat,
            },
            caused_by: Some(idx_war),
        });
    }

    let cap = if a < b {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    };
    st.feuds.insert(cap);

    // Một khu định cư của bên thua bị bỏ.
    if quay(cfg.seed, nam, "ruin").is_multiple_of(3) {
        let vung = quay(cfg.seed, nam, "ruin.region") % 4_096;
        let ten = format!("{thua}.settlement.{nam}");
        st.ruins.insert(vung, ten.clone());
        st.events.push(MacroEvent {
            at_year: nam,
            at_tick: tick,
            kind: MacroKind::SettlementAbandoned {
                settlement: ten,
                region: vung,
            },
            caused_by: Some(idx_war),
        });
    }
}

/// Một năm hòa bình: dòng họ phân nhánh, tuyến thương mại mở.
fn mot_nam_hoa_binh(st: &mut DangChay, cfg: &PrehistoryConfig, nam: u32, ds: &[String]) {
    let tick = Tick(u64::from(nam) * TICK_MOI_NAM);

    if quay(cfg.seed, nam, "lineage").is_multiple_of(40) {
        let cha = &ds[(quay(cfg.seed, nam, "lineage.p") as usize) % ds.len()];
        let nhanh = format!("{cha}.branch.{nam}");
        st.lineages.insert(nhanh.clone(), cha.clone());
        st.events.push(MacroEvent {
            at_year: nam,
            at_tick: tick,
            kind: MacroKind::LineageSplit {
                parent: cha.clone(),
                branch: nhanh,
            },
            caused_by: None,
        });
    }

    // Tuyến thương mại — chỉ giữa hai bên **không** thù nhau.
    if quay(cfg.seed, nam, "trade").is_multiple_of(30) {
        let a = &ds[(quay(cfg.seed, nam, "trade.a") as usize) % ds.len()];
        let b = &ds[(quay(cfg.seed, nam, "trade.b") as usize) % ds.len()];
        let cap = if a < b {
            (a.clone(), b.clone())
        } else {
            (b.clone(), a.clone())
        };
        if a != b && !st.feuds.contains(&cap) {
            st.trade_routes.insert(cap.clone());
            st.events.push(MacroEvent {
                at_year: nam,
                at_tick: tick,
                kind: MacroKind::TradeRoute {
                    from: cap.0,
                    to: cap.1,
                },
                caused_by: None,
            });
        }
    }
}

/// Chạy tiền sử ở mức aggregate (`§7.6.4`).
///
/// **Không gọi LLM.** `§7.6.4` nói rõ; và lý do sâu hơn là một tiền sử gọi LLM
/// thì không tái lập được, nên hai người tải cùng worldseed sẽ nhận hai lịch
/// sử khác nhau.
///
/// Mọi event mang tick thật, tính từ `at_year` — đó là toàn bộ nội dung của
/// quy tắc *"tiến qua thời gian thật"*.
pub fn run_prehistory(cfg: &PrehistoryConfig) -> MacroDelta {
    let mut st = DangChay {
        events: Vec::new(),
        borders: cfg
            .initial_polities
            .iter()
            .map(|p| (p.clone(), 10))
            .collect(),
        ruins: BTreeMap::new(),
        feuds: BTreeSet::new(),
        lineages: BTreeMap::new(),
        trade_routes: BTreeSet::new(),
    };

    for p in &cfg.initial_polities {
        st.events.push(MacroEvent {
            at_year: 0,
            at_tick: Tick(0),
            kind: MacroKind::PolityFounded { polity: p.clone() },
            caused_by: None,
        });
    }

    let ds = cfg.initial_polities.clone();
    for nam in 1..=cfg.years {
        if ds.len() < 2 {
            continue;
        }
        // Chiến tranh: thưa, và cặp chọn xác định theo năm.
        if quay(cfg.seed, nam, "war").is_multiple_of(25) {
            mot_nam_chien_tranh(&mut st, cfg, nam, &ds);
        }
        mot_nam_hoa_binh(&mut st, cfg, nam, &ds);
    }

    MacroDelta {
        events: st.events,
        ruins: st.ruins,
        borders: st.borders,
        feuds: st.feuds,
        lineages: st.lineages,
        trade_routes: st.trade_routes,
        ends_at: Tick(u64::from(cfg.years) * TICK_MOI_NAM),
        // Dòng cuối cùng: mọi thứ đã tính xong thì mới chốt.
        sealed: true,
    }
}

/// Vì sao một lần mở chunk bị từ chối.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChunkError {
    /// Tiền sử chưa chốt.
    #[error(
        "chunk mở trước khi tiền sử chốt macro-delta (§22.46) — lịch sử sẽ phụ thuộc \
         vào đường đi của camera"
    )]
    PrehistoryNotSealed,
}

/// Chi tiết hóa một chunk **từ** macro-delta đã chốt (`§22.46`).
///
/// Chữ *"từ"* là toàn bộ điểm: hàm này **tra** macro-delta, nó không sinh thêm
/// lịch sử. Chữ ký nhận `&MacroDelta` chứ không `&mut` — không có đường nào để
/// việc mở chunk viết thêm vào lịch sử.
pub fn detail_chunk(delta: &MacroDelta, region: u64) -> Result<ChunkDetail, ChunkError> {
    if !delta.is_sealed() {
        return Err(ChunkError::PrehistoryNotSealed);
    }
    Ok(ChunkDetail {
        region,
        ruin: delta.ruin_at(region).map(str::to_owned),
    })
}

/// Những gì một chunk nhận được từ lịch sử vĩ mô.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDetail {
    /// Vùng nào.
    pub region: u64,
    /// Tàn tích ở đó, nếu có.
    pub ruin: Option<String>,
}
