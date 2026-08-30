# Playground Gemini — My Open World Simulation Demo

> Bản thử nghiệm độc lập cho giao diện, đồ họa, tương tác và mô phỏng thế giới sống 2D top-down của dự án **"My Open World"**.

---

## 1. Hướng dẫn cài đặt & Cách chạy

Dự án sử dụng **Vite + TypeScript + Canvas 2D Engine** tối ưu hiệu năng cao, không phụ thuộc backend ngoài.

### Yêu cầu môi trường
- **Node.js**: phiên bản `>= 18.0.0` (khuyến nghị Node 20+)
- **npm**: phiên bản `>= 9.0.0`

### Các bước chạy

#### Cách 1: Chế độ Development (Khuyến nghị để trải nghiệm trực tiếp)
```bash
# 1. Cài đặt dependencies
npm install

# 2. Khởi động Vite dev server
npm run dev
```
Trình duyệt sẽ mở tại `http://localhost:5173`.

#### Cách 2: Build sản phẩm & Chạy Preview
```bash
npm run build
npm run preview
```

#### Cách 3: Chạy trực tiếp file tĩnh (Single-file Portable)
Sau khi chạy `npm run build`, thư mục `dist/index.html` được đóng gói thành **một file HTML duy nhất** chứa toàn bộ CSS, JS và tài nguyên inline. Bạn có thể mở trực tiếp `dist/index.html` bằng bất kỳ trình duyệt nào (Chrome, Edge, Firefox, Safari) mà không cần cấu hình server.

---

## 2. Kiến trúc hệ thống (Architecture)

Mã nguồn được tổ chức theo kiến trúc module hóa hướng đối tượng (OOP) kết hợp Data-Driven & Event-Driven Decoupled Pattern:

```
playground-gemini/
├── index.html                  # HTML entry skeleton
├── package.json                # Cấu hình dự án & dependencies
├── tsconfig.json               # Cấu hình TypeScript compiler
├── vite.config.ts              # Cấu hình Vite & single-file inlining
├── src/
│   ├── main.ts                 # Bootstrap ứng dụng, khởi tạo dân số, game loop 60 FPS
│   ├── env.d.ts                # TypeScript type declarations
│   ├── styles/
│   │   └── main.css            # Dark theme glassmorphism, responsive UI, colorblind filters
│   ├── core/
│   │   ├── Types.ts            # Định nghĩa toàn bộ Interface, Enums (Biomes, Needs, Items, States)
│   │   ├── Noise.ts            # Simplex & Perlin 2D noise, fBm đa octave, domain warping
│   │   ├── Clock.ts            # Đồng hồ mô phỏng, chu kỳ Ngày/Đêm, 4 Mùa, quản lý tốc độ
│   │   └── EventBus.ts         # Hệ thống Pub/Sub Event Bus tách biệt hoàn toàn UI và Engine
│   ├── world/
│   │   ├── Biomes.ts           # Danh mục 12 Biome tự nhiên với thuộc tính vật lý và bảng màu
│   │   ├── WorldMap.ts         # Quản lý lưới 2D 72x72, không gian, tái sinh tài nguyên, density grid
│   │   ├── WorldGenerator.ts   # Thuật toán sinh địa hình mượt mà, lưu vực sông, làng mạc, đền thờ
│   │   ├── Overlays.ts         # Tính toán dữ liệu nhiệt độ, độ ẩm, mật độ dân số, mana flux
│   │   └── Pathfinding.ts      # Thuật toán tìm đường A* 8 hướng tối ưu với trọng số địa hình
│   ├── entities/
│   │   ├── Species.ts          # Cấu hình chủng tộc (Human, Elf, Dwarf, Deer, Wolf, Wisp)
│   │   ├── Items.ts            # Cơ sở dữ liệu 10+ vật phẩm, thức ăn, tài nguyên, trang bị, artifact
│   │   ├── Inventory.ts        # Hệ thống túi đồ theo ô, giới hạn trọng lượng (kg), xếp chồng
│   │   ├── Homeostasis.ts      # Hệ thống nhu cầu sinh học động (Máu, Đói, Thể lực, Tâm trạng, Mana)
│   │   ├── Dialogue.ts         # Trình sinh lời thoại & suy nghĩ RPG phát sinh (Emergent story)
│   │   ├── AIController.ts     # Cây hành vi AI ưu tiên (Tìm ăn, Ngủ nghỉ, Lao động, Trò chuyện)
│   │   └── Entity.ts           # Lớp thực thể tổng hợp, hoạt ảnh bước đi, bong bóng thoại, ký ức
│   ├── renderer/
│   │   ├── Camera.ts           # Camera pan/zoom mượt mà, nội suy theo dõi nhân vật, tính bounds
│   │   ├── Lighting.ts         # Hệ thống ánh sáng 2D đa nguồn: bóng tối ngày/đêm & hào quang radial
│   │   ├── ParticleSystem.ts   # Hiệu ứng hạt (Chữ "Zzz", trái tim, khói lửa trại, lấp lánh mana)
│   │   ├── WeatherRenderer.ts  # Hiệu ứng thời tiết (Mưa nghiêng + giọt bắn, Sương mù, Bão mana)
│   │   ├── Minimap.ts          # Bản đồ thu nhỏ tương tác thời gian thực, hiển thị khung nhìn camera
│   │   └── CanvasRenderer.ts   # Engine vẽ Canvas 2D chính (culling, animated tiles, sprite nhân vật)
│   ├── ui/
│   │   ├── TopBar.ts           # Thanh điều khiển: đồng hồ, tốc độ, đổi thời tiết, lớp phủ, bộ lọc màu
│   │   ├── InspectorPanel.ts   # Bảng thông tin chi tiết thực thể (thanh chỉ số, túi đồ, can thiệp True God)
│   │   ├── OverlayLegend.ts    # Thanh chú giải dải màu động với đơn vị đo (°C, %, nJ/m³, m)
│   │   ├── InventoryModal.ts   # Thẻ chi tiết vật phẩm RPG (Item Card) với chức năng Dùng / Vứt
│   │   ├── ChronicleLog.ts     # Bảng thông báo biên niên sử thế giới thời gian thực
│   │   └── HelpModal.ts        # Hướng dẫn chi tiết phím tắt và cơ chế hoạt động
│   └── accessibility/
│       ├── ColorblindPalettes.ts   # Bảng màu khoa học Viridis, Cividis, Plasma & ma trận biến đổi
│       └── AccessibilityManager.ts # Bộ quản lý chế độ mù màu & bộ lọc CSS toàn cục
```

---

## 3. Những gì đã hoàn thành (Feature Showcase)

Bản demo đáp ứng trọn vẹn và vượt mức cả **7 yêu cầu cốt lõi** trong `BRIEF.md`:

### 1. Bản đồ Tile 2D Top-Down mượt mà (Pan & Zoom)
- **12 Biome phong phú**: Biển sâu, Biển nông ven bờ, Bãi cát vàng, Đồng cỏ xanh tươi, Rừng sồi rậm rạp, Cánh đồng nông nghiệp, Khu định cư & Đường lát đá, Cao nguyên đá, Đỉnh núi tuyết, Rừng Mana thần bí, Miệng núi lửa, Đồi cát sa mạc.
- **Địa hình hữu cơ**: Dòng sông uốn lượn từ hồ băng phía bắc đổ ra vịnh biển phía nam với cầu gỗ bắc ngang; giếng nước làng; nhà tranh có giường ngủ; lửa trại ấm cúng; luống lúa mì chín; bụi dâu rừng mọc lại sau khi hái.
- **Điều khiển camera linh hoạt**: Kéo chuột (Drag) mượt mà, lăn chuột (Wheel) phóng to/thu nhỏ tại tâm trỏ chuột, phím `W A S D` hoặc mũi tên, click minimap để dịch chuyển tức thời, tính năng khóa camera theo dõi nhân vật.
- **Hoạt ảnh môi trường sống động**: Sóng nước nhấp nhô, lửa trại bập bùng, lúa mì lay động trong gió, khối tinh thể mana trôi nổi phát sáng.

### 2. Quần thể Entity tự hành thông minh (AI Homeostasis)
- **6 chủng loại sinh vật**:
  - **Con người**: Thợ làm bánh, Trưởng làng, Nông dân, Thợ đốn củi, Thợ dệt.
  - **Tiên tộc (Elf)**: Druid rừng, Du mục gió, Dược sư.
  - **Người lùn (Dwarf)**: Thợ mỏ sâu, Thợ đẽo đá.
  - **Động vật hoang dã**: Hươu rừng gặm cỏ, Sói xám săn mồi.
  - **Linh hồn ánh sáng (Wisp)**: Hộ vệ đền thờ mana.
- **Vòng lặp nhu cầu sinh học (Homeostasis)**:
  - **Đói (Hunger 0-100)**: Tăng dần theo thời gian. Khi đói (>60), tự tìm thức ăn trong túi đồ hoặc dò đường đến bụi dâu/ruộng lúa mì gần nhất để thu hoạch và ăn.
  - **Thể lực (Energy 0-100)**: Giảm khi di chuyển/lao động. Khi mệt (<30), tự tìm về giường trong nhà tranh hoặc ngồi cạnh lửa trại để ngủ (phát ra hạt "Zzz").
  - **Tâm trạng (Mood 0-100)**: Tăng khi no, ngủ đủ giấc, sưởi lửa trại, hoặc nói chuyện với bạn bè; giảm khi đói rét.
  - **Mana (0-100)**: Tích tụ tự nhiên tại các đền thờ tinh thể mana.
- **Thuật toán tìm đường A\***: 8 hướng mượt mà, tránh vật cản (nước, tường nhà), ưu tiên đi trên đường lát đá có hệ số tốc độ cao (+30% speed).

### 3. Bảng Inspector chi tiết trạng thái thực thể & Ô đất
- **Avatar động theo thủ tục**: Render chân dung SVG trực tiếp theo màu tóc, màu da, trang phục và hiệu ứng ban phước.
- **Đồng hồ đo chỉ số trực tiếp**: Thanh Máu, Đói, Năng lượng, Tâm trạng, Mana với màu sắc trực quan và cảnh báo trạng thái nguy hiểm.
- **Mục tiêu & Kế hoạch**: Hiển thị rõ hành động hiện tại (`SEEK_FOOD`, `SLEEPING`, `WORK_GATHER`, `SOCIALIZE`), tọa độ đích đến và **vẽ đường lộ trình di chuyển (Path Preview)** trên bản đồ.
- **Quyền năng True God (Can thiệp thần thánh)**:
  - 🌟 **Bless (Ban phước)**: Hồi phục toàn bộ chỉ số, tạo hào quang ánh sáng quanh nhân vật.
  - 🍞 **Feed (Ban tiệc)**: Lập tức lấp đầy cơn đói bằng thức ăn thần thánh.
  - ⚡ **Awaken (Thức tỉnh)**: Hồi phục 100% thể lực ngay lập tức.
  - 📍 **Track (Theo dõi)**: Khóa camera bám sát từng bước đi của nhân vật.
- **Chế độ Inspector Ô đất (Tile Mode)**: Xem nhiệt độ, độ ẩm, độ cao, mật độ mana, tài nguyên và triệu hồi (Spawn) sinh vật mới tại ô đó.

### 4. Lớp phủ bản đồ (Overlays) & Chú giải (Legend) chuẩn khoa học
- Hỗ trợ chuyển đổi nhanh các bản đồ chuyên đề:
  1. **Nhiệt độ (Temperature)**: thang đo `-15°C` đến `45°C` (Đơn vị: `°C`).
  2. **Độ ẩm (Moisture)**: thang đo `0%` đến `100%` (Đơn vị: `%`).
  3. **Mật độ dân số (Population Density)**: tính bằng kernel Gauss (Đơn vị: `entities/chunk`).
  4. **Cộng hưởng Mana (Mana Flux)**: trường năng lượng thần bí `0` đến `1000` (Đơn vị: `nJ/m³`).
  5. **Địa hình (Elevation)**: bình đồ độ cao từ biển sâu đến đỉnh núi tuyết (Đơn vị: `m`).
- **Legend động**: Tự động hiển thị thanh gradient màu liên tục, các mốc giá trị và mô tả ý nghĩa vật lý tương ứng.

### 5. Hệ thống thời gian, Đồng hồ & Ánh sáng 2D ngày/đêm
- **Đồng hồ thế giới**: Hiển thị Kỷ nguyên, Năm, Mùa (Xuân, Hạ, Thu, Đông), Ngày, Giờ:Phút:Giây (24h) và Pha thời gian.
- **Bộ điều khiển tốc độ**:
  - `⏸ Pause` / `▶ Play` (Phím tắt `Space`)
  - `⏯ Step 1 Tick` (Phím tắt `T`)
  - Các mức tốc độ: `0.5x`, `1x`, `2x`, `4x`, `16x` (Phím tắt `1`, `2`, `3`, `4`, `5`).
- **Ánh sáng 2D đa nguồn thời gian thực**:
  - Bình minh (05:00 - 08:00): Ánh ửng hồng & vàng cam dịu nhẹ.
  - Ban ngày (08:00 - 17:00): Chiếu sáng tự nhiên rõ nét.
  - Hoàng hôn (17:00 - 20:00): Sắc tím hổ phách huyền ảo.
  - Ban đêm (20:00 - 05:00): Bóng tối xanh thẫm khí quyển.
  - **Nguồn sáng điểm (Point Lights)**: Hào quang tỏa sáng có độ nhấp nháy tự nhiên từ Lửa trại làng, Đèn lồng đường phố, Đền thờ Mana và Linh hồn Wisp.
- **Thời tiết động**: Nắng trong xanh, Mưa rơi nghiêng kèm giọt bắn, Sương mù bảng lảng trôi, Bão sấm sét Mana.

### 6. Khả năng tiếp cận & Chế độ hỗ trợ người mù màu (Accessibility)
- Sử dụng các bảng màu chuẩn hóa khoa học được thiết kế riêng cho thị giác: **Viridis**, **Cividis**, **Plasma**.
- **Bộ lọc mô phỏng thị giác (Colorblind Modes)**:
  - `Normal Vision` (Màu chuẩn rực rỡ)
  - `Deuteranopia` (Mù màu xanh lá)
  - `Protanopia` (Mù màu đỏ)
  - `Tritanopia` (Mù màu xanh dương)
  - `High Contrast` (Độ tương phản cao)
- Mọi thông tin trạng thái đều đi kèm biểu tượng hình học, nhãn chữ hoặc đơn vị rõ ràng, **không bao giờ phụ thuộc duy nhất vào màu sắc**.

### 7. Hệ thống Túi đồ & Thẻ vật phẩm (Inventory & Item Cards)
- Túi đồ dạng lưới với hiển thị icon, số lượng xếp chồng và **thanh đo tải trọng (Weight Capacity)** chính xác (vd: `12.5 / 25.0 kg`).
- Click vào ô đồ sẽ mở **Thẻ vật phẩm RPG (Item Card)** cao cấp:
  - Khung viền phát sáng theo độ hiếm: *Common, Uncommon, Rare, Epic, Celestial*.
  - Thông số chi tiết: Trọng lượng, Giá trị vàng, Mô tả cốt truyện (Lore).
  - Tác động chỉ số: `🍗 Hunger -50`, `⚡ Energy +15`, `❤️ Health +40`, `😊 Mood +20`.
  - Nút tương tác: **🍴 Dùng (Consume)** để nạp năng lượng trực tiếp và **📦 Vứt (Drop)** xuống đất.

---

## 4. Những quyết định thiết kế đáng chú ý (Design Decisions)

1. **Kiến trúc Tách biệt Đồng hồ Mô phỏng & Tốc độ Khung hình (Fixed Simulation vs Render Loop)**:
   - Mô phỏng logic (AI, nhu cầu, tài nguyên) chạy theo nhịp tick cố định (20 tick/giây tại 1x speed), độc lập hoàn toàn với `requestAnimationFrame` của trình duyệt. Nhờ đó, khi tua nhanh `16x` hoặc làm chậm `0.5x`, chuyển động và hoạt ảnh của renderer vẫn duy trì độ mượt 60 FPS mà không bị xé hình.

2. **2D Canvas Layering & Radial Lighting Pass**:
   - Sử dụng kỹ thuật đục lỗ bóng tối (`destination-out`) kết hợp mặt nạ hòa trộn (`screen` / radial gradient) trên canvas đệm để tạo hiệu ứng ánh sáng ban đêm chân thực mà không cần cài đặt WebGL shader nặng nề, đảm bảo chạy mượt trên mọi thiết bị kể cả máy cấu hình yếu.

3. **Event-Driven UI Decoupling**:
   - Giao diện người dùng (TopBar, Inspector, Minimap, Chronicle) hoàn toàn không gọi trực tiếp vào vòng lặp của nhau mà giao tiếp qua `EventBus` kiểu strongly-typed. Điều này giúp dễ dàng bảo trì, mở rộng thêm các panel mới mà không làm vỡ luồng dữ liệu cũ.

4. **Tích hợp Bản đồ thu nhỏ Tương tác 2 chiều (Bidirectional Minimap)**:
   - Minimap không chỉ hiển thị vị trí thực thể và khung nhìn camera dưới dạng hình chữ nhật, mà còn cho phép người chơi nhấp hoặc rê chuột trực tiếp lên minimap để dịch chuyển tức thời vùng quan sát của True God.

5. **Đóng gói Độc lập Tuyệt đối (Single-File Distribution)**:
   - Tích hợp plugin `vite-plugin-singlefile` để xuất bản ra đúng một file `dist/index.html` tự chứa toàn bộ code, CSS và icons. Người dùng chỉ cần nhấp đúp là trải nghiệm được ngay mà không cần cài thêm web server.

---

## 5. Những gì còn có thể mở rộng (Future Roadmap)

- **Hệ thống Xây dựng & Chế tạo (Crafting & Building)**: Cho phép dân làng thu thập đủ gỗ và đá để tự xây dựng thêm nhà ở, cầu cống, mở rộng ranh giới làng mạc.
- **Hệ thống Quan hệ Xã hội & Gia tộc (Social Dynamics & Factions)**: Mô phỏng tình bạn, thù địch, hôn nhân, mua bán hàng hóa tại chợ và chiến tranh giữa các phe phái.
- **Hệ thống Phép thuật & Cổng Đa vũ trụ (Portals & Metaphysics)**: Kết nối Gaia với Umbral Abyss và Pantheon theo đúng đặc tả sâu trong tài liệu `docs/idea.md`.
- **Tầng nhận thức LLM (Cognition Layer)**: Tích hợp mô hình ngôn ngữ lớn (Gemini) để sinh ra các đoạn độc thoại nội tâm, nhật ký cá nhân và phản ứng xã hội phức tạp khi có sự kiện bất ngờ xảy ra.

---

## 6. Phím tắt nhanh (Cheatsheet)

| Phím | Chức năng |
|---|---|
| `W`, `A`, `S`, `D` / `Mũi tên` | Di chuyển Camera |
| `Kéo chuột trái` | Pan bản đồ |
| `Cuộn chuột` | Phóng to / Thu nhỏ mượt mà |
| `Space` | Tạm dừng / Tiếp tục mô phỏng |
| `T` | Bước tới 1 Tick mô phỏng |
| `1`, `2`, `3`, `4`, `5` | Đổi tốc độ (`0.5x`, `1x`, `2x`, `4x`, `16x`) |
| `Click Chuột` vào Entity | Mở bảng Inspector nhân vật |
| `Click Chuột` vào Ô đất | Mở bảng Inspector địa hình & Menu triệu hồi |
| `Escape` | Bỏ chọn thực thể |
| `?` hoặc `H` | Mở bảng hướng dẫn True God Guide |

---

*Phát triển độc lập bởi Antigravity Pair Programmer.*