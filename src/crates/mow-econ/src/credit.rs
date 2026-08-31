//! Tín dụng, vỡ nợ và khủng hoảng dây chuyền (`idea.md §12.8.8`, `PD-12`).
//!
//! > **Chỉ một primitive này** sinh ra: tín dụng thương mại, cho vay nặng lãi,
//! > tháo chạy khỏi nhà băng, tịch biên, lao dịch trừ nợ, bán mình làm nô, và
//! > **khủng hoảng dây chuyền** khi một con nợ lớn sụp kéo theo chủ nợ của nó.
//!
//! ## Vì sao dây chuyền phải là **hệ quả**, không phải một sự kiện
//!
//! Cách rẻ là viết một event `financial_crisis` và kích hoạt nó khi thấy hợp lý.
//! Nó cho ra một cuộc khủng hoảng đúng lúc, và người chơi không bao giờ học được
//! điều đáng học: rằng khủng hoảng lan qua **đúng những sợi dây mà họ nhìn thấy**,
//! và rằng cho một người vay quá nhiều là tự đặt mình vào chuỗi đó.
//!
//! Ở đây [`Ledger::cascade_default`] chỉ đi theo các khoản vay có thật. Nếu
//! không ai vay chồng chéo thì không có dây chuyền nào cả — và đó là câu trả
//! lời đúng, không phải một kịch bản bị bỏ lỡ.
//!
//! ## Thứ tự ưu tiên khi thanh lý là nội dung, không phải chi tiết
//!
//! Ai được trả trước khi tài sản không đủ quyết định ai sống sót. Đó là chỗ
//! "lao dịch trừ nợ" và "bán mình làm nô" xuất hiện: khi thế chấp đã hết mà nợ
//! vẫn còn, thứ duy nhất con nợ còn lại là chính họ.

use mow_core::{EntityId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Hạng ưu tiên khi thanh lý. **Nhỏ hơn được trả trước.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seniority {
    /// Có thế chấp cụ thể.
    Secured,
    /// Không thế chấp nhưng ưu tiên.
    Senior,
    /// Thường.
    Junior,
}

/// Một khoản vay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loan {
    /// Định danh.
    pub id: u64,
    /// Người vay.
    pub debtor: EntityId,
    /// Người cho vay.
    pub creditor: EntityId,
    /// Gốc.
    pub principal: i64,
    /// Lãi mỗi kỳ, phần nghìn.
    pub interest_per_period: u16,
    /// Đến hạn lúc nào.
    pub due: Tick,
    /// Giá trị tài sản thế chấp.
    pub collateral: i64,
    /// Người bảo lãnh, nếu có.
    ///
    /// Đây là sợi dây làm khủng hoảng lan **ra ngoài** những người trực tiếp vay
    /// mượn: một người chưa từng vay gì vẫn sụp vì đã đứng ra bảo lãnh.
    pub guarantor: Option<EntityId>,
    /// Hạng ưu tiên.
    pub seniority: Seniority,
    /// Đã trả bao nhiêu.
    pub repaid: i64,
}

impl Loan {
    /// Còn nợ bao nhiêu ở tick `now`, gồm cả lãi tích lũy.
    pub fn outstanding(&self, now: Tick, period_ticks: u64) -> i64 {
        if period_ticks == 0 {
            return (self.principal - self.repaid).max(0);
        }
        let ky = now
            .0
            .saturating_sub(self.due.0.saturating_sub(period_ticks))
            / period_ticks;
        let mut n = self.principal;
        // Lãi kép theo kỳ. Dùng vòng lặp số nguyên thay vì lũy thừa số thực:
        // `§P10.2.1` cấm số thực trên đường commit, và một khoản nợ **là** state.
        for _ in 0..ky.min(1_000) {
            n += n * i64::from(self.interest_per_period) / 1_000;
        }
        (n - self.repaid).max(0)
    }

    /// Đã quá hạn chưa.
    pub fn overdue(&self, now: Tick) -> bool {
        now.0 > self.due.0
    }
}

/// Một lần vỡ nợ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Default_ {
    /// Ai vỡ.
    pub debtor: EntityId,
    /// Nợ bao nhiêu.
    pub owed: i64,
    /// Thu hồi được bao nhiêu từ thế chấp.
    pub recovered: i64,
    /// Chủ nợ chịu thiệt bao nhiêu.
    pub creditor_loss: i64,
    /// Ai phải gánh: người bảo lãnh, nếu có.
    pub fell_to_guarantor: Option<EntityId>,
    /// Đợt thứ mấy trong dây chuyền. `0` là người sụp đầu tiên.
    pub wave: u32,
}

/// Sổ nợ.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ledger {
    loans: Vec<Loan>,
}

impl Ledger {
    /// Rỗng.
    pub fn new() -> Ledger {
        Ledger::default()
    }

    /// Thêm một khoản vay.
    pub fn lend(&mut self, l: Loan) -> &mut Ledger {
        self.loans.push(l);
        self
    }

    /// Các khoản vay.
    pub fn loans(&self) -> &[Loan] {
        &self.loans
    }

    /// Tổng nợ của một người.
    pub fn debt_of(&self, who: EntityId, now: Tick, period: u64) -> i64 {
        self.loans
            .iter()
            .filter(|l| l.debtor == who)
            .map(|l| l.outstanding(now, period))
            .sum()
    }

    /// Tổng cho vay của một người.
    pub fn credit_of(&self, who: EntityId, now: Tick, period: u64) -> i64 {
        self.loans
            .iter()
            .filter(|l| l.creditor == who)
            .map(|l| l.outstanding(now, period))
            .sum()
    }

    /// **Khủng hoảng dây chuyền.**
    ///
    /// `assets` là tài sản khả dụng của từng người, ngoài thế chấp. Một người
    /// sụp khi nợ vượt tài sản; chủ nợ của họ mất phần không thu hồi được, và
    /// nếu mất đủ nhiều thì chính chủ nợ cũng sụp — sang đợt sau.
    ///
    /// Trả về theo **thứ tự đợt**, nên UI vẽ được đúng thứ tự domino thay vì một
    /// đống người cùng phá sản một lúc.
    pub fn cascade_default(
        &self,
        assets: &BTreeMap<EntityId, i64>,
        now: Tick,
        period: u64,
    ) -> Vec<Default_> {
        // **Khả năng trả gồm cả khoản phải thu.**
        //
        // Không cộng vào thì một chủ nợ trông như đã vỡ nợ ngay từ đợt 0, trước
        // khi con nợ của họ kịp sụp — và "dây chuyền" biến thành "mọi người cùng
        // phá sản một lúc", tức là mất đúng thứ mà `§12.8.8` muốn mô hình hóa.
        //
        // Đây cũng là cách một nhà băng *thật sự* mất khả năng thanh toán: không
        // phải vì nó nghèo đi, mà vì thứ tài sản lớn nhất của nó — lời hứa trả nợ
        // của người khác — bốc hơi trong một buổi sáng.
        let mut con_lai: BTreeMap<EntityId, i64> = assets.clone();
        for l in &self.loans {
            *con_lai.entry(l.creditor).or_insert(0) += l.outstanding(now, period);
        }
        let mut da_sup: BTreeSet<EntityId> = BTreeSet::new();
        let mut ra: Vec<Default_> = Vec::new();

        for wave in 0..64u32 {
            // Ai đang mất khả năng trả.
            let mut sup_dot_nay: Vec<EntityId> = Vec::new();
            let moi_nguoi: BTreeSet<EntityId> = self.loans.iter().map(|l| l.debtor).collect();
            for who in moi_nguoi {
                if da_sup.contains(&who) {
                    continue;
                }
                let no = self.debt_of(who, now, period);
                let co = con_lai.get(&who).copied().unwrap_or(0);
                if no > co {
                    sup_dot_nay.push(who);
                }
            }
            if sup_dot_nay.is_empty() {
                break;
            }

            for who in sup_dot_nay {
                da_sup.insert(who);
                let no = self.debt_of(who, now, period);
                let co = con_lai.get(&who).copied().unwrap_or(0).max(0);

                // Thanh lý theo **thứ tự ưu tiên**: ai được trả trước quyết định
                // ai sống sót.
                let mut cua_who: Vec<&Loan> =
                    self.loans.iter().filter(|l| l.debtor == who).collect();
                cua_who.sort_by(|a, b| a.seniority.cmp(&b.seniority).then(a.id.cmp(&b.id)));

                let mut quy = co;
                let mut bao_lanh: Option<EntityId> = None;
                let mut tong_thu = 0;

                for l in cua_who {
                    let phai_tra = l.outstanding(now, period);
                    // Thế chấp trước, rồi tới quỹ chung.
                    let tu_the_chap = l.collateral.min(phai_tra);
                    let con_thieu = phai_tra - tu_the_chap;
                    let tu_quy = quy.min(con_thieu).max(0);
                    quy -= tu_quy;
                    let thu = tu_the_chap + tu_quy;
                    tong_thu += thu;

                    let mat = phai_tra - thu;
                    if mat > 0 {
                        // Người bảo lãnh gánh phần còn thiếu — và đó là cách một
                        // người chưa từng vay gì cũng bị kéo vào.
                        if let Some(g) = l.guarantor {
                            bao_lanh = Some(g);
                            *con_lai.entry(g).or_insert(0) -= mat;
                        } else {
                            *con_lai.entry(l.creditor).or_insert(0) -= mat;
                        }
                    }
                }

                ra.push(Default_ {
                    debtor: who,
                    owed: no,
                    recovered: tong_thu,
                    creditor_loss: (no - tong_thu).max(0),
                    fell_to_guarantor: bao_lanh,
                    wave,
                });
            }
        }

        ra
    }
}
