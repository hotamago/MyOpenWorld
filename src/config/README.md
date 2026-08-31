# `config/` — cấu hình nằm ở đâu, và cái gì nằm ở đâu

Hai chỗ, và ranh giới giữa chúng là **bí mật hay không**, chứ không phải quan
trọng hay không.

| Chỗ | Chứa gì | Commit? |
|---|---|---|
| `src/config/*.yaml` | mọi thứ ảnh hưởng thế giới: model nào, bao nhiêu chiều, độ trễ nhận thức mấy tick | **có** |
| `.env` (gốc repo) | chỉ bí mật: API key, DSN có mật khẩu | **không** (`.gitignore` chặn) |

`config/*.yaml` được commit **vì** nó ảnh hưởng mô phỏng: `§8.4` đòi mọi thứ
đổi kết quả phải đọc lại được từ lịch sử. Một `temperature` nằm ngoài repo là
một thế giới không replay được.

## Thứ tự lớp, sau ghi đè trước

```
base.yaml  →  <env>.yaml  →  biến môi trường MOW_*  →  tham số dòng lệnh
```

- `base.yaml` luôn được nạp. Môi trường khác chỉ ghi đè phần cần đổi.
- `<env>.yaml` là tùy chọn. Không cần tạo file rỗng chỉ để tồn tại.
- Biến môi trường dùng `__` làm dấu phân cấp: `MOW_LLM__MODE=LIVE` → `llm.mode`.

Bốn môi trường có sẵn:

| `--env` | Dùng khi | LLM | Embedding |
|---|---|---|---|
| `dev` *(mặc định)* | sửa code hằng ngày | `STUB` | `STUB` |
| `test` | `cargo test` | `STUB` | `STUB` |
| `live` | thử mô hình thật trên máy bạn | `LIVE` | `LIVE` |
| `prod` | triển khai thật (log JSON) | `LIVE` | `LIVE` |

`dev` **phải** chạy offline, miễn phí và xác định. Đó là môi trường mà vòng lặp
sửa–chạy dùng, và một chuyển đổi lặng lẽ sang `LIVE` ở đó nghĩa là mỗi lần chạy
test đều tốn token — phát hiện bằng hóa đơn, không bằng test đỏ.

## `api_key_env` nhận TÊN biến, không nhận khóa

```yaml
llm:
  api_key_env: OPENROUTER_API_KEY     # ĐÚNG — tên biến
  api_key_env: sk-or-v1-8825...       # SAI — và file này được commit
```

`AppConfig::validate` từ chối khởi động ở dòng thứ hai. Nó cũng từ chối một tên
bắt đầu bằng `MOW_`: tiền tố đó thuộc về lớp cấu hình, nên `MOW_FOO_API_KEY`
sẽ bị đọc thành **field** `foo_api_key` và chết bằng `unknown field` — một
thông báo không nhắc gì tới khóa API. (Lỗi này đã xảy ra thật trong lúc dựng
phần cấu hình này.)

## Bắt đầu từ đâu

```bash
cp .env.example .env          # rồi điền OPENROUTER_API_KEY
cargo run -p mow-cli -- config check
```

`config check` nạp `.env`, nạp config, kiểm, rồi in tóm tắt — kể cả một dòng
"tóm lại" nói thẳng cái gì sẽ thật sự chạy. Nó **không** chạm mạng và **không**
in giá trị khóa (chỉ tên biến và có/thiếu), nên đầu ra của nó dán vào issue
được.

Ba mức kiểm, tăng dần:

```bash
mow-cli config check --env live   # file có hợp lệ, biến có mặt chưa   (offline)
mow-cli llm ping     --env live   # khóa có DÙNG ĐƯỢC không            (gọi thật)
mow-cli embed probe  --env live   # máy chủ embedding đúng số chiều chưa
```

Hai lệnh sau phải gọi tường minh vì chúng tiêu token. Một lệnh kiểm cấu hình mà
lặng lẽ gọi mạng là một lệnh người ta chỉ dám chạy một lần.

## Mô hình đang dùng

| | |
|---|---|
| LLM | `deepseek/deepseek-v4-flash-0731` qua OpenRouter |
| Embedding | `jinaai/jina-embeddings-v5-text-small-retrieval`, chạy cục bộ bằng vLLM (`./mow ai up`) |
| Số chiều | **1024**, khai đúng một chỗ: `vector.dimension` |

`embedding` cố ý **không** có trường `dimension`. Hai chỗ khai cùng một con số
là hai chỗ để chúng lệch nhau, và khi lệch thì triệu chứng là một chỉ mục im
lặng trả về kết quả sai, không phải một lỗi.

Hai điều đã học được khi chạy thật, ghi lại vì cả hai đều mất thời gian:

- **`deepseek-v4-flash` là model suy luận.** Nó tiêu token cho phần suy luận
  *trước* khi sinh chữ nào. `max_output_tokens: 32` trả về **chuỗi rỗng** kèm
  `finish_reason: length` — trông y hệt "mô hình không có gì để nói".
- **Bản `-retrieval` không cắt Matryoshka được.** Model gốc thì có, nhưng bản đã
  gộp adapter thì vLLM trả `400 does not support Matryoshka embeddings`. Nên
  `send_dimensions: false`, và `vector.dimension` để đúng 1024 gốc.

## Không có khóa thì sao

Chạy được hết. `STUB` không phải chỗ giữ chỗ:

- `llm.mode: STUB` — câu trả lời cố định theo `prompt_id`.
- `embedding.mode: STUB` — băm đặc trưng chạy tại chỗ, xác định tuyệt đối.
  Cho tương đồng **từ vựng**, không phải ngữ nghĩa. Đủ để cả đường ống ký ức
  chạy, kiểm và replay bit-perfect trước khi có bất kỳ khóa nào.

So sánh trên cùng ba câu (`mow-cli embed probe`):

| | gần (kiếm ↔ cuốc) | xa (kiếm ↔ mưa sao băng) |
|---|---|---|
| `STUB` (băm từ vựng) | 0.800 | 0.000 |
| jina v5 qua vLLM | 0.855 | 0.060 |

Con số `0.000` của `STUB` chính là chỗ nó thú nhận mình là gì: hai câu không
chia một từ nào thì trực giao hoàn toàn, dù người đọc thấy cả hai đều nói về
thế giới. Model thật cho `0.060` — nhỏ, nhưng khác 0.

## Đổi config ảnh hưởng mô phỏng

`sim.*`, `budget.*`, `llm.cognitive_latency_ticks` và `llm.temperature_milli`
đều ảnh hưởng kết quả. Đổi chúng giữa chừng phải ghi vào event log (`§8.4`),
nếu không replay sẽ lệch mà không có gì trong lịch sử giải thích tại sao.

`temperature_milli` là số nguyên phần nghìn chứ không phải số thực, và đó không
phải sự cầu kỳ: một `f32` trong event log là một giá trị có thể tuần tự hóa
khác nhau giữa hai bản build (`§P10.2.1`).
