//! Di truyền định lượng (`idea.md §9.5.1`, `PD-22`).
//!
//! > Nếu không có mô hình thật thì việc lai giống, dòng dõi quý tộc và thuần hóa
//! > quái vật chỉ là **trang trí**.
//!
//! ```text
//! phenotype = giá_trị_di_truyền_cộng_gộp
//!           + hiệu_ứng_môi_trường      (dinh dưỡng, bệnh, khí hậu, mana)
//!           + tương_tác_gen×môi_trường
//!           + nhiễu
//! ```
//!
//! ## Ba tham số quyết định **cảm giác chơi**
//!
//! **1. `h²` khác nhau cho từng trait.**
//!
//! > Đặt khác nhau cho từng trait là cách rẻ nhất để có một thế giới nơi "con
//! > nhà nòi" **đúng với vài thứ và sai với nhiều thứ khác**.
//!
//! Một `h²` toàn cục cho ra hai thế giới đều nhàm: hoặc dòng dõi quyết định tất
//! cả, hoặc dòng dõi chẳng có nghĩa gì. Cái đáng chơi nằm ở chỗ chiều cao thì di
//! truyền còn tính khí thì không — và người chơi phải tự phát hiện ra điều đó.
//!
//! **2. Tương tác gen×môi trường.**
//!
//! > Cùng một genotype cho ra phenotype khác nhau ở vùng đói kém và vùng trù
//! > phú. Điều này khiến **chọn giống ở một nơi rồi mang sang nơi khác có thể
//! > thất bại**.
//!
//! Đây là vế mà một mô hình `phenotype = gen + môi_trường` (cộng thuần) **không**
//! diễn đạt được: với phép cộng, một giống tốt hơn ở đâu cũng tốt hơn đúng bằng
//! ấy. Phải có số hạng tương tác thì "giống lúa này chỉ hợp đất phù sa" mới có chỗ.
//!
//! **3. Cận huyết, và mức giảm không phải một hình phạt cố định.**
//!
//! > Mức giảm phụ thuộc **cả kiểu giao phối lẫn cấu trúc di truyền của quần thể**
//! > — không phải một hình phạt cố định.
//!
//! Nên [`inbreeding_depression`] nhận cả `f` lẫn `population_load`: cùng một hệ
//! số cận huyết gây hại nhiều hơn hẳn ở một quần thể đã tích lũy nhiều alen lặn
//! có hại. Đó là lý do một quần thể rồng bị săn xuống dưới ngưỡng **mắc kẹt**:
//! không phải vì ít con, mà vì nút thắt di truyền đã xảy ra.
//!
//! ## Số học
//!
//! Mọi thứ ở đây là số nguyên thang `0`–`1000`. `§P10.2.1` cấm số thực trên
//! đường commit, và phenotype **là** state.

use serde::{Deserialize, Serialize};

/// Một tính trạng đa gen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trait {
    /// Định danh: `height`, `stamina`, `mana_affinity`.
    pub id: String,
    /// **Hệ số di truyền** `h²`, `0`–`1000`.
    ///
    /// `1000` là hoàn toàn do gen; `0` là hoàn toàn do hoàn cảnh. Đặt khác nhau
    /// cho từng trait — xem docstring của module.
    pub heritability: u16,
    /// Trait này có gắn với **sức sống** không.
    ///
    /// Chỉ những trait gắn sức sống mới chịu suy thoái cận huyết. Chiều cao thì
    /// có, màu mắt thì không — và gộp chúng lại làm cận huyết trở thành một hình
    /// phạt chung chung thay vì một cơ chế sinh học.
    pub fitness_linked: bool,
    /// Số hạng tương tác gen×môi trường, `0`–`1000`.
    ///
    /// `0` là gen và môi trường cộng thuần. Càng cao thì một genotype càng phụ
    /// thuộc vào việc nó ở đâu.
    pub gxe: u16,
}

/// Môi trường mà một cá thể lớn lên.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    /// Dinh dưỡng, `0`–`1000`.
    pub nutrition: u16,
    /// Gánh nặng bệnh tật, `0`–`1000`. Cao là xấu.
    pub disease_load: u16,
    /// Khí hậu có thuận không, `0`–`1000`.
    pub climate: u16,
    /// Mật độ mana, `0`–`1000`.
    pub mana: u16,
}

impl Environment {
    /// Chất lượng môi trường tổng hợp, `0`–`1000`.
    pub fn quality(&self) -> u16 {
        let tot = i64::from(self.nutrition) + i64::from(self.climate) + i64::from(self.mana);
        let xau = i64::from(self.disease_load);
        u16::try_from((tot / 3 - xau / 2).clamp(0, 1_000)).unwrap_or(0)
    }
}

/// Giá trị di truyền cộng gộp của một cá thể cho một trait, `0`–`1000`.
///
/// Trung bình của cha mẹ, cộng phân ly — con không phải bản sao trung bình của
/// cha mẹ, và đó là toàn bộ lý do chọn giống mất nhiều thế hệ.
pub fn breeding_value(sire: u16, dam: u16, segregation: i16) -> u16 {
    let tb = i64::midpoint(i64::from(sire), i64::from(dam));
    u16::try_from((tb + i64::from(segregation)).clamp(0, 1_000)).unwrap_or(0)
}

/// **Suy thoái cận huyết**, tính bằng số điểm bị trừ.
///
/// Hai tham số, và cả hai đều cần:
///
/// - `f` — hệ số cận huyết của cá thể, `0`–`1000`.
/// - `population_load` — quần thể này đã tích lũy bao nhiêu alen lặn có hại,
///   `0`–`1000`.
///
/// Tích của chúng, không phải chỉ `f`. Cùng một cặp anh em ruột cho ra hậu quả
/// nhẹ ở một quần thể lớn khỏe mạnh và nặng ở một quần thể đã qua nút thắt — và
/// đó là khác biệt giữa "cận huyết là xấu" với một mô hình có thật.
pub fn inbreeding_depression(f: u16, population_load: u16) -> u16 {
    u16::try_from(i64::from(f) * i64::from(population_load) / 1_000).unwrap_or(0)
}

/// Biểu hiện một trait ra kiểu hình.
///
/// **Hàm thuần và xác định.** `noise` là tham số, không phải một lần tung xúc
/// xắc bên trong: cùng gen, cùng môi trường, cùng nhiễu thì luôn ra cùng kết quả,
/// nên `§22.9` giữ được.
pub fn express(
    t: &Trait,
    breeding_value: u16,
    env: &Environment,
    inbreeding: u16,
    population_load: u16,
    noise: i16,
) -> u16 {
    let g = i64::from(breeding_value);
    let e = i64::from(env.quality());
    let h2 = i64::from(t.heritability);

    // Phần cộng gộp: gen và môi trường chia nhau theo `h²`.
    let cong_gop = g * h2 / 1_000 + e * (1_000 - h2) / 1_000;

    // **Tương tác gen×môi trường.** Đây là số hạng mà phép cộng thuần không có:
    // một genotype tốt chỉ phát huy được ở môi trường tốt. Lấy độ lệch của cả
    // hai so với mức trung bình rồi nhân — nên gen tốt ở môi trường xấu **mất**
    // nhiều hơn gen xấu ở môi trường xấu.
    let lech_g = g - 500;
    let lech_e = e - 500;
    let tuong_tac = lech_g * lech_e / 1_000 * i64::from(t.gxe) / 1_000;

    // Chỉ trait gắn sức sống mới chịu cận huyết.
    let can_huyet = if t.fitness_linked {
        i64::from(inbreeding_depression(inbreeding, population_load))
    } else {
        0
    };

    u16::try_from((cong_gop + tuong_tac - can_huyet + i64::from(noise)).clamp(0, 1_000))
        .unwrap_or(0)
}

/// Một quần thể, nhìn từ góc độ di truyền.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Population {
    /// Số cá thể sinh sản được.
    pub effective_size: u32,
    /// Gánh nặng alen lặn có hại, `0`–`1000`.
    pub deleterious_load: u16,
}

impl Population {
    /// Hệ số cận huyết **tăng thêm mỗi thế hệ** trong một quần thể nhỏ.
    ///
    /// `ΔF ≈ 1 / (2·Nₑ)`, quy về thang phần nghìn. Đây là chỗ "nút thắt di
    /// truyền" trở thành một con số: một quần thể còn 10 con tích cận huyết
    /// nhanh gấp 50 lần một quần thể 500 con.
    pub fn inbreeding_per_generation(&self) -> u16 {
        if self.effective_size == 0 {
            return 1_000;
        }
        u16::try_from((1_000 / (2 * i64::from(self.effective_size))).clamp(0, 1_000))
            .unwrap_or(1_000)
    }

    /// Quần thể này đã qua **nút thắt** chưa.
    ///
    /// Dưới ngưỡng thì cận huyết tích nhanh hơn chọn lọc kịp loại bỏ, và quần
    /// thể **mắc kẹt**: thêm con không cứu được, vì cái mất là đa dạng chứ không
    /// phải số lượng.
    pub fn bottlenecked(&self, threshold: u32) -> bool {
        self.effective_size < threshold
    }

    /// Cho quần thể chạy qua một thế hệ khép kín.
    pub fn advance_closed(&mut self) {
        // Cận huyết tích lũy đẩy gánh nặng lên: alen lặn có hại lộ ra và ở lại.
        let them = self.inbreeding_per_generation();
        self.deleterious_load = self.deleterious_load.saturating_add(them).min(1_000);
    }
}
