//! Utility AI: phản xạ, thói quen, kế hoạch chiến thuật (`idea.md §10.3`, `PB-12`).
//!
//! ## Vì sao khu định cư phải sống được **không cần LLM**
//!
//! Cả Giai đoạn B chạy ở `llm_mode: STUB`, và `progress.md` nói thẳng lý do:
//!
//! > Nếu cần LLM để khu định cư hoạt động thì **thiết kế đã sai**.
//!
//! Đó không phải một ràng buộc về chi phí. Nó là một ràng buộc về **kiến trúc**:
//!
//! - Một thế giới có mười nghìn dân không thể gọi mười nghìn lời LLM mỗi tick.
//!   Phần lớn cư dân phải sống bằng luật, và LLM chỉ dành cho những người mà
//!   người chơi đang thật sự chú ý.
//! - Nếu hành vi nền cần LLM, thì `LOD Far` không thể tồn tại — và không có LOD
//!   thì thế giới không lớn được.
//! - Và nếu LLM hỏng, hết hạn mức, hay chậm, thì thế giới phải vẫn chạy. Fallback
//!   không phải một chế độ suy thoái; nó là **nền** mà LLM đứng lên trên.
//!
//! ## Ba tầng, và LLM là tầng thứ tư
//!
//! ```text
//! 4  chiến lược   LLM            "ta muốn trở thành ai"        Giai đoạn C
//! 3  chiến thuật  utility AI     "làm gì với tình huống này"   ở đây
//! 2  thói quen    lịch sinh hoạt "giờ này thường làm gì"       ở đây
//! 1  phản xạ      luật cứng      "tay chạm lửa thì rụt lại"    ở đây
//! ```
//!
//! Tầng thấp hơn **luôn thắng** khi kích hoạt. Một người đang bàn triết học mà
//! bị đâm thì né, không cần suy nghĩ — và không cần một lời gọi mô hình.

use crate::perception::CognitionContext;
use mow_math::{CanonicalHash, StateHasher};
use serde::{Deserialize, Serialize};

/// Tầng ra quyết định.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// Phản xạ: luật cứng, không cân nhắc.
    Reflex,
    /// Thói quen: lịch sinh hoạt theo giờ.
    Routine,
    /// Chiến thuật: cân nhắc bằng điểm hữu dụng.
    Tactical,
    /// Chiến lược: LLM. Chưa có ở Giai đoạn B.
    Strategic,
}

impl CanonicalHash for Layer {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(match self {
            Layer::Reflex => "reflex",
            Layer::Routine => "routine",
            Layer::Tactical => "tactical",
            Layer::Strategic => "strategic",
        });
    }
}

/// Một cân nhắc: một yếu tố đóng góp vào điểm hữu dụng.
///
/// Tách thành nhiều cân nhắc thay vì một công thức lớn, vì `§18.13` đòi mọi giá
/// trị suy ra phải bấm được về nguồn. "Vì sao nó đi ăn" phải trả lời được bằng
/// *"đói 8 điểm, có thức ăn gần 3 điểm, không nguy hiểm 2 điểm"* chứ không phải
/// bằng *"điểm 13"*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consideration {
    /// Tên, hiện trong lời giải thích.
    pub name: &'static str,
    /// Đóng góp. Âm là ngăn cản.
    pub score: i64,
}

/// Một lựa chọn đã chấm điểm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Hành động.
    pub action: String,
    /// Mục tiêu, nếu có.
    pub target: Option<String>,
    /// Tầng đề xuất nó.
    pub layer: Layer,
    /// Tổng điểm.
    pub score: i64,
    /// Từng phần đóng góp.
    pub considerations: Vec<Consideration>,
}

impl Candidate {
    /// Dựng từ một danh sách cân nhắc.
    pub fn new(
        action: &str,
        target: Option<String>,
        layer: Layer,
        considerations: Vec<Consideration>,
    ) -> Candidate {
        Candidate {
            action: action.to_owned(),
            target,
            score: considerations.iter().map(|c| c.score).sum(),
            layer,
            considerations,
        }
    }

    /// Lời giải thích đọc được, cho panel Entity Mind (`§18.3`).
    pub fn explain(&self) -> String {
        let phan: Vec<String> = self
            .considerations
            .iter()
            .map(|c| format!("{} {:+}", c.name, c.score))
            .collect();
        format!("{} = {} [{}]", self.action, self.score, phan.join(", "))
    }
}

/// Một luật phản xạ.
pub struct Reflex {
    /// Tên.
    pub id: &'static str,
    /// Có kích hoạt không.
    pub trigger: fn(&CognitionContext) -> bool,
    /// Hành động khi kích hoạt.
    pub action: &'static str,
}

impl core::fmt::Debug for Reflex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Reflex")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Một mục trong lịch sinh hoạt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutineSlot {
    /// Giờ bắt đầu trong ngày, `0`–`23`.
    pub from_hour: u8,
    /// Giờ kết thúc.
    pub to_hour: u8,
    /// Hành động.
    pub action: String,
}

impl RoutineSlot {
    /// Giờ này có nằm trong khung không.
    ///
    /// Xử lý được khung vắt qua nửa đêm (`22 → 6`) — nếu không, giấc ngủ sẽ là
    /// khung duy nhất không hoạt động, và đó là khung quan trọng nhất.
    pub fn contains(&self, hour: u8) -> bool {
        if self.from_hour <= self.to_hour {
            hour >= self.from_hour && hour < self.to_hour
        } else {
            hour >= self.from_hour || hour < self.to_hour
        }
    }
}

/// Một cân nhắc chiến thuật.
pub struct Scorer {
    /// Hành động mà nó chấm.
    pub action: &'static str,
    /// Hàm chấm.
    pub score: fn(&CognitionContext) -> Vec<Consideration>,
}

impl core::fmt::Debug for Scorer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Scorer")
            .field("action", &self.action)
            .finish_non_exhaustive()
    }
}

/// Bộ não không cần LLM.
#[derive(Debug, Default)]
pub struct Brain {
    reflexes: Vec<Reflex>,
    routine: Vec<RoutineSlot>,
    scorers: Vec<Scorer>,
}

impl Brain {
    /// Rỗng.
    pub fn new() -> Brain {
        Brain::default()
    }

    /// Thêm một phản xạ.
    pub fn add_reflex(&mut self, r: Reflex) -> &mut Self {
        self.reflexes.push(r);
        // Sắp theo id: nếu hai phản xạ cùng kích hoạt, cái nào thắng phải là
        // hàm của dữ liệu, không của thứ tự đăng ký.
        self.reflexes.sort_by_key(|r| r.id);
        self
    }

    /// Thêm một mục lịch sinh hoạt.
    pub fn add_routine(&mut self, s: RoutineSlot) -> &mut Self {
        self.routine.push(s);
        self.routine
            .sort_by(|a, b| a.from_hour.cmp(&b.from_hour).then(a.action.cmp(&b.action)));
        self
    }

    /// Thêm một bộ chấm điểm chiến thuật.
    pub fn add_scorer(&mut self, s: Scorer) -> &mut Self {
        self.scorers.push(s);
        self.scorers.sort_by_key(|s| s.action);
        self
    }

    /// Quyết định làm gì.
    ///
    /// Tầng thấp thắng: phản xạ trước, rồi chiến thuật, rồi thói quen. Chiến
    /// thuật đứng **trên** thói quen vì một tình huống cụ thể phải thắng một
    /// lịch chung — người ta bỏ bữa khi nhà cháy.
    ///
    /// Trả `None` khi không có gì đáng làm. `None` là một câu trả lời hợp lệ:
    /// đứng yên cũng là một hành vi, và ép mọi thực thể luôn làm gì đó sẽ tạo
    /// ra một thế giới bồn chồn không nghỉ.
    pub fn decide(&self, ctx: &CognitionContext, hour: u8) -> Option<Candidate> {
        // ── Tầng 1: phản xạ ──────────────────────────────────────────────────
        for r in &self.reflexes {
            if (r.trigger)(ctx) && ctx.knows_action(r.action) {
                return Some(Candidate::new(
                    r.action,
                    None,
                    Layer::Reflex,
                    vec![Consideration {
                        name: r.id,
                        score: i64::MAX / 4,
                    }],
                ));
            }
        }

        // ── Tầng 3: chiến thuật ──────────────────────────────────────────────
        let mut ung_vien: Vec<Candidate> = self
            .scorers
            .iter()
            .filter(|s| ctx.knows_action(s.action))
            .map(|s| Candidate::new(s.action, None, Layer::Tactical, (s.score)(ctx)))
            .filter(|c| c.score > 0)
            .collect();

        // Sắp theo điểm giảm dần, phá hòa bằng tên hành động. Không có vế phá
        // hòa thì hai hành động cùng điểm sẽ chọn theo thứ tự đăng ký, và thứ
        // tự đăng ký không phải một phần của thế giới.
        ung_vien.sort_by(|a, b| b.score.cmp(&a.score).then(a.action.cmp(&b.action)));
        if let Some(c) = ung_vien.into_iter().next() {
            return Some(c);
        }

        // ── Tầng 2: thói quen ────────────────────────────────────────────────
        self.routine
            .iter()
            .find(|s| s.contains(hour) && ctx.knows_action(&s.action))
            .map(|s| {
                Candidate::new(
                    &s.action,
                    None,
                    Layer::Routine,
                    vec![Consideration {
                        name: "lịch sinh hoạt",
                        score: 1,
                    }],
                )
            })
    }

    /// Mọi lựa chọn đã chấm, kèm điểm — cho panel Entity Mind (`§18.3`).
    ///
    /// Khác [`Brain::decide`] ở chỗ nó trả về **tất cả**, kể cả những cái bị
    /// loại. Người xem cần biết nhân vật đã cân nhắc gì rồi bỏ, không chỉ biết
    /// nó đã chọn gì.
    pub fn deliberate(&self, ctx: &CognitionContext) -> Vec<Candidate> {
        let mut v: Vec<Candidate> = self
            .scorers
            .iter()
            .map(|s| Candidate::new(s.action, None, Layer::Tactical, (s.score)(ctx)))
            .collect();
        v.sort_by(|a, b| b.score.cmp(&a.score).then(a.action.cmp(&b.action)));
        v
    }

    /// Số phản xạ.
    pub fn reflex_count(&self) -> usize {
        self.reflexes.len()
    }
}

/// Bộ não sinh hoạt thường ngày, đủ để một khu định cư sống mà không cần LLM.
///
/// Đây là **bằng chứng** cho lời khẳng định ở đầu module. Nếu hàm này không đủ
/// để một ngôi làng vận hành, thì kiến trúc đã sai và nên biết ngay ở Giai đoạn
/// B chứ không phải ở Giai đoạn C.
pub fn villager_brain() -> Brain {
    let mut b = Brain::new();

    b.add_reflex(Reflex {
        id: "core.reflex.flee_pain",
        // Đau nhiều thì bỏ chạy, không cần cân nhắc.
        trigger: |ctx| ctx.internal.iter().any(|(k, v)| k == "pain" && *v > 70),
        action: "core.flee",
    });
    b.add_reflex(Reflex {
        id: "core.reflex.collapse",
        trigger: |ctx| ctx.internal.iter().any(|(k, v)| k == "hunger" && *v <= 0),
        action: "core.collapse",
    });

    b.add_scorer(Scorer {
        action: "core.eat",
        score: |ctx| {
            let doi = ctx
                .internal
                .iter()
                .find(|(k, _)| k == "hunger")
                .map_or(10_000, |(_, v)| *v);
            let mut c = vec![Consideration {
                name: "đói",
                // Càng đói càng muốn ăn; dưới 40% mới bắt đầu tính.
                score: ((4_000 - doi) / 100).max(0),
            }];
            if ctx
                .observations
                .iter()
                .any(|o| o.signs.iter().any(|s| s == "food"))
            {
                c.push(Consideration {
                    name: "thấy thức ăn",
                    score: 3,
                });
            }
            c
        },
    });

    b.add_scorer(Scorer {
        action: "core.sleep",
        score: |ctx| {
            let met = ctx
                .internal
                .iter()
                .find(|(k, _)| k == "fatigue")
                .map_or(10_000, |(_, v)| *v);
            vec![Consideration {
                name: "mệt",
                score: ((3_000 - met) / 100).max(0),
            }]
        },
    });

    b.add_routine(RoutineSlot {
        from_hour: 6,
        to_hour: 12,
        action: "core.work".into(),
    });
    b.add_routine(RoutineSlot {
        from_hour: 12,
        to_hour: 20,
        action: "core.socialize".into(),
    });
    // Khung vắt qua nửa đêm — khung quan trọng nhất, và cũng là khung dễ hỏng nhất.
    b.add_routine(RoutineSlot {
        from_hour: 22,
        to_hour: 6,
        action: "core.sleep".into(),
    });

    b
}
