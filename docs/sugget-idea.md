# Đề xuất mở rộng cho My Open World

> Tài liệu đề xuất để review, được xây dựng sau khi đọc `docs/idea.md` và chuỗi commit `7762122 → f0256a7 → fa2faea`. Chưa phải đặc tả đã được chấp nhận.

## 1. Review ngắn bản hiện tại

Thay đổi gần nhất bổ sung khoảng 814 dòng và xóa 20 dòng trong `docs/idea.md`. Những phần cải thiện rõ nhất:

- `Animate`/`Sapient` thay cho một loại entity thông minh chung.
- Worldseed, scenario, genesis command và prehistory.
- Homeostasis không tick từng entity.
- Effect pipeline dùng chung cho bệnh, độc, phép và chấn thương.
- Personality nhiều tầng, reputation tách khỏi trait thật.
- Tội phạm, chứng cứ, tư pháp, tổ chức tội phạm và ranh giới nội dung nhạy cảm.
- Talent, revelation và spell synthesis.
- Law DSL kết hợp deterministic WASM.
- Content pack và plugin capability.

Tài liệu đã chuyển từ ý tưởng tổng quát sang gần một simulation specification. Khoảng trống lớn nhất hiện tại là:

1. Cách cá thể thật sự chiến đấu và giao tiếp theo cơ chế RPG.
2. Các thể chế trung gian khiến xã hội vận hành mà không cần Yuu tạo event trực tiếp.
3. Một số contract về thời gian, worldseed, Effect và WASM cần chốt để giữ determinism.

## 2. Cơ chế RPG theo lượt dựa trên thời gian

Kiến trúc hiện tại đã có event/deadline scheduler và mỗi action đã có `duration`. Vì vậy không nên tạo một hệ round cố định tách khỏi simulation. Nên dùng chính simulation timeline làm hệ thống lượt.

### 2.1. Chrono-Turn Timeline

Mỗi actor có `ready_at_local_tick`. Khi tới thời điểm này actor mới được chọn action. Action càng tốn thời gian thì lượt tiếp theo càng xa; nhân vật nhanh tự nhiên được hành động nhiều lần trước nhân vật chậm.

```text
duration = max(min_duration, ceil(base_work / effective_rate))
```

Ví dụ đơn giản:

```text
Speedster: effective_rate = 300 → attack mất 4 tick
Guard:     effective_rate = 100 → attack mất 12 tick

t=4   Speedster impact
t=8   Speedster impact
t=12  Speedster impact + Guard impact
```

Hai impact ở tick 12 được giải quyết đồng thời. Đây là trường hợp nhân vật cực nhanh thực hiện ba đòn trong thời gian người thường thực hiện một đòn.

### 2.2. Bốn loại tốc độ riêng

Tách `perception/reaction_speed`, `motor/movement_speed`, `casting_speed` và `speech/cognition_rate`. Một speedster có thể chạy và đánh rất nhanh nhưng không mặc định suy nghĩ, nói hoặc niệm thần chú nhanh gấp 100 lần.

### 2.3. Action Phase State Machine

Mỗi action có các pha `wind_up → impact → recovery`. Pha chuẩn bị tạo telegraph; impact mới phát effect; recovery khóa action tiếp theo. Cơ chế này tạo feint, cancel, ngắt phép, vũ khí nặng, đòn nhanh và đòn charge mà không cần viết trường hợp đặc biệt.

### 2.4. Reaction và Interrupt Timeline

Đỡ, né, phản đòn, ngắt phép, bảo vệ đồng đội và chen lời dùng một reaction timeline riêng. Reaction chỉ được tạo sau khi actor thật sự quan sát stimulus, phải trả stamina/focus và có thể làm chậm lượt chính tiếp theo.

### 2.5. Simultaneous Commit

Mọi impact cùng tick được gom thành proposal rồi resolve theo stage:

```text
movement
  → ward/shield
  → hit/collision
  → injury/effect
  → death/reaction
```

Không để thứ tự `EntityId` quyết định ai sống. Hai kiếm sĩ có thể thật sự đâm trúng nhau cùng lúc.

### 2.6. Giới hạn vật lý của speed

Dùng minimum phase duration, perception latency, acceleration, quán tính vũ khí, stamina, heat và cooldown để ngăn speed trở thành stat thống trị tuyệt đối. Người nhanh vẫn rất mạnh, nhưng người chậm có thể thắng bằng chuẩn bị, bẫy, khiên, địa hình, area denial hoặc dự đoán hành động.

### 2.7. Tactical Grid và Control Zone

Thêm facing, reach, cover, elevation, footing, formation, friendly fire và zone of control. Vị trí ở cửa hẹp, trên cao hoặc sau đồng đội phải quan trọng hơn một phép cộng combat score.

### 2.8. Social Action Semantics

Giao tiếp cũng chạy trên timeline. Các action có thể gồm:

- `speak`, `listen`, `consider`, `interrupt`.
- `present_evidence`, `question`, `verify_claim`.
- `lie`, `threaten`, `promise`, `offer`, `withdraw`.
- `invoke_status`, `invoke_law`, `appeal_to_value`.

Kết quả cập nhật belief, trust, fear, obligation và commitment; không dùng một thanh “persuasion HP”.

### 2.9. Dialogue không dừng vật lý

Một câu dài tốn thời gian thật. Trong lúc nói, người khác có thể bỏ đi, tấn công, chen lời hoặc một sự kiện ngoài scene có thể xảy ra. Chế độ avatar có thể `pause-on-ready` để giữ cảm giác turn-based mà simulation vẫn dùng cùng một timeline authoritative.

### 2.10. Timeline UI tuân theo tri thức cục bộ

UI chỉ hiển thị lượt của avatar và action địch đã được telegraph qua observation. Không tự động cho người chơi xem tên spell bí mật hoặc chính xác thời điểm impact nếu avatar chưa đủ perception/knowledge.

Phân tầng LLM hiện tại đã phù hợp: LLM chọn tactic khi encounter bắt đầu hoặc kế hoạch gãy; tactical policy chọn từng micro-action khi `ready_at` tới. Không cần thêm một controller LLM mới cho từng đòn đánh.

## 3. Những contract nền tảng nên chốt lại

### 3.1. Worldseed Lockfile

Worldseed hiện dùng version range cho engine/pack nhưng lại cam kết cùng hash. Trước genesis cần resolve thành lockfile chứa chính xác:

- Engine build.
- Pack version và content hash.
- WASM runtime/ABI và module hash.
- Migration set.
- Generator/law version.
- Quy tắc cấp ID deterministic.

Worldseed dùng để chia sẻ sẽ trỏ tới lockfile bất biến này.

### 3.2. Prehistory Timeline

Genesis bắt đầu ở tick 0, nhưng N năm tiền sử phải thật sự tiến qua N năm local time. Khi người chơi xuất hiện, tuổi, luật, dòng họ, event và ruin phải mang timestamp thật thay vì bị nén về tick 0.

### 3.3. Canonical Macro-History

Ruins, trade route, biên giới và grievance do prehistory tạo phải được commit ở dạng macro-delta trước khi người chơi mở chunk. Việc khám phá chunk chỉ chi tiết hóa kết quả đã khóa, không được làm đổi lịch sử theo đường camera.

### 3.4. Proper-Time Process

Mỗi need/effect/process phải khai báo clock domain:

- World local time.
- DivineTime.
- Proper time của entity.
- Một clock domain đặc biệt do law quy định.

Khi entity qua portal hoặc world đổi time scale, deadline phải được rebase theo rule của process. Nếu không, một người bệnh có thể khỏi hoặc chết tức thì chỉ vì đi vào world có tốc độ thời gian khác.

### 3.5. Effect chỉ biểu diễn hậu quả

Bệnh, độc, blessing và thương tích phù hợp với `Effect`. Cấm vận, kiểm duyệt và dị giáo không nên dùng Effect làm nguồn sự thật; chúng là policy/claim/relationship được actor duy trì. Effect chỉ biểu diễn hậu quả dẫn xuất như giá tăng, mất access hoặc giảm reputation.

### 3.6. Hai loại WASM Context

Tách hai contract:

- `AgentModuleContext`: chỉ thấy observation của actor; dùng cho spell, tactic và behavior.
- `SystemResolverContext`: đọc một authoritative read-set được giới hạn bằng capability; dùng cho terrain generator, dịch tễ, khí hậu và economy resolver.

Cả hai đều là hàm thuần, có canonical input/output, fuel và proposal commit qua Core.

### 3.7. True God và invariant ngoài simulation

Tài liệu đồng thời nói True God toàn quyền và có một số content invariant mà cả Hard Override cũng không phá được. Nên chốt cách diễn đạt:

> True God có toàn quyền trong simulation; host safety policy đứng ngoài simulation và không phải một loại sức mạnh trong thế giới.

## 4. Quan hệ xã hội và thể chế

### 4.1. Kinship Graph và Household Lifecycle

Tách cha mẹ sinh học, cha mẹ xã hội, hôn phối, giám hộ, người thừa kế và thành viên cùng hộ. Hộ có thể tách, nhập, nhận con nuôi hoặc tuyệt tự, tự sinh tranh chấp kế vị và nghĩa vụ gia đình.

### 4.2. Demography và Care Economy

Mô hình cohort tuổi, fertility, mortality, dependency ratio và care task. Trẻ nhỏ, người già, người bệnh và người khuyết tật cần thời gian chăm sóc; chiến tranh hoặc dịch bệnh có thể tạo khủng hoảng chăm sóc dù tổng dân số vẫn cao.

Tham khảo: [UN DESA — Households and Living Arrangements Data](https://www.un.org/development/desa/pd/data/household-and-living-arrangements).

### 4.3. Collective Action Threshold

Đình công, nổi dậy, dân quân, quyên góp và phong trào cải cách dùng cùng primitive:

- Ngưỡng tham gia cá nhân.
- Kỳ vọng số người khác sẽ tham gia.
- Chi phí và rủi ro theo belief.
- Free-rider.
- Cam kết công khai/bí mật.
- Tín hiệu đàn áp hoặc nhượng bộ.

Một khác biệt nhỏ trong phân bố ngưỡng có thể khiến hai đám đông tương tự đi tới kết quả hoàn toàn khác.

Tham khảo: [Granovetter — Threshold Models of Collective Behavior](https://doi.org/10.1086/226707) và [Centola & Macy — Complex Contagions](https://doi.org/10.1086/521848).

### 4.4. Commons Governance

Rừng, đồng cỏ, hệ thống tưới, mana well và ngư trường có:

- Boundary tài nguyên và nhóm được quyền sử dụng.
- Hạn mức khai thác phù hợp điều kiện địa phương.
- Monitoring.
- Graduated sanction.
- Cơ chế giải quyết tranh chấp rẻ.
- Quyền sửa luật của người bị ảnh hưởng.
- Các tầng quản trị lồng nhau.

Tài nguyên chung không mặc định phải tư hữu hóa hoặc bị khai thác tới cạn.

Tham khảo: [Ostrom Workshop — Design Principles](https://ostromworkshop.indiana.edu/courses-teaching/teaching-tools/ostrom-design/index.html).

### 4.5. State Capacity và Delegation Chain

Một quốc gia không tự động thực hiện policy. Quyết định phải đi qua:

```text
office → mandate → ngân sách → quan chức → đơn vị thực thi → kết quả
```

Mỗi cạnh có chậm trễ, tham nhũng, thiếu năng lực, hiểu sai và principal–agent risk. Thuế, census, bổ nhiệm và hệ thống báo cáo quyết định năng lực thật của nhà nước.

### 4.6. Sources of Legitimacy và Compliance Choice

Tách chính danh từ:

- Kết quả đạt được.
- Thủ tục công bằng.
- Truyền thống.
- Charisma.
- Tôn giáo.
- Bản sắc cộng đồng.

Một người có thể tuân vì tin luật đúng, vì sợ hình phạt hoặc vì mọi người xung quanh đang tuân. Ba động cơ tạo kết quả khác nhau khi state suy yếu.

Tham khảo: [World Development Report 2017 — Governance and the Law](https://www.worldbank.org/en/publication/wdr2017).

### 4.7. Legal Pluralism và Conflict of Laws

Một cá thể có thể đồng thời chịu luật quốc gia, guild, dòng họ, tôn giáo và treaty liên-world. Cần mô hình hóa:

- Precedence giữa các hệ luật.
- Venue và jurisdiction phù hợp.
- Dẫn độ.
- Miễn trừ.
- Chống xét xử hai lần.
- Version luật tại lúc hành vi xảy ra.
- Luật thủ tục tại lúc xét xử.

Tham khảo: [Sally Engle Merry — Legal Pluralism](https://doi.org/10.2307/3053638).

### 4.8. Message Lifecycle và Rumor Cascades

Tin tức là message được sao chép qua contact graph với latency, fidelity, attention, source trust và động cơ sửa nội dung. Nhiều phiên bản cùng cạnh tranh sẽ tạo tuyên truyền, đính chính, moral panic và tin đồn tự chết mà không cần Yuu quyết định kết quả.

### 4.9. Adoption Bias cho Practice, Norm và Technique

Tách “nhận được thông điệp” khỏi “chấp nhận làm theo”. Entity có thể bắt chước theo:

- Conformity.
- Prestige.
- Thành công quan sát được.
- Huyết thống/ingroup.
- Người hướng dẫn có chuyên môn.
- Hành động khó giả mạo.

Fashion, taboo, tôn giáo và kỹ thuật nhờ vậy có tốc độ lan khác nhau trên cùng một mạng.

Tham khảo: [Cultural Evolution of Conformity and Anticonformity](https://doi.org/10.1073/pnas.2004102117).

### 4.10. Religion Doctrine–Ritual–Authority

Tôn giáo cần doctrine graph, lịch nghi lễ, sacred site, giáo sĩ, quyền diễn giải và schism. Belief của tín đồ tách khỏi việc thần có thật; một giáo hội có thể hiểu sai chính vị thần mình thờ.

### 4.11. Credibility-Enhancing Ritual

Lời giảng chỉ tạo message; hy sinh tài sản, giữ lời thề, hành hương hoặc sống khổ hạnh tạo bằng chứng về commitment. Ritual ảnh hưởng trust, cooperation và mức người khác tin doctrine thay vì chỉ cộng `faith_point`.

Tham khảo: [Henrich — Credibility-Enhancing Displays](https://www2.psych.ubc.ca/~henrich/pdfs/evolution%20of%20costly%20displays%20_henrich%202009.pdf).

### 4.12. Language Evolution và Translation

Ngôn ngữ có từ vựng, dialect, mutual intelligibility, loanword và semantic drift. Phiên dịch sai có thể làm hỏng treaty, spell, lời tiên tri hoặc bài giảng mà không cần nhân vật cố tình nói dối.

### 4.13. Status, Estate và Social Mobility

Địa vị là bó quyền, nghĩa vụ và access theo estate/caste/profession, không phải personality trait. Đường thay đổi địa vị có thể gồm sinh ra, hôn nhân, mua chức, thi cử, cải đạo hoặc chiến công, tạo elite closure và tầng lớp mới nổi.

## 5. Kinh tế, đô thị và đời sống thường nhật

### 5.1. Property Bundle và Land Tenure

Tách quyền sử dụng, loại trừ, hưởng lợi, chuyển nhượng và thừa kế. Một parcel có thể đồng thời có vua là chủ danh nghĩa, tá điền canh tác, làng có quyền lấy củi và giáo hội có sacred claim.

### 5.2. Credit, Default và Bankruptcy

Khoản vay có principal, maturity, collateral, guarantor, seniority và thủ tục default. Một primitive này sinh ra trade credit, cho vay nặng lãi, bank run, tịch biên, lao dịch vì nợ và khủng hoảng dây chuyền.

### 5.3. Labor Contract, Firm và Guild

Lao động có wage, thời hạn, giờ làm, rủi ro, quyền nghỉ và trách nhiệm công cụ. Firm/guild gom capital, hợp đồng và knowledge để hình thành thất nghiệp, bóc lột, đình công, đào tạo nghề và cạnh tranh giành chuyên gia.

### 5.4. Shipment và Chain of Custody

Hàng hóa không teleport giữa hai inventory. Shipment có carrier, capacity, route, departure, spoilage, guard và custody transfer. Tắc cầu hoặc cướp đường sẽ lan thành thiếu hàng, tăng giá và vi phạm hợp đồng có cause chain.

### 5.5. Parcel-Based Urban Growth

Parcel có rent, accessibility, frontage, nước, hazard và quyền sử dụng. Hộ/cơ sở chọn hoặc chiếm vị trí dựa trên các yếu tố này, từ đó đường mòn, chợ, khu giàu, khu ổ chuột và ngoại ô tự hình thành.

Tham khảo: [UN-Habitat — Economic Foundations for Sustainable Urbanization](https://unhabitat.org/economic-foundations-for-sustainable-urbanization-a-study-on-three-pronged-approach-planned-city).

### 5.6. Daily Places và Social Routine

Giếng, bếp, chợ, quán, nhà tắm, đền, bến xe và chỗ ngủ có reservation/queue thật. Việc gặp nhau lặp lại ở các địa điểm này tạo contact graph cho tình bạn, gossip, tán tỉnh, xích mích và lây bệnh.

### 5.7. Festival, Art, Cuisine và Fashion

Thêm lịch lễ hội, thi đấu, âm nhạc, món ăn, trang phục và nghệ thuật. Chúng tiêu thụ tài nguyên, tạo việc làm, phát tín hiệu địa vị và lan theo prestige; thế giới cần niềm vui và sự tầm thường, không chỉ thảm họa và tội phạm.

### 5.8. Education Institution và Archive

Trường học, apprenticeship, thi cử, thư viện và archive có tuyển sinh, curriculum, gatekeeping và kinh phí. Một triều đại có thể kiểm duyệt archive, một cuộc cháy có thể làm mất tri thức, hoặc một bản chép sai có thể tạo trường phái phép mới.

## 6. Di cư, sinh thái và tiếp xúc liên-world

### 6.1. Migration, Refugee và Diaspora Network

Quyết định di cư dựa trên belief về an toàn, lương, chi phí đường đi và contact ở đích. Diaspora gửi tiền, môi giới việc, giữ ngôn ngữ và có lòng trung thành kép. Di cư là quyết định của hộ/mạng xã hội, không chỉ của một cá thể.

### 6.2. Ecological Succession và Invasive Species

Ngoài predator/carrying capacity, thêm thụ phấn, phân hủy, phát tán hạt, tạo đất và habitat patch. Portal có thể mang loài, ký sinh, mầm bệnh hoặc mana ecology sang world mới, tạo một dạng “Columbian exchange” siêu nhiên.

### 6.3. Contact Regime và Portal Quarantine

Sau khi portal mở, hai phía phải đặt:

- Quarantine và kiểm tra sinh học/ma thuật.
- Thuế quan.
- Chuẩn đo lường.
- Quy chế pháp nhân.
- Quyền cư trú.
- Luật mang sinh vật, vật phẩm và soul qua cổng.
- Phiên dịch và cơ chế giải quyết tranh chấp.

Một cổng có thể trở thành trade enclave, trại tị nạn, thuộc địa hoặc ổ dịch.

Tham khảo: [WHO — International Health Regulations](https://www.who.int/publications/i/item/9789241580410) và [IPBES — Invasive Alien Species Assessment](https://ict.ipbes.net/ipbes-ict-guide/data-and-knowledge-management/citations-of-ipbes-assessments/invasive-alien-species-assessment).

### 6.4. Disaster Response và Mutual Aid

Thiên tai cần warning, evacuation, shelter, kho dự phòng, cứu hộ, volunteer network và reconstruction capacity. Cùng một trận động đất có thể chỉ gây thiệt hại cục bộ ở xã hội có tổ chức hoặc làm sụp nhà nước ở xã hội mất chính danh.

## 7. Thứ tự ưu tiên đề xuất

1. **Chrono-Turn Timeline + action phases + reactions** — tạo gameplay thật ngay.
2. **Proper-Time Process + Simultaneous Commit** — bảo vệ tính đúng khi có speed cực cao và nhiều world.
3. **Kinship/household + daily places + message lifecycle** — hiệu quả cao nhất để NPC trông như đang sống.
4. **State capacity + collective action + legal pluralism** — làm chính trị và hệ tội phạm hiện có hoạt động đúng.
5. **Religion + cultural transmission** — đặc biệt quan trọng cho Pantheon.
6. **Credit/property/logistics** — biến kinh tế từ thống kê thành cause chain.
7. **Portal contact/quarantine** — khiến multi-world khác teleport map thông thường.
8. **Urban growth, ecology và plugin mở rộng** — triển khai sau khi primitive xã hội đã ổn.

## 8. Những ý không lặp lại

Hai cơ chế sau đã được `docs/idea.md` mô hình hóa đủ rõ nên không cần đề xuất lại:

- LLM chọn kế hoạch chiến thuật, còn policy xử lý micro-action và hành vi thường ngày.
- Treaty dùng con tin, giám sát, thương mại hoặc bảo chứng để tạo cam kết đáng tin.