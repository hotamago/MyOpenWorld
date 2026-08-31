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

use crate::preview::{compare, snapshot, Diff, JournalEntry, Refusal};
use mow_content::{load_pack, PackContent};
use mow_core::{
    BranchId, Clock, Command, CommandResult, EntityId, EventDraft, EventSeq, Sim, SimConfig, Tick,
    Value, WorldId,
};
use mow_econ::village::{run_day, tools_sufficient, DayReport, Shortage, Stock, Workforce};
use mow_math::{StateHash, StateHasher, WorldSeed};
use mow_scenario::slice::{self, WORLD};
use mow_settle::{Role as SettleRole, SettleRequest};
use mow_society::routine::{decide, Intent, Place, Role, Situation, HUNGER_STARVING};
use mow_spatial::pathfind::{find_path, PathOutcome, PathRequest};
use mow_worldgen::strata::material_at;
use mow_worldgen::{BaseCell, GenerationProfile, Worldgen};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

/// Bán kính vùng người chơi thấy được, tính bằng ô.
pub const TAM_NHIN: i64 = 24;

/// Thế giới đang chạy.
pub struct Game {
    sim: Sim,
    gen: Worldgen,
    seed: u64,
    /// Điểm mà cái nhìn của vị thần đang đặt vào, tính bằng ô.
    ///
    /// **Không** phải một thực thể. Người chơi là một true god: không thân xác,
    /// không tọa độ trong thế giới, không đi lại được ai chặn. Bản đầu cho thần
    /// một avatar tên "Nguoi Choi" đứng giữa làng — và câu hỏi đầu tiên người
    /// chơi hỏi là *"tại sao mặc định true god lại có cơ thể?"*.
    ///
    /// Vì nó là **cái nhìn** chứ không phải thế giới, nó không đi vào
    /// `state_hash` và đổi nó không sinh sự kiện nào (`§P6.8`: camera là một
    /// truy vấn khung nhìn, không phải một lệnh).
    eye: (i64, i64),
    /// Lát `z` mà người chơi đang xem. Trạng thái **giao diện**, không phải
    /// trạng thái thế giới — nên nó ở đây chứ không đi qua một command.
    z: i64,
    /// Tốc độ thời gian, tính theo **phần nghìn** của tốc độ chuẩn.
    ///
    /// Số nguyên chứ không phải số thực, cùng lý do với `temperature_milli`:
    /// tốc độ đi vào event log khi nó đổi (`§8.4`), và một `f32` trong event
    /// log là một giá trị có thể tuần tự hóa khác nhau giữa hai bản build
    /// (`§P10.2.1`).
    ///
    /// `1` là ×0.001, `1_000` là ×1, `100_000` là ×100. `0` là tạm dừng.
    ///
    /// Tốc độ **không** đổi kết quả mô phỏng — nó chỉ đổi tốc độ thời gian thật
    /// trôi giữa hai tick. Cùng seed chạy ×0.001 hay ×100 vẫn cho cùng
    /// `state_hash` tại cùng số tick, và có một bài test giữ điều đó.
    speed_milli: u32,
    /// Đường đi còn lại của từng thực thể, ô một.
    ///
    /// Kế hoạch nằm ở **server**, không ở client, vì hai lý do. Thứ nhất, client
    /// gửi từng bước một thì mỗi bước tốn một vòng mạng và nhân vật đi giật.
    /// Thứ hai và quan trọng hơn: một kế hoạch ở client là một kế hoạch không
    /// đi qua `Sim::apply`, tức là một đường ghi thứ hai (`§22.1`).
    ///
    /// Mỗi bước vẫn là một `core.walk` riêng, nên luật thế giới vẫn chặn được
    /// từng bước — nhân vật đi vào chỗ vừa sập thì bước đó hỏng và kế hoạch
    /// dừng, chứ không đi xuyên qua.
    plans: BTreeMap<EntityId, VecDeque<(i64, i64)>>,
    /// Sự kiện đã sinh ra kế hoạch đi của mỗi thực thể.
    ///
    /// `§18.10`: cạnh nhân quả phải ghi **lúc tạo** sự kiện. Bước đi thứ mười
    /// của một cư dân không tự giải thích được nó; thứ giải thích nó là cái ý
    /// định đã đặt ra đích. Giữ số thứ tự đó ở đây để mỗi bước gắn ngược về
    /// đúng nguồn.
    plan_cause: BTreeMap<EntityId, EventSeq>,
    /// Địa điểm chung của khu định cư: giếng và quảng trường.
    ///
    /// Chúng là **nơi chốn**, không phải thực thể: một cái giếng không đi lại,
    /// không có nhu cầu, và biến nó thành entity chỉ để có tọa độ sẽ làm mọi
    /// vòng lặp qua thực thể phải học cách bỏ qua nó.
    landmarks: BTreeMap<Place, (i64, i64)>,
    /// Nhớ tạm địa hình gốc: `(x, y)` → cả ô worldgen.
    ///
    /// `Worldgen::base_cell` chạy cả chuỗi tầng nhiễu cho mỗi ô, và **bốn** chỗ
    /// hỏi nó nhiều nhất đều hỏi cùng những ô đó nhiều lần: tìm đường (A* xét
    /// mỗi ô tới tám lần), quy hoạch làng, bộ chấm điểm đất bằng, và
    /// [`Game::tile`] — thứ mà mỗi lô ô gọi hàng nghìn lần.
    ///
    /// Bản đầu chỉ nhớ `(cao độ, có nước không)`, đủ cho ba chỗ trên và **không**
    /// đủ cho `tile`. Hậu quả đo được: bản đồ thu nhỏ hỏi 128×128 = 16.384 ô mỗi
    /// 2,5 giây, mỗi ô chạy lại toàn bộ worldgen **trong khi đang giữ khóa thế
    /// giới** — và luồng tick tụt từ 4.480 xuống 120 nhịp mỗi giây ngay khi có
    /// một trình duyệt nối vào. `§P6.8` quy tắc 2 nói luồng tick không bao giờ
    /// chờ client; điều đó chỉ đúng khi việc phục vụ client cũng rẻ.
    ///
    /// Không có bảng nhớ này, riêng việc chọn chỗ đặt làng đã mất bảy mươi giây.
    ///
    /// `RefCell` chứ không phải `&mut`: đây là nhớ tạm, không phải state. Nó
    /// **không** đi vào `state_hash`, và xóa nó đi không đổi một hạt nào của
    /// thế giới — nó chỉ là một cách hỏi lại nhanh hơn.
    terrain: RefCell<BTreeMap<(i64, i64), BaseCell>>,
    /// Ruộng: `(x, y)` → pha sinh trưởng `0..CROP_STAGES.len()`.
    ///
    /// Ruộng đứng im quanh năm là thứ nói cho người chơi biết thế giới này chỉ
    /// giả vờ sống. Một mảnh ruộng đi qua đất trống → mầm → xanh → chín → bị
    /// gặt là **cùng một cơ chế** với kinh tế đã có: ngày gặt là ngày kho lương
    /// được cộng thêm. Nhìn thấy ruộng chín là nhìn thấy trước ngày mai no đủ.
    crops: BTreeMap<(i64, i64), u8>,
    /// Vết chân đã tích trên mỗi ô: `(x, y)` → độ mòn `0..=TRAMPLE_MAX`.
    ///
    /// Đây là thứ rẻ nhất biến một bản đồ đúng thành một bản đồ **có người
    /// sống**. Không ai vẽ con đường từ nhà thợ rèn ra giếng; nó hiện ra vì
    /// người ta đi qua đó mỗi ngày, và nó mờ đi ở nơi không còn ai đi. Một
    /// người chơi quay lại sau ba mươi ngày đọc được ngay làng này bận rộn ở
    /// đâu — mà không cần một bảng số nào.
    ///
    /// Là **sự thật về thế giới**, nên nó vào `state_hash`: hai thế giới cùng
    /// seed cùng số tick phải mòn giống hệt nhau.
    trample: BTreeMap<(i64, i64), u32>,
    /// Ô đã bị ghi đè: `(x, y)` → vật liệu.
    ///
    /// Địa hình là hàm thuần của seed (`§7.2`), nên **chỉ** những ô ai đó đã
    /// sửa mới cần lưu. Đây đúng là mô hình `seed + delta` của `§P3.4`: một
    /// ngôi làng là vài trăm ô, không phải một bản đồ.
    ///
    /// Chỉ lưu theo `(x, y)` chứ không theo `(x, y, z)`: ở lát cắt hiện tại,
    /// công trình nằm trên mặt đất. Thêm `z` lúc chưa có tầng hầm sẽ là ba lần
    /// tra cứu cho một thông tin không ai dùng.
    overrides: BTreeMap<(i64, i64), String>,
    /// Vật liệu, vật phẩm và sự kiện của content pack đang nạp.
    ///
    /// Server **phát bảng này cho client** thay vì để client giữ một bản chép
    /// tay. Đó là toàn bộ điểm của `§19.7`: thêm một thư mục
    /// `content/<pack>/blocks/<id>/` là có vật liệu mới ở cả hai đầu, không sửa
    /// một dòng mã nào.
    ///
    /// `None` khi không nạp được. Không phải lỗi chí mạng: client có bảng dự
    /// phòng, và một thế giới vẽ bằng màu dự phòng vẫn tốt hơn một màn hình
    /// trắng kèm stack trace.
    content: Option<PackContent>,
    /// Thư mục content pack đang nạp, để khởi nguyên lại còn biết nạp lại gì.
    ///
    /// Content **không** thuộc về seed: nó là dữ liệu bên ngoài mà tiến trình
    /// được khởi động cùng. Một thế giới mới vẫn phải biết những vật liệu ấy,
    /// nếu không thì bấm "khởi nguyên" xong bản đồ hiện toàn màu dự phòng.
    content_dir: Option<String>,
    /// Mọi lệnh đã áp, theo thứ tự.
    ///
    /// Đây là thứ làm preview đúng: dựng lại thế giới từ seed rồi phát lại nhật
    /// ký cho ra **đúng** thế giới đang xem, chứ không phải một thế giới gần
    /// giống. Xem tài liệu của `preview`.
    ///
    /// Nó cũng là một bài kiểm liên tục cho `§8.4`: nếu phát lại nhật ký không
    /// ra cùng `state_hash`, nghĩa là có gì đó đã đổi thế giới ngoài đường
    /// ghi — và preview sẽ nói ra điều đó thay vì giấu đi.
    journal: Vec<JournalEntry>,
    /// Kho của làng: bốn vòng kinh tế từ `mow_econ::village`, đã kiểm chứng
    /// sẵn ở đó bằng 18 test.
    ///
    /// Nằm ở `Game`, không phải một biến bên lề: đây chính là điều nhiệm vụ
    /// này tồn tại để sửa. Một kho không phải state — không đi vào
    /// `state_hash`, không sống sót qua save/load, không ai chứng minh được
    /// nó không lệch giữa hai bản chạy — thì không khác gì không có kho. Xem
    /// `Game::mix` để biết kho được trộn vào hash thế nào, và
    /// `run_economy_day` để biết nó được cập nhật khi nào.
    stock: Stock,
    /// Lực lượng lao động, suy ra từ cư dân **thật** (đếm theo `npc.role`),
    /// không phải một hằng số hay một con số nhớ tạm cho khớp bản thiết kế.
    ///
    /// Đo lại mỗi ngày kinh tế (xem `compute_workforce`), nên cư dân chết hay
    /// đổi vai thì con số này đổi theo ở đúng ranh giới ngày kế tiếp — không
    /// tức thời, vì `mow_econ::village::run_day` cũng chỉ nhận một
    /// `Workforce` cho *cả* một ngày, không phải cho từng tick.
    work: Workforce,
    /// Báo cáo của ngày kinh tế gần nhất, để lớp gọi (API, sau này) đọc lại
    /// mà không phải đợi đúng khoảnh khắc ranh giới ngày.
    ///
    /// **Không** đi vào `state_hash`: nó là một bản chụp lại của `stock` và
    /// `work` tại một mốc đã qua, suy ra được hoàn toàn từ hai trường đó cộng
    /// lịch sử tick, giống hệt lý do `terrain` (bảng nhớ tạm địa hình) đứng
    /// ngoài — đưa nó vào hash chỉ ghi trùng một sự thật đã có ở nơi khác.
    last_day_report: Option<DayReport>,
}

/// Trần độ dài nhật ký cho một lần preview.
///
/// Phát lại là O(số lệnh), nên một phiên chơi dài sẽ làm mỗi lần xem trước tốn
/// hàng giây. Vượt trần thì nói thẳng là cần snapshot, thay vì để người chơi
/// ngồi nhìn một nút không phản hồi và đoán xem nó hỏng hay đang nghĩ.
pub const PREVIEW_JOURNAL_LIMIT: usize = 20_000;

/// Trần số ô A* được mở cho một lần bấm chuột.
///
/// Thế giới vô hạn (`§7.1`), nên không có trần thì một ô đích không tới được sẽ
/// quét mãi và treo luồng đang giữ khóa. 20 nghìn ô đủ để đi vòng qua một vịnh
/// lớn mà vẫn trả lời trong vài mili giây.
pub const PATH_BUDGET: usize = 20_000;

/// Trần tốc độ: ×100.
pub const MAX_SPEED_MILLI: u32 = 100_000;

/// Số tick tối đa chạy trong một lần thức dậy của luồng tick.
///
/// Ở ×100 một nhịp 50 ms đáng lẽ chạy ~17 tick; trần này chặn trường hợp máy
/// vừa ngủ dậy hoặc luồng bị treo lâu rồi bỗng phải chạy bù hàng nghìn tick
/// **trong lúc đang giữ khóa** — mọi yêu cầu HTTP sẽ đứng, và trò chơi trông
/// như bị treo đúng lúc người chơi tăng tốc.
pub const MAX_TICKS_PER_WAKE: u32 = 240;

/// Một ô đã giải xong, đủ để vẽ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    /// Vật liệu tại lát `z` đang xem.
    ///
    /// `String` chứ không phải `&'static str`: từ khi vật liệu đến từ content
    /// pack (`§19.7`), tập giá trị không còn đóng lúc biên dịch. Giữ `&'static
    /// str` sẽ buộc phải rò bộ nhớ cho mỗi id của pack.
    pub material: String,
    /// Ô này đã bị sửa so với địa hình sinh ra chưa.
    ///
    /// Client dùng nó để biết chỗ nào là công trình, chỗ nào là thiên nhiên —
    /// và về sau, chỗ nào True God đã chạm vào.
    pub built: bool,
    /// Vật liệu của ô rắn trên cùng của cột này.
    ///
    /// Tồn tại vì một lát `z` thuần túy cho ra một bản đồ **đen kịt**: đứng ở
    /// cao độ 85 m thì mọi ô thấp hơn 85 m đều là không khí, và màn hình đầu
    /// tiên của người chơi là một khoảng trống có ba chấm màu.
    ///
    /// `§18.1` đã lường trước: "có thể ghost 1–3 lớp trên/dưới với opacity
    /// thấp". Đây là dữ liệu cho việc đó — client vẽ mặt đất bên dưới, mờ dần
    /// theo `drop`, nên người chơi thấy địa hình mà vẫn biết mình đang ở lát nào.
    pub surface: String,
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
        let sim = build_world(seed);
        let gen = Worldgen::new(WorldSeed(seed), GenerationProfile::default());
        // Chỗ ở được: `§P2` nói thế giới sinh ra từ seed, và seed không hứa
        // rằng gốc tọa độ nằm trên cạn. Với seed 42, ô `(0, 0)` nằm **dưới mực
        // biển**.
        let (sx, sy) = find_habitable(&gen);
        let mut g = Game {
            sim,
            gen,
            seed,
            eye: (sx, sy),
            z: 0,
            speed_milli: 1_000,
            plans: BTreeMap::new(),
            plan_cause: BTreeMap::new(),
            landmarks: BTreeMap::new(),
            crops: BTreeMap::new(),
            trample: BTreeMap::new(),
            terrain: RefCell::new(BTreeMap::new()),
            overrides: BTreeMap::new(),
            content: None,
            content_dir: None,
            journal: Vec::new(),
            // Kho khởi điểm của làng chuẩn (`§mow_econ::village::Stock`). Lực
            // lượng lao động thật thì chưa đo được — làng chưa dựng — nên bắt
            // đầu ở `default()` rồi đo lại ngay dưới đây, sau `raise_village`.
            stock: Stock::starting_village(),
            work: Workforce::default(),
            last_day_report: None,
        };

        // Trung tâm làng là khoảnh đất **bằng** gần chỗ ở được nhất, không phải
        // chính chỗ ở được. Hai thứ khác nhau: "khô ráo" chỉ loại nước ra, và
        // một mỏm đá dựng đứng thì vẫn khô ráo.
        let (vx, vy) = g.flattest_near(sx, sy);
        g.raise_village(vx, vy);
        // Đo lực lượng lao động **sau khi** làng đã dựng: `raise_village` vừa
        // gán vai cho từng cư dân, và đo trước đó sẽ luôn ra một làng rỗng
        // người, kể cả khi mười cư dân đã đứng sẵn trên bản đồ.
        g.work = g.compute_workforce();

        // Cái nhìn mở ra ở quảng trường, không ở gốc tọa độ và không ở một chỗ
        // ngẫu nhiên: cảnh đầu tiên phải là ngôi làng đang sống, không phải một
        // bãi đất trống mà người chơi phải đi tìm làng.
        g.eye = g.landmarks.get(&Place::Square).copied().unwrap_or((vx, vy));

        // Lát bắt đầu là mặt đất dưới cái nhìn, không phải `z = 0`. Nếu quảng
        // trường nằm trên một ngọn đồi cao 300 m thì lát 0 nằm sâu trong đá và
        // màn hình đầu tiên là một khối đen đặc — trông y hệt một lỗi renderer,
        // và người ta sẽ đi sửa renderer.
        g.z = g.ground(g.eye.0, g.eye.1).0;

        // Đẩy đồng hồ tới buổi sáng **sau** khi đã dựng làng: dựng xong mới
        // chạy đồng hồ nghĩa là mọi cư dân sinh ra ở nhà mình rồi mới thức dậy,
        // chứ không phải sinh ra giữa ban ngày ở một nơi họ chưa từng đi tới.
        let _ = g.sim.advance(DAY_START);
        g
    }

    /// Tâm của khoảnh đất bằng nhất trong bán kính quét quanh `(x, y)`.
    ///
    /// Chấm điểm bằng số ô dựng được trong một ô vuông 7×7, chứ không bằng độ
    /// dốc tại một điểm: một ô phẳng nằm lọt giữa các vách đá vẫn là chỗ tồi để
    /// đặt làng, và chỉ có ô lân cận mới nói ra điều đó.
    ///
    /// Quét thưa (bước 4) vì đây chạy một lần lúc khởi tạo và mục tiêu là "đủ
    /// tốt", không phải "tối ưu": lệch vài ô so với chỗ bằng nhất tuyệt đối thì
    /// không ai nhận ra, còn quét từng ô trên bán kính 40 là 6.400 lần lấy mẫu
    /// địa hình cho một câu trả lời gần như y hệt.
    fn flattest_near(&self, x: i64, y: i64) -> (i64, i64) {
        let score = |cx: i64, cy: i64| -> u32 {
            let mut n = 0;
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if self.buildable(cx + dx, cy + dy) {
                        n += 1;
                    }
                }
            }
            n
        };
        let mut best = (x, y);
        let mut best_score = score(x, y);
        // 49 là điểm tối đa; đạt rồi thì dừng, không có gì hơn được nữa.
        if best_score == 49 {
            return best;
        }
        let mut r = 4;
        while r <= FLAT_SEARCH_RADIUS {
            for dy in (-r..=r).step_by(4) {
                for dx in (-r..=r).step_by(4) {
                    // Chỉ xét viền của vòng: bên trong đã xét ở vòng trước.
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let p = (x + dx, y + dy);
                    let sc = score(p.0, p.1);
                    if sc > best_score {
                        best_score = sc;
                        best = p;
                    }
                }
            }
            if best_score == 49 {
                return best;
            }
            r += 4;
        }
        best
    }

    /// Quy hoạch rồi dựng một ngôi làng quanh `(cx, cy)`.
    ///
    /// Bộ quy hoạch là hàm thuần và không biết gì về `Sim`; ở đây ta biến bản
    /// kế hoạch thành hai thứ của thế giới: **ô địa hình** và **cư dân**. Tách
    /// như vậy để quy hoạch kiểm được bằng test thường, còn phần đụng vào thế
    /// giới thì đi qua đúng một đường ghi.
    fn raise_village(&mut self, cx: i64, cy: i64) {
        let req = SettleRequest {
            seed: self.seed,
            center: (cx, cy),
            radius: 32,
        };
        let plan = mow_settle::plan(&req, &|x, y| self.buildable(x, y));
        if plan.buildings.is_empty() {
            // Không đủ đất khô. Không phải lỗi: một hòn đảo nhỏ là một thế giới
            // hợp lệ, và thà không có làng còn hơn có một cái làng dưới biển.
            return;
        }

        for (x, y, m) in &plan.cells {
            self.overrides.insert((*x, *y), (*m).to_owned());
            // Ô nào bộ quy hoạch đánh dấu là ruộng thì vào vòng đời cây trồng.
            // Lệch pha theo toạ độ: cả làng gặt cùng một ngày trông như một cỗ
            // máy, không như một cộng đồng nơi mỗi thửa gieo một hôm khác nhau.
            if CROP_STAGES.contains(m) {
                let skew = u8::try_from((x.rem_euclid(3) + y.rem_euclid(2)) as u64).unwrap_or(0);
                self.crops
                    .insert((*x, *y), skew % (CROP_STAGES.len() as u8));
            }
        }

        // Giếng là công trình đầu tiên, và quảng trường là ô cửa của nó.
        let well = plan.buildings[0];
        self.set_landmark(Place::Well, (well.origin.0, well.origin.1));
        self.set_landmark(Place::Square, well.door);

        // Dời người chơi ra **quảng trường**, không để đứng giữa giếng.
        //
        // Làng dựng quanh chỗ sinh, và giếng nằm đúng tâm — nên nếu không dời,
        // màn hình đầu tiên là một người đứng trong nước. Bài test "không sinh
        // ra dưới biển" bắt được đúng chuyện đó, ở một chỗ không ai ngờ tới.
        let (bx, by) = well.door;
        let movers: Vec<(EntityId, i64, i64)> = self
            .sim
            .store()
            .with_attr("core.pos.x")
            .enumerate()
            .map(|(i, id)| (id, bx + i64::try_from(i).unwrap_or(0), by + 1))
            .collect();
        for (id, x, y) in movers {
            self.dat(id, "core.pos.x", Value::Int(x));
            self.dat(id, "core.pos.y", Value::Int(y));
        }

        self.stock_larder(well.door);

        for r in &plan.residents {
            let home = plan.buildings.get(r.home).map_or(well.door, |b| b.door);
            let work = plan
                .buildings
                .get(r.workplace)
                .map_or(well.door, |b| b.door);
            let who = self.spawn_villager(&r.name, r.start);
            self.assign_role(who, settle_role(r.role), home, work);
        }
    }

    /// Đặt kho lương của làng: vài ổ bánh nằm quanh quảng trường.
    ///
    /// Vật phẩm nằm trên đất chứ không nằm trong một con số kho ẩn: `§22.33`
    /// nói một vật ở đúng một nơi, và một cái kho vô hình là một nơi người chơi
    /// không nhìn thấy, không cướp được, không ban thêm được. Một vị thần muốn
    /// gieo nạn đói thì phải có thứ để lấy đi.
    fn stock_larder(&mut self, at: (i64, i64)) {
        // Xếp quanh quảng trường theo một vòng nhỏ, không chồng lên nhau và
        // không chồng lên cư dân.
        for (i, (dx, dy)) in [(1, 0), (0, 1), (-1, 0), (0, -1)].into_iter().enumerate() {
            let p = (at.0 + dx, at.1 + dy);
            if !self.walkable(p.0, p.1) {
                continue;
            }
            self.spawn_bread(&format!("O Banh {}", i + 1), p);
        }
    }

    /// Một ổ bánh nằm trên đất tại `at`.
    fn spawn_bread(&mut self, name: &str, at: (i64, i64)) -> EntityId {
        self.apply(&Command::new(
            "core.spawn",
            WORLD,
            Value::Map(
                [
                    ("kind".to_owned(), Value::Text("item".to_owned())),
                    ("name".to_owned(), Value::Text(name.to_owned())),
                ]
                .into_iter()
                .collect(),
            ),
        ))
        .expect("tạo vật phẩm");
        let it = self.sim.store().ids().next_back().expect("vừa tạo");
        self.dat(it, "core.pos.x", Value::Int(at.0));
        self.dat(it, "core.pos.y", Value::Int(at.1));
        self.dat(it, "item.def", Value::Text("core.bread".to_owned()));
        self.dat(it, "item.nutrition", Value::Int(4_000));
        self.dat(it, "loc.cell", Value::Int(1));
        // Dấu hiệu giác quan: cư dân **nhìn thấy** nó là thức ăn, chứ không phải
        // biết vì engine bảo thế (`§10.4`).
        self.dat(it, "sign.sight.food", Value::Bool(true));
        it
    }

    /// Sinh một cư dân và trả về định danh.
    fn spawn_villager(&mut self, name: &str, at: (i64, i64)) -> EntityId {
        self.apply(&Command::new(
            "core.spawn",
            WORLD,
            Value::Map(
                [
                    ("kind".to_owned(), Value::Text("entity".to_owned())),
                    ("name".to_owned(), Value::Text(name.to_owned())),
                    ("age".to_owned(), Value::Int(30)),
                    (
                        "tags".to_owned(),
                        Value::List(vec![Value::Text("sapient".to_owned())]),
                    ),
                ]
                .into_iter()
                .collect(),
            ),
        ))
        .expect("tạo cư dân");
        let who = self.sim.store().ids().next_back().expect("vừa tạo");
        self.dat(who, "core.pos.x", Value::Int(at.0));
        self.dat(who, "core.pos.y", Value::Int(at.1));
        // Đói ban đầu khác nhau một chút theo định danh: cả làng cùng đi ăn một
        // lúc trông như một đàn máy, không như một cộng đồng.
        let lech = i64::try_from(who.get() % 7).unwrap_or(0) * 200;
        self.dat(who, "need.hunger", Value::Int(1_500 + lech));
        who
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
        self.apply(&cmd).expect("đặt thuộc tính");
    }

    /// Nạp content pack. Trả về lỗi dạng chuỗi để chỗ gọi in ra rồi chạy tiếp.
    /// Thư mục content pack đang nạp, nếu có.
    pub fn content_dir(&self) -> Option<&str> {
        self.content_dir.as_deref()
    }

    pub fn load_content(&mut self, dir: &str) -> Result<usize, String> {
        let pack = load_pack(dir).map_err(|e| e.to_string())?;
        let n = pack.blocks.len();
        // `load_pack` coi thư mục con vắng mặt là sổ rỗng, nên một đường dẫn gõ
        // sai trả về `Ok` với 0 vật liệu. Ở đây điều đó **là** lỗi: một pack
        // không có vật liệu nào thì không vẽ được ô nào, và im lặng chấp nhận
        // nó nghĩa là người dùng gõ nhầm `--content` rồi ngồi nhìn một bản đồ
        // toàn màu dự phòng mà không biết vì sao.
        if n == 0 {
            return Err(format!("`{dir}` không có vật liệu nào — sai đường dẫn?"));
        }
        self.content = Some(pack);
        self.content_dir = Some(dir.to_owned());
        Ok(n)
    }

    /// Bảng vật liệu đang nạp, nếu có.
    pub fn blocks(&self) -> Option<&mow_content::BlockRegistry> {
        self.content.as_ref().map(|c| &c.blocks)
    }

    /// Seed đã dựng thế giới này.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Ô mà cái nhìn của vị thần đang đặt vào.
    pub fn eye(&self) -> (i64, i64) {
        self.eye
    }

    /// Dời cái nhìn. Không sinh sự kiện: đây là khung nhìn, không phải thế giới.
    ///
    /// `§P6.8` tách rạch ròi **truy vấn khung nhìn** khỏi **lệnh**. Ghi một sự
    /// kiện mỗi lần người chơi kéo bản đồ sẽ nhấn chìm nhật ký bằng thứ không
    /// phải lịch sử của thế giới, và làm mọi chuỗi nhân quả dài thêm vô ích.
    pub fn look_at(&mut self, x: i64, y: i64) {
        self.eye = (x, y);
    }

    /// Lát `z` đang xem.
    pub fn z(&self) -> i64 {
        self.z
    }

    /// Tốc độ thời gian hiện tại, phần nghìn.
    pub fn speed_milli(&self) -> u32 {
        self.speed_milli
    }

    /// Đặt tốc độ thời gian. Kẹp về `[0, MAX_SPEED_MILLI]`.
    pub fn set_speed_milli(&mut self, v: u32) {
        self.speed_milli = v.min(MAX_SPEED_MILLI);
    }

    /// Số tick cần chạy trong lần thức dậy này.
    ///
    /// `carry` là phần dư mang theo giữa các lần gọi, tính bằng **phần nghìn
    /// tick**. Không mang dư thì ×0.001 làm tròn về 0 tick mỗi nhịp và thế giới
    /// đứng im — đúng lớp lỗi đã cắn ở tỉ lệ đột biến và ở ngăn S/E/I/R.
    pub fn ticks_due(&self, wake_ms: u64, base_tick_ms: u64, carry: &mut u64) -> u32 {
        if self.speed_milli == 0 || base_tick_ms == 0 {
            return 0;
        }
        // Cộng dồn **tử số**, chia sau. Bản đầu viết
        // `carry += wake_ms * speed / base_tick_ms` và ở ×0.001 phép chia đó là
        // `50 * 1 / 300 = 0` — làm tròn về 0 *trước khi* cộng dồn, nên phần dư
        // không bao giờ lớn lên và thế giới đứng im vĩnh viễn.
        //
        // Đây đúng là lớp lỗi mà bài test bên dưới tồn tại để bắt, và nó vẫn
        // lọt vào lần viết đầu.
        let nguong = base_tick_ms * 1_000;
        *carry += wake_ms * u64::from(self.speed_milli);
        let n = *carry / nguong;
        *carry -= n * nguong;
        u32::try_from(n).unwrap_or(u32::MAX).min(MAX_TICKS_PER_WAKE)
    }

    /// Đổi lát đang xem. Không phải command: nó không đổi thế giới (`§P6.8`).
    pub fn set_z(&mut self, z: i64) {
        self.z = z;
    }

    /// Tick hiện tại.
    pub fn tick(&self) -> Tick {
        self.sim.clock().local()
    }

    /// Hash trạng thái, **gồm cả địa hình đã sửa**.
    ///
    /// `Sim::state_hash` chỉ biết về ECS. Địa hình là `seed + delta` (`§7.2`),
    /// nằm ngoài ECS — nên nếu chỉ dùng hash của `Sim` thì hai thế giới có hai
    /// ngôi làng khác nhau sẽ cho **cùng một hash**, và cả preview lẫn replay
    /// mất khả năng phát hiện sai lệch.
    ///
    /// Bản đầu của `set_cell` có một chú thích nói rằng ô ghi đè đi vào hash.
    /// Nó không đi. Chú thích sai còn nguy hơn thiếu chú thích, vì người đọc
    /// tin nó và thôi kiểm tra.
    pub fn state_hash(&self) -> StateHash {
        Self::mix(
            self.sim.state_hash(),
            &self.overrides,
            &self.stock,
            &self.work,
            &self.trample,
        )
    }

    /// Trộn hash của ECS với địa hình đã sửa và với kho + lực lượng lao động
    /// của làng.
    ///
    /// `stock`/`work` **không** nằm trong ECS — chúng là state riêng của
    /// `Game`, đúng vị trí mà `overrides` (địa hình ghi đè) đã từng đứng một
    /// mình ở đây. Không trộn chúng vào thì hai thế giới cùng seed, cùng số
    /// tick, nhưng có kho khác nhau — vì một đường ghi nào đó sau này sửa
    /// thẳng `stock` mà bỏ qua `run_day` — vẫn ra cùng `state_hash`. Đó đúng
    /// là loại lỗi im lặng mà bản đầu của `overrides` từng mắc (xem chú thích
    /// cũ trên `set_cell` nói dối rằng ô ghi đè đã vào hash), và nhiệm vụ này
    /// tồn tại một phần để không lặp lại nó ở một trường mới.
    fn mix(
        sim: StateHash,
        overrides: &BTreeMap<(i64, i64), String>,
        stock: &Stock,
        work: &Workforce,
        trample: &BTreeMap<(i64, i64), u32>,
    ) -> StateHash {
        if overrides.is_empty()
            && trample.is_empty()
            && *stock == Stock::default()
            && *work == Workforce::default()
        {
            // Không có gì sửa và kinh tế ở trạng thái trung tính (rỗng) thì
            // hash đúng bằng hash của `Sim`. Trong một thế giới thật điều
            // kiện này gần như không bao giờ đúng — mọi làng đều có kho khác
            // `Stock::default()` — nhưng giữ nó nghĩa là công cụ nào đã so
            // hash từ trước khi có `overrides` vẫn đúng nguyên cho một `Sim`
            // rỗng hoàn toàn.
            return sim;
        }
        let mut h = StateHasher::with_domain("mow.server.world.v1");
        h.write_bytes(&sim.0);
        // `BTreeMap` nên thứ tự lặp là thứ tự khóa — hash không phụ thuộc thứ
        // tự người chơi đặt ô.
        for ((x, y), m) in overrides {
            h.write_i64(*x);
            h.write_i64(*y);
            h.write_str(m);
        }
        // Lối mòn cũng là sự thật về thế giới: hai thế giới cùng seed cùng số
        // tick phải mòn giống hệt nhau, nếu không thì "xác định" chỉ đúng với
        // phần thế giới mà ta nhớ đưa vào hash.
        for ((x, y), v) in trample {
            h.write_i64(*x);
            h.write_i64(*y);
            h.write_u64(u64::from(*v));
        }
        // Sáu trường của `Stock`, viết tay từng trường thay vì dựa vào
        // `CanonicalHash` (kiểu này không có, và không nên có: `mow-econ`
        // không cần biết gì về hashing của `mow-core`, xem tài liệu module
        // của nó — "không biết gì về `Sim`, HTTP hay ECS").
        h.write_i64(stock.food);
        h.write_i64(stock.water);
        h.write_i64(stock.wood);
        h.write_i64(stock.tools);
        h.write_i64(stock.seed_grain);
        h.write_i64(stock.soil_fertility);
        h.write_u64(u64::from(work.farmers));
        h.write_u64(u64::from(work.hunters));
        h.write_u64(u64::from(work.smiths));
        h.write_u64(u64::from(work.elders));
        h.write_u64(u64::from(work.children));
        h.finish()
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
        // Qua bảng nhớ, không gọi thẳng worldgen: một lô ô gọi hàm này hàng
        // nghìn lần, và bản đồ thu nhỏ gọi nó 16.384 lần mỗi 2,5 giây.
        let c: BaseCell = self.cell(x, y);

        let sea = self.gen.profile().sea_level_m;
        // Ô đã xây đè lên địa hình, nhưng **chỉ ở mặt đất**: nhìn xuống lát sâu
        // hơn vẫn thấy đá, nhìn lên trời vẫn thấy không khí. Không có ràng buộc
        // này thì một ngôi nhà kéo dài vô tận theo trục z.
        let built = self.overrides.get(&(x, y));
        let on_surface = self.z == c.elevation.height_m;
        // Mặt nhìn thấy được: mặt nước nếu ô chìm, mặt đất nếu không.
        let dinh = if c.elevation.submerged {
            sea
        } else {
            c.elevation.height_m
        };
        let natural = material_at(&c.elevation, &c.strata, sea, self.z).as_str();
        Tile {
            material: match built {
                Some(m) if on_surface => m.clone(),
                _ => natural.to_owned(),
            },
            built: built.is_some(),
            surface: match built {
                Some(m) => m.clone(),
                None => material_at(&c.elevation, &c.strata, sea, dinh)
                    .as_str()
                    .to_owned(),
            },
            drop: (self.z - dinh).max(0),
            biome: c.biome.as_str(),
            height: c.elevation.height_m,
            river: c.flow.is_river,
            edited: false,
        }
    }

    /// Gán vai và nơi chốn cho một cư dân.
    ///
    /// Lưu vào thuộc tính của thực thể chứ không vào một bảng riêng trong
    /// server: nơi ở là **sự thật về thế giới**, nên nó phải nằm trong state và
    /// đi vào `state_hash`. Một bảng bên cạnh sẽ biến mất khi save/load.
    pub fn assign_role(&mut self, who: EntityId, role: Role, home: (i64, i64), work: (i64, i64)) {
        self.dat(who, "npc.role", Value::Text(role_name(role).to_owned()));
        self.dat(who, "npc.home.x", Value::Int(home.0));
        self.dat(who, "npc.home.y", Value::Int(home.1));
        self.dat(who, "npc.work.x", Value::Int(work.0));
        self.dat(who, "npc.work.y", Value::Int(work.1));
    }

    /// Đặt một địa điểm chung của làng.
    pub fn set_landmark(&mut self, place: Place, at: (i64, i64)) {
        self.landmarks.insert(place, at);
    }

    /// Nơi mà một ý định trỏ tới, tính bằng tọa độ.
    fn place_of(&self, who: EntityId, place: Place) -> Option<(i64, i64)> {
        match place {
            Place::Home => Some((
                self.sim.store().attr_int(who, "npc.home.x")?,
                self.sim.store().attr_int(who, "npc.home.y")?,
            )),
            Place::Workplace | Place::Field => Some((
                self.sim.store().attr_int(who, "npc.work.x")?,
                self.sim.store().attr_int(who, "npc.work.y")?,
            )),
            Place::Well | Place::Square => self.landmarks.get(&place).copied(),
        }
    }

    /// Cư dân này đang đứng ở đâu, theo cách nhìn của bộ lập lịch.
    ///
    /// Bán kính 1 ô chứ không phải trùng khít: một người đứng cạnh cửa nhà mình
    /// thì **đang ở nhà**, và đòi trùng khít sẽ làm họ mãi mãi "đang trên đường
    /// về" trong khi thực ra đã tới.
    fn where_is(&self, who: EntityId) -> Place {
        let x = self.attr_int(who, "core.pos.x");
        let y = self.attr_int(who, "core.pos.y");
        let near = |p: Option<(i64, i64)>| {
            p.is_some_and(|(px, py)| (px - x).abs() <= 1 && (py - y).abs() <= 1)
        };
        for place in [Place::Home, Place::Workplace, Place::Well, Place::Square] {
            if near(self.place_of(who, place)) {
                return place;
            }
        }
        // Không ở đâu trong bốn nơi trên thì coi như đang ngoài đồng — đó là
        // mặc định đúng cho một làng, nơi mọi khoảng trống đều là đất canh tác
        // hoặc đường đi giữa hai nơi.
        Place::Field
    }

    /// Ghi đè vật liệu một ô.
    ///
    /// Không đi qua `Sim::apply` vì địa hình **không** phải state của ECS: nó là
    /// `seed + delta` theo `§7.2`. Nhưng nó vẫn là state của thế giới, nên nó đi
    /// vào `state_hash` — nếu không, hai thế giới có làng khác nhau sẽ cho cùng
    /// một hash và replay sẽ không phát hiện được sai lệch.
    pub fn set_cell(&mut self, x: i64, y: i64, material: &str) {
        self.overrides.insert((x, y), material.to_owned());
    }

    /// Số ô đã bị sửa.
    pub fn built_cells(&self) -> usize {
        self.overrides.len()
    }

    /// Ô này đi bộ được không.
    ///
    /// Luật hiện tại: đất khô. Không lội nước, không đi trên magma.
    ///
    /// Cố ý **không** xét độ dốc ở bước này. Thêm độ dốc mà chưa có leo trèo sẽ
    /// khoanh người chơi vào một thung lũng mà không nói cho họ biết vì sao —
    /// một luật vô hình thì tệ hơn một luật lỏng.
    pub fn walkable(&self, x: i64, y: i64) -> bool {
        !self.ground(x, y).1
    }

    /// Một ô worldgen, qua bảng nhớ tạm.
    ///
    /// Ô ngoài thế giới trả về một ô thay thế **chìm dưới nước**: không đi vào
    /// được và không dựng được, đúng thứ ta muốn ở rìa bản đồ.
    fn cell(&self, x: i64, y: i64) -> BaseCell {
        // `BaseCell` không `Copy` (nó mang `Strata`, `Climate`, `Flow`), nên
        // nhân bản chứ không sao chép — vẫn rẻ hơn nhiều so với chạy lại cả
        // chuỗi tầng nhiễu, vốn là toàn bộ lý do bảng nhớ này tồn tại.
        if let Some(v) = self.terrain.borrow().get(&(x, y)) {
            return v.clone();
        }
        let v = self.gen.base_cell(x, y).unwrap_or_else(|_| edge_cell(&self.gen));
        let mut cache = self.terrain.borrow_mut();
        // Trần để một ván dài không biến bảng nhớ thành chỗ rò bộ nhớ. Xóa sạch
        // chứ không đuổi từng ô: không có thứ tự truy cập nào để dựa vào, và
        // dựng lại thì rẻ.
        if cache.len() >= TERRAIN_CACHE_CAP {
            cache.clear();
        }
        cache.insert((x, y), v.clone());
        v
    }

    /// Cao độ và "có nước không" của một ô.
    fn ground(&self, x: i64, y: i64) -> (i64, bool) {
        let c = self.cell(x, y);
        let wet = c.elevation.submerged
            || c.flow.is_water_body
            || matches!(
                c.strata.surface,
                mow_worldgen::Material::Water | mow_worldgen::Material::Magma
            );
        (c.elevation.height_m, wet)
    }

    /// Ruộng lớn thêm một pha, nếu tới ngày.
    ///
    /// Chỉ đổi **vật liệu bị ghi đè**, không đụng tới worldgen: một mùa vụ là
    /// thứ người ta làm ra trên mặt đất, không phải một thay đổi của mặt đất.
    fn grow_crops(&mut self, day: u64) {
        if !day.is_multiple_of(CROP_DAYS_PER_STAGE) {
            return;
        }
        let updates: Vec<((i64, i64), u8)> = self
            .crops
            .iter()
            .map(|(k, v)| (*k, (*v + 1) % (CROP_STAGES.len() as u8)))
            .collect();
        for (at, stage) in updates {
            self.crops.insert(at, stage);
            if let Some(m) = CROP_STAGES.get(stage as usize) {
                self.overrides.insert(at, (*m).to_owned());
            }
        }
    }

    /// Dựng hồ sơ cho Yuu: **chỉ** những gì engine đã biết.
    ///
    /// Đây là chỗ giữ lời hứa chống nói dối. Yuu không được đọc thế giới; nó
    /// đọc **cái này**. Mọi câu nó nói phải trích dẫn một `seq` có trong đây,
    /// nếu không thì bị cắt (`mow_yuu::read_answer`). Nên hồ sơ hẹp bao nhiêu
    /// thì Yuu nói ít bấy nhiêu — và nói ít mà đúng thì hơn nói nhiều mà đoán.
    pub fn dossier(&self, recent: usize) -> mow_yuu::Dossier {
        let s = self.sim.store();
        let folk = s
            .with_attr("npc.role")
            .map(|id| mow_yuu::FolkBrief {
                id: id.get(),
                name: s.attr_text(id, "core.name").unwrap_or("?").to_owned(),
                role: s.attr_text(id, "npc.role").unwrap_or("?").to_owned(),
                intent: s.attr_text(id, "npc.intent").unwrap_or("idle").to_owned(),
                hunger: s.attr_int(id, "need.hunger").unwrap_or(0) / 100,
            })
            .collect();

        // Lấy `recent` sự kiện **mới nhất**, nhưng giữ thứ tự thời gian tăng
        // dần: một chuỗi nhân quả đọc xuôi thì hiểu được, đọc ngược thì không.
        let all: Vec<&mow_core::Event> = self.sim.log().iter().collect();
        let start = all.len().saturating_sub(recent);
        let events = all[start..]
            .iter()
            .map(|e| mow_yuu::EventBrief {
                seq: e.seq.0,
                tick: e.tick.0,
                kind: e.kind.0.clone(),
                actor: e.actor.map(EntityId::get),
                cause: e.cause.map(|c| c.0),
                summary: crate::preview::summarize(&e.kind.0, &e.payload),
            })
            .collect();

        mow_yuu::Dossier {
            tick: self.sim.clock().local().0,
            stock: vec![
                ("food".to_owned(), self.stock.food),
                ("water".to_owned(), self.stock.water),
                ("wood".to_owned(), self.stock.wood),
                ("tools".to_owned(), self.stock.tools),
                ("seed_grain".to_owned(), self.stock.seed_grain),
                ("soil_fertility".to_owned(), self.stock.soil_fertility),
            ],
            folk,
            events,
        }
    }

    /// Số thửa ruộng đang chín — thứ báo trước một ngày no đủ.
    pub fn ripe_fields(&self) -> usize {
        let ripe = (CROP_STAGES.len() - 1) as u8;
        self.crops.values().filter(|v| **v == ripe).count()
    }

    /// Một bàn chân đặt xuống ô `(x, y)`.
    fn tread(&mut self, x: i64, y: i64) {
        let e = self.trample.entry((x, y)).or_insert(0);
        *e = (*e + TRAMPLE_PER_STEP).min(TRAMPLE_MAX);
        if self.trample.len() > TRAMPLE_CAP {
            self.prune_trample();
        }
    }

    /// Bỏ những lối mòn nhạt nhất khi bảng đã quá đầy.
    fn prune_trample(&mut self) {
        let mut faint: Vec<((i64, i64), u32)> =
            self.trample.iter().map(|(k, v)| (*k, *v)).collect();
        faint.sort_by_key(|(_, v)| *v);
        for (k, _) in faint.into_iter().take(TRAMPLE_CAP / 4) {
            self.trample.remove(&k);
        }
    }

    /// Cả làng nguôi đi một chút sau mỗi ngày.
    fn fade_trample(&mut self) {
        self.trample.retain(|_, v| {
            *v = v.saturating_sub(TRAMPLE_DECAY_PER_DAY);
            *v > 0
        });
    }

    /// Độ mòn của một ô, `0..=TRAMPLE_MAX`.
    pub fn trample_at(&self, x: i64, y: i64) -> u32 {
        self.trample.get(&(x, y)).copied().unwrap_or(0)
    }

    /// Chênh cao lớn nhất giữa một ô và bốn ô kề nó, tính bằng mét.
    ///
    /// Đây là thước đo độ dốc rẻ nhất còn đúng: một ô nằm giữa sườn có thể có
    /// độ dốc trung bình nhỏ trong khi một phía của nó là vách đứng, và trung
    /// bình sẽ giấu mất đúng cái vách đó.
    fn slope_m(&self, x: i64, y: i64) -> i64 {
        let h = self.ground(x, y).0;
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .map(|(dx, dy)| (self.ground(x + dx, y + dy).0 - h).abs())
            .max()
            .unwrap_or(0)
    }

    /// Ô này dựng nhà lên được không.
    ///
    /// Khác [`Game::walkable`] ở đúng một điều kiện, và điều kiện đó quan trọng:
    /// **độ dốc**. Đi bộ lên một sườn dốc là chuyện bình thường; dựng một cái
    /// nhà lên đó thì không. Bản đầu chỉ hỏi "có phải nước không", và ngôi làng
    /// đầu tiên nằm vắt qua một vách 64 mét — đường sỏi chạy thẳng xuống vực,
    /// và nửa làng chìm trong bóng đổ của chính cái vách đó.
    ///
    /// Ngưỡng `MAX_BUILD_SLOPE_M` là chênh cao giữa hai ô kề nhau, không phải
    /// độ dốc trung bình cả khu: một bậc thềm cao 3 mét giữa hai nhà là thứ
    /// người ta phải trèo qua mỗi ngày.
    pub fn buildable(&self, x: i64, y: i64) -> bool {
        self.walkable(x, y) && self.slope_m(x, y) <= MAX_BUILD_SLOPE_M
    }

    /// Đặt một đích tới cho một thực thể. Trả về số bước đã lên kế hoạch.
    ///
    /// Bấm ra giữa biển **không** làm nhân vật đứng im: A* cạn ngân sách sẽ trả
    /// về đường tới ô gần đích nhất đã thấy, và nhân vật đi tới mép bờ. Đứng im
    /// mà không nói gì là câu trả lời tệ nhất cho một cú bấm chuột.
    pub fn set_destination(&mut self, who: EntityId, to: (i64, i64)) -> (usize, &'static str) {
        let from = (
            self.attr_int(who, "core.pos.x"),
            self.attr_int(who, "core.pos.y"),
        );
        let req = PathRequest {
            from,
            to,
            max_nodes: PATH_BUDGET,
        };
        let (path, why) = match find_path(&req, &|x, y| self.walkable(x, y)) {
            PathOutcome::Found(p) => (p, "found"),
            PathOutcome::BudgetExhausted { best_effort } => (best_effort, "partial"),
            PathOutcome::Unreachable => (Vec::new(), "unreachable"),
        };

        // Bỏ ô đầu: nhân vật đang đứng ở đó rồi.
        let mut steps: VecDeque<(i64, i64)> = path.into_iter().collect();
        if steps.front() == Some(&from) {
            steps.pop_front();
        }
        let n = steps.len();
        if n == 0 {
            self.plans.remove(&who);
        } else {
            self.plans.insert(who, steps);
        }
        (n, why)
    }

    /// Hủy kế hoạch đi của một thực thể.
    pub fn cancel_destination(&mut self, who: EntityId) {
        self.plans.remove(&who);
    }

    /// Số bước còn lại trong kế hoạch của một thực thể.
    /// Tổng số bước còn lại của mọi kế hoạch đi đang chạy.
    ///
    /// Client dùng nó để biết khi nào xóa đường vẽ. Trước đây chỗ này hỏi số
    /// bước của avatar; giờ không có avatar nào, và câu hỏi đúng là *"còn ai
    /// đang trên đường không"*.
    pub fn pending_steps(&self) -> usize {
        self.plans.values().map(VecDeque::len).sum()
    }

    pub fn remaining_steps(&self, who: EntityId) -> usize {
        self.plans.get(&who).map_or(0, VecDeque::len)
    }

    /// Đường đi còn lại, để client vẽ ra cho người chơi thấy.
    ///
    /// Vẽ đường đi không phải trang trí: nó là câu trả lời cho *"nó có hiểu tôi
    /// bấm vào đâu không"*. Không có nó, một cú bấm vào chỗ không tới được và
    /// một cú bấm chưa kịp xử lý trông giống hệt nhau.
    pub fn planned_path(&self, who: EntityId) -> Vec<(i64, i64)> {
        self.plans
            .get(&who)
            .map_or_else(Vec::new, |p| p.iter().copied().collect())
    }

    /// Đi một bước theo kế hoạch, nếu có.
    fn follow_plan(&mut self, who: EntityId) {
        let Some(next) = self.plans.get_mut(&who).and_then(VecDeque::pop_front) else {
            return;
        };
        let x = self.attr_int(who, "core.pos.x");
        let y = self.attr_int(who, "core.pos.y");
        let (dx, dy) = ((next.0 - x).signum(), (next.1 - y).signum());
        if dx == 0 && dy == 0 {
            return;
        }
        if self.walk(who, dx, dy).is_err() {
            // Bước hỏng nghĩa là thế giới đã đổi từ lúc lên kế hoạch. Bỏ cả kế
            // hoạch thay vì thử lại: thử lại một bước bất khả thi mỗi tick là
            // cách nhân vật rung tại chỗ mãi mãi.
            self.plans.remove(&who);
        }
    }

    /// Một bước đi, qua đúng đường ghi của mọi thứ khác.
    ///
    /// Mang theo nguyên nhân nếu bước này thuộc một kế hoạch: nhờ vậy panel
    /// "vì sao" trả lời được *"vì cô ấy định ra đồng"* thay vì *"vì cô ấy đã
    /// bước"*.
    fn walk(&mut self, who: EntityId, dx: i64, dy: i64) -> Result<(), mow_core::Failure> {
        let mut fields = vec![
            ("who".to_owned(), Value::Uint(who.get())),
            ("dx".to_owned(), Value::Int(dx)),
            ("dy".to_owned(), Value::Int(dy)),
        ];
        if let Some(c) = self.plan_cause.get(&who) {
            fields.push(("cause".to_owned(), Value::Uint(c.0)));
        }
        let cmd = Command::new("core.walk", WORLD, Value::Map(fields.into_iter().collect()));
        let r = self.apply(&cmd);
        if r.is_ok() {
            // Ghi vết ở ô **vừa tới**, không phải ô vừa rời: lối mòn là nơi bàn
            // chân đặt xuống.
            let (x, y) = (
                self.attr_int(who, "core.pos.x"),
                self.attr_int(who, "core.pos.y"),
            );
            self.tread(x, y);
        }
        r
    }

    /// Áp một lệnh. Đây là **đường ghi duy nhất** (`§22.1`).
    ///
    /// Mọi lệnh — của người chơi, của NPC, của kế hoạch đi — đều đi qua đây, và
    /// vì thế đều vào nhật ký. Một đường ghi thứ hai sẽ làm phát lại lệch, và
    /// preview sẽ nói dối mà không ai biết.
    pub fn apply(&mut self, cmd: &Command) -> CommandResult<()> {
        let r = self.sim.apply(cmd).map(|_| ());
        if r.is_ok() {
            self.journal.push(JournalEntry {
                tick: self.sim.clock().local().0,
                kind: cmd.kind.0.clone(),
                payload: cmd.payload.clone(),
            });
        }
        r
    }

    /// Số lệnh đã áp.
    pub fn journal_len(&self) -> usize {
        self.journal.len()
    }

    /// Dựng lại thế giới hiện tại từ seed rồi phát lại nhật ký.
    ///
    /// Trả về `None` khi nhật ký quá dài — xem [`PREVIEW_JOURNAL_LIMIT`].
    fn replay(&self) -> Option<Sim> {
        if self.journal.len() > PREVIEW_JOURNAL_LIMIT {
            return None;
        }
        // `build_world`, không `slice::build_empty_world`: nhật ký có thể
        // chứa lệnh `econ.day`/`econ.shortage`, và bản sao phải có đúng
        // handler cho chúng — nếu không, phát lại một thế giới đã qua ít nhất
        // một ngày sẽ hỏng ở đúng lệnh kinh tế đầu tiên với `UnknownCommand`.
        let mut copy = build_world(self.seed);
        for e in &self.journal {
            // Đưa bản sao tới đúng tick của lệnh **trước khi** áp nó. Không có
            // bước này thì mọi lệnh rơi vào tick 0 và `state_hash` lệch, vì sự
            // kiện mang tick.
            let now = copy.clock().local().0;
            if e.tick > now {
                let _ = copy.advance(e.tick - now);
            }
            // Lệnh trong nhật ký đã từng thành công; nếu giờ hỏng thì phát lại
            // đã lệch, và ta muốn biết bằng hash lệch chứ không bằng một panic.
            let _ = copy.apply(&e.to_command(WORLD));
        }
        // Đuổi nốt phần thời gian trôi sau lệnh cuối.
        let now = copy.clock().local().0;
        let target = self.sim.clock().local().0;
        if target > now {
            let _ = copy.advance(target - now);
        }
        Some(copy)
    }

    /// Xem trước một can thiệp mà **không** khắc nó vào thế giới.
    pub fn preview(&self, cmd: &Command) -> Result<Diff, String> {
        let Some(mut copy) = self.replay() else {
            return Err(format!(
                "lịch sử đã dài {} lệnh, vượt trần {PREVIEW_JOURNAL_LIMIT} — cần snapshot",
                self.journal.len()
            ));
        };

        let base_hash = self.state_hash();
        // Phát lại phải cho đúng thế giới đang xem. Không khớp nghĩa là có thứ
        // đã đổi thế giới ngoài đường ghi (`§22.1`), và một preview dựng trên
        // một thế giới khác thì vô giá trị — nên nói thẳng thay vì đưa ra những
        // con số đúng về một vũ trụ không tồn tại.
        if Self::mix(
            copy.state_hash(),
            &self.overrides,
            &self.stock,
            &self.work,
            &self.trample,
        ) != base_hash
        {
            return Err("phát lại lịch sử không dựng lại được thế giới hiện tại — \
                 có thay đổi nằm ngoài nhật ký lệnh"
                .to_owned());
        }

        let before = snapshot(copy.store());
        let outcome = copy.apply(cmd);
        let after = snapshot(copy.store());

        let (error, events) = match outcome {
            Ok(c) => (
                None,
                // `Committed.events` là danh sách **số thứ tự**, không phải
                // sự kiện. Tra ngược qua nhật ký để lấy nội dung.
                c.events
                    .iter()
                    .filter_map(|seq| copy.log().get(*seq))
                    .map(|e| {
                        (
                            e.kind.0.clone(),
                            crate::preview::summarize(&e.kind.0, &e.payload),
                        )
                    })
                    .collect(),
            ),
            Err(e) => (Some(e.to_string()), Vec::new()),
        };

        Ok(Diff {
            command: cmd.kind.0.clone(),
            base_hash,
            after_hash: Self::mix(
                copy.state_hash(),
                &self.overrides,
                &self.stock,
                &self.work,
                &self.trample,
            ),
            error,
            events,
            changes: compare(&before, &after),
        })
    }

    /// Khắc một can thiệp vào thế giới, nhưng **chỉ khi** thế giới chưa đổi.
    ///
    /// Xem tài liệu của `preview`: đây là ràng buộc giữ cho thứ được khắc đúng
    /// bằng thứ người chơi đã xem.
    pub fn commit_checked(&mut self, cmd: &Command, base_hash: &str) -> Result<Diff, Refusal> {
        let now = self.state_hash().to_hex();
        if now != base_hash {
            return Err(Refusal::WorldMoved {
                expected: base_hash.to_owned(),
                actual: now,
            });
        }
        let before = snapshot(self.sim.store());
        let committed = self
            .sim
            .apply(cmd)
            .map_err(|e| Refusal::CommandFails(e.to_string()))?;
        self.journal.push(JournalEntry {
            tick: self.sim.clock().local().0,
            kind: cmd.kind.0.clone(),
            payload: cmd.payload.clone(),
        });
        let after = snapshot(self.sim.store());
        Ok(Diff {
            command: cmd.kind.0.clone(),
            base_hash: mow_math::StateHash([0; 32]),
            after_hash: self.state_hash(),
            error: None,
            events: committed
                .events
                .iter()
                .filter_map(|seq| self.sim.log().get(*seq))
                .map(|e| {
                    (
                        e.kind.0.clone(),
                        crate::preview::summarize(&e.kind.0, &e.payload),
                    )
                })
                .collect(),
            changes: compare(&before, &after),
        })
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

        // Ranh giới ngày kinh tế. `t` là tick **trước khi** advance ở cuối
        // hàm, và `tick_once` luôn được gọi đúng một lần cho mỗi tick bất kể
        // tốc độ (xem tài liệu `speed_milli`) — nên kiểm tra bằng chia hết ở
        // đây thấy đúng một lần mỗi `TICKS_PER_DAY` tick, không phụ thuộc
        // nhanh hay chậm. Bỏ qua `t == 0`: chưa trôi ngày nào thì chưa có gì
        // để chạy `run_day`.
        if t > 0 && t.is_multiple_of(TICKS_PER_DAY) {
            let day = t / TICKS_PER_DAY;
            self.run_economy_day(day);
            self.grow_crops(day);
            self.fade_trample();
        }

        // Kế hoạch đi chạy **mỗi tick**, không phải mỗi 4 tick như NPC lang
        // thang: người chơi vừa bấm chuột và đang nhìn nhân vật đứng yên thì
        // ba tick chờ cũng là ba tick quá dài.
        let walkers: Vec<EntityId> = self.plans.keys().copied().collect();
        for w in walkers {
            self.follow_plan(w);
        }

        // Một bước mỗi 4 tick: đủ chậm để mắt theo kịp, đủ nhanh để thấy là
        // thế giới đang sống.
        if !t.is_multiple_of(4) {
            let _ = self.sim.advance(1);
            return;
        }

        // Điểm neo cho NPC lang thang là **cái nhìn của thần**, không phải một
        // avatar: không còn avatar nào để neo vào. Về mặt cảm giác điều này còn
        // đúng hơn — thứ sinh vật lang thang tụ lại quanh chỗ thần đang nhìn.
        let (ax, ay) = self.eye;

        let npcs: Vec<EntityId> = self
            .sim
            .store()
            .with_attr("core.pos.x")
            .filter(|id| self.sim.store().attr_text(*id, "core.name").is_some())
            .filter(|id| self.sim.store().attr_int(*id, "item.nutrition").is_none())
            .collect();

        for npc in npcs {
            // Có vai thì sống theo lịch; không có vai thì lang thang như cũ.
            // Giữ cả hai nhánh để thế giới lát cắt (ba thực thể, không có làng)
            // vẫn chạy được — nếu không, mọi bài test cũ sẽ phải dựng một làng.
            if self.sim.store().attr_text(npc, "npc.role").is_some() {
                self.follow_routine(npc, t);
                continue;
            }
            let (dx, dy) = self.wander_step(npc, t, ax, ay);
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
            if let Err(e) = self.apply(&cmd) {
                debug_assert!(
                    matches!(e.code, mow_core::FailureCode::PreconditionFailed),
                    "bước NPC hỏng vì lý do ngoài dự kiến: {e:?}"
                );
            }
        }

        let _ = self.sim.advance(1);
    }

    /// Một bước theo lịch sinh hoạt.
    ///
    /// Bộ lập lịch trả về **ý định**, không phải hành động. Việc đổi ý định
    /// thành đường đi là việc của server, và tách như vậy có lý do: `decide` là
    /// hàm thuần kiểm được bằng test thường, còn tìm đường thì cần cả thế giới.
    fn follow_routine(&mut self, who: EntityId, tick: u64) {
        // Đang đi dở thì cứ đi tiếp. Tính lại ý định mỗi tick sẽ làm cư dân
        // đổi ý giữa đường và rung tại chỗ.
        if self.remaining_steps(who) > 0 {
            return;
        }

        let role = role_from(
            self.sim
                .store()
                .attr_text(who, "npc.role")
                .unwrap_or("farmer"),
        );
        let x = self.attr_int(who, "core.pos.x");
        let y = self.attr_int(who, "core.pos.y");
        let nearby = self
            .sim
            .store()
            .with_attr("core.pos.x")
            .filter(|o| *o != who)
            .filter(|o| {
                (self.attr_int(*o, "core.pos.x") - x).abs() <= 2
                    && (self.attr_int(*o, "core.pos.y") - y).abs() <= 2
            })
            .count();

        let s = Situation {
            tick,
            ticks_per_day: TICKS_PER_DAY,
            role,
            hunger: self.attr_int(who, "need.hunger") / 100,
            fatigue: i64::try_from((tick % TICKS_PER_DAY) * 100 / TICKS_PER_DAY).unwrap_or(0),
            at: self.where_is(who),
            nearby: u32::try_from(nearby).unwrap_or(u32::MAX),
            nearest: None,
        };

        let intent = decide(&s);
        let label = intent_key(intent);

        // Ý định chỉ đáng ghi khi nó **đổi**. Ghi lại cùng một ý định mỗi tick
        // sẽ nhấn chìm nhật ký và làm chuỗi nhân quả dài vô nghĩa.
        let changed = self.sim.store().attr_text(who, "npc.intent") != Some(label.as_str());
        if changed {
            // Thuộc tính do chính handler đặt, không đặt riêng ở đây: hai đường
            // ghi cho cùng một sự thật là hai đường để chúng lệch nhau
            // (`§22.1`).
            //
            // Ý định là một **sự kiện**, không chỉ một thuộc tính. Đó là điều
            // kiện để nó làm nguyên nhân được: `Event::cause` trỏ tới một
            // `EventSeq`, và một thuộc tính thì không có số thứ tự.
            let cmd = Command::new(
                "npc.intend",
                WORLD,
                Value::Map(
                    [
                        ("who".to_owned(), Value::Uint(who.get())),
                        ("intent".to_owned(), Value::Text(label.clone())),
                    ]
                    .into_iter()
                    .collect(),
                ),
            );
            if self.apply(&cmd).is_ok() {
                // Sự kiện vừa ghi là mắt gốc của mọi bước sắp tới.
                let seq = EventSeq(self.sim.log().next_seq().0.saturating_sub(1));
                self.plan_cause.insert(who, seq);
            }
        }

        if let Intent::GoTo { place } = intent {
            if let Some(to) = self.place_of(who, place) {
                self.set_destination(who, to);
            }
        }
    }

    fn attr_int(&self, id: EntityId, k: &str) -> i64 {
        self.sim.store().attr_int(id, k).unwrap_or(0)
    }

    /// Bước kế tiếp của một NPC. Hàm thuần của `(id, tick, vị trí avatar)`.
    ///
    /// Xác định là bắt buộc, không phải tùy chọn: hai lần chạy từ cùng seed
    /// phải cho cùng thế giới (`§P7.5`), và một `rand::random()` ở đây phá điều
    /// đó mà không có bài test nào đỏ.
    fn wander_step(&self, npc: EntityId, t: u64, ax: i64, ay: i64) -> (i64, i64) {
        let nx = self.attr_int(npc, "core.pos.x");
        let ny = self.attr_int(npc, "core.pos.y");
        let d = (nx - ax).abs() + (ny - ay).abs();

        // Gần người chơi thì lảng vảng quanh đó; xa thì đi về phía người chơi.
        // Không phải AI, chỉ là đủ để thế giới không đứng hình.
        if d > 6 {
            return ((ax - nx).signum(), (ay - ny).signum());
        }
        let h = npc
            .get()
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(t);
        match h % 5 {
            0 => (1, 0),
            1 => (-1, 0),
            2 => (0, 1),
            3 => (0, -1),
            _ => (0, 0),
        }
    }

    /// Kho của làng, sau ngày kinh tế gần nhất.
    ///
    /// Chưa có chỗ gọi nào ngoài test: `/api/village` (`§idea.md`, chưa nối)
    /// sẽ là chỗ gọi đầu tiên, và nó nối vào `api.rs` — một tệp nhiệm vụ này
    /// cố tình không đụng tới, vì có agent khác đang sửa nó cùng lúc.
    #[allow(dead_code)]
    pub fn stock(&self) -> &Stock {
        &self.stock
    }

    /// Lực lượng lao động, đo lần gần nhất — xem tài liệu trường `work`.
    ///
    /// Chưa dùng ngoài test, cùng lý do với `stock`.
    #[allow(dead_code)]
    pub fn workforce(&self) -> &Workforce {
        &self.work
    }

    /// Báo cáo của ngày kinh tế gần nhất, nếu đã có ít nhất một ngày trôi qua.
    ///
    /// Chưa dùng ngoài test, cùng lý do với `stock`.
    #[allow(dead_code)]
    pub fn last_day_report(&self) -> Option<&DayReport> {
        self.last_day_report.as_ref()
    }

    /// Đếm lực lượng lao động từ cư dân **thật** đang sống trong thế giới.
    ///
    /// Không nhớ một tổng dân số riêng: `Workforce::population` đã nói lý do
    /// ở tài liệu của nó — một tổng lưu riêng là một tổng sẽ lệch. Ở đây cùng
    /// một nguyên tắc áp cho chính `Workforce`: đếm lại từ `npc.role` mỗi lần
    /// gọi, không tin vào một bản sao cũ.
    fn compute_workforce(&self) -> Workforce {
        let mut w = Workforce::default();
        for id in self.sim.store().with_attr("npc.role") {
            match self.sim.store().attr_text(id, "npc.role") {
                Some("smith") => w.smiths += 1,
                Some("hunter") => w.hunters += 1,
                Some("elder") => w.elders += 1,
                Some("child") => w.children += 1,
                // Tên lạ rơi về Farmer, giống hệt `role_from` bên dưới — một
                // content pack gán vai server chưa biết vẫn phải đếm được
                // vào lực lượng lao động, không được lặng lẽ biến mất khỏi
                // mô hình kinh tế.
                _ => w.farmers += 1,
            }
        }
        w
    }

    /// Một ngày của vòng kinh tế: chạy `mow_econ::village::run_day`, áp kết
    /// quả vào kho, phát sự kiện, rồi cho cư dân **cảm thấy** nó.
    ///
    /// Gọi từ `tick_once` mỗi khi đồng hồ vượt một ranh giới `TICKS_PER_DAY`.
    /// Tách khỏi `tick_once` vì đây là ba việc khác hẳn nhau — cập nhật kho,
    /// ghi sổ, chạm vào cư dân — và nhét cả ba vào `tick_once` sẽ làm hàm đó
    /// dài ra đúng chỗ không ai muốn đọc mỗi khi chỉ sửa đường đi.
    fn run_economy_day(&mut self, day: u64) {
        // Đo lại lực lượng lao động **mỗi ngày**, không dùng con số của hôm
        // qua: cư dân có thể đã chết hoặc đổi vai, và một `Workforce` cũ kỹ
        // là đúng loại "số trong kho mà không ai cảm thấy" — nhiệm vụ này tồn
        // tại để xóa đúng loại đó.
        self.work = self.compute_workforce();
        let tools_ok = tools_sufficient(&self.stock, &self.work);
        let report = run_day(&self.stock, &self.work, tools_ok);
        self.stock = report.stock;

        // `econ.day` mang đủ ba con số chính: thu hoạch (`produced`), tiêu
        // thụ (`consumed`), và tồn kho sau ngày (`stock`). Phát qua
        // `self.apply` như mọi lệnh khác trong file này — không viết thẳng
        // vào `self.journal`, để một bản replay còn phát lại được đúng ngày
        // này thay vì phải tính lại `run_day` (mà tính lại thì hai nơi cùng
        // giữ logic kinh tế, sớm muộn chúng lệch nhau).
        let payload = Value::Map(
            [
                ("day".to_owned(), Value::Uint(day)),
                ("produced".to_owned(), stock_value(&report.produced)),
                ("consumed".to_owned(), stock_value(&report.consumed)),
                ("stock".to_owned(), stock_value(&self.stock)),
            ]
            .into_iter()
            .collect(),
        );
        let day_cause = self
            .apply(&Command::new("econ.day", WORLD, payload))
            .ok()
            .map(|()| EventSeq(self.sim.log().next_seq().0.saturating_sub(1)));

        // Một `econ.shortage` cho mỗi loại đang thiếu, gắn nguyên nhân về
        // đúng sự kiện `econ.day` đã báo nó — khuôn giống hệt `walk`/
        // `follow_routine`: nguyên nhân đi trong payload dưới khóa `cause`,
        // và handler (đăng ký ở `build_world`) tự gắn nó qua `emit_caused`.
        for shortage in &report.shortages {
            let mut fields = vec![(
                "kind".to_owned(),
                Value::Text(shortage_key(*shortage).to_owned()),
            )];
            if let Some(c) = day_cause {
                fields.push(("cause".to_owned(), Value::Uint(c.0)));
            }
            let cmd = Command::new(
                "econ.shortage",
                WORLD,
                Value::Map(fields.into_iter().collect()),
            );
            let _ = self.apply(&cmd);
        }

        self.last_day_report = Some(report);
        self.apply_hunger_for_day();
    }

    /// Cho cư dân **cảm thấy** kho, thay vì để nó là một con số không ai chạm
    /// tới.
    ///
    /// Cố ý nhị phân — kho rỗng hay không — không có một mức "vừa đủ" ở
    /// giữa: đó đúng là hai trạng thái nhiệm vụ này khai ra, và bịa thêm một
    /// ngưỡng thứ ba sẽ là một con số không ai đo được ở đâu cả.
    fn apply_hunger_for_day(&mut self) {
        let granary_empty = self.stock.food <= 0;
        let villagers: Vec<EntityId> = self.sim.store().with_attr("npc.role").collect();
        for who in villagers {
            let before = self.attr_int(who, "need.hunger");
            let delta = if granary_empty {
                HUNGER_GAIN_ON_EMPTY_GRANARY
            } else {
                -HUNGER_EASE_PER_DAY
            };
            let after = (before + delta).clamp(0, 10_000);
            if after != before {
                self.dat(who, "need.hunger", Value::Int(after));
            }

            // Ngưỡng chết đói dùng chung với bộ lập lịch (`mow_society::routine::HUNGER_STARVING`),
            // nhân trăm vì `need.hunger` là thang phần trăm của thang đó — xem
            // `follow_routine`, nơi phép chia ngược lại (`/ 100`) diễn ra.
            // Không bịa một ngưỡng riêng cho vòng kinh tế: hai ngưỡng khác
            // nhau cho cùng một khái niệm là hai cơ hội để chúng lệch nhau.
            let starving = after >= HUNGER_STARVING * 100;
            if starving != self.attr_bool(who, "npc.starving") {
                self.dat(who, "npc.starving", Value::Bool(starving));
            }
        }
    }

    /// Thuộc tính dưới dạng cờ đúng/sai. Vắng mặt hoặc sai kiểu đều là `false`
    /// — cùng quy ước với `attr_int` ngay dưới đây.
    fn attr_bool(&self, id: EntityId, key: &str) -> bool {
        matches!(self.sim.store().attr(id, key), Some(Value::Bool(true)))
    }
}

/// Số tick một ngày. Khớp `TICKS_PER_DAY` của tầng vẽ.
///
/// Hai chỗ cùng một con số là hai chỗ để chúng lệch nhau, và khi lệch thì mặt
/// trời trên màn hình mọc lúc cư dân đi ngủ. Đây là chỗ đúng để đặt nó, vì thế
/// giới quyết định thời gian chứ không phải renderer.
pub const TICKS_PER_DAY: u64 = 2_400;

/// Nhịp mà một thế giới mới bắt đầu: khoảng sáng sớm, lúc cả làng vừa thức.
///
/// Nhịp 0 là nửa đêm, và một ngôi làng nửa đêm là mười người nằm ngủ trong mười
/// cái nhà đóng cửa. Người chơi mở trò chơi ra sẽ thấy đúng thứ họ đã phàn nàn:
/// một thế giới không có gì xảy ra cả. Bắt đầu vào lúc cả làng vừa dậy là cách
/// rẻ nhất để cảnh đầu tiên có người đi lại trên đường.
///
/// Chọn theo **phần trăm ngày** chứ không phải một hằng số nhịp, để đổi độ dài
/// ngày không lặng lẽ đẩy thế giới về lại giữa đêm.
pub const DAY_START: u64 = TICKS_PER_DAY * 30 / 100;

/// Chênh cao tối đa giữa hai ô kề nhau mà vẫn dựng nhà được, tính bằng mét.
///
/// Con số này là một đánh đổi đo được, không phải một ước đoán. Trên seed 42,
/// trong ô vuông 53×53 quanh chỗ ở được:
///
/// | Ngưỡng | Ô dựng được | Cỡ làng |
/// |---|---|---|
/// | 2 m | 731 / 2809 | 147 ô — không đọc ra một cộng đồng |
/// | 3 m | 1305 / 2809 | 257 ô |
/// | 4 m | 1792 / 2809 | 646 ô |
///
/// Chọn 4: đủ đất để có một ngôi làng thật, và vẫn loại được cái vách 64 mét mà
/// bản đầu đã dựng nhà vắt qua — đường sỏi chạy thẳng xuống vực, nửa làng chìm
/// trong bóng đổ của chính nó.
pub const MAX_BUILD_SLOPE_M: i64 = 4;

/// Bán kính quét quanh chỗ ở được để tìm khoảnh đất bằng nhất.
pub const FLAT_SEARCH_RADIUS: i64 = 40;

/// Ô thay thế cho toạ độ ngoài thế giới.
///
/// Chìm dưới nước có chủ ý: rìa bản đồ không đi vào được và không dựng được, và
/// một ô "đất khô mặc định" ở đó sẽ mời người chơi đi ra khỏi thế giới.
fn edge_cell(gen: &Worldgen) -> BaseCell {
    BaseCell {
        elevation: mow_worldgen::Elevation {
            height_m: 0,
            slope: 0,
            submerged: true,
        },
        climate: gen
            .base_cell(0, 0)
            .map_or_else(|_| unreachable!("ô gốc luôn sinh được"), |b| b.climate),
        flow: mow_worldgen::Flow {
            dx: 0,
            dy: 0,
            accumulation: 0,
            is_river: false,
            is_water_body: true,
        },
        strata: mow_worldgen::Strata {
            surface: mow_worldgen::Material::Water,
            soil_depth_m: 0,
            bedrock_depth_m: 0,
            ore_present: false,
            cave: false,
        },
        biome: mow_worldgen::Biome::Alpine,
    }
}

/// Các pha của một mảnh ruộng, theo thứ tự.
///
/// Bốn pha chứ không phải hai: một ruộng chỉ có "trống" và "xanh" thì nhấp nháy
/// giữa hai trạng thái, còn bốn pha đọc ra một **mùa vụ**. Tên vật liệu phải có
/// thật trong content pack (`§19.7`) — nếu thiếu, bản đồ hiện màu tím và người
/// chơi sẽ đi báo lỗi cho renderer.
pub const CROP_STAGES: [&str; 4] = ["farmland", "crop_sprout", "crop_green", "crop_ripe"];

/// Bao nhiêu ngày một pha kéo dài.
///
/// Năm ngày mỗi pha ⇒ hai mươi ngày một mùa vụ. Đủ dài để chín là một sự kiện
/// đáng chờ, đủ ngắn để người chơi ở tốc độ ×25 thấy trọn một vụ trong khoảng
/// một phút thật.
pub const CROP_DAYS_PER_STAGE: u64 = 5;

/// Độ mòn tối đa của một ô. Trên mức này thì thêm bước chân cũng không đổi gì.
pub const TRAMPLE_MAX: u32 = 255;

/// Mỗi bước chân thêm bao nhiêu độ mòn.
///
/// Mười hai nghĩa là khoảng hai mươi mốt lần đi qua thì ô chạm trần. Với một
/// cư dân đi làm mỗi ngày, con đường từ nhà ra ruộng hiện rõ sau khoảng ba
/// tuần trong thế giới — đủ lâu để nó **có nghĩa**, đủ nhanh để người chơi
/// nhìn thấy nó hình thành ở tốc độ ×25.
pub const TRAMPLE_PER_STEP: u32 = 12;

/// Mỗi ngày, mỗi ô mòn nguôi đi bao nhiêu.
///
/// Trừ một số cố định chứ **không** nhân với một phân số: nhân số nguyên với
/// `15/16` sẽ cắt cụt mọi giá trị nhỏ về 0 ngay lập tức, và một lối mòn nhạt
/// sẽ biến mất sau đúng một đêm thay vì phai dần. Đây cùng loại lỗi với việc
/// chia trước khi cộng dồn mà dự án này đã dính ba lần.
pub const TRAMPLE_DECAY_PER_DAY: u32 = 3;

/// Trần số ô giữ trong bảng lối mòn.
///
/// Một ván rất dài với người chơi lang thang khắp bản đồ có thể chạm trần này.
/// Khi đó bỏ những ô **nhạt nhất**: chúng đúng là những ô sắp biến mất sau vài
/// ngày nữa, nên mất chúng sớm hơn một chút không đổi thứ người chơi thấy.
pub const TRAMPLE_CAP: usize = 200_000;

/// Trần số ô giữ trong bảng nhớ địa hình. Vượt là xóa sạch và dựng lại.
pub const TERRAIN_CACHE_CAP: usize = 250_000;

/// Mỗi ngày no đủ, cư dân bớt đói đi chừng này điểm trên thang `need.hunger`
/// (0 là no hẳn, càng cao càng đói — xem tài liệu `HUNGER_STARVING`).
///
/// Nhỏ hơn nhiều so với [`HUNGER_GAIN_ON_EMPTY_GRANARY`] một cách cố ý: no đủ
/// là hồi phục **dần**, không phải một cú xóa sạch cơn đói đã tích từ ba ngày
/// thiếu trước đó.
pub const HUNGER_EASE_PER_DAY: i64 = 250;

/// Kho lương cạn (`Stock::food <= 0`) thì mỗi ngày cư dân đói thêm chừng này —
/// mạnh hơn hẳn mức hồi phục bình thường, vì một ngày không có gì để chia
/// không phải "chưa kịp ăn", nó là một ngày nhịn đói thật.
///
/// Chọn `1_500`: từ mức đói khởi điểm (`1_500..=2_700`, xem `spawn_villager`),
/// một làng mất sạch kho lương sẽ đẩy cư dân đói nhất qua ngưỡng
/// [`HUNGER_STARVING`] (`8_500` trên cùng thang) trong khoảng bốn đến năm
/// ngày — cùng cỡ với khoảng ngày mà `mow_econ::village` đã đo cho việc cạn
/// kho (một làng mất hết Farmer thì cạn kho ở ngày 13, sau khi đã báo trước
/// từ ngày 9): không chết ngay hôm kho vừa hết, nhưng cũng không phải một
/// nguy cơ mơ hồ kéo dài hàng tháng mà người chơi không kịp thấy hậu quả.
pub const HUNGER_GAIN_ON_EMPTY_GRANARY: i64 = 1_500;

/// Dựng thế giới với đúng bộ handler của lát cắt (`mow_scenario::slice`),
/// cộng hai động từ của vòng kinh tế: `econ.day` và `econ.shortage`.
///
/// `slice_handlers()` trả về một `HandlerRegistry` bình thường — có thể thêm
/// handler vào nó trước khi đưa cho `Sim::new`, không cần sửa `mow-scenario`.
/// Đây là cách hợp lệ để mở rộng bộ động từ mà vẫn giữ đúng ranh giới sở hữu
/// crate của nhiệm vụ này.
///
/// Cả hai handler đều **không tính toán gì**: mọi con số đã được
/// `mow_econ::village::run_day` chốt xong trước khi lệnh được gửi (xem
/// `Game::run_economy_day`), nên handler chỉ việc phát lại đúng payload đã
/// nhận qua `emit_caused`. Tính lại ở đây sẽ là hai nơi cùng giữ một logic
/// kinh tế — sớm muộn chúng lệch nhau.
///
/// `Game::replay` cũng phải dựng thế giới bằng hàm này, không phải
/// `slice::build_empty_world`: nếu không, một nhật ký có lệnh kinh tế sẽ
/// không phát lại được trên bản sao, và preview sẽ báo "thế giới đã lệch" kể
/// từ ngày đầu tiên có kinh tế.
fn build_world(seed: u64) -> Sim {
    let mut handlers = slice::slice_handlers();
    handlers
        .on("econ.day", |ctx| {
            ctx.emit_caused(EventDraft::new("econ.day", ctx.command.payload.clone()));
            Ok(())
        })
        .on("econ.shortage", |ctx| {
            ctx.emit_caused(EventDraft::new(
                "econ.shortage",
                ctx.command.payload.clone(),
            ));
            Ok(())
        });

    Sim::new(
        SimConfig {
            world: WORLD,
            branch: BranchId(1),
            seed: WorldSeed(seed),
            clock: Clock::synchronous(),
        },
        handlers,
    )
}

/// Đóng gói một `Stock` thành payload sự kiện.
///
/// Dùng chung cho cả ba trường của `DayReport` (`stock`, `produced`,
/// `consumed`) vì cả ba đều **cùng kiểu** — đó chính là bất biến cân đối mà
/// tài liệu `mow_econ::village::DayReport` mô tả.
fn stock_value(s: &Stock) -> Value {
    Value::Map(
        [
            ("food".to_owned(), Value::Int(s.food)),
            ("water".to_owned(), Value::Int(s.water)),
            ("wood".to_owned(), Value::Int(s.wood)),
            ("tools".to_owned(), Value::Int(s.tools)),
            ("seed_grain".to_owned(), Value::Int(s.seed_grain)),
            ("soil_fertility".to_owned(), Value::Int(s.soil_fertility)),
        ]
        .into_iter()
        .collect(),
    )
}

/// Khóa ổn định của một loại thiếu hụt, để lưu vào payload sự kiện.
///
/// Cùng lý do với `intent_key`: không dùng `format!("{shortage:?}")`, vì
/// `Debug` đổi theo cách enum được khai báo, không cam kết gì với người đọc
/// ở đầu kia của một API sau này.
fn shortage_key(s: Shortage) -> &'static str {
    match s {
        Shortage::Food => "food",
        Shortage::Water => "water",
        Shortage::Wood => "wood",
        Shortage::Tools => "tools",
        Shortage::SeedGrain => "seed_grain",
        Shortage::Fertility => "fertility",
    }
}

/// Đổi vai của bộ quy hoạch sang vai của bộ lập lịch.
///
/// Hai enum tồn tại riêng là có chủ ý: quy hoạch cần biết `Keeper` để đặt người
/// coi kho, còn lập lịch thì không phân biệt được coi kho với làm đồng — cả hai
/// đều là "tới chỗ làm rồi làm việc ở đó". Gộp hai enum sẽ buộc một trong hai
/// bên mang một khái niệm nó không dùng.
fn settle_role(r: SettleRole) -> Role {
    match r {
        SettleRole::Farmer | SettleRole::Keeper => Role::Farmer,
        SettleRole::Smith => Role::Smith,
        SettleRole::Hunter => Role::Hunter,
        SettleRole::Elder => Role::Elder,
        SettleRole::Child => Role::Child,
    }
}

/// Khóa ổn định của một ý định, để lưu vào thuộc tính và để i18n tra ra chữ.
///
/// **Không** dùng `format!("{intent:?}")`. `Debug` là để lập trình viên đọc lúc
/// gỡ lỗi; nó đổi khi ai đó thêm một trường vào enum, và nó đã lọt thẳng lên
/// màn hình người chơi dưới dạng `GoTo { place: Field }`. Một khóa snake_case
/// thì ổn định qua các phiên bản, dịch được, và so sánh được mà không phải phân
/// tích chuỗi.
fn intent_key(i: Intent) -> String {
    match i {
        Intent::GoTo { place } => format!("goto.{}", place_key(place)),
        Intent::Sleep => "sleep".to_owned(),
        Intent::Eat => "eat".to_owned(),
        Intent::Work => "work".to_owned(),
        Intent::Socialize { .. } => "socialize".to_owned(),
        Intent::Idle => "idle".to_owned(),
    }
}

/// Khóa ổn định của một nơi chốn.
fn place_key(p: Place) -> &'static str {
    match p {
        Place::Home => "home",
        Place::Workplace => "workplace",
        Place::Well => "well",
        Place::Square => "square",
        Place::Field => "field",
    }
}

/// Tên ổn định của một vai, để lưu vào thuộc tính.
fn role_name(r: Role) -> &'static str {
    match r {
        Role::Farmer => "farmer",
        Role::Smith => "smith",
        Role::Hunter => "hunter",
        Role::Elder => "elder",
        Role::Child => "child",
    }
}

/// Đọc vai từ thuộc tính. Tên lạ rơi về `Farmer`.
///
/// Rơi về chứ không panic: một content pack có thể đặt vai mà server chưa biết,
/// và một cư dân làm ruộng thì vẫn là một cư dân — còn một server sập thì không.
fn role_from(s: &str) -> Role {
    match s {
        "smith" => Role::Smith,
        "hunter" => Role::Hunter,
        "elder" => Role::Elder,
        "child" => Role::Child,
        _ => Role::Farmer,
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
fn find_habitable(gen: &Worldgen) -> (i64, i64) {
    let is_dry = |x: i64, y: i64| {
        gen.base_cell(x, y).is_ok_and(|c| {
            !c.elevation.submerged
                && !c.flow.is_water_body
                && c.strata.surface != mow_worldgen::Material::Water
        })
    };
    if is_dry(0, 0) {
        return (0, 0);
    }

    let mut tam = (0i64, 0i64);
    for (buoc, ban_kinh) in [(256i64, 12_288i64), (16, 256), (1, 16)] {
        match scan_rings(tam, buoc, ban_kinh, &is_dry) {
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
fn scan_rings(
    tam: (i64, i64),
    buoc: i64,
    ban_kinh: i64,
    is_dry: &impl Fn(i64, i64) -> bool,
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
        if let Some(p) = ung_vien.into_iter().find(|(x, y)| is_dry(*x, *y)) {
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
    #[ignore = "do toc do, chay tay bang --ignored"]
    fn bench_tick_cost() {
        let mut g = Game::new(42);
        for _ in 0..2_400 {
            g.tick_once();
        }
        let t0 = std::time::Instant::now();
        const N: u32 = 2_000;
        for _ in 0..N {
            g.tick_once();
        }
        let per = t0.elapsed().as_micros() / u128::from(N);
        eprintln!("tick: {per} us  => {} tick/s", 1_000_000 / per.max(1));

        let t1 = std::time::Instant::now();
        for _ in 0..200 {
            let _ = g.state_hash();
        }
        eprintln!("state_hash: {} us", t1.elapsed().as_micros() / 200);
    }

    #[test]
    fn asking_for_a_tile_twice_does_not_run_worldgen_twice() {
        // Đây là bài cho một lỗi đã đo được, không phải một lo xa.
        //
        // Bản đồ thu nhỏ hỏi 128×128 = 16.384 ô mỗi 2,5 giây, và `tile` từng gọi
        // thẳng `Worldgen::base_cell` cho **mỗi** ô — trong khi đang giữ khóa
        // thế giới. Hệ quả đo trên server đang chạy: luồng tick tụt từ 4.480
        // xuống **120** nhịp mỗi giây ngay khi một trình duyệt nối vào, tức là
        // thanh tốc độ ×100 thực ra chạy chậm hơn ×1. Sau khi nhớ cả `BaseCell`:
        // 4.400 nhịp mỗi giây với đúng cái trình duyệt đó.
        //
        // Kiểm **cơ chế**, không kiểm đồng hồ: một bài đo thời gian sẽ đỏ tuỳ
        // máy, còn "hỏi lần hai không sinh thêm mục nhớ" thì đúng ở mọi máy.
        let g = Game::new(42);
        let _ = g.tile(9_000, 9_000);
        let sau_lan_dau = g.terrain.borrow().len();
        let _ = g.tile(9_000, 9_000);
        assert_eq!(
            g.terrain.borrow().len(),
            sau_lan_dau,
            "hỏi lại cùng một ô đã sinh thêm một mục nhớ — tức là đã chạy lại worldgen"
        );
    }

    #[test]
    fn a_repeated_minimap_batch_is_far_cheaper_than_the_first() {
        // Bản đồ thu nhỏ hỏi **cùng một** lô 128×128 mỗi 2,5 giây. Lần đầu phải
        // sinh thật và tốn thật; từ lần thứ hai trở đi nó phải gần như miễn phí,
        // và đó chính là thứ đã đưa luồng tick từ 120 lên 4.400 nhịp mỗi giây.
        //
        // So **tỉ lệ** chứ không so một con số tuyệt đối: một trần tính bằng
        // mili giây sẽ đỏ tuỳ máy và tuỳ bản build, còn "lần hai rẻ hơn nhiều
        // lần" thì đúng ở mọi nơi. Ghi nhận trung thực: lần **đầu** vẫn đắt
        // (khoảng một giây ở bản debug), nên kéo bản đồ sang vùng chưa từng
        // sinh vẫn giữ khóa một lúc — đó là việc còn phải làm, không phải việc
        // đã xong.
        let g = Game::new(42);
        let quet = |g: &Game| {
            let t0 = std::time::Instant::now();
            for dy in 0..128 {
                for dx in 0..128 {
                    let _ = g.tile(180 + dx, 190 + dy);
                }
            }
            t0.elapsed().as_micros().max(1)
        };
        let dau = quet(&g);
        let sau = quet(&g);
        assert!(
            sau * 5 < dau,
            "lần hai ({sau} us) không rẻ hơn lần đầu ({dau} us) tới năm lần —              bảng nhớ ô worldgen đã hỏng"
        );
    }

    #[test]
    fn fields_go_through_a_season() {
        // Ruộng đứng im quanh năm là thứ nói cho người chơi biết thế giới này
        // chỉ giả vờ sống.
        let mut g = Game::new(42);
        assert!(!g.crops.is_empty(), "làng không có thửa ruộng nào");

        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..(TICKS_PER_DAY * CROP_DAYS_PER_STAGE * 5) {
            g.tick_once();
            for at in g.crops.keys() {
                if let Some(m) = g.overrides.get(at) {
                    seen.insert(m.clone());
                }
            }
        }
        assert_eq!(
            seen.len(),
            CROP_STAGES.len(),
            "một mùa vụ chỉ đi qua {} pha: {seen:?}",
            seen.len()
        );
    }

    #[test]
    fn every_crop_stage_is_a_real_material() {
        // Một pha trỏ tới vật liệu không có trong content pack sẽ hiện màu tím
        // trên bản đồ, và người chơi sẽ đi báo lỗi cho renderer (`§19.7`).
        // Đường dẫn tính từ `CARGO_MANIFEST_DIR`, không từ thư mục làm việc:
        // `cargo test` chạy với cwd là thư mục crate, còn server chạy từ `src/`.
        // Một bài test phụ thuộc cwd là một bài test đỏ tuỳ chỗ ai gõ lệnh.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/core/blocks");
        for stage in CROP_STAGES {
            let p = dir.join(stage).join("metadata.yaml");
            assert!(p.exists(), "pha `{stage}` không có định nghĩa ở {p:?}");
        }
    }

    #[test]
    fn fields_do_not_all_ripen_on_the_same_day() {
        // Cả làng gặt cùng một hôm trông như một cỗ máy, không như một cộng
        // đồng nơi mỗi thửa gieo một ngày khác nhau.
        let g = Game::new(42);
        let stages: std::collections::BTreeSet<u8> = g.crops.values().copied().collect();
        assert!(stages.len() > 1, "mọi thửa ruộng khởi đầu ở cùng một pha");
    }

    #[test]
    fn walking_wears_a_path_into_the_ground() {
        // Không ai vẽ con đường từ nhà ra ruộng. Nó hiện ra vì người ta đi qua
        // đó mỗi ngày — và đó là thứ rẻ nhất biến một bản đồ **đúng** thành một
        // bản đồ **có người sống**.
        let mut g = Game::new(42);
        for _ in 0..400 {
            g.tick_once();
        }
        let worn: usize = g.trample.values().filter(|v| **v > 0).count();
        assert!(
            worn > 0,
            "bốn trăm nhịp mà không một bàn chân nào để lại dấu"
        );

        // Và nó phải nằm ở chỗ **có người đi**, không rải đều khắp bản đồ.
        let deepest = g.trample.values().copied().max().unwrap_or(0);
        assert!(
            deepest >= TRAMPLE_PER_STEP,
            "vết mòn sâu nhất chỉ {deepest}"
        );
    }

    #[test]
    fn paths_fade_where_nobody_walks_any_more() {
        // Trừ một số cố định chứ **không** nhân với một phân số: nhân số nguyên
        // với `15/16` cắt cụt mọi giá trị nhỏ về 0 ngay, và một lối mòn nhạt sẽ
        // biến mất sau đúng một đêm thay vì phai dần.
        let mut g = Game::new(42);
        g.tread(1_000, 1_000);
        g.tread(1_000, 1_000);
        let before = g.trample_at(1_000, 1_000);
        assert_eq!(before, TRAMPLE_PER_STEP * 2);

        g.fade_trample();
        let after = g.trample_at(1_000, 1_000);
        assert_eq!(after, before - TRAMPLE_DECAY_PER_DAY);
        assert!(after > 0, "một đêm không được xoá sạch một lối mòn");

        // Đủ nhiều đêm thì nó biến mất hẳn, và ô được thu hồi khỏi bảng.
        for _ in 0..100 {
            g.fade_trample();
        }
        assert_eq!(g.trample_at(1_000, 1_000), 0);
        assert!(!g.trample.contains_key(&(1_000, 1_000)));
    }

    #[test]
    fn worn_paths_are_part_of_the_world_hash() {
        // Lối mòn là **sự thật về thế giới**. Để nó ngoài `state_hash` nghĩa là
        // "xác định" chỉ đúng với phần thế giới mà ta nhớ đưa vào hash — đúng
        // loại lỗi mà `overrides` đã từng mắc.
        let a = Game::new(42);
        let mut b = Game::new(42);
        assert_eq!(a.state_hash(), b.state_hash());
        b.tread(5, 5);
        assert_ne!(
            a.state_hash(),
            b.state_hash(),
            "giẫm một bàn chân mà hash không đổi"
        );
    }

    #[test]
    fn the_true_god_has_no_body() {
        // Người chơi hỏi thẳng: *"tại sao mặc định true god lại có cơ thể?"*.
        // Câu trả lời đúng là "không có" — và bài này giữ nó ở đó.
        let g = Game::new(42);
        let named: Vec<&str> = g
            .sim()
            .store()
            .with_attr("core.name")
            .filter_map(|id| g.sim().store().attr_text(id, "core.name"))
            .collect();
        assert!(
            !named.contains(&"Nguoi Choi"),
            "thế giới vẫn còn một thân xác của người chơi: {named:?}"
        );
        // Và mọi sinh mệnh đều phải là cư dân thật, không phải đồ dựng tạm của
        // bài test lát cắt. Vật phẩm thì được: kho lương của làng là thứ có
        // thật trong thế giới.
        assert!(!named.is_empty(), "thế giới không có ai cả");
        for id in g.sim().store().with_attr("core.name") {
            let is_item = g.sim().store().attr_int(id, "item.nutrition").is_some();
            assert!(
                is_item || g.sim().store().attr_text(id, "npc.role").is_some(),
                "thực thể {id:?} không phải cư dân cũng không phải vật phẩm —                  đồ dựng tạm lọt vào thế giới thật"
            );
        }
    }

    /// Một cư dân bất kỳ, để các bài test cũ có ai đó mà ra lệnh.
    ///
    /// Trước đây chúng dùng avatar. Giờ không có avatar, và một vị thần ra lệnh
    /// cho cư dân là đúng cách trò chơi vận hành.
    fn a_villager(g: &Game) -> EntityId {
        g.sim()
            .store()
            .with_attr("npc.role")
            .next()
            .expect("làng phải có cư dân")
    }

    fn eye_pos(g: &Game) -> (i64, i64) {
        g.eye()
    }

    /// Vị trí của một thực thể. Các bài dưới từng dùng vị trí avatar; giờ chúng
    /// phải hỏi đúng người mà chúng đang ra lệnh.
    fn pos_of(g: &Game, who: EntityId) -> (i64, i64) {
        (g.attr_int(who, "core.pos.x"), g.attr_int(who, "core.pos.y"))
    }

    #[test]
    fn the_starting_slice_is_the_ground_under_the_gaze() {
        // Không phải `z = 0`: nếu avatar ở trên một ngọn đồi thì lát 0 nằm
        // trong đá và màn hình đầu tiên đen đặc.
        let g = Game::new(42);
        let (ax, ay) = eye_pos(&g);
        let t = g.tile(ax, ay);
        assert_eq!(g.z(), t.height);
        assert_ne!(t.material, "air", "ô dưới chân phải là chất rắn");
    }

    #[test]
    fn the_gaze_never_opens_under_the_sea() {
        // `build_slice_world` đặt mọi thứ ở `(0, 0)` vì test không quan tâm địa
        // hình. Với seed 42, ô đó nằm dưới mực biển — người chơi mở game ra và
        // thấy mình ở đáy biển. Bài này giữ cho chuyện đó không quay lại, ở
        // nhiều seed chứ không chỉ seed may mắn.
        for seed in [1u64, 7, 42, 1234, 99_999] {
            let g = Game::new(seed);
            let (ax, ay) = eye_pos(&g);
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
        assert!(
            khac > 100,
            "hai seed chỉ khác {khac} ô — worldgen bỏ qua seed?"
        );
    }

    #[test]
    fn doi_lat_z_doi_vat_lieu() {
        let g = Game::new(42);
        let (ax, ay) = eye_pos(&g);
        let mut g2 = Game::new(42);

        g2.set_z(g.z() + 5);
        assert_eq!(
            g2.tile(ax, ay).material,
            "air",
            "5 m trên đầu phải là không khí"
        );

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
    fn speed_does_not_change_the_world() {
        // Tốc độ là thứ của người xem, không phải của thế giới. Cùng số tick
        // phải cho cùng hash bất kể chạy nhanh hay chậm — nếu không thì tua
        // nhanh sẽ lặng lẽ tạo ra một lịch sử khác.
        let mut slow = Game::new(11);
        let mut fast = Game::new(11);
        slow.set_speed_milli(1);
        fast.set_speed_milli(MAX_SPEED_MILLI);
        for _ in 0..50 {
            slow.tick_once();
            fast.tick_once();
        }
        assert_eq!(slow.state_hash(), fast.state_hash());
    }

    #[test]
    fn slowest_speed_still_advances_eventually() {
        // ×0.001 phải là **chậm**, không phải **đứng**. Chia số nguyên không
        // mang dư sẽ làm tròn về 0 tick mỗi nhịp và thế giới đứng im mãi mãi —
        // cùng lớp lỗi đã cắn ở tỉ lệ đột biến và ở ngăn S/E/I/R.
        let mut g = Game::new(3);
        g.set_speed_milli(1);
        let mut carry = 0u64;
        let mut total = 0u32;
        // Ở ×0.001 với nhịp gốc 300 ms, một tick tốn 300 giây thật, tức 6000
        // nhịp 50 ms. Chạy 8000 nhịp để chắc chắn vượt qua mốc đó.
        for _ in 0..8_000 {
            total += g.ticks_due(50, 300, &mut carry);
        }
        assert!(total > 0, "×0.001 phải nhích được, không phải đứng im");
    }

    #[test]
    fn paused_means_zero_ticks() {
        let mut g = Game::new(3);
        g.set_speed_milli(0);
        let mut carry = 0u64;
        assert_eq!(g.ticks_due(1_000, 300, &mut carry), 0);
    }

    #[test]
    fn fastest_speed_is_capped_per_wake() {
        // Không có trần thì một luồng bị treo lâu sẽ chạy bù hàng nghìn tick
        // trong lúc giữ khóa, và mọi yêu cầu HTTP đứng theo.
        let mut g = Game::new(3);
        g.set_speed_milli(MAX_SPEED_MILLI);
        let mut carry = 0u64;
        assert_eq!(g.ticks_due(600_000, 300, &mut carry), MAX_TICKS_PER_WAKE);
    }

    #[test]
    fn speed_is_clamped_not_wrapped() {
        let mut g = Game::new(3);
        g.set_speed_milli(u32::MAX);
        assert_eq!(g.speed_milli(), MAX_SPEED_MILLI);
    }

    #[test]
    fn every_material_the_map_can_show_has_a_definition() {
        // Đây là bài test giữ đúng lời hứa của `§19.7`. Không có nó, thêm một
        // vật liệu vào `mow-worldgen` mà quên thư mục `content/` sẽ cho ra một
        // ô màu tím trên bản đồ và không có gì báo cho tới khi ai đó nhìn thấy.
        let mut g = Game::new(42);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/core");
        g.load_content(root.to_str().expect("đường dẫn hợp lệ"))
            .expect("content pack `core` phải nạp được");
        let blocks = g.blocks().expect("vừa nạp xong");

        for m in [
            mow_worldgen::Material::Air,
            mow_worldgen::Material::Water,
            mow_worldgen::Material::Ice,
            mow_worldgen::Material::Sand,
            mow_worldgen::Material::Topsoil,
            mow_worldgen::Material::Clay,
            mow_worldgen::Material::Sedimentary,
            mow_worldgen::Material::Metamorphic,
            mow_worldgen::Material::Igneous,
            mow_worldgen::Material::Ore,
            mow_worldgen::Material::Magma,
        ] {
            assert!(
                blocks.contains(m.as_str()),
                "vật liệu `{}` không có định nghĩa trong content/core/blocks",
                m.as_str()
            );
        }
    }

    #[test]
    fn missing_content_is_not_fatal() {
        // Pack hỏng thì bản đồ vẽ bằng màu dự phòng, không phải màn hình trắng.
        let mut g = Game::new(42);
        assert!(g.load_content("khong-co-thu-muc-nay").is_err());
        assert!(g.blocks().is_none());
        assert_ne!(g.tile(0, 0).material, "", "vẫn phải vẽ được");
    }

    fn walk_cmd(who: EntityId, dx: i64, dy: i64) -> Command {
        Command::new(
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
        )
    }

    #[test]
    fn a_villagers_step_traces_back_to_the_intent_behind_it() {
        // `§18.10`: chuỗi nhân quả phải trả lời được *vì sao*, không chỉ *cái
        // gì*. Trước bài này mọi chuỗi dài đúng một mắt — tính năng "chạy" mà
        // rỗng nghĩa.
        let mut g = Game::new(42);
        for _ in 0..600 {
            g.tick_once();
        }
        let moved = g
            .sim()
            .log()
            .iter()
            .filter(|e| e.kind.0 == "core.entity.moved" && e.cause.is_some())
            .last();
        let e = moved.expect("phải có một bước đi mang nguyên nhân");
        let chain = g.sim().log().cause_chain(e.seq, 8);
        assert!(chain.len() >= 2, "chuỗi chỉ dài {}", chain.len());
        assert!(
            chain.iter().any(|c| c.kind.0 == "npc.intended"),
            "chuỗi không dẫn về một ý định: {:?}",
            chain.iter().map(|c| &c.kind.0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn intent_labels_are_stable_keys_not_debug_output() {
        // `format!("{intent:?}")` đã từng lọt lên màn hình dưới dạng
        // `GoTo { place: Field }`. Một khóa hiển thị được thì không có dấu ngoặc
        // nhọn, không có khoảng trắng, và dịch được.
        let mut g = Game::new(42);
        for _ in 0..600 {
            g.tick_once();
        }
        let mut seen = 0_usize;
        for who in g.sim().store().with_attr("npc.intent") {
            let k = g
                .sim()
                .store()
                .attr_text(who, "npc.intent")
                .expect("vừa lọc theo khóa này");
            assert!(
                k.chars().all(|c| c.is_ascii_lowercase() || c == '.'),
                "nhãn ý định `{k}` không phải khóa ổn định"
            );
            seen += 1;
        }
        assert!(seen > 0, "không có cư dân nào mang ý định");
    }

    #[test]
    fn an_unchanged_intent_is_not_logged_again() {
        // Ghi lại cùng một ý định mỗi tick sẽ nhấn chìm nhật ký.
        let mut g = Game::new(42);
        for _ in 0..600 {
            g.tick_once();
        }
        let intents = g
            .sim()
            .log()
            .iter()
            .filter(|e| e.kind.0 == "npc.intended")
            .count();
        let moves = g
            .sim()
            .log()
            .iter()
            .filter(|e| e.kind.0 == "core.entity.moved")
            .count();
        assert!(
            intents < moves,
            "{intents} ý định cho {moves} bước đi — ý định đang được ghi mỗi tick"
        );
    }

    #[test]
    fn the_world_starts_with_a_village_not_three_dots() {
        // Đây là bài test cho đúng lời phàn nàn đã dẫn tới Giai đoạn G: "tôi là
        // một vị thần mà nơi bắt đầu trông chả ra làm sao".
        let g = Game::new(42);
        assert!(
            g.built_cells() > 200,
            "làng chỉ có {} ô — chưa đủ để đọc ra là một cộng đồng",
            g.built_cells()
        );
        let residents = g.sim().store().with_attr("npc.role").count();
        assert!(residents >= 8, "chỉ có {residents} cư dân");
    }

    #[test]
    fn the_village_stands_on_ground_gentle_enough_to_build_on() {
        // Bản đầu chỉ hỏi "có phải nước không" trước khi đặt nhà, và ngôi làng
        // đầu tiên nằm vắt qua một vách 64 mét: đường sỏi chạy thẳng xuống vực,
        // và nửa làng chìm trong bóng đổ của chính cái vách đó. Không có bài
        // test nào bắt được — nó chỉ lộ ra khi nhìn màn hình.
        let g = Game::new(42);
        let steep: Vec<(i64, i64)> = g
            .overrides
            .keys()
            .copied()
            .filter(|(x, y)| g.slope_m(*x, *y) > MAX_BUILD_SLOPE_M)
            .collect();
        assert!(
            steep.is_empty(),
            "{} ô của làng nằm trên dốc quá đứng, ví dụ {:?}",
            steep.len(),
            &steep[..steep.len().min(4)]
        );
    }

    #[test]
    fn every_villager_has_a_home_and_a_workplace() {
        // Một cư dân không có nhà sẽ đứng im mãi mãi ở pha "về nhà".
        let g = Game::new(42);
        for who in g.sim().store().with_attr("npc.role") {
            for k in ["npc.home.x", "npc.home.y", "npc.work.x", "npc.work.y"] {
                assert!(
                    g.sim().store().attr_int(who, k).is_some(),
                    "cư dân {who:?} thiếu `{k}`"
                );
            }
        }
    }

    #[test]
    fn the_village_stands_on_dry_land() {
        let g = Game::new(42);
        let wet = g
            .overrides
            .keys()
            .filter(|(x, y)| !g.walkable(*x, *y))
            .count();
        assert_eq!(wet, 0, "{wet} ô của làng nằm dưới nước");
    }

    #[test]
    fn villagers_live_a_varied_day() {
        // Cư dân chỉ làm một việc cả ngày là cư dân chết. Quét một ngày đầy đủ
        // và đếm số ý định khác nhau mà cả làng sinh ra.
        let mut g = Game::new(42);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..TICKS_PER_DAY / 4 {
            g.tick_once();
            for who in g.sim().store().with_attr("npc.intent") {
                if let Some(i) = g.sim().store().attr_text(who, "npc.intent") {
                    seen.insert(i.to_owned());
                }
            }
        }
        assert!(
            seen.len() >= 3,
            "cả làng chỉ có {} loại ý định: {seen:?}",
            seen.len()
        );
    }

    #[test]
    fn the_village_is_the_same_for_the_same_seed() {
        let a = Game::new(7);
        let b = Game::new(7);
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.built_cells(), b.built_cells());
    }

    #[test]
    fn edited_terrain_changes_the_world_hash() {
        // Nếu địa hình không vào hash thì hai thế giới có hai ngôi làng khác
        // nhau cho cùng một hash, và replay mất khả năng phát hiện sai lệch.
        let mut g = Game::new(42);
        let before = g.state_hash();
        g.set_cell(10, 20, "path_gravel");
        assert_ne!(g.state_hash(), before);
    }

    #[test]
    fn a_world_without_edited_cells_hashes_exactly_like_its_sim() {
        // Giữ tính chất này nghĩa là mọi công cụ đã so hash từ trước vẫn đúng.
        //
        // Không dùng `Game::new` được nữa: từ khi mọi thế giới mọc lên một ngôi
        // làng, không còn thế giới nào có `overrides` rỗng, và không còn kho
        // nào bằng `Stock::default()`. Bài test hỏi về **phép trộn**, nên nó
        // hỏi thẳng phép trộn, với cả ba kênh ở trạng thái trung tính.
        let g = Game::new(42);
        let sim_hash = g.sim().state_hash();
        assert_eq!(
            Game::mix(
                sim_hash,
                &BTreeMap::new(),
                &Stock::default(),
                &Workforce::default(),
                &BTreeMap::new()
            ),
            sim_hash
        );
    }

    #[test]
    fn terrain_hash_ignores_the_order_cells_were_placed() {
        let mut a = Game::new(42);
        let mut b = Game::new(42);
        a.set_cell(1, 1, "farmland");
        a.set_cell(5, 3, "roof_dark");
        b.set_cell(5, 3, "roof_dark");
        b.set_cell(1, 1, "farmland");
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn preview_still_works_after_terrain_edits() {
        // Preview phát lại nhật ký **lệnh**; địa hình không nằm trong nhật ký,
        // nên nó phải được trộn vào hai bên khi so sánh.
        let mut g = Game::new(42);
        g.set_cell(3, 3, "path_gravel");
        let who = a_villager(&g);
        let d = g
            .preview(&walk_cmd(who, 1, 0))
            .expect("vẫn phải xem trước được");
        assert_eq!(d.base_hash, g.state_hash());
    }

    #[test]
    fn replay_rebuilds_exactly_the_world_being_viewed() {
        // Đây là tính chất làm preview có giá trị. Nếu phát lại chỉ ra một thế
        // giới *gần giống*, preview sẽ đưa ra những con số đúng về một vũ trụ
        // không tồn tại.
        let mut g = Game::new(42);
        for _ in 0..25 {
            g.tick_once();
        }
        let who = a_villager(&g);
        g.apply(&walk_cmd(who, 1, 0)).unwrap();

        let d = g.preview(&walk_cmd(who, 0, 1)).expect("preview chạy được");
        assert_eq!(d.base_hash, g.state_hash());
    }

    #[test]
    fn preview_does_not_touch_the_world() {
        let g = Game::new(42);
        let who = a_villager(&g);
        let before = g.state_hash();
        let n = g.journal_len();
        let d = g.preview(&walk_cmd(who, 1, 0)).unwrap();
        assert!(d.changes_anything());
        assert_eq!(g.state_hash(), before, "xem trước đã đổi thế giới");
        assert_eq!(g.journal_len(), n, "xem trước đã ghi vào nhật ký");
    }

    #[test]
    fn preview_reports_who_moves_and_where() {
        let g = Game::new(42);
        let who = a_villager(&g);
        let (ax, ay) = pos_of(&g, who);
        let d = g.preview(&walk_cmd(who, 1, 0)).unwrap();
        let mine = d
            .changes
            .iter()
            .find(|c| c.id == who)
            .expect("người được ra lệnh phải đổi chỗ");
        assert_eq!(mine.from, Some((ax, ay)));
        assert_eq!(mine.to, Some((ax + 1, ay)));
        assert!(mine.moved());
    }

    #[test]
    fn preview_of_an_illegal_command_says_so_instead_of_lying() {
        let g = Game::new(42);
        let who = a_villager(&g);
        let d = g.preview(&walk_cmd(who, 5, 0)).unwrap();
        assert!(d.error.is_some(), "đi 5 ô một bước phải hỏng");
        assert!(!d.changes_anything());
        assert!(d.changes.is_empty());
    }

    #[test]
    fn commit_refuses_when_the_world_moved() {
        // Ràng buộc quan trọng nhất của cả console True God: thứ được khắc phải
        // đúng bằng thứ người chơi đã xem.
        let mut g = Game::new(42);
        let who = a_villager(&g);
        let d = g.preview(&walk_cmd(who, 1, 0)).unwrap();
        let stale = d.base_hash.to_hex();

        // Thế giới nhích lên trong lúc người chơi còn đang cân nhắc.
        for _ in 0..8 {
            g.tick_once();
        }

        let e = g
            .commit_checked(&walk_cmd(who, 1, 0), &stale)
            .expect_err("phải từ chối");
        assert!(matches!(e, Refusal::WorldMoved { .. }));
    }

    #[test]
    fn commit_goes_through_when_the_world_is_still() {
        let mut g = Game::new(42);
        let who = a_villager(&g);
        let d = g.preview(&walk_cmd(who, 1, 0)).unwrap();
        let (ax, ay) = pos_of(&g, who);
        let done = g
            .commit_checked(&walk_cmd(who, 1, 0), &d.base_hash.to_hex())
            .expect("thế giới chưa đổi thì phải khắc được");
        assert_eq!(g.state_hash(), done.after_hash);
        assert_eq!(pos_of(&g, who), (ax + 1, ay));
    }

    #[test]
    fn a_previewed_hash_matches_what_commit_produces() {
        // Lời hứa cốt lõi: `after_hash` của preview đúng bằng hash sau khi khắc.
        let mut g = Game::new(42);
        let who = a_villager(&g);
        let d = g.preview(&walk_cmd(who, 0, 1)).unwrap();
        g.commit_checked(&walk_cmd(who, 0, 1), &d.base_hash.to_hex())
            .unwrap();
        assert_eq!(g.state_hash(), d.after_hash, "xem trước hứa sai kết quả");
    }

    #[test]
    fn clicking_a_reachable_tile_walks_there() {
        let mut g = Game::new(42);
        let who = a_villager(&g);
        let (ax, ay) = pos_of(&g, who);
        // Một ô đất khô cách vài bước.
        let goal = (ax + 4, ay + 2);
        assert!(
            g.walkable(goal.0, goal.1),
            "chọn một đích đi được cho bài này"
        );

        let (n, why) = g.set_destination(who, goal);
        assert_eq!(why, "found");
        assert!(n > 0);

        for _ in 0..40 {
            g.tick_once();
            if g.remaining_steps(who) == 0 {
                break;
            }
        }
        assert_eq!(pos_of(&g, who), goal, "không tới được đích đã bấm");
    }

    #[test]
    fn clicking_into_the_sea_walks_to_the_shore_not_nowhere() {
        // Đứng im mà không nói gì là câu trả lời tệ nhất cho một cú bấm chuột.
        let mut g = Game::new(42);
        let who = a_villager(&g);
        let (ax, ay) = eye_pos(&g);

        // Tìm một ô nước trong tầm nhìn.
        let sea = (-60..60)
            .flat_map(|dx| (-60..60).map(move |dy| (dx, dy)))
            .map(|(dx, dy)| (ax + dx, ay + dy))
            .find(|(x, y)| !g.walkable(*x, *y));
        let Some(sea) = sea else {
            return; // seed này không có biển gần — bài không áp dụng
        };

        let (n, why) = g.set_destination(who, sea);
        assert_ne!(why, "found", "ô nước không được coi là tới được");
        if n > 0 {
            for _ in 0..80 {
                g.tick_once();
                if g.remaining_steps(who) == 0 {
                    break;
                }
            }
            assert_ne!(eye_pos(&g), (ax, ay), "đã có kế hoạch mà không nhúc nhích");
        }
    }

    #[test]
    fn a_new_destination_replaces_the_old_one() {
        // Bấm chỗ khác giữa đường phải đổi hướng ngay, không đi nốt đường cũ.
        let mut g = Game::new(42);
        let who = a_villager(&g);
        let (ax, ay) = pos_of(&g, who);
        g.set_destination(who, (ax + 6, ay));
        let first = g.remaining_steps(who);
        g.set_destination(who, (ax + 1, ay));
        assert!(g.remaining_steps(who) < first);
    }

    #[test]
    fn walking_stays_deterministic() {
        // Kế hoạch đi đi qua `Sim::apply`, nên nó nằm trong event log và phải
        // replay được. Hai thế giới cùng seed, cùng lệnh, phải cùng hash.
        let mut a = Game::new(42);
        let mut b = Game::new(42);
        let (ax, ay) = eye_pos(&a);
        let wa = a_villager(&a);
        let wb = a_villager(&b);
        a.set_destination(wa, (ax + 5, ay + 3));
        b.set_destination(wb, (ax + 5, ay + 3));
        for _ in 0..30 {
            a.tick_once();
            b.tick_once();
        }
        assert_eq!(a.state_hash(), b.state_hash());
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
            .expect("làng phải có kho lương");
        assert!(g.placed().contains(&banh), "bánh phải đang nằm trên đất");

        let who = a_villager(&g);
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

    // ─────────────────────── Vòng kinh tế ───────────────────────

    #[test]
    fn mot_ngay_troi_qua_thi_kho_doi() {
        let mut g = Game::new(42);
        let before = *g.stock();
        assert!(g.last_day_report().is_none(), "chưa trôi ngày nào");

        let mut steps = 0;
        while g.last_day_report().is_none() {
            g.tick_once();
            steps += 1;
            assert!(steps < 10_000, "ranh giới ngày kinh tế không bao giờ tới");
        }

        assert_ne!(*g.stock(), before, "qua một ngày mà kho không đổi gì");
    }

    #[test]
    fn workforce_theo_dung_cu_dan_that() {
        let g = Game::new(42);
        let dem_lai = g.compute_workforce();
        assert_eq!(
            *g.workforce(),
            dem_lai,
            "Workforce lưu trong Game lệch với cư dân thật ngay sau khi dựng làng"
        );

        let dan_so = g.sim().store().with_attr("npc.role").count();
        assert_eq!(
            dem_lai.population(),
            i64::try_from(dan_so).unwrap_or(i64::MAX),
            "tổng Workforce phải khớp đúng số cư dân có vai trên bản đồ"
        );
    }

    #[test]
    fn doi_vai_cu_dan_thi_workforce_doi_theo_o_ngay_ke_tiep() {
        // `Workforce` không phải một hằng số hay một bản chụp giữ mãi từ lúc
        // dựng làng: nó phải đổi theo cư dân thật. Đổi vai là cách kiểm được
        // điều đó mà không cần một cơ chế "chết" chưa tồn tại ở tầng engine.
        let mut g = Game::new(42);
        let mot_nong_dan = g
            .sim()
            .store()
            .with_attr("npc.role")
            .find(|id| g.sim().store().attr_text(*id, "npc.role") == Some("farmer"))
            .expect("làng mẫu phải có ít nhất một Farmer");
        let truoc = g.compute_workforce();
        assert!(truoc.farmers > 0);

        g.dat(mot_nong_dan, "npc.role", Value::Text("hunter".to_owned()));
        g.run_economy_day(1);

        let sau = *g.workforce();
        assert_eq!(
            sau.farmers,
            truoc.farmers - 1,
            "Farmer không giảm sau khi đổi vai"
        );
        assert_eq!(
            sau.hunters,
            truoc.hunters + 1,
            "Hunter không tăng sau khi đổi vai"
        );
    }

    #[test]
    fn kho_va_luc_luong_lao_dong_di_vao_state_hash() {
        let mut g = Game::new(42);
        let before = g.state_hash();

        g.stock.food += 1;
        assert_ne!(
            g.state_hash(),
            before,
            "sửa kho tay mà state_hash không đổi — hai thế giới có kho khác \
             nhau sẽ lặng lẽ trông giống nhau"
        );

        // Trả kho về cũ, đổi lực lượng lao động thay vào — phải đổi hash độc
        // lập với kho, không phải "chỉ một trong hai trường thật sự vào hash".
        g.stock.food -= 1;
        assert_eq!(g.state_hash(), before, "trả kho về cũ mà hash không về cũ");
        g.work.farmers += 1;
        assert_ne!(
            g.state_hash(),
            before,
            "sửa lực lượng lao động mà state_hash không đổi"
        );
    }

    #[test]
    fn ngay_kinh_te_phat_econ_day_va_econ_shortage_co_nguyen_nhan() {
        let mut g = Game::new(9);
        // Ép một ngày thiếu ngay lập tức, không phải đợi kho tự cạn qua nhiều
        // ngày — bài test này hỏi về **sự kiện**, không hỏi về tốc độ cạn kho.
        g.stock = Stock {
            food: 0,
            water: 0,
            ..g.stock
        };
        g.run_economy_day(1);

        let ngay = g
            .sim()
            .log()
            .iter()
            .find(|e| e.kind.0 == "econ.day")
            .expect("phải có sự kiện econ.day mỗi ngày");
        let thieu = g
            .sim()
            .log()
            .iter()
            .find(|e| e.kind.0 == "econ.shortage")
            .expect("kho rỗng mà không báo econ.shortage");
        assert_eq!(
            thieu.cause,
            Some(ngay.seq),
            "econ.shortage phải gắn nguyên nhân về đúng econ.day đã báo nó"
        );
    }

    #[test]
    fn mat_het_farmer_thi_doi_ap_toi_trong_khoang_ngay_hop_ly() {
        // Đối chứng với `mow_econ::village`, đọc kỹ tài liệu của
        // `HUNGER_GAIN_ON_EMPTY_GRANARY`: làng mất hết Farmer thì cạn kho
        // trong một số ngày đo được, và đói phải ập tới **sau đó**, trong một
        // khoảng ngày cũng đo được — không phải ngay lập tức, không phải một
        // nguy cơ mơ hồ không bao giờ tới.
        let mut g = Game::new(42);
        let farmers: Vec<EntityId> = g
            .sim()
            .store()
            .with_attr("npc.role")
            .filter(|id| g.sim().store().attr_text(*id, "npc.role") == Some("farmer"))
            .collect();
        assert!(
            !farmers.is_empty(),
            "làng mẫu phải có Farmer để bài test có ý nghĩa"
        );
        for id in farmers {
            g.dat(id, "npc.role", Value::Text("hunter".to_owned()));
        }

        let mut kho_can_ngay: Option<u64> = None;
        let mut doi_ngay: Option<u64> = None;
        for ngay in 1..=60u64 {
            g.run_economy_day(ngay);
            if kho_can_ngay.is_none() && g.stock().food == 0 {
                kho_can_ngay = Some(ngay);
            }
            if doi_ngay.is_none() {
                let co_ai_doi = g
                    .sim()
                    .store()
                    .with_attr("npc.role")
                    .any(|id| g.attr_bool(id, "npc.starving"));
                if co_ai_doi {
                    doi_ngay = Some(ngay);
                }
            }
        }

        let kho_can_ngay =
            kho_can_ngay.expect("mất hết Farmer mà kho lương không bao giờ cạn trong 60 ngày");
        let doi_ngay =
            doi_ngay.expect("kho đã cạn mà không ai đói tới ngưỡng chết đói trong 60 ngày");
        assert!(
            doi_ngay >= kho_can_ngay,
            "có người đói tới ngưỡng chết trước cả khi kho cạn: đói ngày {doi_ngay}, cạn ngày {kho_can_ngay}"
        );
        assert!(
            doi_ngay - kho_can_ngay <= 10,
            "kho cạn rồi mà phải đợi tới {} ngày mới có người đói tới ngưỡng chết — \
             quá lâu để người chơi cảm nhận được hậu quả",
            doi_ngay - kho_can_ngay
        );
    }

    #[test]
    fn cung_seed_cung_so_tick_cho_cung_state_hash_qua_nhieu_ngay_kinh_te() {
        // Xác định phải giữ được **qua** ranh giới ngày kinh tế, không chỉ
        // trong vài chục tick đầu: 40 tick (bài `tick_xac_dinh` ở trên) không
        // chạm tới `run_economy_day` lần nào.
        let so_tick = TICKS_PER_DAY * 3 + 50;
        let mut a = Game::new(123);
        let mut b = Game::new(123);
        for _ in 0..so_tick {
            a.tick_once();
            b.tick_once();
        }
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(*a.stock(), *b.stock());
        assert_eq!(*a.workforce(), *b.workforce());
    }
}
