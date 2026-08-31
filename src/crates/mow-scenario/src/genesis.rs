//! Genesis — biên dịch worldseed thành command tại tick 0 (`§22.28`).
//!
//! > Scenario khởi tạo được biên dịch thành genesis command tại tick 0; **không
//! > có đường ghi thẳng state vào save**.
//!
//! Đường tắt ở đây rất cám dỗ. Dựng một thế giới ban đầu bằng cách gọi thẳng
//! vào kho thực thể nhanh hơn nhiều so với việc đi qua transaction handler cho
//! từng thứ một. Nhưng cái giá phải trả trả về sau và trả rất đắt:
//!
//! - Nhật ký sự kiện bắt đầu từ **sau** genesis, nên chuỗi nhân quả của một
//!   ngôi làng cụt ở chỗ "ngôi làng đã tồn tại". Câu hỏi *"vì sao có ngôi làng
//!   ở đây"* không trả lời được.
//! - Genesis đi một đường, gameplay đi một đường khác, và hai đường đó sẽ lệch.
//!   Một bug chỉ xuất hiện với thực thể do genesis tạo ra thì không tái hiện
//!   được bằng cách chơi.
//! - Bất biến chạy trên thế giới mới tạo sẽ bắt được những vi phạm mà đường
//!   command bình thường không thể tạo ra — và người ta sẽ nới bất biến ra để
//!   nó im, thay vì sửa genesis.

use crate::worldseed::Worldseed;
use mow_core::{Command, EntityId, Sim, Value, WorldId};
use std::collections::BTreeMap;
use thiserror::Error;

/// Lỗi khi chạy genesis.
#[derive(Debug, Error)]
pub enum GenesisError {
    /// Worldseed sai cấu trúc.
    #[error("worldseed `{id}` không hợp lệ:\n{}", .errors.iter().map(|e| format!("  {e}")).collect::<Vec<_>>().join("\n"))]
    Invalid {
        /// Định danh worldseed.
        id: String,
        /// Danh sách lỗi.
        errors: Vec<String>,
    },

    /// Một bước genesis thất bại.
    ///
    /// Genesis thất bại giữa chừng để lại một thế giới hỏng một nửa, nên lỗi
    /// phải nói rõ **bước thứ mấy** và **lệnh gì** — đủ để sửa worldseed mà
    /// không phải chạy lại và đoán.
    #[error("genesis bước {index} (`{command}`) thất bại: {source}")]
    StepFailed {
        /// Thứ tự bước, đếm từ 0.
        index: usize,
        /// Lệnh.
        command: String,
        /// Nguyên nhân.
        #[source]
        source: mow_core::Failure,
    },

    /// Bước tham chiếu tới một tên chưa được đặt.
    #[error("genesis bước {index} tham chiếu `${name}` nhưng chưa bước nào đặt tên đó")]
    UnknownName {
        /// Thứ tự bước.
        index: usize,
        /// Tên bị thiếu.
        name: String,
    },
}

/// Kết quả chạy genesis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenesisResult {
    /// Tên → thực thể đã tạo.
    pub named: BTreeMap<String, EntityId>,
    /// Số command đã áp.
    pub commands: usize,
    /// Số sự kiện đã ghi.
    pub events: usize,
}

/// Chạy genesis lên một `Sim` rỗng.
///
/// Phải gọi **trước khi** đồng hồ tiến khỏi tick 0. Không kiểm điều đó ở đây vì
/// `Sim` không cho phép lùi đồng hồ, nên vi phạm sẽ tự lộ ra thành sự kiện
/// genesis mang tick khác 0 — thứ mà bất biến bắt được.
pub fn run(sim: &mut Sim, seed: &Worldseed) -> Result<GenesisResult, GenesisError> {
    seed.validate().map_err(|errors| GenesisError::Invalid {
        id: seed.id.clone(),
        errors,
    })?;

    let world: WorldId = sim.world_id();
    let mut named: BTreeMap<String, EntityId> = BTreeMap::new();
    let mut so_event = 0usize;

    for (i, buoc) in seed.genesis.iter().enumerate() {
        let payload = doi_args(&buoc.args, &named, i)?;
        let cmd = Command::new(&buoc.command, world, payload);

        let ket = sim.apply(&cmd).map_err(|source| GenesisError::StepFailed {
            index: i,
            command: buoc.command.clone(),
            source,
        })?;
        so_event += ket.events.len();

        // Gán tên cho thực thể vừa tạo. Lấy id lớn nhất vì bộ cấp phát tăng
        // đơn điệu, nên thứ vừa tạo luôn có id lớn nhất.
        if let Some(ten) = &buoc.name {
            if let Some(id) = sim.store().ids().next_back() {
                named.insert(ten.clone(), id);
            }
        }
    }

    Ok(GenesisResult {
        named,
        commands: seed.genesis.len(),
        events: so_event,
    })
}

/// Đổi tham số YAML sang [`Value`], giải tham chiếu `$tên`.
fn doi_args(
    args: &BTreeMap<String, serde_yaml::Value>,
    named: &BTreeMap<String, EntityId>,
    index: usize,
) -> Result<Value, GenesisError> {
    let mut m = std::collections::BTreeMap::new();
    for (k, v) in args {
        m.insert(k.clone(), doi_gia_tri(v, named, index)?);
    }
    Ok(Value::Map(m))
}

fn doi_gia_tri(
    v: &serde_yaml::Value,
    named: &BTreeMap<String, EntityId>,
    index: usize,
) -> Result<Value, GenesisError> {
    Ok(match v {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::Uint(u)
            } else {
                // Số thực trong worldseed là lỗi soạn thảo — `§P10.2` cấm chúng
                // trên đường commit. Giữ nguyên văn để handler báo lỗi đúng chỗ.
                Value::Text(n.to_string())
            }
        }
        serde_yaml::Value::String(s) => {
            if let Some(ten) = s.strip_prefix('$') {
                let id = named.get(ten).ok_or_else(|| GenesisError::UnknownName {
                    index,
                    name: ten.to_owned(),
                })?;
                Value::Uint(id.get())
            } else {
                Value::Text(s.clone())
            }
        }
        serde_yaml::Value::Sequence(xs) => Value::List(
            xs.iter()
                .map(|x| doi_gia_tri(x, named, index))
                .collect::<Result<_, _>>()?,
        ),
        serde_yaml::Value::Mapping(mm) => {
            let mut out = std::collections::BTreeMap::new();
            for (k, val) in mm {
                if let Some(ks) = k.as_str() {
                    out.insert(ks.to_owned(), doi_gia_tri(val, named, index)?);
                }
            }
            Value::Map(out)
        }
        serde_yaml::Value::Tagged(t) => doi_gia_tri(&t.value, named, index)?,
    })
}
