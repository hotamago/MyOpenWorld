# My Open World — Bảng theo dõi tiến độ

> File này là **trạng thái**, không phải nhật ký và không phải tài liệu thiết kế.
> Thiết kế thế giới ở `docs/idea.md` (`§x.y`). Kiến trúc và cách thi công ở `docs/plan.md` (`§Px.y`).
> Mỗi task ở đây là một **kết quả cần đạt**, không phải một cách làm. Agent tự chọn cách tốt nhất.

## Dashboard

| Phase | Xong | Tổng | % |
|---|---|---|---|
| Phase 0 — Bộ khung và harness | 9 | 21 | 43% |
| Phase A — Hạt nhân không gian | 0 | 15 | 0% |
| Phase B — Khu định cư sống | 0 | 26 | 0% |
| Phase C — Nhận thức LLM và ký ức | 0 | 21 | 0% |
| Phase D — Xã hội, tri thức, kinh tế | 0 | 26 | 0% |
| Phase E — Ma thuật và đa thế giới | 0 | 17 | 0% |
| Phase F — Yuu/True God và mở rộng | 0 | 21 | 0% |
| **Tổng** | **9** | **147** | **6%** |

Đếm lại bằng lệnh này rồi dán số vào bảng — **không đếm tay, không đọc cả file để đếm**:

```bash
awk '/^## Phase/{p=$0} /^- \[/{t[p]++; if(/^- \[x\]/) d[p]++}
     END{ta=0;da=0; for(k in t){ta+=t[k];da+=d[k]; printf "%-40s %3d/%3d\n",k,d[k],t[k]}
          printf "%-40s %3d/%3d (%d%%)\n","TONG",da,ta,ta?da*100/ta:0}' docs/progress.md | sort
```

---

## Quy tắc cho agent

**Đọc ít, sửa gọn, chạy nhiều.** Mười quy tắc dưới đây tồn tại để tiết kiệm token và giữ tập trung.

### Đọc

1. **Không bao giờ đọc cả file này.** Vào phiên mới chỉ đọc: bảng Dashboard, mục Quy tắc, và **đúng một phase đang làm**. Lấy phase bằng `sed -n '/^## Phase B/,/^## Phase C/p' docs/progress.md`.
2. **Không đọc trước các phase sau.** Chúng sẽ đổi khi tới nơi.
3. Task trỏ tới `§` nào thì mở đúng mục đó, không đọc cả `idea.md` hay `plan.md`.

### Cập nhật

4. **Chỉ cập nhật ở checkpoint**, không cập nhật sau mỗi task. Checkpoint là một trong bốn:
   - Xong một batch **≥ 5 task**;
   - Chuyển phase;
   - Một task chuyển sang chặn `[!]`;
   - Kết thúc phiên làm việc.
5. **Một checkpoint = một lần sửa file.** Gộp tất cả thay đổi trạng thái vào một thao tác, cộng một dòng nhật ký, cộng cập nhật Dashboard.
6. **Không viết vào file này**: code, log, stack trace, mô tả thiết kế dài, phân tích. Nếu cần ghi lại một quyết định kiến trúc thì tạo `docs/adr/`, không nhét vào đây.

### Trạng thái

7. Ký hiệu: `[ ]` chưa làm · `[x]` xong · `[~]` đang làm · `[!]` bị chặn · `[-]` bỏ, kèm lý do một câu.
8. **Tối đa 3 task ở `[~]` cùng lúc.** Vượt quá là dấu hiệu đang trải mỏng.
9. **`[x]` chỉ khi acceptance đã chạy được thật**, không phải khi code đã viết xong. Không đánh dấu cả batch rồi mới chạy test.

### Thay đổi danh sách task

10. **ID không bao giờ tái sử dụng và không bao giờ đánh số lại.**
    - Tách một task: thêm `PB-04a`, `PB-04b`; task gốc chuyển `[-]` với lý do "tách".
    - Thêm việc mới phát hiện: lấy ID kế tiếp trong phase.
    - Task hóa ra không cần: `[-]` kèm lý do, **không xóa dòng**.
11. **Được phép làm khác cách mô tả.** Task ghi *kết quả*; nếu có đường tốt hơn thì đi, và ghi một dòng nhật ký. Không cần xin phép, không cần viết lại task.
12. **Thứ tự trong phase là gợi ý, không phải ràng buộc.** Chỉ `-GATE` là bắt buộc cuối cùng.

### Cổng phase

13. Task `-GATE` chỉ `[x]` khi toàn bộ điều kiện hoàn thành của phase đó trong `plan.md §P9` đã chạy xanh. **Không mở phase sau khi GATE còn hở** — trừ khi việc đó gỡ được một task `[!]`, và ghi rõ trong nhật ký.

---

## Phase 0 — Bộ khung và harness

> Mục tiêu: khi phase này xong, agent tự vào được thế giới, tự kiểm chứng, và mọi bug đều tái hiện được. Mọi thứ sau đó xây nhanh hơn nhiều.

- [x] **P0-01** Monorepo và toolchain — workspace Rust, uv workspace Python, pnpm cho web, `Makefile` với các target ở `§P12`, pre-commit. Xong khi `make setup` chạy sạch trên máy mới.
- [x] **P0-02** Môi trường dev — chạy được **không cần Docker**: SQLite, bus in-process, chỉ mục nhúng. `docker-compose` cho server mode là tùy chọn, chỉ cần thiết từ Giai đoạn C. `§P3.4`
- [x] **P0-03** `mow-math` — **bảng miền số học** theo `§P10.2.1` (Q16.16 cho tỉ lệ, `u64` thang cho xác suất nhỏ, hữu tỉ cho tốc độ, nguyên có đơn vị cho vật lý), tọa độ i64/i128 checked, hash canonical, named RNG stream. Proptest ở biên `i64::MIN/MAX` **và** test chứng minh `mutation_rate 2.1e-8` không bị làm tròn về 0. `§4.3`, `§19.6`, `§P10.2.1`
- [ ] **P0-04** Sinh mã và hợp đồng — `proto/` và `schemas/` sinh ra Rust/Python/TS. CI fail khi mã sinh khác mã đã commit. `§P4`
- [x] **P0-05** Config — layer `base → env → MOW_* → CLI`, validate lúc khởi động, sai thì thoát với đường dẫn field. Cả Rust và Python. `§P6.1`
- [x] **P0-06** `mow-core` tối thiểu — ECS, clock, event log append-only, transaction handler. Không có đường ghi state nào đi vòng qua nó. `§22.1`
- [x] **P0-07** `mow-persist` — trait persistence cộng **một** hiện thực SQLite, và bộ test hợp đồng viết sẵn để backend thứ hai dùng lại nguyên vẹn ở `PC-20`. `§P6.6`, `§P3.4`
- [x] **P0-08** Trait `MessageBus` — **một** hiện thực in-process bền trên SQLite, đủ để không mất proposal khi crash. Không tự dựng lại JetStream. `§P3.4`
- [x] **P0-09** Trait `VectorIndex` — **một** hiện thực nhúng, kho ký ức nằm ở file riêng **không nằm trong file save** để không tranh chấp khóa với tiến trình sim. `§P3.4`, `§P6.3`
- [ ] **P0-10** LLM Gateway — `ModelClient` provider-agnostic, bốn chế độ `LIVE/RECORD/REPLAY/STUB`, bảng `llm_call` có unique trên `request_hash`. `§P6.7`
- [ ] **P0-11** `mow-devtool` — gRPC `Debug` sau feature flag. Có test trên artifact release chứng minh không còn symbol nào của devtool. `§P7.1`
- [ ] **P0-12** `mow-mcp` — MCP server chạy được từ Claude Code với nhóm World/Time/Query/Verify. Xong khi agent tạo world, tiến 1000 tick, đọc entity và nhận báo cáo invariant. `§P7.2`
- [ ] **P0-13** Invariant runner — khung `INV-22-<n>`, ba mức chi phí, và 5 bất biến đầu tiên chạy thật. `§P7.4`
- [ ] **P0-14** Scenario runner — DSL `given/when/then`, khối `bind` với bộ chọn thứ tự toàn phần, 3 scenario khói. `§P7.3`
- [ ] **P0-15** Determinism harness — chạy N lần khác số luồng, so state hash, **bisect theo tick** khi lệch. `§P7.5`
- [ ] **P0-16** Repro bundle — capture, run, bisect. Xong khi một bundle chụp rồi chạy lại cho đúng cùng state hash. `§P7.6`
- [ ] **P0-17** Prompt registry — YAML + Jinja2 + version + filter `untrusted` + prompt leak guard. Xong khi guard bắt được một ca rò cố tình cài vào test. `§P6.2`, `§8.10.3`
- [ ] **P0-18** CI — lint, unit, contract, scenario smoke, determinism nhanh, schema validate, quét secret. `§P8.1`
- [ ] **P0-19** Observability tối thiểu — OTel trace `command → event`, log JSON luôn kèm `branch/world/tick`. `§P8`
- [x] **P0-20** Registry pack tối thiểu — manifest, namespace bắt buộc, thứ tự load deterministic, content hash. `content/core` nạp qua đúng cơ chế này, không có loader đặc quyền. `§19.7.2`, `§19.7.3`, `§P10.7`
- [ ] **P0-GATE** Cổng Giai đoạn 0 — toàn bộ điều kiện hoàn thành ở `§P9` Giai đoạn 0 xanh.

---

## Phase A — Hạt nhân không gian

- [ ] **PA-01** `mow-worldgen` — pipeline địa hình 10 bước, deterministic theo generation profile snapshot. `§7.2`, `§7.3`
- [ ] **PA-02** Topology và trường vĩ mô — sông, dãy núi, lưu vực không đứt ở biên chunk. `§7.4`
- [ ] **PA-03** `mow-spatial` — chunk, occupancy, tải lười, save `seed + delta`. `§7.1`, `§7.2`
- [ ] **PA-04** Worldseed và genesis — worldseed, lockfile đã resolve, scenario biên dịch thành command tại tick 0. `§7.6`, `§7.6.6`
- [ ] **PA-05** Khung frontend — Vue + Vite + Pinia, WS client và decode trong Web Worker. `§P6.8`
- [ ] **PA-06** Renderer — PixiJS v8 + tilemap, lát `z`, pan/zoom, floating origin, chỉ rebuild chunk bẩn. `§18.1`, `§18.4`
- [ ] **PA-07** Giao thức đồng bộ — `ViewSubscription` và `SetSimulationFocus` là **hai loại thông điệp khác nhau**; đổi camera không ghi event. `§8.4`, `§P6.8`
- [ ] **PA-08** Tọa độ lớn — BigInt ở biên, camera-local trong worker. Test camera vượt `2^53` chọn đúng cell và không rung. `§4.3`, `§22.10`
- [ ] **PA-09** Bản desktop sớm — Tauri chạy được với backend nhúng, dù chỉ hiện bản đồ. Đây là **smoke build trong CI**, không phải bề mặt sản phẩm phải bảo trì; nó tồn tại để bắt sớm các lỗi đóng gói, đường dẫn file và quyền hệ thống. `§P3.3`, `§P3.4`
- [ ] **PA-10** Scenario `spatial/*` — seam, tọa độ xa, đào/đặt/save/load/replay cùng hash.
- [ ] **PA-11** Tile atlas sinh từ dữ liệu — diện mạo vật liệu suy ra từ material definition, nướng một lần lúc nạp. Modder thêm vật liệu là có tile ngay. `§18.5.1`
- [ ] **PA-12** Overlay là data texture — một kênh, lấy mẫu trong shader, không phải sprite mỗi ô. Legend bắt buộc kèm đơn vị thật. `§18.6`, `§P6.9.2`
- [ ] **PA-13** Bảng màu qua CI — thang tuần tự/phân kỳ/phân loại nằm trong `content/`, có bước CI kiểm tra tương phản và mù màu cho cả hai chế độ. Bảng dùng cho bản đồ quá 3 định danh làm CI fail. `§18.6.2`, `§P6.9.3`
- [ ] **PA-14** Hệ icon hợp thành — ~100 bóng nguyên thủy SVG trong `content/`, hợp thành 5 lớp, nướng thành atlas, khóa là hàm thuần của dữ liệu. CI kiểm tra mọi def giải ra được icon. `§18.14.1`, `§18.14.2`, `§P6.9.6`
- [ ] **PA-GATE** Cổng Giai đoạn A.

---

## Phase B — Khu định cư sống, chưa cần LLM

> Cả phase này chạy ở `llm_mode: STUB`. Nếu cần LLM để khu định cư hoạt động thì thiết kế đã sai.

- [ ] **PB-01** Homeostasis — tích phân đóng, wake-up theo ngưỡng, `clock_domain` bắt buộc. Không có vòng lặp per-tick per-entity. `§9.7`, `§22.24`
- [ ] **PB-02** Bộ gen nén — suy ra từ cha mẹ + seed tái tổ hợp + đột biến, không lưu genome đầy đủ. Chỉ thừa kế đặc tính cơ bản. `§9.5.2`
- [ ] **PB-03** Lão hóa — đường cong tử vong Gompertz và lão hóa không đáng kể; tác động qua effect. `§9.5.6`
- [ ] **PB-04** `mow-effect` — modifier pipeline, 5 chính sách stacking, chuỗi ward, thứ tự áp dụng ổn định. `§9.8`
- [ ] **PB-05** `perceptible_as` — mọi effect khai báo cách nó bị nhận biết; chẩn sai là kết quả hợp lệ. `§9.8.2`, `§22.22`
- [ ] **PB-06** Bệnh và dịch — ủ bệnh, lây theo tiếp xúc ở mức cá thể, ngăn S/E/I/R ở mức khu định cư. `§9.8.5`
- [ ] **PB-07** `mow-items` — instance/stack/aggregate, `CraftQuality` bất biến tách khỏi `Condition`, chế tác và sửa chữa. `§8.5`–`§8.7`
- [ ] **PB-08** `mow-action` — registry, precondition authoritative tính từ state, failure code. `§10.5`, `§22.5`
- [ ] **PB-09** Chrono-turn — `ready_at`, bốn loại tốc độ tách biệt, ba pha `wind_up/impact/recovery`. `§10.7`, `§10.8`
- [ ] **PB-10** Giải quyết đồng thời — tầng cố định; property test chứng minh đảo `EntityId` không đổi kết quả. `§10.9`, `§22.43`
- [ ] **PB-11** Perception — nguồn duy nhất của observation; không có đường đọc world truth nào khác cho entity. `§10.2`
- [ ] **PB-12** Utility AI — reflex, routine, tactical plan cho sinh hoạt thường ngày. `§10.3`
- [ ] **PB-13** Kinh tế nhỏ — tài nguyên có nguồn thật, recipe, thị trường địa phương hình thành giá. `§12.2`
- [ ] **PB-14** Hộ gia đình và địa điểm — huyết thống, vòng đời hộ, giếng/chợ/quán có hàng đợi thật tạo contact graph. `§12.9`, `§12.18.2`
- [ ] **PB-15** LOD đầu tiên — active/near/far, bảo toàn dân số, tài nguyên, quan hệ khi chuyển mức. `§8.3`, `§22.14`
- [ ] **PB-16** Panel Inspector và Timeline — đọc state thật, truy được cause chain. `§18.3`
- [ ] **PB-17** Ngôn ngữ thị giác — kênh thị giác phân bổ cố định, overlay là nhóm loại trừ, cutaway, vật thể nhiều tầng có dấu hiệu che. `§18.5`
- [ ] **PB-18** Điều khiển thời gian — pause/step/tốc độ, `pause-on-ready`, "chạy đến khi", mốc tự dừng kèm giải thích lấy từ event thật. `§18.8`
- [ ] **PB-19** Túi đồ theo thể tích và khối lượng — hai thanh riêng, quá tải làm chậm chứ không chặn; đống và cá thể hiển thị khác nhau. `§18.15.1`, `§18.15.2`
- [ ] **PB-20** Thẻ vật phẩm — chất lượng chế tác và tình trạng theo bộ phận tách hẳn, kèm lịch sử sửa chữa và ai đã sửa. `§18.15.3`
- [ ] **PB-21** Trang bị theo bộ phận cơ thể — có lớp, giải phẫu khác thì chỗ mặc khác, che phủ quyết định thương tích ở đâu. `§18.15.4`
- [ ] **PB-22** Mô hình thương tích — body part có mô, chức năng, máu, đau, nhiễm trùng; `vitality` chỉ là chỉ số suy ra cho UI. `§9.4`
- [ ] **PB-23** Chiến trường chiến thuật — facing, tầm với, che chắn, độ cao, mặt nền, đội hình, bắn nhầm, vùng kiểm soát; trần vật lý của tốc độ. `§10.10`
- [ ] **PB-24** Ràng buộc ưng thuận ở tầng engine — validator từ chối tại thời điểm tạo action nếu thiếu `Sapient`, thiếu `maturity_years` hoặc thiếu capacity. Không plugin, không override nào cấp ngoại lệ. `§12.7.2`, `§12.7.5`, `§22.26`
- [ ] **PB-25** **Lát cắt chơi được** — tạo avatar, đi lại, nhặt, ăn, nói chuyện, quan sát theo tri giác, cộng một lệnh True God có preview và commit. Không có nó thì Giai đoạn B–E chỉ chứng minh một simulator, chưa bao giờ chứng minh một trò chơi. `§3.1`
- [ ] **PB-GATE** Cổng Giai đoạn B.

---

## Phase C — Nhận thức LLM và ký ức

- [ ] **PC-01** `agent-service` — khung LangGraph, consumer `MessageBus`, schema pydantic sinh từ proto. `§P5`
- [ ] **PC-02** Cognition cycle — đủ 9 bước, output có `evidence_refs`, không có đường tắt. `§10.4`
- [ ] **PC-03** Nén context — prompt builder lấy đúng phần cần, giữ link về source để audit. `§20.9`
- [ ] **PC-04** Validator — action tồn tại, entity biết action, reference thuộc đúng cognition context. `§10.4`, `§22.4`
- [ ] **PC-05** `memory-service` — mem0 + `VectorIndex`, lớp ACL duy nhất, lọc branch/owner/version. `§11.1`, `§22.16`
- [ ] **PC-06** Chỉ mục dựng lại được — xóa sạch vector store rồi rebuild từ event log không mất dữ liệu. `§P6.3`
- [ ] **PC-07** Tombstone — quên/sửa/xóa vô hiệu embedding cũ trước khi reindex; vector cũ không trả về trong khoảng rebuild. `§11.5`
- [ ] **PC-08** Budget scheduler — **selection deterministic trong Rust**, throttling ở gateway; hai thứ không được trộn. `§20.2.1`
- [ ] **PC-09** Fallback có ghi event — timeout, breaker mở, hết ngân sách, hạ cấp model đều tạo event ghi model thật sự đã dùng. `§20.10`
- [ ] **PC-10** Personality 5 lớp — trait/values/affective/clinical/self-narrative, lấy mẫu có tương quan. `§9.9`
- [ ] **PC-11** Reputation — belief theo bộ ba, tách khỏi trait thật; trật tự chuẩn mực bậc một/bậc hai là dữ liệu văn hóa. `§9.9.3`, `§9.9.4`
- [ ] **PC-12** Trao đổi xã hội — volition tính bằng quy tắc trên social state; LLM chọn ý định, engine tính kết quả. `§10.12`
- [ ] **PC-13** Chống trôi persona — state là mỏ neo, Auditor so hành vi với trait và báo lệch không có nguyên nhân. `§20.11`
- [ ] **PC-14** Panel Entity Mind — observation, goal, plan, belief, ký ức đã truy xuất, lý do chọn action. `§18.3`
- [ ] **PC-15** Ba chế độ nhận thức — hóa thân/quan sát/True God, **lọc ở read model chứ không ẩn ở client**. Test e2e chụp payload WS và khẳng định không rò. `§18.9`, `§P6.9.4`
- [ ] **PC-16** Cause chain viewer — truy ngược và truy xuôi từ event bất kỳ, kèm version law và `norm_set` lúc đó, nhảy được tới đúng chỗ đúng lúc. Chỉ hiện event có thật. `§18.10`
- [ ] **PC-17** Chân dung tối thiểu — vài lớp từ phenotype, deterministic theo `genotype_seed` nên con giống cha mẹ; cập nhật theo đói, bệnh, sẹo, tuổi. Bộ 15 lớp đầy đủ ở `PF-19`. `§18.14.4`
- [ ] **PC-18** Biểu tượng tuân thủ tri giác — chưa thẩm định thì dấu hỏi, phép ẩn không hiện, cải trang hiện lớp cải trang, giá là ước lượng của nhân vật. `§18.14.5`, `§18.15.6`
- [ ] **PC-19** Batch đúng cách — gộp request cùng loại có cách ly context, validator loại reference chéo, và **không batch** khi cần cách ly bí mật hoặc khi thứ tự phát ngôn quan trọng. `§20.6`
- [ ] **PC-20** Hiện thực thứ hai cho server mode — Postgres, NATS, Qdrant, dùng lại nguyên bộ test hợp đồng của `P0-07`–`P0-09` để chứng minh tương đương. `§P3.4`
- [ ] **PC-GATE** Cổng Giai đoạn C.

---

## Phase D — Xã hội, tri thức, kinh tế

- [ ] **PD-01** `norm_set` — jurisdiction chồng lấn, `coverage_by_district`, version luật tại thời điểm hành vi được ghi vào event. `§12.5.1`, `§22.49`
- [ ] **PD-02** Pipeline tội phạm — động cơ → cơ hội → rủi ro theo belief → hành vi → nhân chứng → chứng cứ → xét xử → hình phạt. `§12.5.2`
- [ ] **PD-03** Chứng cứ và thủ tục — vật chứng, nhân chứng có động cơ, văn bản giả mạo được, phép truy vấn sự thật có counter. `§12.5.3`
- [ ] **PD-04** Tổ chức và nhà nước — chức vụ, mệnh lệnh, ngân sách, chuỗi ủy quyền; `coverage` sinh ra từ năng lực thật. `§12.13.1`
- [ ] **PD-05** Chính danh — ba động cơ tuân thủ cho kết quả khác nhau khi nhà nước yếu đi. `§12.13.2`
- [ ] **PD-06** Đa tầng pháp luật — precedence, thẩm quyền, dẫn độ, miễn trừ. `§12.14`
- [ ] **PD-07** Hành động tập thể — ngưỡng cá nhân, kỳ vọng về người khác, kẻ ăn theo, tín hiệu đàn áp. `§12.11`
- [ ] **PD-08** Tài nguyên chung — bảy yếu tố quản trị; thiếu yếu tố nào thất bại theo kiểu của yếu tố đó. `§12.12`
- [ ] **PD-09** Tổ chức tội phạm và tệ nạn — băng đảng là organization, chợ đen là market có risk premium, nghiện dùng lại effect. `§12.6`
- [ ] **PD-10** Sở hữu và claim — possession tách khỏi claim, bó quyền tài sản, claim chỉ mạnh bằng cơ chế cưỡng chế. `§12.8.1`, `§12.8.7`
- [ ] **PD-11** Tiền tệ — thang tiến hóa, đồng xu là item có thể pha loãng, vòi/cống khai báo rõ, Auditor báo nguyên nhân lạm phát. `§12.8.2`–`§12.8.4`
- [ ] **PD-12** Tín dụng — gốc, kỳ hạn, thế chấp, bảo lãnh, vỡ nợ, khủng hoảng dây chuyền. `§12.8.8`
- [ ] **PD-13** Lao động và vận chuyển — hợp đồng, phường hội, và **hàng hóa không teleport**: shipment có tuyến, hao hụt, chuỗi bàn giao. `§12.17`
- [ ] **PD-14** Thông điệp — vòng đời message, nhiều phiên bản cạnh tranh, tách "nhận được" khỏi "làm theo". `§12.15`
- [ ] **PD-15** Tôn giáo — doctrine graph, giáo sĩ, ly giáo, nghi lễ tốn kém là bằng chứng chứ không phải điểm số. `§12.16`
- [ ] **PD-22** Di truyền định lượng — `h²` theo trait, tương tác gen×môi trường, hệ số cận huyết và suy thoái cận huyết. Tách khỏi `PB-02` vì đây là toán nhiều tính trạng, không thuộc phạm vi khu định cư tối thiểu. `§9.5.1`
- [ ] **PD-16** `mow-knowledge` — graph thống nhất, thang `UNKNOWN→MASTERED`, dạy học có hao hụt, nghiên cứu có thất bại, project nhiều bên. `§13.1`–`§13.5`
- [ ] **PD-17** Storylet và Director — pool có precondition trên state thật, chọn theo salience, `outcomes` luôn rỗng, audit view chỉ đúng storylet đã kích hoạt. `§15.6`, `§22.53`
- [ ] **PD-18** Biên niên sử hai lớp — "đã xảy ra" đặt cạnh "người ta tin là đã xảy ra", đánh dấu chỗ lệch và truy được lệch từ đời nào. `§18.11`
- [ ] **PD-19** Panel xã hội và tri thức — society view, knowledge graph, biểu đồ kinh tế; thang zoom có chỉ báo "ước lượng theo mô hình vùng". `§18.3`, `§18.7`
- [ ] **PD-20** Huy hiệu tối thiểu — bảng chia trường và màu tĩnh đã qua kiểm tra, cộng **dấu nhánh thứ thừa kế**, đủ để đọc đồ thị huyết thống từ lá cờ. Bộ giải văn phạm đầy đủ ở `PF-18`. `§18.14.3`
- [ ] **PD-21** So sánh vật phẩm — bảng cạnh nhau theo từng chiều, **không rút về một điểm số duy nhất**; hiện rõ cái gì mất đi nếu đổi. `§18.15.7`
- [ ] **PD-23** Chiến tranh — quân đội, hậu cần, vây hãm, morale, thương vong, đầu hàng và hiệp ước thực thi được; thắng bại không do tổng điểm chiến đấu. `§12.4`
- [ ] **PD-24** Vật phẩm mang thông tin — sách, bản đồ, thư; đọc là truyền dạy có hao hụt, sao chép sinh lỗi tích lũy, và đốt hết bản sao làm mất tri thức thật. `§8.8`
- [ ] **PD-25** Giáo dục và lưu trữ — trường, học nghề, thi cử, thư viện; gác cửa quyết định ai lên được địa vị, cháy thư viện xóa vĩnh viễn một nhánh tri thức. `§13.10`
- [ ] **PD-GATE** Cổng Giai đoạn D.

---

## Phase E — Ma thuật và đa thế giới

- [ ] **PE-01** DSL Tier 0 — parser, kiểm kiểu và đơn vị, interpreter fixed-point, đảm bảo dừng. `§15.3`, `§13.9.1`
- [ ] **PE-02** WASM Tier 1 — wasmtime với fuel, trần bộ nhớ, import whitelist, không WASI. Hết fuel là lỗi xác định. `§13.9.3`
- [ ] **PE-03** Hai loại context — `AgentModuleContext` chỉ thấy observation; registry **từ chối nạp** module xin sai loại. `§13.9.6`, `§22.48`
- [ ] **PE-04** Version luật — event ghi version đã dùng; sửa luật không hồi tố lên lịch sử. `§13.9.5`
- [ ] **PE-05** Vật phẩm mang hành vi — module ref đóng băng, 8 loại cổng sử dụng, charges, rủi ro khi thiếu cổng. `§8.10.1`, `§8.10.2`
- [ ] **PE-06** Bí mật không vào prompt — view lọc theo người quan sát, Auditor quét mọi prompt đã gửi. Rò một lần là bug nghiêm trọng. `§8.10.3`, `§22.40`
- [ ] **PE-07** NPC tổng hợp module — chỉ ghép từ node đã biết, trần độ phức tạp theo skill, qua đúng validator như luật Yuu sinh. `§8.10.4`, `§22.41`
- [ ] **PE-08** Thiên phú và khải thị — talent di truyền, revelation có provenance điều tra được, tháo ngược trả về node tri thức. `§13.8`, `§8.10.6`
- [ ] **PE-09** Portal — state machine, transfer 9 bước nguyên tử, không nhân đôi và không mất entity. `§6.2`, `§22.8`
- [ ] **PE-10** Clock domain rebase — qua portal thì mọi deadline rebase theo domain của chính tiến trình. `§4.5`, `§22.42`
- [ ] **PE-11** Chế độ tiếp xúc — kiểm dịch, hàng cấm, chuẩn đo lường, quyền cư trú, giải quyết tranh chấp xuyên world. `§6.4`
- [ ] **PE-12** Sinh thái — diễn thế theo thời gian, loài xâm lấn và mầm bệnh đi qua cổng. `§9.10`
- [ ] **PE-13** Linh hồn và thần — soul policy, triệu hồi, thăng thần, domain authority không ghi thẳng kết quả. `§14`
- [ ] **PE-14** Scenario `magic/*` và `multiworld/*`.
- [ ] **PE-15** Rào cản liên loài — năm trục độc lập: sinh sản, sinh lý/môi trường, tri giác, thời gian, cấu trúc xã hội. Không gộp thành một chỉ số quan hệ chủng tộc. `§9.11`
- [ ] **PE-16** Vật phẩm huyền thoại — bốn con đường thành huyền thoại, chuỗi provenance đặt cạnh truyền thuyết, vật phẩm có tri giác, hủy diệt là thật. `§8.9`
- [ ] **PE-GATE** Cổng Giai đoạn E.

---

## Phase F — Yuu/True God và mở rộng

- [ ] **PF-01** `mow-plugin` — manifest, namespace bắt buộc, thứ tự load deterministic, quyền theo capability. `§19.7`, `§22.29`
- [ ] **PF-02** Save và pack set — ghi version + content hash; thiếu hoặc lệch thì từ chối load thay vì load một phần. `§22.30`
- [ ] **PF-03** Vòng lặp content pack — validate, test, nạp nóng ở dev qua đúng đường migration. `§P10.7`
- [ ] **PF-04** Seed Vault — duyệt, preview có báo cáo rủi ro, fork, diff ở mức dữ liệu, xuất/nhập có checksum. `§7.6.5`
- [ ] **PF-05** Tiền sử — chạy aggregate qua thời gian thật, macro-delta commit **trước** khi người chơi mở chunk. `§7.6.4`, `§22.46`
- [ ] **PF-06** Yuu tạo nội dung — World Architect, Species Foundry với viability check, Law Forge với sandbox. `§15.1`–`§15.3`
- [ ] **PF-07** Yuu Auditor và Historian — dùng chung bộ invariant với harness; biên niên sử chỉ dùng event có thật. `§22.17`
- [ ] **PF-08** True God console — query/proposal/command, preview, snapshot tự động, rollback, branch. `§15.5`, `§16`
- [ ] **PF-09** Hóa thân và chỉnh prompt — possession, phân tầng quyền prompt, mọi can thiệp có provenance. `§16.3`, `§16.4`
- [ ] **PF-10** Soak và World Health Report — 3 world × 200 năm, cảnh báo dạng "lạm phát không giải thích được". `§P7.7`
- [ ] **PF-11** Hiệu năng — đạt toàn bộ ngân sách ở bảng `§P8.1`; vượt ngân sách làm CI fail.
- [ ] **PF-12** Đóng gói phát hành — Tauri bundle, sidecar Python, `tauri-driver` cho đường riêng của desktop. `§P3.4`
- [ ] **PF-13** Tài liệu modder — schema có `$id` version, một content pack mẫu của "bên thứ ba" nạp được và không đổi hash của world không dùng nó.
- [ ] **PF-14** Bộ test đầy đủ — regression từ mọi bug đã sửa, e2e Playwright cho các panel chính.
- [ ] **PF-15** Console Yuu và True God — proposal hiện dưới dạng diff có preview, báo cáo rủi ro, commit/rollback; audit view lọc theo provenance. `§18.12`
- [ ] **PF-16** Dễ đọc — triệu chứng trước con số, mọi giá trị suy ra bấm được về nguồn, affordance "vì sao?" ở khắp nơi. `§18.13`
- [ ] **PF-17** Tiếp cận được — hoa văn thay màu, bảng số cho mọi overlay, chế độ tối là thang riêng đã qua kiểm tra. `§18.6.3`, `§18.6.4`
- [ ] **PF-18** Huy hiệu đầy đủ — bộ giải văn phạm blazon. Tách khỏi `PD-20` vì đây là phần đắt; `PD-20` chỉ cần bảng tĩnh cộng dấu nhánh thứ để đọc được huyết thống. `§18.14.3`
- [ ] **PF-19** Chân dung đầy đủ — 15 lớp paper-doll. Tách khỏi `PC-17` vốn chỉ cần lớp tối thiểu: loài, tuổi, trạng thái thấy được. `§18.14.4`
- [ ] **PF-20** Di cư và ứng phó thảm họa — quyết định rời đi theo belief, cộng đồng ly tán, cảnh báo/sơ tán/tái thiết; cùng một trận động đất cho hai kết cục khác nhau tùy chính danh. `§12.19`, `§12.20`
- [ ] **PF-GATE** Cổng Giai đoạn F.

---

## Nhật ký

> Mỗi checkpoint thêm **một dòng**, dạng `YYYY-MM-DD — <việc đã xong hoặc quyết định> — <ID liên quan>`.
> Giữ tối đa **20 dòng gần nhất**; vượt thì xóa dòng cũ nhất. Đây không phải nơi kể chuyện.

- 2026-08-31 — Dựng `src/` (workspace Rust + uv + pnpm + Makefile), 7 crate, 89 test xanh — P0-01/02/03/05/06/07/08/09/20
- 2026-08-31 — Thêm `deploy/` + `./mow` một lệnh: toolbox cách ly, infra sau `profiles`, kiểm chứng determinism theo số luồng — P0-02
- 2026-08-31 — `quantize` chuẩn hóa L2 thay vì theo max; chuẩn hóa max làm ký ức gần giống nhau hòa điểm rồi xếp theo id — P0-09
