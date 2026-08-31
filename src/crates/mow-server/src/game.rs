//! Thế giới chơi được: địa hình từ `Worldgen`, thực thể từ `Sim`.
//!
//! ## Vì sao module này tồn tại
//!
//! `mow-scenario::slice` đã chứng minh **vòng lặp chơi** khép kín — đi, nhặt,
//! ăn, nói, quan sát theo tri giác, True God preview/commit — nhưng nó chứng
//! minh bằng test. Không ai nhìn thấy gì.
//!
//! Ở giữa engine và màn hình còn thiếu đúng một thứ: một chỗ **giữ thế giới
//! sống** và trả lời được ba câu hỏi mà một renderer hỏi mỗi khung hình:
//!
//! 1. Ô `(x, y, z)` này là gì?
//! 2. Ai đang đứng ở đâu?
//! 3. Từ lần trước tới giờ có gì xảy ra?
//!
//! ## Địa hình không nằm trong state
//!
//! Câu 1 trả lời bằng [`mow_worldgen`], **không** bằng dữ liệu đã lưu. Địa hình
//! là hàm thuần của seed (`§7.2`), nên client hỏi một lần rồi cache vĩnh viễn;
//! chỉ những ô người chơi đã đào mới nằm trong `ChunkStore` và mới cần đồng bộ.
//!
//! Đây là khác biệt giữa "gửi vài KB lúc mở bản đồ" và "gửi vài MB mỗi lần
//! kéo camera".
//!
//! ## NPC ở đây là chỗ giữ chỗ, và nói thẳng ra
//!
//! [`Game::tick`] cho NPC một chính sách nhỏ, xác định, tính từ `(tick, id)`.
//! Nó **không** phải `agent-service`, và nó không giả vờ là. Nó tồn tại để màn
//! hình có thứ chuyển động trong lúc tầng nhận thức được nối vào — và vì một
//! NPC đứng im làm người ta không phát hiện được lỗi đồng bộ nào cả.
//!
//! Quan trọng: nó đi qua **đúng đường ghi** như người chơi (`Sim::apply`), nên
//! khi thay bằng LLM thì không có đường nào phải mở thêm (`§22.1`).

use mow_core::{Command, CommandResult, EntityId, EventSeq, Sim, Tick, Value, WorldId};
use mow_math::{StateHash, WorldSeed};
use mow_scenario::slice::{self, WORLD};
use mow_worldgen::strata::material_at;
use mow_worldgen::{BaseCell, GenerationProfile, Worldgen};

/// Bán kính vùng người chơi thấy được, tính bằng ô.
pub const TAM_NHIN: i64 = 24;

/// Thế giới đang chạy.
pub struct Game {
    sim: Sim,
    gen: Worldgen,
    seed: u64,
    avatar: EntityId,
    /// Lát `z` mà người chơi đang xem. Trạng thái **giao diện**, không phải
    /// trạng thái thế giới — nên nó ở đây chứ không đi qua một command.
    z: i64,
}

/// Một ô đã giải xong, đủ để vẽ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    /// Vật liệu tại lát `z` đang xem.
    pub material: &'static str,
    /// Vật liệu của ô rắn trên cùng của cột này.
    ///
    /// Tồn tại vì một lát `z` thuần túy cho ra một bản đồ **đen kịt**: đứng ở
    /// cao độ 85 m thì mọi ô thấp hơn 85 m đều là không khí, và màn hình đầu
    /// tiên của người chơi là một khoảng trống có ba chấm màu.
    ///
    /// `§18.1` đã lường trước: "có thể ghost 1–3 lớp trên/dưới với opacity
    /// thấp". Đây là dữ liệu cho việc đó — client vẽ mặt đất bên dưới, mờ dần
    /// theo `drop`, nên người chơi thấy địa hình mà vẫn biết mình đang ở lát nào.
    pub surface: &'static str,
    /// Số mét từ lát đang xem xuống tới mặt đất. `0` khi đang đứng ngay trên nó.
    pub drop: i64,
    /// Quần xã của cột này.
    pub biome: &'static str,
    /// Cao độ mặt đất, mét.
    pub height: i64,
    /// Ô này có phải lòng sông không.
    pub river: bool,
    /// Người chơi đã sửa ô này chưa (`ChunkStore` có delta).
    pub edited: bool,
}

impl Game {
    /// Dựng thế giới mới từ seed.
    pub fn new(seed: u64) -> Game {
        let sim = slice::build_slice_world(seed);
        let gen = Worldgen::new(WorldSeed(seed), GenerationProfile::default());
        let avatar = sim
            .store()
            .ids()
            .find(|id| {
                sim.store()
                    .attr_text(*id, "core.name")
                    .is_some_and(|n| n == "Nguoi Choi")
            })
            .unwrap_or(EntityId::new(1));

        // `build_slice_world` đặt mọi thứ ở `(0, 0)` vì test không quan tâm
        // địa hình. Màn hình thì có: với seed 42, ô `(0, 0)` nằm **dưới mực
        // biển**, nên người chơi mở game ra và thấy mình ở đáy biển.
        let (sx, sy) = tim_cho_o_duoc(&gen);
        let mut g = Game {
            sim,
            gen,
            seed,
            avatar,
            z: 0,
        };
        g.doi_cho_o(sx, sy);

        // Lát bắt đầu là mặt đất dưới chân avatar, không phải `z = 0`. Nếu
        // avatar đứng trên một ngọn đồi cao 300 m thì lát 0 nằm sâu trong đá và
        // màn hình đầu tiên là một khối đen đặc — trông y hệt một lỗi renderer,
        // và người ta sẽ đi sửa renderer.
        g.z = g
            .gen
            .base_cell(sx, sy)
            .map(|c| c.elevation.height_m)
            .unwrap_or(0);
        g
    }

    /// Dời avatar, người đồng hành và ổ bánh về quanh `(sx, sy)`.
    ///
    /// Đi qua `core.set_attr`, tức là đúng đường ghi của mọi thứ khác. Sửa
    /// thẳng `Store` sẽ nhanh hơn và sẽ là lần đầu tiên có một thay đổi không
    /// nằm trong event log (`§22.1`).
    fn doi_cho_o(&mut self, sx: i64, sy: i64) {
        let ids: Vec<EntityId> = self.sim.store().with_attr("core.pos.x").collect();
        // Giữ nguyên bố cục tương đối mà lát cắt đã dựng: người chơi ở giữa,
        // bạn đồng hành cách 3 ô, ổ bánh cách 1 ô. Dời từng cái về một chỗ sẽ
        // làm ba thực thể chồng lên nhau.
        for id in ids {
            let x = self.attr_int(id, "core.pos.x") + sx;
            let y = self.attr_int(id, "core.pos.y") + sy;
            self.dat(id, "core.pos.x", Value::Int(x));
            self.dat(id, "core.pos.y", Value::Int(y));
        }
    }

    fn dat(&mut self, id: EntityId, key: &str, v: Value) {
        let cmd = Command::new(
            "core.set_attr",
            WORLD,
            Value::Map(
                [
                    ("entity".to_owned(), Value::Uint(id.get())),
                    ("key".to_owned(), Value::Text(key.to_owned())),
                    ("value".to_owned(), v),
                ]
                .into_iter()
                .collect(),
            ),
        );
        self.sim.apply(&cmd).expect("đặt thuộc tính");
    }

    /// Seed đã dựng thế giới này.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Avatar của người chơi.
    pub fn avatar(&self) -> EntityId {
        self.avatar
    }

    /// Lát `z` đang xem.
    pub fn z(&self) -> i64 {
        self.z
    }

    /// Đổi lát đang xem. Không phải command: nó không đổi thế giới (`§P6.8`).
    pub fn set_z(&mut self, z: i64) {
        self.z = z;
    }

    /// Tick hiện tại.
    pub fn tick(&self) -> Tick {
        self.sim.clock().local()
    }

    /// Hash trạng thái.
    pub fn state_hash(&self) -> StateHash {
        self.sim.state_hash()
    }

    /// Thế giới.
    pub fn world(&self) -> WorldId {
        WORLD
    }

    /// Truy cập chỉ đọc cho tầng API.
    pub fn sim(&self) -> &Sim {
        &self.sim
    }

    /// Ô địa hình tại `(x, y)` ở lát `z` đang xem.
    pub fn tile(&self, x: i64, y: i64) -> Tile {
        let c: BaseCell = self.gen.base_cell(x, y).unwrap_or_else(|_| BaseCell {
            elevation: mow_worldgen::Elevation {
                height_m: 0,
                slope: 0,
                submerged: false,
            },
            climate: self.gen.base_cell(0, 0).map_or_else(
                |_| unreachable!("ô gốc luôn sinh được"),
                |b| b.climate,
            ),
            flow: mow_worldgen::Flow {
                dx: 0,
                dy: 0,
                accumulation: 0,
                is_river: false,
                is_water_body: false,
            },
            strata: mow_worldgen::Strata {
                surface: mow_worldgen::Material::Igneous,
                soil_depth_m: 0,
                bedrock_depth_m: 0,
                ore_present: false,
                cave: false,
            },
            biome: mow_worldgen::Biome::Alpine,
        });

        let sea = self.gen.profile().sea_level_m;
        // Mặt nhìn thấy được: mặt nước nếu ô chìm, mặt đất nếu không.
        let dinh = if c.elevation.submerged {
            sea
        } else {
            c.elevation.height_m
        };
        Tile {
            material: material_at(&c.elevation, &c.strata, sea, self.z).as_str(),
            surface: material_at(&c.elevation, &c.strata, sea, dinh).as_str(),
            drop: (self.z - dinh).max(0),
            biome: c.biome.as_str(),
            height: c.elevation.height_m,
            river: c.flow.is_river,
            edited: false,
        }
    }

    /// Áp một lệnh. Đây là **đường ghi duy nhất** (`§22.1`).
    pub fn apply(&mut self, cmd: &Command) -> CommandResult<()> {
        self.sim.apply(cmd).map(|_| ())
    }

    /// Sự kiện sau `after`.
    pub fn events_after(&self, after: EventSeq) -> Vec<&mow_core::Event> {
        self.sim
            .log()
            .iter()
            .filter(|e| e.seq.0 > after.0)
            .collect()
    }

    /// Sự kiện mới nhất.
    pub fn last_seq(&self) -> EventSeq {
        self.sim.log().next_seq()
    }

    /// Mọi thực thể có vị trí.
    pub fn placed(&self) -> Vec<EntityId> {
        self.sim
            .store()
            .with_attr("core.pos.x")
            .filter(|id| {
                // Vật phẩm nằm trong túi ai đó vẫn giữ tọa độ cũ. Vẽ chúng lên
                // bản đồ nghĩa là ổ bánh vẫn nằm dưới đất sau khi đã bị nhặt —
                // một lỗi mà người chơi phát hiện ngay còn test thì không.
                self.sim.store().attr_entity(*id, "loc.inventory").is_none()
            })
            .collect()
    }

    /// Một bước cho thế giới, kèm chính sách NPC tạm.
    ///
    /// Xem tài liệu module: NPC ở đây là chỗ giữ chỗ và đi qua đúng đường ghi
    /// của người chơi.
    pub fn tick_once(&mut self) {
        let t = self.sim.clock().local().0;
        // Một bước mỗi 4 tick: đủ chậm để mắt theo kịp, đủ nhanh để thấy là
        // thế giới đang sống.
        if t % 4 != 0 {
            let _ = self.sim.advance(1);
            return;
        }

        let ax = self.attr_int(self.avatar, "core.pos.x");
        let ay = self.attr_int(self.avatar, "core.pos.y");

        let npcs: Vec<EntityId> = self
            .sim
            .store()
            .with_attr("core.pos.x")
            .filter(|id| *id != self.avatar)
            .filter(|id| self.sim.store().attr_text(*id, "core.name").is_some())
            .filter(|id| self.sim.store().attr_int(*id, "item.nutrition").is_none())
            .collect();

        for npc in npcs {
            let (dx, dy) = self.buoc_npc(npc, t, ax, ay);
            if dx == 0 && dy == 0 {
                continue;
            }
            let cmd = Command::new(
                "core.walk",
                WORLD,
                Value::Map(
                    [
                        ("who".to_owned(), Value::Uint(npc.get())),
                        ("dx".to_owned(), Value::Int(dx)),
                        ("dy".to_owned(), Value::Int(dy)),
                    ]
                    .into_iter()
                    .collect(),
                ),
            );
            // Bước bị từ chối là chuyện bình thường (ra ngoài tầm, ô bị chặn).
            // Nuốt lỗi ở đây là đúng: NPC thử, không được thì thôi — nhưng
            // **không** nuốt bằng cách bỏ qua `Result`, vì thế thì một lỗi thật
            // cũng biến mất.
            if let Err(e) = self.sim.apply(&cmd) {
                debug_assert!(
                    matches!(e.code, mow_core::FailureCode::PreconditionFailed),
                    "bước NPC hỏng vì lý do ngoài dự kiến: {e:?}"
                );
            }
        }

        let _ = self.sim.advance(1);
    }

    fn attr_int(&self, id: EntityId, k: &str) -> i64 {
        self.sim.store().attr_int(id, k).unwrap_or(0)
    }

    /// Bước kế tiếp của một NPC. Hàm thuần của `(id, tick, vị trí avatar)`.
    ///
    /// Xác định là bắt buộc, không phải tùy chọn: hai lần chạy từ cùng seed
    /// phải cho cùng thế giới (`§P7.5`), và một `rand::random()` ở đây phá điều
    /// đó mà không có bài test nào đỏ.
    fn buoc_npc(&self, npc: EntityId, t: u64, ax: i64, ay: i64) -> (i64, i64) {
        let nx = self.attr_int(npc, "core.pos.x");
        let ny = self.attr_int(npc, "core.pos.y");
        let d = (nx - ax).abs() + (ny - ay).abs();

        // Gần người chơi thì lảng vảng quanh đó; xa thì đi về phía người chơi.
        // Không phải AI, chỉ là đủ để thế giới không đứng hình.
        if d > 6 {
            return ((ax - nx).signum(), (ay - ny).signum());
        }
        let h = npc.get().wrapping_mul(6_364_136_223_846_793_005).wrapping_add(t);
        match h % 5 {
            0 => (1, 0),
            1 => (-1, 0),
            2 => (0, 1),
            3 => (0, -1),
            _ => (0, 0),
        }
    }
}

/// Tìm ô đất khô gần gốc tọa độ nhất.
///
/// ## Bước nhảy phải khớp thang của trường, không phải khớp trực giác
///
/// Hai bản trước đều sai theo cùng một kiểu, và cái sai đáng ghi lại.
///
/// Bản đầu quét từng ô, bán kính 400: bộ test chạy **197 giây**. Bản thứ hai
/// nhảy 6 ô, bán kính 540: nhanh, nhưng seed 1234 vẫn sinh người chơi xuống
/// biển — vì `continental_cell` là **4096 ô**, nên bán kính 540 chỉ dò được
/// 13% của một ô lục địa. Ở giữa Thái Bình Dương thì quét kỹ hơn không giúp gì;
/// phải quét **xa hơn**.
///
/// Nên tìm ba mức, mỗi mức khớp một tầng của trường độ cao:
///
/// | Mức | Bước | Bán kính | Trả lời câu hỏi |
/// |---|---|---|---|
/// | thô | 256 | 12288 (3 ô lục địa) | lục địa nằm hướng nào |
/// | vừa | 16 | 256 | bờ biển ở đâu |
/// | tinh | 1 | 16 | ô khô nào gần nhất |
///
/// Tổng số lần lấy mẫu khoảng 10 nghìn thay vì 600 nghìn, và nó phủ một vùng
/// rộng gấp 30 lần.
fn tim_cho_o_duoc(gen: &Worldgen) -> (i64, i64) {
    let kho_rao = |x: i64, y: i64| {
        gen.base_cell(x, y).is_ok_and(|c| {
            !c.elevation.submerged
                && !c.flow.is_water_body
                && c.strata.surface != mow_worldgen::Material::Water
        })
    };
    if kho_rao(0, 0) {
        return (0, 0);
    }

    let mut tam = (0i64, 0i64);
    for (buoc, ban_kinh) in [(256i64, 12_288i64), (16, 256), (1, 16)] {
        match quet_vong(tam, buoc, ban_kinh, &kho_rao) {
            Some(p) => tam = p,
            // Mức thô không thấy gì nghĩa là quanh đây thật sự toàn nước. Trả
            // về chỗ tốt nhất tìm được thay vì quét mãi: một hành tinh đại
            // dương là một thế giới hợp lệ, và treo lúc khởi động thì không.
            None => return tam,
        }
    }
    tam
}

/// Quét các vòng vuông quanh `tam`, bước `buoc`, tới `ban_kinh`.
///
/// Trả về ô khô gần `tam` nhất trong lưới đó, hoặc `None`.
fn quet_vong(
    tam: (i64, i64),
    buoc: i64,
    ban_kinh: i64,
    kho_rao: &impl Fn(i64, i64) -> bool,
) -> Option<(i64, i64)> {
    let mut r = 0;
    while r <= ban_kinh {
        let mut ung_vien: Vec<(i64, i64)> = Vec::new();
        if r == 0 {
            ung_vien.push(tam);
        } else {
            let mut d = -r;
            while d <= r {
                ung_vien.push((tam.0 + d, tam.1 - r));
                ung_vien.push((tam.0 + d, tam.1 + r));
                ung_vien.push((tam.0 - r, tam.1 + d));
                ung_vien.push((tam.0 + r, tam.1 + d));
                d += buoc;
            }
        }
        // Thứ tự cố định: cùng seed phải chọn cùng chỗ, nếu không thì "cùng
        // seed cho cùng thế giới" hỏng ngay ở ô đầu tiên.
        ung_vien.sort_unstable();
        ung_vien.dedup();
        if let Some(p) = ung_vien.into_iter().find(|(x, y)| kho_rao(*x, *y)) {
            return Some(p);
        }
        r += buoc;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dung_duoc_va_co_avatar() {
        let g = Game::new(42);
        assert!(g.sim().store().contains(g.avatar()));
        assert_eq!(
            g.sim().store().attr_text(g.avatar(), "core.name"),
            Some("Nguoi Choi")
        );
    }

    fn vi_tri_avatar(g: &Game) -> (i64, i64) {
        (
            g.attr_int(g.avatar(), "core.pos.x"),
            g.attr_int(g.avatar(), "core.pos.y"),
        )
    }

    #[test]
    fn lat_bat_dau_la_mat_dat_duoi_chan_avatar() {
        // Không phải `z = 0`: nếu avatar ở trên một ngọn đồi thì lát 0 nằm
        // trong đá và màn hình đầu tiên đen đặc.
        let g = Game::new(42);
        let (ax, ay) = vi_tri_avatar(&g);
        let t = g.tile(ax, ay);
        assert_eq!(g.z(), t.height);
        assert_ne!(t.material, "air", "ô dưới chân phải là chất rắn");
    }

    #[test]
    fn avatar_khong_sinh_ra_duoi_bien() {
        // `build_slice_world` đặt mọi thứ ở `(0, 0)` vì test không quan tâm địa
        // hình. Với seed 42, ô đó nằm dưới mực biển — người chơi mở game ra và
        // thấy mình ở đáy biển. Bài này giữ cho chuyện đó không quay lại, ở
        // nhiều seed chứ không chỉ seed may mắn.
        for seed in [1u64, 7, 42, 1234, 99_999] {
            let g = Game::new(seed);
            let (ax, ay) = vi_tri_avatar(&g);
            let t = g.tile(ax, ay);
            assert_ne!(
                t.material, "water",
                "seed {seed}: avatar sinh ra dưới nước ở ({ax},{ay}),                  cách gốc {} ô",
                ax.abs() + ay.abs()
            );
        }
    }

    #[test]
    fn ba_thuc_the_khong_chong_len_nhau_sau_khi_doi_cho() {
        let g = Game::new(42);
        let mut cho: Vec<(i64, i64)> = g
            .placed()
            .into_iter()
            .map(|id| (g.attr_int(id, "core.pos.x"), g.attr_int(id, "core.pos.y")))
            .collect();
        let truoc = cho.len();
        cho.sort_unstable();
        cho.dedup();
        assert_eq!(truoc, cho.len(), "dời chỗ ở đã dồn mọi thứ vào một ô");
    }

    #[test]
    fn dia_hinh_xac_dinh_theo_seed() {
        let a = Game::new(7);
        let b = Game::new(7);
        for (x, y) in [(0, 0), (5, -3), (-40, 17), (1_000_000, -2_000_000)] {
            assert_eq!(a.tile(x, y), b.tile(x, y), "ô ({x},{y}) lệch giữa hai lần");
        }
    }

    #[test]
    fn seed_khac_thi_the_gioi_khac() {
        let a = Game::new(1);
        let b = Game::new(2);
        let khac = (-30..30)
            .flat_map(|x| (-30..30).map(move |y| (x, y)))
            .filter(|(x, y)| a.tile(*x, *y) != b.tile(*x, *y))
            .count();
        assert!(khac > 100, "hai seed chỉ khác {khac} ô — worldgen bỏ qua seed?");
    }

    #[test]
    fn doi_lat_z_doi_vat_lieu() {
        let g = Game::new(42);
        let (ax, ay) = vi_tri_avatar(&g);
        let mut g2 = Game::new(42);

        g2.set_z(g.z() + 5);
        assert_eq!(g2.tile(ax, ay).material, "air", "5 m trên đầu phải là không khí");

        g2.set_z(g.z() - 60);
        assert_ne!(g2.tile(ax, ay).material, "air", "sâu 60 m phải là đá");
    }

    #[test]
    fn tick_lam_the_gioi_doi() {
        let mut g = Game::new(42);
        let h0 = g.state_hash();
        for _ in 0..8 {
            g.tick_once();
        }
        assert_ne!(h0, g.state_hash(), "8 tick mà thế giới không đổi gì");
        assert!(g.tick().0 >= 8);
    }

    #[test]
    fn tick_xac_dinh() {
        // Cùng seed, cùng số tick ⇒ cùng hash. Đây là điều kiện mà một
        // `rand::random()` trong chính sách NPC sẽ phá mà không ai thấy.
        let mut a = Game::new(9);
        let mut b = Game::new(9);
        for _ in 0..40 {
            a.tick_once();
            b.tick_once();
        }
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn vat_pham_trong_tui_khong_con_tren_ban_do() {
        let mut g = Game::new(42);
        let banh = g
            .sim()
            .store()
            .with_attr("item.nutrition")
            .next()
            .expect("thế giới lát cắt có một ổ bánh");
        assert!(g.placed().contains(&banh), "bánh phải đang nằm trên đất");

        let who = g.avatar();
        // Đi tới chỗ bánh rồi nhặt.
        let bx = g.attr_int(banh, "core.pos.x");
        let by = g.attr_int(banh, "core.pos.y");
        while g.attr_int(who, "core.pos.x") != bx || g.attr_int(who, "core.pos.y") != by {
            let dx = (bx - g.attr_int(who, "core.pos.x")).signum();
            let dy = (by - g.attr_int(who, "core.pos.y")).signum();
            g.apply(&Command::new(
                "core.walk",
                WORLD,
                Value::Map(
                    [
                        ("who".to_owned(), Value::Uint(who.get())),
                        ("dx".to_owned(), Value::Int(dx)),
                        ("dy".to_owned(), Value::Int(dy)),
                    ]
                    .into_iter()
                    .collect(),
                ),
            ))
            .expect("đi được");
        }
        g.apply(&Command::new(
            "core.take",
            WORLD,
            Value::Map(
                [
                    ("who".to_owned(), Value::Uint(who.get())),
                    ("what".to_owned(), Value::Uint(banh.get())),
                ]
                .into_iter()
                .collect(),
            ),
        ))
        .expect("nhặt được");

        assert!(
            !g.placed().contains(&banh),
            "nhặt rồi mà ổ bánh vẫn nằm dưới đất"
        );
    }
}
