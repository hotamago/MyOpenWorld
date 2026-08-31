//! [`Sim`] — một thế giới đang chạy.
//!
//! Đây là chỗ ba mảnh gặp nhau: kho thực thể, đồng hồ và nhật ký. Và đây là
//! chỗ bất biến `§22.1` được giữ: `Sim` **không cho mượn** `&mut Store` ra
//! ngoài. Muốn đổi thế giới thì chỉ có một cửa, là [`Sim::apply`].

use crate::clock::Clock;
use crate::command::{Command, CommandResult, Failure, FailureCode};
use crate::ecs::Store;
use crate::event::{EventLog, EventSeq};
use crate::ids::{BranchId, IdAllocator, WorldId};
use crate::invariant::{InvariantReport, InvariantRunner};
use crate::transaction::{Committed, Ctx, HandlerRegistry, Mutation};
use mow_math::{CanonicalHash, RngStreams, StateHash, StateHasher, WorldSeed};
use std::collections::BTreeSet;

/// Một thế giới đang chạy trên một nhánh.
pub struct Sim {
    world: WorldId,
    branch: BranchId,
    seed: WorldSeed,

    store: Store,
    clock: Clock,
    log: EventLog,
    ids: IdAllocator,

    handlers: HandlerRegistry,
    /// `request_id` đã xử lý, để idempotency (`§20.2.2`).
    ///
    /// `BTreeSet` chứ không phải `HashSet` vì nó nằm trong state hash: hai lần
    /// chạy phải cho cùng một hash, và thứ tự duyệt của `HashSet` thì không.
    seen_requests: BTreeSet<u64>,
}

impl core::fmt::Debug for Sim {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Sim")
            .field("world", &self.world)
            .field("branch", &self.branch)
            .field("tick", &self.clock.local())
            .field("entities", &self.store.len())
            .field("events", &self.log.len())
            .field("hash", &self.state_hash().short())
            .finish_non_exhaustive()
    }
}

/// Cấu hình dựng một [`Sim`].
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Thế giới.
    pub world: WorldId,
    /// Nhánh.
    pub branch: BranchId,
    /// Seed gốc — quyết định mọi dòng ngẫu nhiên (`§19.6`).
    pub seed: WorldSeed,
    /// Đồng hồ khởi tạo.
    pub clock: Clock,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            world: WorldId(1),
            branch: BranchId(1),
            seed: WorldSeed(0),
            clock: Clock::synchronous(),
        }
    }
}

impl Sim {
    /// Dựng một thế giới rỗng tại tick 0.
    pub fn new(cfg: SimConfig, handlers: HandlerRegistry) -> Sim {
        Sim {
            world: cfg.world,
            branch: cfg.branch,
            seed: cfg.seed,
            store: Store::new(),
            clock: cfg.clock,
            log: EventLog::new(),
            ids: IdAllocator::new(),
            handlers,
            seen_requests: BTreeSet::new(),
        }
    }

    // ── Đọc ─────────────────────────────────────────────────────────────────

    /// Kho thực thể, **chỉ đọc**.
    ///
    /// Không có phiên bản `&mut` của hàm này, và sẽ không bao giờ có. Nếu bạn
    /// thấy mình cần nó, thứ bạn thật sự cần là một handler mới.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Đồng hồ.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Nhật ký sự kiện.
    pub fn log(&self) -> &EventLog {
        &self.log
    }

    /// Thế giới.
    pub fn world_id(&self) -> WorldId {
        self.world
    }

    /// Nhánh.
    pub fn branch_id(&self) -> BranchId {
        self.branch
    }

    /// Dòng ngẫu nhiên của thế giới này.
    pub fn rng(&self) -> RngStreams {
        RngStreams::new(self.seed)
    }

    /// Sổ handler.
    pub fn handlers(&self) -> &HandlerRegistry {
        &self.handlers
    }

    /// Hash toàn bộ state.
    ///
    /// Gồm **tất cả** những gì ảnh hưởng tương lai: thực thể, đồng hồ, nhật ký,
    /// bộ cấp phát id, và tập `request_id` đã thấy. Thiếu một trong số đó thì
    /// hai thế giới sẽ có cùng hash nhưng diễn tiến khác nhau — dạng lỗi tệ
    /// nhất, vì harness sẽ báo xanh trong khi mọi thứ đã lệch.
    pub fn state_hash(&self) -> StateHash {
        let mut h = StateHasher::with_domain("mow.sim.v1");
        self.world.canonical_hash(&mut h);
        self.branch.canonical_hash(&mut h);
        h.write_u64(self.seed.0);
        self.clock.canonical_hash(&mut h);
        self.store.canonical_hash(&mut h);
        self.log.canonical_hash(&mut h);
        self.ids.canonical_hash(&mut h);
        h.write_seq(self.seen_requests.iter().copied(), |hh, r| {
            hh.write_u64(r);
        });
        h.finish()
    }

    // ── Ghi: một cửa duy nhất ───────────────────────────────────────────────

    /// Áp một command.
    ///
    /// Toàn bộ giao dịch là **hoặc tất cả hoặc không gì**. Handler thất bại thì
    /// không mutation nào được áp, không sự kiện nào được ghi, và bộ cấp phát
    /// id lùi về đúng chỗ cũ.
    pub fn apply(&mut self, cmd: &Command) -> CommandResult<Committed> {
        if cmd.world != self.world {
            return Err(Failure::new(
                FailureCode::PreconditionFailed,
                format!("command gửi tới {} nhưng sim là {}", cmd.world, self.world),
            ));
        }

        // Idempotency trước mọi thứ khác: một kết quả LLM tới muộn rồi tới lại
        // lần nữa không được phép có tác dụng hai lần (`§20.2.2`).
        if let Some(rid) = cmd.request_id {
            if self.seen_requests.contains(&rid) {
                return Err(Failure::new(
                    FailureCode::DuplicateRequest,
                    format!("request {rid} đã được xử lý"),
                ));
            }
        }

        let handler = self.handlers.get(&cmd.kind.0).ok_or_else(|| {
            Failure::new(
                FailureCode::UnknownCommand,
                format!("không có handler cho `{}`", cmd.kind.0),
            )
        })?;

        // Chụp lại bộ cấp phát để lùi được nếu giao dịch bị từ chối.
        let id_truoc = self.ids.clone();
        let rng = RngStreams::new(self.seed);

        let ket_qua = {
            let mut ctx = Ctx::new(
                &self.store,
                &self.clock,
                self.world,
                rng,
                cmd,
                &mut self.ids,
            );
            match handler.handle(&mut ctx) {
                Ok(()) => Ok(ctx.into_parts()),
                Err(e) => Err(e),
            }
        };

        let (mutations, events) = match ket_qua {
            Ok(v) => v,
            Err(e) => {
                self.ids = id_truoc;
                return Err(e);
            }
        };

        // Kiểm tra trước, áp sau. Một mutation không áp được là **bug của
        // handler**, không phải lỗi người chơi, nên nó phải nổ thành
        // `InvariantViolated` chứ không lặng lẽ bị bỏ qua.
        if let Err(e) = self.kiem_tra_mutations(&mutations) {
            self.ids = id_truoc;
            return Err(e);
        }

        let mut so_mutation = 0usize;
        for m in &mutations {
            self.ap_mutation(m);
            so_mutation += 1;
        }

        let tick = self.clock.local();
        let mut seqs = Vec::with_capacity(events.len());
        for d in events {
            seqs.push(self.log.append(d, self.branch, self.world, tick));
        }

        if let Some(rid) = cmd.request_id {
            self.seen_requests.insert(rid);
        }

        Ok(Committed {
            events: seqs,
            mutations: so_mutation,
        })
    }

    /// Tiến đồng hồ `n` tick thần.
    pub fn advance(&mut self, n: u64) -> CommandResult<()> {
        self.clock
            .advance_divine(n)
            .map(|_| ())
            .map_err(|e| Failure::new(FailureCode::Arithmetic, e.to_string()))
    }

    /// Chạy bộ bất biến lên state hiện tại.
    pub fn check(&self, runner: &InvariantRunner) -> InvariantReport {
        runner.run(self)
    }

    // ── Riêng tư ────────────────────────────────────────────────────────────

    /// Xác nhận mọi mutation áp được, **trước khi** áp cái đầu tiên.
    ///
    /// Đây là thứ biến "giao dịch" từ một từ ngữ thành một bảo đảm.
    fn kiem_tra_mutations(&self, ms: &[Mutation]) -> CommandResult<()> {
        // Theo dõi các thực thể mà chính giao dịch này tạo ra hoặc xóa, để
        // "spawn rồi set attr ngay" là hợp lệ còn "set attr lên thứ vừa xóa"
        // thì không.
        let mut them: BTreeSet<_> = BTreeSet::new();
        let mut bot: BTreeSet<_> = BTreeSet::new();

        let ton_tai = |id, them: &BTreeSet<_>, bot: &BTreeSet<_>| {
            (self.store.contains(id) || them.contains(&id)) && !bot.contains(&id)
        };

        for m in ms {
            match m {
                Mutation::Spawn { id } => {
                    if ton_tai(*id, &them, &bot) {
                        return Err(Failure::new(
                            FailureCode::InvariantViolated,
                            format!("spawn {id} nhưng nó đã tồn tại"),
                        ));
                    }
                    them.insert(*id);
                    bot.remove(id);
                }
                Mutation::Despawn { id } => {
                    if !ton_tai(*id, &them, &bot) {
                        return Err(Failure::new(
                            FailureCode::NoSuchEntity,
                            format!("despawn {id} nhưng nó không tồn tại"),
                        ));
                    }
                    bot.insert(*id);
                    them.remove(id);
                }
                Mutation::SetAttr { id, .. } | Mutation::RemoveAttr { id, .. } => {
                    if !ton_tai(*id, &them, &bot) {
                        return Err(Failure::new(
                            FailureCode::NoSuchEntity,
                            format!("đổi thuộc tính của {id} nhưng nó không tồn tại"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn ap_mutation(&mut self, m: &Mutation) {
        match m {
            Mutation::Spawn { id } => {
                self.store.spawn(*id);
            }
            Mutation::Despawn { id } => {
                self.store.despawn(*id);
            }
            Mutation::SetAttr { id, key, value } => {
                self.store.set_attr(*id, key.clone(), value.clone());
            }
            Mutation::RemoveAttr { id, key } => {
                self.store.remove_attr(*id, key);
            }
        }
    }
}

/// Số thứ tự sự kiện cuối cùng đã ghi, nếu có.
impl Sim {
    /// Sự kiện mới nhất.
    pub fn last_event(&self) -> Option<EventSeq> {
        if self.log.is_empty() {
            None
        } else {
            Some(EventSeq(self.log.len() as u64 - 1))
        }
    }
}
