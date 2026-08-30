//! Thời điểm áp kết quả LLM (`idea.md §20.2.2`).
//!
//! Đây là module quan trọng nhất trong crate, và lý do nó tồn tại là một lỗi
//! mà **hai mô hình review độc lập đều tìm ra**:
//!
//! > `§20.2.1` chốt rằng *chọn ai được nghĩ* là xác định. Nhưng nó im lặng về
//! > *khi nào kết quả được áp*. Nếu kết quả được áp ngay khi về, thì một mô
//! > hình trả lời trong 500ms và một mô hình trả lời trong 3 giây sẽ tạo ra hai
//! > thế giới khác nhau **từ cùng một seed** — proposal vào ở tick 110 hay tick
//! > 125. Bảng `llm_call` không ghi tick commit, nên replay cũng không cứu được.
//!
//! Cách chữa: thực thể nghĩ ở tick `T` thì scheduler **ấn định luôn** độ trễ
//! `D` (suy từ `cognition_rate`, `§10.7.1`), và kết quả được áp đúng tại
//! `T + D`, bất kể mô hình trả lời nhanh hay chậm.
//!
//! ```text
//! Scheduled ─(gửi)→ Pending ─┬─(có kết quả trước T+D)──→ Accepted   @ tick T+D
//!                            ├─(chưa có kết quả tại T+D)→ Fallback   @ tick T+D
//!                            ├─(điều kiện tiền đề mất)───→ Cancelled
//!                            └─(quá hạn giữ chỗ)─────────→ Expired
//! ```
//!
//! Hệ quả đáng nói ra thành lời: **mô hình trả lời nhanh hơn `D` không làm nhân
//! vật phản ứng nhanh hơn.** Muốn nghĩ nhanh thì tăng `cognition_rate` — đó là
//! thuộc tính của thế giới. May mắn về đường truyền thì không.

use mow_core::{EntityId, Tick};
use mow_math::{CanonicalHash, StateHash, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Trạng thái của một lời gọi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    /// Đã lên lịch, chưa gửi.
    Scheduled,
    /// Đã gửi, đang chờ.
    Pending,
    /// Kết quả về kịp và đã được áp tại `T+D`.
    Accepted,
    /// Không kịp; hành vi dự phòng đã được áp tại `T+D`.
    Fallback,
    /// Điều kiện tiền đề mất trước `T+D` — nhân vật chết, mục tiêu biến mất.
    Cancelled,
    /// Quá hạn giữ chỗ mà chưa bao giờ được áp.
    Expired,
}

impl CallState {
    /// Trạng thái cuối, không đổi được nữa.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            CallState::Accepted | CallState::Fallback | CallState::Cancelled | CallState::Expired
        )
    }

    /// Tên ổn định trên đường truyền và trong cơ sở dữ liệu.
    pub fn as_str(self) -> &'static str {
        match self {
            CallState::Scheduled => "scheduled",
            CallState::Pending => "pending",
            CallState::Accepted => "accepted",
            CallState::Fallback => "fallback",
            CallState::Cancelled => "cancelled",
            CallState::Expired => "expired",
        }
    }
}

impl CanonicalHash for CallState {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_str(self.as_str());
    }
}

/// Một lời gọi đang được theo dõi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    /// Khóa idempotency. Cùng `request_id` chỉ được có tác dụng một lần.
    pub request_id: u64,
    /// Thực thể đang nghĩ.
    pub entity: EntityId,
    /// Tick gửi yêu cầu, tức `T`.
    pub request_tick: Tick,
    /// Tick áp kết quả, tức `T + D`. **Ấn định lúc lên lịch, không đổi.**
    pub admission_tick: Tick,
    /// Trạng thái.
    pub state: CallState,
    /// Định danh prompt và phiên bản, để audit.
    pub prompt_id: String,
    /// Phiên bản prompt.
    pub prompt_version: u32,
    /// Hash của yêu cầu, để dò trùng và để phát lại.
    pub request_hash: StateHash,
    /// Mô hình **thật sự đã dùng**, kể cả khi bị hạ cấp (`§20.10`).
    pub model: String,
    /// Kết quả, nếu đã về.
    pub response: Option<String>,
}

impl CanonicalHash for Call {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_u64(self.request_id);
        self.entity.canonical_hash(h);
        self.request_tick.canonical_hash(h);
        self.admission_tick.canonical_hash(h);
        self.state.canonical_hash(h);
        h.write_str(&self.prompt_id);
        h.write_u64(u64::from(self.prompt_version));
        h.write_hash(self.request_hash);
        h.write_str(&self.model);
        // **Không** đưa `response` vào hash khi chưa được áp: một kết quả đã về
        // nhưng chưa tới `T+D` không được ảnh hưởng state hash, vì nếu có thì
        // tốc độ mạng lại lọt vào thế giới qua cửa sau.
        h.write_option(
            match self.state {
                CallState::Accepted => self.response.as_deref(),
                _ => None,
            },
            |hh, r| {
                hh.write_str(r);
            },
        );
    }
}

/// Kết quả của một lần áp tại một tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    /// Lời gọi.
    pub call: Call,
    /// Có phải dùng kết quả thật không. `false` nghĩa là đã fallback.
    pub used_response: bool,
}

/// Lỗi của sổ theo dõi.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdmissionError {
    /// Trùng `request_id`.
    #[error("request {0} đã tồn tại")]
    Duplicate(u64),
    /// Không tìm thấy.
    #[error("không có request {0}")]
    NotFound(u64),
    /// Chuyển trạng thái không hợp lệ.
    #[error("không thể chuyển {from:?} → {to:?} cho request {id}")]
    BadTransition {
        /// Request.
        id: u64,
        /// Trạng thái hiện tại.
        from: CallState,
        /// Trạng thái muốn chuyển tới.
        to: CallState,
    },
    /// Độ trễ bằng 0.
    #[error(
        "cognitive_latency phải lớn hơn 0: bằng 0 nghĩa là kết quả được áp ngay khi về, \
         và thế giới sẽ phụ thuộc tốc độ đường truyền thay vì seed (§20.2.2)"
    )]
    ZeroLatency,
}

/// Sổ theo dõi lời gọi, có thẩm quyền về thời điểm áp.
///
/// `BTreeMap` chứ không phải `HashMap`: sổ này nằm trong state hash, và thứ tự
/// duyệt của `HashMap` sẽ làm hai lần chạy giống hệt nhau cho hai hash khác nhau.
#[derive(Debug, Clone, Default)]
pub struct AdmissionLedger {
    calls: BTreeMap<u64, Call>,
}

impl AdmissionLedger {
    /// Sổ rỗng.
    pub fn new() -> AdmissionLedger {
        AdmissionLedger::default()
    }

    /// Lên lịch một lời gọi tại tick `T` với độ trễ `D`.
    ///
    /// `admission_tick` được tính **ngay tại đây** và không bao giờ đổi sau đó.
    /// Đó là toàn bộ cơ chế: thời điểm áp là một hàm của `(T, D)`, hai đại
    /// lượng của thế giới, chứ không phải của thời điểm gói tin về tới.
    #[allow(clippy::too_many_arguments)]
    pub fn schedule(
        &mut self,
        request_id: u64,
        entity: EntityId,
        request_tick: Tick,
        latency: u64,
        prompt_id: &str,
        prompt_version: u32,
        request_hash: StateHash,
        model: &str,
    ) -> Result<&Call, AdmissionError> {
        if latency == 0 {
            return Err(AdmissionError::ZeroLatency);
        }
        if self.calls.contains_key(&request_id) {
            return Err(AdmissionError::Duplicate(request_id));
        }
        let admission_tick = Tick(request_tick.0.saturating_add(latency));
        self.calls.insert(
            request_id,
            Call {
                request_id,
                entity,
                request_tick,
                admission_tick,
                state: CallState::Scheduled,
                prompt_id: prompt_id.to_owned(),
                prompt_version,
                request_hash,
                model: model.to_owned(),
                response: None,
            },
        );
        Ok(&self.calls[&request_id])
    }

    /// Đánh dấu đã gửi.
    pub fn mark_sent(&mut self, request_id: u64) -> Result<(), AdmissionError> {
        let c = self
            .calls
            .get_mut(&request_id)
            .ok_or(AdmissionError::NotFound(request_id))?;
        if c.state != CallState::Scheduled {
            return Err(AdmissionError::BadTransition {
                id: request_id,
                from: c.state,
                to: CallState::Pending,
            });
        }
        c.state = CallState::Pending;
        Ok(())
    }

    /// Ghi nhận kết quả đã về.
    ///
    /// **Không áp ngay.** Kết quả nằm chờ tới `admission_tick`. Đây chính là
    /// chỗ mà tốc độ đường truyền bị chặn lại ở ngoài thế giới.
    ///
    /// Kết quả về sau khi lời gọi đã ở trạng thái cuối thì bị **bỏ qua**, và
    /// hàm trả `false` để chỗ gọi ghi metric — một tỉ lệ bỏ qua cao nghĩa là
    /// `D` đang quá ngắn so với mô hình đang dùng.
    pub fn record_response(
        &mut self,
        request_id: u64,
        model_thuc_te: &str,
        response: String,
    ) -> Result<bool, AdmissionError> {
        let c = self
            .calls
            .get_mut(&request_id)
            .ok_or(AdmissionError::NotFound(request_id))?;
        if c.state.is_terminal() {
            return Ok(false);
        }
        // Ghi model **thật sự đã dùng**, kể cả khi gateway đã hạ cấp (`§20.10`).
        // Không ghi cái đã yêu cầu, vì lúc đọc lại log ta cần biết cái gì đã
        // thật sự sinh ra câu trả lời này.
        c.model = model_thuc_te.to_owned();
        c.response = Some(response);
        Ok(true)
    }

    /// Hủy vì điều kiện tiền đề mất.
    pub fn cancel(&mut self, request_id: u64) -> Result<(), AdmissionError> {
        let c = self
            .calls
            .get_mut(&request_id)
            .ok_or(AdmissionError::NotFound(request_id))?;
        if c.state.is_terminal() {
            return Err(AdmissionError::BadTransition {
                id: request_id,
                from: c.state,
                to: CallState::Cancelled,
            });
        }
        c.state = CallState::Cancelled;
        Ok(())
    }

    /// Áp mọi lời gọi tới hạn tại `tick`.
    ///
    /// Trả về theo thứ tự `request_id` tăng dần — thứ tự xác định, không phụ
    /// thuộc thứ tự kết quả về.
    pub fn admit_due(&mut self, tick: Tick) -> Vec<Admitted> {
        let toi_han: Vec<u64> = self
            .calls
            .iter()
            .filter(|(_, c)| !c.state.is_terminal() && c.admission_tick <= tick)
            .map(|(id, _)| *id)
            .collect();

        let mut ra = Vec::with_capacity(toi_han.len());
        for id in toi_han {
            let c = self.calls.get_mut(&id).expect("vừa lọc ra");
            let co_ket_qua = c.response.is_some();
            c.state = if co_ket_qua {
                CallState::Accepted
            } else {
                CallState::Fallback
            };
            ra.push(Admitted {
                call: c.clone(),
                used_response: co_ket_qua,
            });
        }
        ra
    }

    /// Dọn những lời gọi đã quá hạn giữ chỗ quá lâu.
    ///
    /// Không có bước này, một lời gọi bị treo vĩnh viễn sẽ ở lại trong sổ mãi
    /// mãi, và sổ nằm trong state hash — nghĩa là rò rỉ bộ nhớ **và** một
    /// state hash lớn dần vô hạn.
    pub fn expire_older_than(&mut self, tick: Tick, grace: u64) -> usize {
        let mut n = 0;
        for c in self.calls.values_mut() {
            if !c.state.is_terminal() && c.admission_tick.0.saturating_add(grace) < tick.0 {
                c.state = CallState::Expired;
                n += 1;
            }
        }
        n
    }

    /// Xóa các lời gọi đã ở trạng thái cuối, sau khi đã ghi vào nhật ký.
    pub fn prune_terminal(&mut self) -> usize {
        let truoc = self.calls.len();
        self.calls.retain(|_, c| !c.state.is_terminal());
        truoc - self.calls.len()
    }

    /// Một lời gọi.
    pub fn get(&self, request_id: u64) -> Option<&Call> {
        self.calls.get(&request_id)
    }

    /// Số lời gọi đang theo dõi.
    pub fn len(&self) -> usize {
        self.calls.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// Số lời gọi chưa ở trạng thái cuối.
    pub fn in_flight(&self) -> usize {
        self.calls
            .values()
            .filter(|c| !c.state.is_terminal())
            .count()
    }
}

impl CanonicalHash for AdmissionLedger {
    fn canonical_hash(&self, h: &mut StateHasher) {
        h.write_seq(self.calls.values(), |hh, c| c.canonical_hash(hh));
    }
}
