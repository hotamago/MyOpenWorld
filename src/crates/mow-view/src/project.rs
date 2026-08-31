//! Chiếu state của thế giới xuống thứ mà một ống kính được phép thấy.
//!
//! ## Bất biến của module này
//!
//! **Không có cách nào dựng một [`EntityView`] mà không đi qua [`project`].**
//!
//! Trường của nó riêng tư, không có constructor công khai, không có
//! `Default`, không có `Deserialize`. Đó không phải là kín đáo vì kín đáo —
//! đó là cách duy nhất khiến một trường mới **không thể** lọt ra dây mà không
//! ai trả lời câu hỏi *"chế độ nào được thấy cái này"*.
//!
//! Cách hỏng mà nó tồn tại để chặn không phải là "bộ lọc viết sai". Bộ lọc viết
//! sai thì test bắt được. Cách hỏng thật là: sáu tháng sau, ai đó thêm
//! `current_goal` vào payload để làm một panel, gán thẳng từ store, và không có
//! gì phản đối. Panel chạy. Không ai nhận ra rằng một người chơi hóa thân giờ
//! đọc được ý định của mọi NPC trong tầm mắt.

use crate::lens::{Lens, Mode};
use mow_core::{EntityId, Value};
use serde::Serialize;
use std::collections::BTreeMap;

/// Một giá trị là **sự thật** hay là **phỏng đoán**.
///
/// `§18.9` ràng buộc 1: *"Belief và sự thật không bao giờ được vẽ giống nhau."*
/// Trường này bắt buộc trên mọi [`Field`], và không có mặc định — một giá trị
/// không nói rõ nó thuộc loại nào thì không đi qua được kiểu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    /// Sự thật của thế giới. Client vẽ đặc.
    Truth,
    /// Ước đoán của nhân vật, kèm mức tin cậy `0`–`1000`.
    Belief {
        /// Mức tin cậy.
        confidence: u16,
    },
}

/// Một trường trong payload, luôn kèm nhãn.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Field {
    /// Giá trị.
    pub value: Value,
    /// Sự thật hay phỏng đoán.
    pub certainty: Certainty,
    /// Ai đã làm nó thành ra thế này. **Chỉ True God thấy** (`§18.9`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<u64>,
}

/// Sự thật của thế giới về một thực thể — **đầu vào** của [`project`].
///
/// Kiểu này không bao giờ ra dây. Nó là thứ máy chủ biết.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorldTruth {
    /// Thuộc tính thật.
    pub attrs: BTreeMap<String, Value>,
    /// Event seq đã đặt giá trị gần nhất, cho provenance.
    pub provenance: BTreeMap<String, u64>,
}

impl WorldTruth {
    /// Rỗng.
    pub fn new() -> WorldTruth {
        WorldTruth::default()
    }

    /// Đặt một thuộc tính kèm event nguồn.
    pub fn set(&mut self, key: &str, v: Value, from_event: u64) -> &mut WorldTruth {
        self.attrs.insert(key.to_owned(), v);
        self.provenance.insert(key.to_owned(), from_event);
        self
    }
}

/// Những gì một ống kính được phép thấy về một thực thể.
///
/// Trường riêng tư có chủ đích — xem docstring của module.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EntityView {
    id: EntityId,
    fields: BTreeMap<String, Field>,
}

impl EntityView {
    /// Định danh.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Các trường thấy được.
    pub fn fields(&self) -> &BTreeMap<String, Field> {
        &self.fields
    }

    /// Một trường cụ thể.
    pub fn field(&self, key: &str) -> Option<&Field> {
        self.fields.get(key)
    }
}

/// Một bóng người thấy được nhưng **không nhận ra**.
///
/// Không có trường `id`, và sẽ không bao giờ có. Đó là điều phân biệt "có ai đó
/// ở kia" với "đó là Bram", và là thứ khiến rình rập, cải trang và sương mù có
/// nghĩa thay vì chỉ là hiệu ứng hình ảnh.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PresenceView {
    /// Vị trí **ước lượng**.
    pub at_x: i64,
    /// Vị trí ước lượng.
    pub at_y: i64,
    /// Dấu hiệu quan sát được — triệu chứng, không phải nguyên nhân.
    pub signs: Vec<String>,
    /// Độ tin cậy của quan sát, `0`–`1000`.
    pub fidelity: u16,
}

/// Những thuộc tính mà **người khác không bao giờ đọc được trực tiếp**.
///
/// Ở chế độ hóa thân, chúng chỉ ra dây khi thực thể được chiếu **chính là**
/// người đang xem. Danh sách này là dữ liệu, không phải luật cứng — nhưng nó
/// mặc định **cấm**, và đó là chiều đúng: thêm một thuộc tính mới thì nó riêng
/// tư cho tới khi có người quyết định ngược lại.
const NOI_TAM: &[&str] = &[
    "goal",
    "plan",
    "intent",
    "secret",
    "belief",
    "hunger",
    "pain",
    "fatigue",
    "money",
    "inventory",
];

fn la_noi_tam(key: &str) -> bool {
    NOI_TAM
        .iter()
        .any(|p| key == *p || key.starts_with(&format!("{p}.")))
}

/// Chiếu sự thật xuống thứ ống kính được phép thấy.
///
/// **Đây là hàm duy nhất dựng được [`EntityView`].**
///
/// Trả `None` khi ống kính không nhận ra thực thể — chứ không phải trả về một
/// view rỗng. Khác biệt này quan trọng: một view rỗng vẫn nói cho client biết
/// *"thực thể này tồn tại và có id đó"*, và đó đã là rò rỉ. `None` thì không nói
/// gì cả.
pub fn project(id: EntityId, truth: &WorldTruth, lens: &Lens) -> Option<EntityView> {
    if !lens.identifies(id) {
        return None;
    }

    let la_minh = lens.viewer() == Some(id);
    let mut fields = BTreeMap::new();

    for (k, v) in &truth.attrs {
        let (certainty, cho_phep) = match lens.mode() {
            // True God thấy mọi thứ, và thấy nó **là sự thật**.
            Mode::TrueGod => (Certainty::Truth, true),

            // Quan sát: sự thật của vùng đang xem, nhưng nội tâm vẫn là nội tâm.
            // Một nhà quan sát theo dõi một thành phố không đọc được ý định của
            // từng người — nếu đọc được thì `§10.2` chỉ còn là trang trí, và
            // toàn bộ kịch tính của việc *đoán* biến mất.
            Mode::Observer => (Certainty::Truth, !la_noi_tam(k)),

            // Hóa thân: nội tâm chỉ của chính mình. Của người khác là phỏng đoán.
            Mode::Embodied => {
                if la_minh {
                    (Certainty::Truth, true)
                } else if la_noi_tam(k) {
                    (Certainty::Belief { confidence: 0 }, false)
                } else {
                    (uoc_doan(lens, id), true)
                }
            }
        };

        if !cho_phep {
            continue;
        }

        fields.insert(
            k.clone(),
            Field {
                value: v.clone(),
                certainty,
                // Provenance **chỉ** True God. Ở chế độ khác nó bị bỏ hẳn khỏi
                // payload, không phải gửi rồi để client ẩn.
                provenance: if lens.mode().sees_provenance() {
                    truth.provenance.get(k).copied()
                } else {
                    None
                },
            },
        );
    }

    Some(EntityView { id, fields })
}

/// Mức tin cậy khi hóa thân nhìn người khác.
///
/// Lấy từ `fidelity` của quan sát đã nhận ra người đó. Chỉ số của người khác là
/// **ước đoán có sai số** (`§18.9`), nên chúng không bao giờ mang nhãn `Truth`.
fn uoc_doan(lens: &Lens, id: EntityId) -> Certainty {
    let confidence = lens
        .context()
        .and_then(|c| {
            c.observations
                .iter()
                .filter(|o| o.identity == Some(id))
                // Quan sát rõ nhất quyết định, không phải quan sát mới nhất:
                // nhìn tận mặt lúc sáng rồi thoáng thấy lúc tối thì cái biết vẫn
                // là cái biết lúc sáng.
                .map(|o| o.fidelity.get().raw())
                .max()
        })
        .map_or(0, |raw| {
            // Q16.16 `[0,1]` → thang `0`–`1000`.
            u16::try_from((raw.max(0) * 1000) >> 16).unwrap_or(1000)
        });
    Certainty::Belief { confidence }
}

/// Chiếu những bóng người chưa nhận ra.
pub fn project_presences(lens: &Lens) -> Vec<PresenceView> {
    let Some(ctx) = lens.context() else {
        return Vec::new();
    };
    if lens.mode() != Mode::Embodied {
        return Vec::new();
    }
    ctx.observations
        .iter()
        .filter(|o| o.identity.is_none())
        .map(|o| PresenceView {
            at_x: o.at.x,
            at_y: o.at.y,
            signs: o.signs.clone(),
            fidelity: u16::try_from((o.fidelity.get().raw().max(0) * 1000) >> 16).unwrap_or(1000),
        })
        .collect()
}
