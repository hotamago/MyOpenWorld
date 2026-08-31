//! DSL luật Tier 0 (`idea.md §15.3`, `§13.9.1`, `PE-01`).
//!
//! > **Không dùng `eval` hoặc chạy code do LLM sinh trực tiếp.**
//!
//! Đó là câu ngắn nhất trong `§15.3` và là câu đắt nhất để tôn trọng. Cách rẻ là
//! nhận một chuỗi biểu thức rồi gọi một trình thông dịch có sẵn — nó chạy, nó
//! linh hoạt, và nó cho một mô hình ngôn ngữ quyền ghi tùy ý vào thế giới.
//!
//! Ở đây biểu thức là một **cây đã phân tích**, các phép toán là một tập **đóng**,
//! và mọi giá trị mang **đơn vị**.
//!
//! ## Ba thứ mà bộ kiểm tĩnh phải bắt được, và vì sao mỗi thứ quan trọng
//!
//! | Kiểm | Bắt được gì | Nếu bỏ qua |
//! |---|---|---|
//! | **kiểu** | cộng một `ratio` vào một `J` | luật chạy, ra số vô nghĩa |
//! | **đơn vị** | cộng `mMU` vào `kJ` | mana biến thành nhiệt lượng, im lặng |
//! | **dừng** | biểu thức tự tham chiếu | treo cả simulation |
//!
//! Dòng thứ hai là dòng đáng sợ nhất: một lỗi đơn vị **không sai cú pháp, không
//! sai kiểu, không panic**. Nó cho ra một con số, và con số đó đi thẳng vào state.
//!
//! ## Fixed-point, không float
//!
//! `§15.3` viết ngay trong ví dụ YAML: *"mọi biểu thức chạy trên fixed-point
//! Q16.16; không có float trong đường commit"*. Nên [`Value`] không có biến thể
//! số thực, và không có hàm nào nhận `f64`.
//!
//! ## Đảm bảo dừng, không phải "hy vọng dừng"
//!
//! [`Expr`] là một **cây**, không phải một đồ thị: không có vòng lặp, không có
//! đệ quy, không có nhảy. Độ sâu bị chặn ở [`MAX_DEPTH`]. Một chương trình như
//! thế **luôn** dừng, và đó là tính chất mà một trình thông dịch tổng quát không
//! có cách nào hứa.

use mow_math::Fx;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Độ sâu tối đa của một biểu thức.
///
/// Chặn ở đây thay vì đếm bước lúc chạy: độ sâu kiểm được **tĩnh**, nên một luật
/// quá phức tạp bị từ chối lúc nạp chứ không lúc đang chạy giữa một trận đánh.
pub const MAX_DEPTH: u32 = 32;

/// Đơn vị của một giá trị.
///
/// Là một phần của **kiểu**, không phải một chú thích. Cộng hai giá trị khác đơn
/// vị là lỗi biên dịch của DSL, không phải một cảnh báo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// Không thứ nguyên, `[0,1]` hoặc hệ số.
    Ratio,
    /// Mana, milli-mana-unit.
    Mmu,
    /// Năng lượng, joule.
    Joule,
    /// Năng lượng, kilojoule.
    Kilojoule,
    /// Khoảng cách, mét.
    Metre,
    /// Thời gian, tick.
    Tick,
}

impl Unit {
    /// Tên ổn định.
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Ratio => "ratio",
            Unit::Mmu => "mMU",
            Unit::Joule => "J",
            Unit::Kilojoule => "kJ",
            Unit::Metre => "m",
            Unit::Tick => "tick",
        }
    }
}

/// Một giá trị có đơn vị. **Không có biến thể số thực.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantity {
    /// Giá trị, Q16.16.
    pub value: Fx,
    /// Đơn vị.
    pub unit: Unit,
}

/// Phép toán — một tập **đóng**.
///
/// Đóng nghĩa là: thêm một phép mới phải sửa file này, đi qua review, và cập nhật
/// [`Expr::typecheck`]. Đó là chỗ khác biệt với `eval`: ở đó tập phép toán là
/// "mọi thứ ngôn ngữ chủ nhà làm được", và không ai kiểm soát nổi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expr {
    /// Hằng số.
    Const(Quantity),
    /// Đọc một biến từ ngữ cảnh.
    Var(String),
    /// Cộng. Hai vế **phải cùng đơn vị**.
    Add(Box<Expr>, Box<Expr>),
    /// Trừ. Hai vế phải cùng đơn vị.
    Sub(Box<Expr>, Box<Expr>),
    /// Nhân. Một vế **phải là `ratio`** — nhân hai đại lượng có thứ nguyên cho ra
    /// một đơn vị mà DSL này không biểu diễn được, nên nó bị cấm thay vì bịa ra.
    Mul(Box<Expr>, Box<Expr>),
    /// Chia. Mẫu phải là `ratio`.
    Div(Box<Expr>, Box<Expr>),
    /// Chặn vào khoảng. Cả ba phải cùng đơn vị.
    Clamp {
        /// Giá trị.
        value: Box<Expr>,
        /// Cận dưới.
        lo: Box<Expr>,
        /// Cận trên.
        hi: Box<Expr>,
    },
    /// Đổi đơn vị **tường minh**, kèm hệ số.
    ///
    /// Không có đổi đơn vị ngầm. `kJ → J` phải viết ra, vì một hệ số 1000 lặng lẽ
    /// là chỗ mọi lỗi đơn vị trốn vào.
    Convert {
        /// Biểu thức.
        inner: Box<Expr>,
        /// Sang đơn vị nào.
        to: Unit,
        /// Nhân với bao nhiêu.
        factor: Fx,
    },
}

/// Lỗi khi kiểm hoặc chạy một luật.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleError {
    /// Hai vế khác đơn vị.
    #[error(
        "khác đơn vị: {left} và {right} — một lỗi không sai cú pháp, không sai kiểu, và im lặng"
    )]
    UnitMismatch {
        /// Đơn vị vế trái.
        left: &'static str,
        /// Đơn vị vế phải.
        right: &'static str,
    },
    /// Nhân hoặc chia mà không vế nào là `ratio`.
    #[error("nhân/chia hai đại lượng có thứ nguyên: {left} × {right} — DSL không biểu diễn được đơn vị kết quả")]
    DimensionalProduct {
        /// Đơn vị vế trái.
        left: &'static str,
        /// Đơn vị vế phải.
        right: &'static str,
    },
    /// Biến không có trong ngữ cảnh.
    #[error("biến `{0}` không có trong ngữ cảnh")]
    UnknownVar(String),
    /// Biểu thức quá sâu.
    #[error(
        "biểu thức sâu {depth} bậc, trần là {MAX_DEPTH} — từ chối lúc nạp, không phải lúc chạy"
    )]
    TooDeep {
        /// Sâu bao nhiêu.
        depth: u32,
    },
    /// Lỗi số học.
    #[error("lỗi số học: {0}")]
    Math(String),
}

impl Expr {
    /// Độ sâu của cây.
    pub fn depth(&self) -> u32 {
        match self {
            Expr::Const(_) | Expr::Var(_) => 1,
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
                1 + a.depth().max(b.depth())
            }
            Expr::Clamp { value, lo, hi } => 1 + value.depth().max(lo.depth()).max(hi.depth()),
            Expr::Convert { inner, .. } => 1 + inner.depth(),
        }
    }

    /// **Kiểm tĩnh**: kiểu, đơn vị, độ sâu — trước khi chạy một lần nào.
    ///
    /// Trả về đơn vị của kết quả. `§15.3` bước 4 gọi đây là *static validation*,
    /// và nó chạy lúc **nạp luật**, không lúc thi hành.
    pub fn typecheck(&self, vars: &BTreeMap<String, Unit>) -> Result<Unit, RuleError> {
        if self.depth() > MAX_DEPTH {
            return Err(RuleError::TooDeep {
                depth: self.depth(),
            });
        }
        self.check_inner(vars)
    }

    fn check_inner(&self, vars: &BTreeMap<String, Unit>) -> Result<Unit, RuleError> {
        match self {
            Expr::Const(q) => Ok(q.unit),
            Expr::Var(n) => vars
                .get(n)
                .copied()
                .ok_or_else(|| RuleError::UnknownVar(n.clone())),
            Expr::Add(a, b) | Expr::Sub(a, b) => {
                let (ua, ub) = (a.check_inner(vars)?, b.check_inner(vars)?);
                if ua == ub {
                    Ok(ua)
                } else {
                    Err(RuleError::UnitMismatch {
                        left: ua.as_str(),
                        right: ub.as_str(),
                    })
                }
            }
            Expr::Mul(a, b) => {
                let (ua, ub) = (a.check_inner(vars)?, b.check_inner(vars)?);
                match (ua, ub) {
                    (Unit::Ratio, u) | (u, Unit::Ratio) => Ok(u),
                    _ => Err(RuleError::DimensionalProduct {
                        left: ua.as_str(),
                        right: ub.as_str(),
                    }),
                }
            }
            Expr::Div(a, b) => {
                let (ua, ub) = (a.check_inner(vars)?, b.check_inner(vars)?);
                if ub == Unit::Ratio {
                    Ok(ua)
                } else {
                    Err(RuleError::DimensionalProduct {
                        left: ua.as_str(),
                        right: ub.as_str(),
                    })
                }
            }
            Expr::Clamp { value, lo, hi } => {
                let (uv, ul, uh) = (
                    value.check_inner(vars)?,
                    lo.check_inner(vars)?,
                    hi.check_inner(vars)?,
                );
                if uv == ul && ul == uh {
                    Ok(uv)
                } else {
                    Err(RuleError::UnitMismatch {
                        left: uv.as_str(),
                        right: if uv == ul { uh.as_str() } else { ul.as_str() },
                    })
                }
            }
            Expr::Convert { inner, to, .. } => {
                inner.check_inner(vars)?;
                Ok(*to)
            }
        }
    }

    /// Chạy biểu thức.
    ///
    /// **Luôn dừng.** Cây hữu hạn, độ sâu đã bị chặn lúc kiểm tĩnh, không có
    /// vòng lặp nào trong ngôn ngữ.
    pub fn eval(&self, ctx: &BTreeMap<String, Quantity>) -> Result<Quantity, RuleError> {
        let loi = |e: mow_math::MathError| RuleError::Math(e.to_string());
        match self {
            Expr::Const(q) => Ok(*q),
            Expr::Var(n) => ctx
                .get(n)
                .copied()
                .ok_or_else(|| RuleError::UnknownVar(n.clone())),
            Expr::Add(a, b) => {
                let (x, y) = (a.eval(ctx)?, b.eval(ctx)?);
                Ok(Quantity {
                    value: x.value.add(y.value).map_err(loi)?,
                    unit: x.unit,
                })
            }
            Expr::Sub(a, b) => {
                let (x, y) = (a.eval(ctx)?, b.eval(ctx)?);
                Ok(Quantity {
                    value: x.value.sub(y.value).map_err(loi)?,
                    unit: x.unit,
                })
            }
            Expr::Mul(a, b) => {
                let (x, y) = (a.eval(ctx)?, b.eval(ctx)?);
                Ok(Quantity {
                    value: x.value.mul(y.value).map_err(loi)?,
                    unit: if x.unit == Unit::Ratio {
                        y.unit
                    } else {
                        x.unit
                    },
                })
            }
            Expr::Div(a, b) => {
                let (x, y) = (a.eval(ctx)?, b.eval(ctx)?);
                Ok(Quantity {
                    value: x.value.div(y.value).map_err(loi)?,
                    unit: x.unit,
                })
            }
            Expr::Clamp { value, lo, hi } => {
                let (v, l, h) = (value.eval(ctx)?, lo.eval(ctx)?, hi.eval(ctx)?);
                Ok(Quantity {
                    value: v.value.clamp(l.value, h.value),
                    unit: v.unit,
                })
            }
            Expr::Convert { inner, to, factor } => {
                let v = inner.eval(ctx)?;
                Ok(Quantity {
                    value: v.value.mul(*factor).map_err(loi)?,
                    unit: *to,
                })
            }
        }
    }
}

/// Một luật ở dạng dữ liệu (`§15.3`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Định danh có version: `magic.firebolt.v1`.
    pub rule_id: String,
    /// **Phiên bản**, để `§13.9.5` không hồi tố.
    pub version: u32,
    /// Action nào kích hoạt.
    pub trigger: String,
    /// Đầu vào và đơn vị của chúng.
    pub inputs: BTreeMap<String, Unit>,
    /// Các đại lượng tính ra.
    pub compute: BTreeMap<String, Expr>,
    /// Đơn vị mong đợi của từng đại lượng — khai ra để bộ kiểm đối chiếu.
    pub output_units: BTreeMap<String, Unit>,
}

impl Rule {
    /// **Kiểm tĩnh toàn bộ luật** trước khi nạp.
    ///
    /// Trả về **mọi** lỗi, không dừng ở lỗi đầu: một luật sai ba chỗ mà chỉ được
    /// báo một chỗ sẽ phải nạp lại ba lần, và người viết luật sẽ đoán mò.
    pub fn validate(&self) -> Vec<RuleError> {
        let mut loi = Vec::new();
        for (ten, e) in &self.compute {
            match e.typecheck(&self.inputs) {
                Ok(u) => {
                    if let Some(mong_doi) = self.output_units.get(ten) {
                        if u != *mong_doi {
                            loi.push(RuleError::UnitMismatch {
                                left: u.as_str(),
                                right: mong_doi.as_str(),
                            });
                        }
                    }
                }
                Err(e) => loi.push(e),
            }
        }
        loi
    }

    /// Chạy luật. Trả về mọi đại lượng đã tính.
    pub fn run(
        &self,
        ctx: &BTreeMap<String, Quantity>,
    ) -> Result<BTreeMap<String, Quantity>, RuleError> {
        let mut ra = BTreeMap::new();
        for (ten, e) in &self.compute {
            ra.insert(ten.clone(), e.eval(ctx)?);
        }
        Ok(ra)
    }
}
