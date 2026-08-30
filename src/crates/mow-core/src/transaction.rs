//! Giao dịch — **đường ghi state duy nhất** (`idea.md §22.1`).
//!
//! Bất biến số 1 của toàn hệ thống: không có đường ghi state nào đi vòng qua
//! đây. Nó được thực thi bằng ba lớp, và cả ba đều cần:
//!
//! 1. Hàm ghi của [`crate::ecs::Store`] là `pub(crate)`. Ngoài crate không gọi
//!    được.
//! 2. [`Sim`] không cho mượn `&mut Store` ra ngoài. Không có `world_mut()`.
//! 3. Handler **không** nhận `&mut Store`. Nó nhận [`Ctx`] chỉ-đọc và trả về
//!    một danh sách [`Mutation`]. Việc áp thật sự xảy ra sau khi handler đã
//!    xong và đã thành công.
//!
//! Lớp thứ ba là lớp đắt nhất và cũng là lớp quan trọng nhất. Nếu handler được
//! ghi thẳng, thì một handler thất bại **nửa chừng** sẽ để lại thế giới ở trạng
//! thái đã sửa một phần: nửa số vật liệu đã bị tiêu, món đồ chưa được tạo. Đó
//! là loại lỗi không thể tái hiện được, vì nó phụ thuộc vào việc handler chết ở
//! đúng dòng nào.
//!
//! Cách này còn cho một thứ miễn phí: `Ctx` chỉ đọc nghĩa là handler **không
//! thể** thấy hiệu ứng của chính mình giữa chừng. Nên thứ tự các mutation bên
//! trong một giao dịch không ảnh hưởng kết quả, và điều đó làm giao dịch dễ
//! suy luận hơn nhiều.
//!
//! [`Sim`]: crate::sim::Sim

use crate::clock::{Clock, Tick};
use crate::command::{Command, CommandResult, Failure, FailureCode};
use crate::ecs::{AttrKey, Store};
use crate::event::{EventDraft, EventSeq};
use crate::ids::{EntityId, IdAllocator, WorldId};
use crate::value::Value;
use mow_math::{CanonicalHash, RngStreams, StateHasher};
use std::collections::BTreeMap;

/// Một thay đổi nguyên tử lên state.
///
/// Danh sách này cố ý ngắn. Mọi thứ phức tạp hơn — chế tác, di chuyển, giao
/// dịch — đều phân rã thành các mutation này, và điều đó khiến state hash chỉ
/// phụ thuộc vào một tập nhỏ phép biến đổi mà ta kiểm chứng được từng cái.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutation {
    /// Tạo thực thể mới.
    Spawn {
        /// Định danh đã được cấp phát.
        id: EntityId,
    },
    /// Xóa thực thể.
    Despawn {
        /// Thực thể bị xóa.
        id: EntityId,
    },
    /// Đặt một thuộc tính.
    SetAttr {
        /// Thực thể.
        id: EntityId,
        /// Khóa.
        key: AttrKey,
        /// Giá trị mới.
        value: Value,
    },
    /// Xóa một thuộc tính.
    RemoveAttr {
        /// Thực thể.
        id: EntityId,
        /// Khóa.
        key: AttrKey,
    },
}

impl CanonicalHash for Mutation {
    fn canonical_hash(&self, h: &mut StateHasher) {
        match self {
            Mutation::Spawn { id } => {
                h.write_str("spawn");
                id.canonical_hash(h);
            }
            Mutation::Despawn { id } => {
                h.write_str("despawn");
                id.canonical_hash(h);
            }
            Mutation::SetAttr { id, key, value } => {
                h.write_str("set_attr");
                id.canonical_hash(h);
                h.write_str(key);
                value.canonical_hash(h);
            }
            Mutation::RemoveAttr { id, key } => {
                h.write_str("remove_attr");
                id.canonical_hash(h);
                h.write_str(key);
            }
        }
    }
}

/// Khung cảnh chỉ-đọc mà handler nhìn thấy.
pub struct Ctx<'a> {
    /// Kho thực thể, chỉ đọc.
    pub store: &'a Store,
    /// Đồng hồ, chỉ đọc.
    pub clock: &'a Clock,
    /// Thế giới đang xử lý.
    pub world: WorldId,
    /// Tick địa phương hiện tại.
    pub tick: Tick,
    /// Dòng ngẫu nhiên có tên.
    pub rng: RngStreams,
    /// Command đang được xử lý.
    pub command: &'a Command,

    // Phần thu thập, riêng tư để handler chỉ đẩy vào qua phương thức.
    mutations: Vec<Mutation>,
    events: Vec<EventDraft>,
    id_alloc: &'a mut IdAllocator,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(
        store: &'a Store,
        clock: &'a Clock,
        world: WorldId,
        rng: RngStreams,
        command: &'a Command,
        id_alloc: &'a mut IdAllocator,
    ) -> Ctx<'a> {
        let tick = clock.local();
        Ctx {
            store,
            clock,
            world,
            tick,
            rng,
            command,
            mutations: Vec::new(),
            events: Vec::new(),
            id_alloc,
        }
    }

    /// Cấp một định danh mới.
    ///
    /// Cấp phát xảy ra **trong** giao dịch, nên nếu giao dịch bị từ chối thì
    /// định danh cũng bị trả lại (xem [`Ctx::into_parts`]). Nếu không, một
    /// command thất bại sẽ để lại lỗ hổng trong dãy id, và dãy id là một phần
    /// của state hash — nghĩa là hai lần chạy khác nhau ở chỗ *có bao nhiêu
    /// command từng thất bại* sẽ cho hai thế giới khác nhau.
    pub fn new_entity_id(&mut self) -> EntityId {
        self.id_alloc.next_entity()
    }

    /// Đẩy một mutation.
    pub fn mutate(&mut self, m: Mutation) -> &mut Self {
        self.mutations.push(m);
        self
    }

    /// Tạo thực thể và trả định danh của nó.
    pub fn spawn(&mut self) -> EntityId {
        let id = self.new_entity_id();
        self.mutate(Mutation::Spawn { id });
        id
    }

    /// Đặt thuộc tính.
    pub fn set(&mut self, id: EntityId, key: &str, value: impl Into<Value>) -> &mut Self {
        self.mutate(Mutation::SetAttr {
            id,
            key: key.to_owned(),
            value: value.into(),
        })
    }

    /// Ghi một sự kiện.
    pub fn emit(&mut self, draft: EventDraft) -> &mut Self {
        self.events.push(draft);
        self
    }

    /// Đòi một thực thể tồn tại, nếu không thì thất bại có mã.
    pub fn require_entity(&self, id: EntityId) -> CommandResult<()> {
        if self.store.contains(id) {
            Ok(())
        } else {
            Err(Failure::new(
                FailureCode::NoSuchEntity,
                format!("không có thực thể {id}"),
            ))
        }
    }

    /// Đòi một trường số nguyên trong payload.
    pub fn require_int(&self, field: &str) -> CommandResult<i64> {
        match self.command.payload.get(field) {
            Some(Value::Int(v)) => Ok(*v),
            Some(other) => Err(Failure::wrong_type(field, "int", other.type_name())),
            None => Err(Failure::missing(field)),
        }
    }

    /// Đòi một trường chuỗi trong payload.
    pub fn require_text(&self, field: &str) -> CommandResult<&str> {
        match self.command.payload.get(field) {
            Some(Value::Text(v)) => Ok(v.as_str()),
            Some(other) => Err(Failure::wrong_type(field, "text", other.type_name())),
            None => Err(Failure::missing(field)),
        }
    }

    /// Đòi một trường định danh thực thể.
    pub fn require_entity_field(&self, field: &str) -> CommandResult<EntityId> {
        match self.command.payload.get(field) {
            Some(Value::Uint(v)) => Ok(EntityId(*v)),
            Some(other) => Err(Failure::wrong_type(
                field,
                "uint (entity id)",
                other.type_name(),
            )),
            None => Err(Failure::missing(field)),
        }
    }

    pub(crate) fn into_parts(self) -> (Vec<Mutation>, Vec<EventDraft>) {
        (self.mutations, self.events)
    }
}

/// Handler của một loại command.
///
/// Chữ ký nói lên toàn bộ hợp đồng: nhận [`Ctx`] chỉ đọc, trả `Result`. Không
/// có `&mut Store` ở đâu cả, nên không có cách nào ghi lén.
pub trait Handler: Send + Sync + 'static {
    /// Loại command mà handler này nhận.
    fn kind(&self) -> &str;

    /// Xử lý. Đẩy mutation và event vào `ctx`; trả `Err` để từ chối toàn bộ.
    fn handle(&self, ctx: &mut Ctx<'_>) -> CommandResult<()>;
}

/// Cho phép dùng closure làm handler trong test và trong mã dựng nhanh.
pub struct FnHandler<F> {
    kind: String,
    f: F,
}

impl<F> FnHandler<F>
where
    F: Fn(&mut Ctx<'_>) -> CommandResult<()> + Send + Sync + 'static,
{
    /// Dựng.
    pub fn new(kind: &str, f: F) -> FnHandler<F> {
        FnHandler {
            kind: kind.to_owned(),
            f,
        }
    }
}

impl<F> Handler for FnHandler<F>
where
    F: Fn(&mut Ctx<'_>) -> CommandResult<()> + Send + Sync + 'static,
{
    fn kind(&self) -> &str {
        &self.kind
    }
    fn handle(&self, ctx: &mut Ctx<'_>) -> CommandResult<()> {
        (self.f)(ctx)
    }
}

/// Sổ đăng ký handler.
///
/// `BTreeMap` để việc liệt kê handler (dùng trong `mow-mcp` và trong báo cáo
/// chẩn đoán) có thứ tự ổn định.
#[derive(Default)]
pub struct HandlerRegistry {
    map: BTreeMap<String, Box<dyn Handler>>,
}

impl HandlerRegistry {
    /// Sổ rỗng.
    pub fn new() -> HandlerRegistry {
        HandlerRegistry::default()
    }

    /// Đăng ký. Trùng loại là **lỗi lập trình**, nên panic ngay lúc khởi tạo
    /// thay vì để một trong hai handler thắng một cách âm thầm.
    pub fn register(&mut self, h: Box<dyn Handler>) -> &mut Self {
        let k = h.kind().to_owned();
        assert!(
            self.map.insert(k.clone(), h).is_none(),
            "đăng ký trùng handler cho `{k}`"
        );
        self
    }

    /// Đăng ký một closure.
    pub fn on<F>(&mut self, kind: &str, f: F) -> &mut Self
    where
        F: Fn(&mut Ctx<'_>) -> CommandResult<()> + Send + Sync + 'static,
    {
        self.register(Box::new(FnHandler::new(kind, f)))
    }

    /// Tra handler.
    pub fn get(&self, kind: &str) -> Option<&dyn Handler> {
        self.map.get(kind).map(std::convert::AsRef::as_ref)
    }

    /// Liệt kê mọi loại command đã đăng ký, theo thứ tự.
    pub fn kinds(&self) -> impl Iterator<Item = &str> {
        self.map.keys().map(String::as_str)
    }

    /// Số handler.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Rỗng hay không.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Kết quả của một giao dịch đã commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// Các sự kiện đã ghi, theo thứ tự.
    pub events: Vec<EventSeq>,
    /// Số mutation đã áp.
    pub mutations: usize,
}
