//! Vòng đời portal và **transfer nguyên tử chín bước** (`idea.md §6.1`, `§6.2`,
//! `§22.8`, `PE-09`).
//!
//! ## Bất biến `INV-22-8`
//!
//! > Portal transfer **không bao giờ nhân đôi hoặc làm mất entity** do commit
//! > nửa chừng.
//!
//! Hai hỏng ngược chiều nhau, và cách chữa nông cho cái này lại tạo ra cái kia:
//!
//! - Spawn ở đích **trước** khi xóa ở nguồn → crash ở giữa ⇒ **nhân đôi**.
//! - Xóa ở nguồn **trước** khi spawn ở đích → crash ở giữa ⇒ **bốc hơi**.
//!
//! `§6.2` nói thẳng rằng cách chữa hiển nhiên không đủ:
//!
//! > Một câu *"commit cả hai phía rồi rollback nếu lỗi"* là chưa đủ: không có
//! > giao thức nào bảo đảm hai bên cùng thành công khi chúng không chia sẻ một
//! > transaction.
//!
//! Và hai world **không** chia sẻ transaction: chúng có thể nằm ở hai partition
//! và hai vòng tick khác nhau.
//!
//! ## Escrow hai pha
//!
//! Nên trạng thái trung gian được **vật chất hóa**: entity rời world nguồn đi
//! vào một [`EscrowRecord`] — một bản ghi tồn tại thật, nằm trong save, dò lại
//! được. Nó không ở world nào cả, và đó là điểm mấu chốt: *"không ở đâu"* là
//! một trạng thái hợp lệ, còn *"ở cả hai nơi"* thì không.
//!
//! Crash ở bất kỳ điểm nào để lại một bản ghi trung chuyển mà [`recover`] đọc
//! được và **hoàn tất hoặc hoàn tác**. Không có nhánh nào để lại hai bản sao,
//! và không có nhánh nào để lại con số không.
//!
//! ```text
//! nguồn        escrow          đích
//!   ●    ──►     ●              ·        Reserved: đã giữ chỗ, chưa rời
//!   ·    ──►     ●              ·        Departed: đã rời nguồn  ← crash ở đây thì recover đi tiếp
//!   ·            ●     ──►      ●        Arrived: đích đã spawn  ← crash ở đây thì recover xác nhận
//!   ·            ·              ●        Released: xong
//! ```
//!
//! Đúng **một** ô đầy ở mọi hàng. Đó là toàn bộ chứng minh.

use crate::clock::{rebase_processes, Process, RebaseAudit, RebaseError};
use crate::contact::{Cargo, ContactRegime, Decision};
use mow_core::clock::Clock;
use mow_core::{EntityId, WorldId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Vòng đời của một cổng (`§6.2`).
///
/// ```text
/// DORMANT → CHARGING → OPEN → UNSTABLE → COLLAPSING → CLOSED
/// ```
///
/// Một chiều, trừ hai lối lùi có thật: nạp dở thì về ngủ, và bất ổn thì có thể
/// ổn định lại nếu được can thiệp. `CLOSED` là hấp thụ — một cổng đã sập thì
/// phải mở cổng **mới**, vì nếu sập rồi mở lại được thì không có gì mất mát và
/// việc phá cổng thôi có ý nghĩa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortalState {
    /// Ngủ — tồn tại nhưng không dẫn đi đâu.
    Dormant,
    /// Đang nạp năng lượng.
    Charging,
    /// Mở, đi qua được.
    Open,
    /// Còn đi được nhưng sai số tọa độ tăng và có thể sập.
    Unstable,
    /// Đang sập — không nhận thêm ai.
    Collapsing,
    /// Đã đóng vĩnh viễn.
    Closed,
}

impl PortalState {
    /// Có đi qua được không.
    ///
    /// `Unstable` **vẫn đi được**, và đó là một lựa chọn có rủi ro chứ không
    /// phải một lỗi — nếu cấm thì cả một loại quyết định thú vị biến mất.
    pub fn passable(self) -> bool {
        matches!(self, PortalState::Open | PortalState::Unstable)
    }

    /// Chuyển sang trạng thái này có hợp lệ không.
    pub fn may_become(self, next: PortalState) -> bool {
        use PortalState::*;
        matches!(
            (self, next),
            (Dormant, Charging)
                | (Charging, Open)
                | (Charging, Dormant)
                | (Open, Unstable)
                | (Open, Collapsing)
                | (Unstable, Open)
                | (Unstable, Collapsing)
                | (Collapsing, Closed)
        )
    }
}

/// Chính sách truy cập (`§6.1`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AccessPolicy {
    /// Chỉ những entity này được qua; rỗng nghĩa là không giới hạn theo entity.
    pub allow_entities: Vec<EntityId>,
    /// Những entity bị cấm hẳn — thắng mọi allow.
    pub deny_entities: Vec<EntityId>,
    /// Loài được phép; rỗng nghĩa là không giới hạn theo loài.
    pub allow_species: Vec<String>,
    /// Cần chữ ký của True God không.
    pub requires_divine_signature: bool,
}

impl AccessPolicy {
    /// Entity này qua được không.
    ///
    /// `deny` xét trước `allow`: một danh sách cấm bị một danh sách cho phép
    /// ghi đè là một danh sách cấm vô nghĩa.
    pub fn permits(&self, who: EntityId, species: &str, has_signature: bool) -> bool {
        if self.deny_entities.contains(&who) {
            return false;
        }
        if self.requires_divine_signature && !has_signature {
            return false;
        }
        if !self.allow_entities.is_empty() && !self.allow_entities.contains(&who) {
            return false;
        }
        if !self.allow_species.is_empty() && !self.allow_species.iter().any(|s| s == species) {
            return false;
        }
        true
    }
}

/// Một cổng (`§6.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portal {
    /// Định danh.
    pub id: u64,
    /// World nguồn.
    pub source: WorldId,
    /// World đích.
    pub dest: WorldId,
    /// Trạng thái.
    pub state: PortalState,
    /// Chính sách truy cập.
    pub access: AccessPolicy,
    /// Băng thông: tổng khối lượng qua được trong một cửa sổ.
    pub bandwidth_mass: u64,
    /// Đã dùng bao nhiêu trong cửa sổ hiện tại.
    pub used_mass: u64,
    /// Chế độ tiếp xúc hai bên đã thỏa thuận.
    pub regime: ContactRegime,
}

impl Portal {
    /// Đổi trạng thái, hoặc từ chối nếu bước chuyển không hợp lệ.
    pub fn transition(&mut self, next: PortalState) -> Result<(), TransferError> {
        if !self.state.may_become(next) {
            return Err(TransferError::BadTransition {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        Ok(())
    }
}

/// Thứ đi qua cổng: một entity và những gì đi cùng nó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Traveller {
    /// Ai.
    pub who: EntityId,
    /// Loài.
    pub species: String,
    /// Khối lượng cả người lẫn hành lý.
    pub mass: u64,
    /// Vật phẩm mang theo.
    pub inventory: Vec<u64>,
    /// Mọi tiến trình có thời hạn đang chạy — **bước 5 rebase từng cái**.
    pub processes: Vec<Process>,
    /// Thứ đi cùng mà chủ nhân không khai: ký sinh, mầm bệnh, hạt giống
    /// (`§9.10.1`).
    ///
    /// Trường này tồn tại vì `§6.2` bước 8 bắt buộc ghi lại *"những gì đã đi
    /// cùng"*. Không có nó thì một dịch bệnh xuyên world sẽ xuất hiện ở đích
    /// mà không ai truy được nó đến bằng cách nào.
    pub hitchhikers: Vec<String>,
    /// Có chữ ký của True God không.
    pub divine_signature: bool,
}

/// Hồ sơ nhu cầu sống ở world đích (`§9.7.5`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsProfile {
    /// Khoảng khí quyển chịu được.
    pub atmosphere: (i32, i32),
    /// Khoảng nhiệt độ chịu được, phần trăm độ C.
    pub temperature: (i32, i32),
    /// Khoảng mật độ mana chịu được.
    pub mana: (i32, i32),
}

/// Điều kiện thật ở world đích.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldConditions {
    /// Khí quyển.
    pub atmosphere: i32,
    /// Nhiệt độ.
    pub temperature: i32,
    /// Mật độ mana.
    pub mana: i32,
}

impl NeedsProfile {
    /// Sống nổi ở đó không, và **thiếu cái gì**.
    ///
    /// Trả về danh sách chứ không phải `bool`: *"không sống nổi"* là một câu
    /// không hành động được, còn *"thiếu khí, chịu được nhiệt"* thì người chơi
    /// biết phải chuẩn bị gì.
    pub fn unsurvivable(&self, c: &WorldConditions) -> Vec<&'static str> {
        let mut v = Vec::new();
        if c.atmosphere < self.atmosphere.0 || c.atmosphere > self.atmosphere.1 {
            v.push("khí quyển ngoài khoảng chịu được");
        }
        if c.temperature < self.temperature.0 || c.temperature > self.temperature.1 {
            v.push("nhiệt độ ngoài khoảng chịu được");
        }
        if c.mana < self.mana.0 || c.mana > self.mana.1 {
            v.push("mật độ mana ngoài khoảng chịu được");
        }
        v
    }
}

/// Pha của một bản ghi trung chuyển.
///
/// **Đúng một pha tại một thời điểm**, và mỗi pha nói rõ entity đang ở đâu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscrowPhase {
    /// Đã giữ chỗ ở nguồn, entity **vẫn ở world nguồn**.
    Reserved,
    /// Đã rời world nguồn, **chưa ở world nào**.
    Departed,
    /// World đích đã spawn, bản ghi **chưa được giải phóng**.
    Arrived,
    /// Xong. Bản ghi giữ lại để dò lịch sử, entity chỉ ở world đích.
    Released,
    /// Đã hoàn tác, entity trở về world nguồn.
    RolledBack,
}

/// Bản ghi trung chuyển — thứ làm cho escrow hai pha thành thật.
///
/// Nó **nằm trong save**. Một bản ghi chỉ tồn tại trong RAM thì crash làm mất
/// nó, và mất nó thì mất luôn entity đang nằm trong đó.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowRecord {
    /// Định danh, để dò lại sau crash.
    pub id: u64,
    /// Cổng nào.
    pub portal: u64,
    /// Ai.
    pub entity: EntityId,
    /// Từ đâu.
    pub source: WorldId,
    /// Đến đâu.
    pub dest: WorldId,
    /// Pha hiện tại.
    pub phase: EscrowPhase,
    /// Vật phẩm đi cùng.
    pub inventory: Vec<u64>,
    /// Tiến trình đã rebase.
    pub processes: Vec<Process>,
    /// Biên bản rebase, đi vào event đến.
    pub rebase: RebaseAudit,
    /// Thứ đi cùng mà không ai khai (`§9.10.1`).
    pub hitchhikers: Vec<String>,
}

/// Vì sao transfer thất bại.
///
/// Mỗi biến thể nói rõ **bước nào** trong chín bước — vì "transfer failed" là
/// một dòng log không sửa được gì.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransferError {
    /// Cổng không ở trạng thái đi qua được.
    #[error("bước 1: cổng đang {state:?}, không đi qua được")]
    NotPassable {
        /// Trạng thái nào.
        state: PortalState,
    },
    /// Bước chuyển trạng thái không hợp lệ.
    #[error("bước chuyển {from:?} → {to:?} không có trong vòng đời §6.2")]
    BadTransition {
        /// Từ.
        from: PortalState,
        /// Sang.
        to: PortalState,
    },
    /// Chính sách truy cập từ chối.
    #[error("bước 2: chính sách truy cập từ chối entity {who:?}")]
    AccessDenied {
        /// Ai.
        who: EntityId,
    },
    /// Vượt băng thông.
    #[error("bước 2: khối lượng {mass} vượt băng thông còn lại {remaining}")]
    OverBandwidth {
        /// Muốn đưa qua bao nhiêu.
        mass: u64,
        /// Còn bao nhiêu.
        remaining: u64,
    },
    /// Chế độ tiếp xúc từ chối.
    #[error("bước 3: kiểm soát cổng từ chối — {reason}")]
    Refused {
        /// Lý do.
        reason: &'static str,
    },
    /// Bị giữ ở vùng cách ly.
    ///
    /// **Không phải một dạng từ chối.** Entity còn ở đó, còn sống, và sẽ được
    /// xét lại — nên chỗ gọi phải xử lý khác hẳn với `Refused`.
    #[error("bước 3: giữ ở vùng cách ly {ticks} tick — {reason}")]
    Held {
        /// Bao lâu.
        ticks: u64,
        /// Vì sao.
        reason: &'static str,
    },
    /// Rebase lỗi.
    #[error("bước 5: {0}")]
    Rebase(#[from] RebaseError),
    /// Bản ghi trung chuyển ở sai pha cho thao tác đang làm.
    #[error("bản ghi trung chuyển {id} đang ở pha {phase:?}, không làm được việc này")]
    WrongPhase {
        /// Bản ghi nào.
        id: u64,
        /// Pha nào.
        phase: EscrowPhase,
    },
}

/// Kết quả bước 4 — sống nổi hay không.
///
/// `§6.2` bước 4: *"Không sống nổi mà vẫn đi là một quyết định hợp lệ, nhưng hệ
/// quả phải được áp ngay khi tới"*. Nên đây **không** là lỗi: nó là một cảnh
/// báo đi kèm vào bản ghi và biến thành hiệu ứng ở world đích.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SurvivalWarning {
    /// Thiếu những gì.
    pub problems: Vec<String>,
}

/// Sổ trung chuyển của cả multiverse.
///
/// Một sổ chung chứ không phải mỗi world một sổ: một bản ghi mà cả hai world
/// đều không sở hữu thì sau crash **không bên nào chịu trách nhiệm dò lại**, và
/// entity nằm trong đó biến mất một cách hoàn toàn im lặng.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscrowLedger {
    records: BTreeMap<u64, EscrowRecord>,
    next_id: u64,
}

impl EscrowLedger {
    /// Sổ rỗng.
    pub fn new() -> EscrowLedger {
        EscrowLedger::default()
    }

    /// Một bản ghi.
    pub fn get(&self, id: u64) -> Option<&EscrowRecord> {
        self.records.get(&id)
    }

    /// Số bản ghi.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Sổ rỗng chưa.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Những bản ghi còn dang dở — thứ mà [`recover`] phải xử sau crash.
    pub fn pending(&self) -> Vec<&EscrowRecord> {
        self.records
            .values()
            .filter(|r| {
                matches!(
                    r.phase,
                    EscrowPhase::Reserved | EscrowPhase::Departed | EscrowPhase::Arrived
                )
            })
            .collect()
    }

    /// **Bước 1–5**: giữ chỗ, kiểm mọi thứ, rebase, và lập bản ghi trung chuyển.
    ///
    /// Sau khi hàm này trả `Ok`, entity **vẫn còn ở world nguồn**
    /// ([`EscrowPhase::Reserved`]). Chưa có gì bị xóa và chưa có gì được tạo:
    /// crash ở đây thì chỉ mất một chỗ đã giữ.
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        &mut self,
        portal: &mut Portal,
        t: &Traveller,
        needs: &NeedsProfile,
        dest_conditions: &WorldConditions,
        source_clock: &Clock,
        dest_clock: &Clock,
    ) -> Result<(u64, SurvivalWarning), TransferError> {
        // Bước 1: cổng có đi qua được không.
        if !portal.state.passable() {
            return Err(TransferError::NotPassable {
                state: portal.state,
            });
        }

        // Bước 2: quyền, khối lượng, năng lượng.
        if !portal.access.permits(t.who, &t.species, t.divine_signature) {
            return Err(TransferError::AccessDenied { who: t.who });
        }
        let con_lai = portal.bandwidth_mass.saturating_sub(portal.used_mass);
        if t.mass > con_lai {
            return Err(TransferError::OverBandwidth {
                mass: t.mass,
                remaining: con_lai,
            });
        }

        // Bước 3: chế độ tiếp xúc (§6.4).
        let cargo = Cargo {
            suspected_pathogen: t.hitchhikers.iter().any(|h| h.starts_with("pathogen.")),
            suspected_taint: t.hitchhikers.iter().any(|h| h.starts_with("taint.")),
            goods: t.inventory.iter().map(|i| format!("item.{i}")).collect(),
            living: true,
            seeds: t.hitchhikers.iter().any(|h| h.starts_with("seed.")),
            souls: t.hitchhikers.iter().any(|h| h.starts_with("soul.")),
        };
        match portal.regime.screen(&cargo) {
            Decision::Refuse { reason } => return Err(TransferError::Refused { reason }),
            Decision::Hold { ticks, reason } => return Err(TransferError::Held { ticks, reason }),
            Decision::Allow => {}
        }

        // Bước 4: sống nổi ở đích không — cảnh báo, **không phải** từ chối.
        let canh_bao = SurvivalWarning {
            problems: needs
                .unsurvivable(dest_conditions)
                .into_iter()
                .map(str::to_owned)
                .collect(),
        };

        // Bước 5: rebase mọi tiến trình theo domain của chính nó (§4.5).
        let (processes, rebase) = rebase_processes(&t.processes, source_clock, dest_clock)?;
        debug_assert!(
            rebase.covers_all(&t.processes),
            "rebase sót tiến trình — đây là bug tệ nhất mà §4.5 cảnh báo"
        );

        self.next_id += 1;
        let id = self.next_id;
        self.records.insert(
            id,
            EscrowRecord {
                id,
                portal: portal.id,
                entity: t.who,
                source: portal.source,
                dest: portal.dest,
                phase: EscrowPhase::Reserved,
                inventory: t.inventory.clone(),
                processes,
                rebase,
                hitchhikers: t.hitchhikers.clone(),
            },
        );
        portal.used_mass = portal.used_mass.saturating_add(t.mass);

        Ok((id, canh_bao))
    }

    /// **Bước 6**: ghi event rời world nguồn, xóa entity khỏi nguồn.
    ///
    /// Sau bước này entity **không ở world nào**. Đó là trạng thái duy nhất
    /// không thể nhân đôi, và là lý do escrow tồn tại.
    pub fn depart(&mut self, id: u64) -> Result<&EscrowRecord, TransferError> {
        self.advance(id, EscrowPhase::Reserved, EscrowPhase::Departed)
    }

    /// **Bước 7–8**: world đích spawn và ghi event đến, kèm bản ghi những gì đã
    /// đi cùng.
    pub fn arrive(&mut self, id: u64) -> Result<&EscrowRecord, TransferError> {
        self.advance(id, EscrowPhase::Departed, EscrowPhase::Arrived)
    }

    /// **Bước 9**: world đích đã commit và **xác nhận**; giải phóng bản ghi.
    ///
    /// Chỉ khi có xác nhận. Giải phóng trước xác nhận thì mất luôn thứ dùng để
    /// dò lại nếu world đích rollback.
    pub fn release(&mut self, id: u64) -> Result<&EscrowRecord, TransferError> {
        self.advance(id, EscrowPhase::Arrived, EscrowPhase::Released)
    }

    /// Hoàn tác: trả entity về world nguồn.
    ///
    /// Chỉ làm được khi world đích **chưa** spawn. Sau `Arrived` thì hoàn tác
    /// sẽ là xóa một entity đã tồn tại ở đích, và nếu xóa hụt thì thành nhân
    /// đôi — nên ở đó [`recover`] chọn đi tiếp chứ không lùi.
    pub fn rollback(&mut self, id: u64) -> Result<&EscrowRecord, TransferError> {
        let r = self.records.get_mut(&id).ok_or(TransferError::WrongPhase {
            id,
            phase: EscrowPhase::Released,
        })?;
        if !matches!(r.phase, EscrowPhase::Reserved | EscrowPhase::Departed) {
            return Err(TransferError::WrongPhase { id, phase: r.phase });
        }
        r.phase = EscrowPhase::RolledBack;
        Ok(r)
    }

    fn advance(
        &mut self,
        id: u64,
        tu: EscrowPhase,
        sang: EscrowPhase,
    ) -> Result<&EscrowRecord, TransferError> {
        let r = self.records.get_mut(&id).ok_or(TransferError::WrongPhase {
            id,
            phase: EscrowPhase::Released,
        })?;
        if r.phase != tu {
            return Err(TransferError::WrongPhase { id, phase: r.phase });
        }
        r.phase = sang;
        Ok(r)
    }
}

/// Việc phải làm với một bản ghi dang dở sau crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recovery {
    /// Chưa rời nguồn ⇒ hoàn tác. Entity vẫn ở nguồn, chỉ cần bỏ chỗ đã giữ.
    Rollback,
    /// Đã rời nguồn, chưa tới đích ⇒ **đi tiếp**.
    ///
    /// Không lùi được: event rời nguồn đã ghi, và một event đã ghi thì không
    /// gỡ. Lùi ở đây nghĩa là viết lại lịch sử, mà `§22.9` cấm.
    Complete,
    /// Đã tới đích, chưa giải phóng ⇒ xác nhận rồi giải phóng.
    Confirm,
}

/// Sau crash, mỗi bản ghi dang dở phải làm gì.
///
/// Hàm này là **lý do escrow đáng công**: nó biến "không biết chuyện gì đã xảy
/// ra" thành một bảng tra ba dòng. Không dòng nào cho ra hai bản sao, và không
/// dòng nào cho ra con số không.
pub fn recover(ledger: &EscrowLedger) -> Vec<(u64, Recovery)> {
    ledger
        .pending()
        .into_iter()
        .map(|r| {
            let v = match r.phase {
                EscrowPhase::Reserved => Recovery::Rollback,
                EscrowPhase::Departed => Recovery::Complete,
                EscrowPhase::Arrived => Recovery::Confirm,
                // `pending()` đã lọc, hai pha còn lại không tới được đây.
                EscrowPhase::Released | EscrowPhase::RolledBack => Recovery::Confirm,
            };
            (r.id, v)
        })
        .collect()
}

/// Đếm số bản sao của một entity trên toàn multiverse.
///
/// Dùng cho invariant `INV-22-8`: hàm này phải trả về **đúng 1** ở mọi thời
/// điểm, kể cả giữa một transfer dở dang.
///
/// - `in_worlds`: entity đang có mặt ở những world nào theo state thật.
/// - Bản ghi ở pha `Departed` tính là **một** bản: entity nằm trong escrow.
/// - Bản ghi ở pha `Reserved` tính là **không**: entity vẫn còn ở world nguồn
///   và đã được `in_worlds` đếm rồi.
pub fn count_copies(entity: EntityId, in_worlds: &[WorldId], ledger: &EscrowLedger) -> usize {
    let trong_escrow = ledger
        .records
        .values()
        .filter(|r| r.entity == entity && r.phase == EscrowPhase::Departed)
        .count();
    in_worlds.len() + trong_escrow
}
