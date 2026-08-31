# `deploy/` — chạy và kiểm thử trong môi trường cách ly

Một lệnh, từ thư mục gốc repo:

```bash
./mow up        # Linux, macOS, Git Bash
.\mow.ps1 up    # PowerShell
```

## Vì sao có thư mục này khi `§P3.4` nói "chạy được không cần Docker"

Hai câu đó không mâu thuẫn, chúng trả lời hai câu hỏi khác nhau.

`§P3.4` nói về **người chơi và vòng lặp phát triển hằng ngày**: bản desktop phải
chạy bằng SQLite, bus in-process và chỉ mục nhúng, không ai phải cài Docker để
sửa một dòng trong `mow-math`. Điều đó vẫn nguyên vẹn — `./mow native cargo test`
chạy thẳng trên máy và là đường nhanh nhất.

`deploy/` trả lời câu hỏi khác: **khi kết quả không khớp thì làm sao biết là do
code hay do máy.** Một bài determinism fail trên CI mà không fail trên máy bạn
là tình huống sẽ xảy ra, và khi nó xảy ra thì việc đầu tiên cần loại trừ là
toolchain. Toolbox loại trừ nó: cùng image, cùng phiên bản Rust/Python/Node,
cùng libc, cùng số luồng.

Nó còn là hàng rào an toàn. `cargo test` của một workspace lớn chạy build script
và proc-macro tùy ý; `pnpm install` chạy script cài đặt của hàng trăm gói. Trong
container, chúng không với tới ổ đĩa, biến môi trường hay khóa SSH của máy thật.

## Hai nhóm dịch vụ

| Nhóm | Profile | Gồm |
|---|---|---|
| **toolbox** | *(mặc định)* | Rust 1.90, Node 24, pnpm, uv, protoc, SQLite, target wasm32 |
| **app** | `app` | frontend Vue (`web`), sidecar nhận thức Python (`agent`) |
| **ai** | `ai` | máy chủ embedding cục bộ (vLLM + GPU) |
| **infra** | `infra` | Postgres 17, NATS JetStream, Qdrant, Jaeger, MinIO |

### "Sao chỉ có toolbox? Frontend, backend, agent đâu?"

Câu hỏi đúng, và câu trả lời cần nói thẳng.

`app` có **hai** tiến trình: `web` (Vue + Pixi, `PA-05`) và `agent` (sidecar
nhận thức, `PC-01`/`PF-12`). Cả hai chạy được ngay bằng `./mow app up`.

**Không có `mow-server`.** `plan.md §P3.1` mô tả `mow-server` và `mow-worker`
là hai binary trung tâm — "một nguồn quyền lực duy nhất" — nhưng `progress.md`
chưa bao giờ có task dựng chúng. 147/147 task xây engine dưới dạng **thư viện**
cộng với `mow-cli`; workspace hôm nay có đúng hai binary, `mow-cli` và
`mow-codegen`.

Hệ quả thực tế: `web` chạy với dữ liệu của chính nó, `agent` trả lời `/health`,
và cầu nối giữa chúng với engine là `mow-cli` chạy tay. Đó là hình trạng
desktop-first của `§P3.4`, chưa phải server mode.

`infra` **không** bật mặc định, và đó là chủ đích. Tới hết Giai đoạn B thì
SQLite cộng bus in-process là đủ (`P0-07`, `P0-08`); bắt mọi người chạy năm
container để sửa một hàm số học là thuế vô ích. `PC-20` mới là lúc dựng hiện
thực thứ hai và chứng minh nó vượt đúng bộ test hợp đồng đã viết từ `P0-07`.

```bash
./mow infra up      # khi tới Giai đoạn C
```

## Bộ lệnh

| Lệnh | Làm gì |
|---|---|
| `./mow up` | dựng toolbox, cài phụ thuộc |
| `./mow shell` | vào trong toolbox |
| `./mow test [args]` | `cargo test` trong container |
| `./mow build` | build workspace |
| `./mow lint` | `fmt --check` + `clippy -D warnings` |
| `./mow determinism` | chạy lại với 1, 2, 8 luồng rồi so state hash (`§P7.5`) |
| `./mow exec <lệnh>` | lệnh bất kỳ bên trong |
| `./mow native <lệnh>` | chạy thẳng trên máy, bỏ qua container |
| `./mow app up\|down\|logs` | frontend + sidecar nhận thức |
| `./mow ai up\|down\|logs` | máy chủ embedding cục bộ (cần GPU NVIDIA) |
| `./mow infra up\|down` | hạ tầng server mode |
| `./mow logs [dịch vụ]` | xem log |
| `./mow doctor` | máy thật có đủ gì, thiếu gì |
| `./mow down` | tắt, giữ volume |
| `./mow reset` | tắt và **xóa volume** (hỏi xác nhận) |

`./mow determinism` là lệnh đáng chú ý nhất. Nó chạy cùng một bộ test ba lần với
`RAYON_NUM_THREADS` khác nhau. Trên máy thật bạn không điều khiển được số nhân
một cách đáng tin; trong container thì có, nên đây là chỗ duy nhất mà "cùng kết
quả bất kể số luồng" thật sự được kiểm chứng chứ không chỉ được hy vọng.

## Cổng ra máy thật

Cố ý đặt lệch dải thường dùng để không đụng Postgres hay Qdrant bạn đang chạy
sẵn cho việc khác — nhưng **dưới 49152**, và vế thứ hai mới là vế đắt:

> Dải 49152–65535 là dải cổng động. Trên Windows, Hyper-V và WinNAT đặt trước
> hàng loạt khối trong dải đó, và khối nào thì **đổi sau mỗi lần khởi động**.
> Bind vào một cổng đã bị đặt trước cho lỗi `An attempt was made to access a
> socket in a way forbidden by its access permissions` — một câu không hề
> nhắc tới Hyper-V, nên nó thường bị đọc nhầm thành "cổng đã có ai chiếm".
>
> Xem máy bạn đang đặt trước những khối nào:
> `netsh interface ipv4 show excludedportrange protocol=tcp`


| Dịch vụ | Cổng | Ghi đè bằng |
|---|---|---|
| Postgres | `15432` | `MOW_PG_PORT` |
| NATS | `14222` | `MOW_NATS_PORT` |
| NATS monitor | `18222` | `MOW_NATS_MON_PORT` |
| Qdrant HTTP | `16333` | `MOW_QDRANT_PORT` |
| Qdrant gRPC | `16334` | `MOW_QDRANT_GRPC_PORT` |
| Jaeger UI | `16686` | `MOW_JAEGER_UI_PORT` |
| OTLP | `14317` | `MOW_OTLP_PORT` |
| MinIO | `19000` / `19001` | `MOW_MINIO_PORT` / `MOW_MINIO_CONSOLE_PORT` |
| Web (Vite) | `15173` | `MOW_WEB_PORT` |
| Agent | `18765` | `MOW_AGENT_PORT` |
| Embedding | `18080` | `MOW_EMBED_PORT` |

## Cache

`cargo`, `target`, `uv` và `pnpm` nằm trên volume có tên, **không** dùng chung
với `target/` của máy thật. Dùng chung là nguồn của những lỗi link khó hiểu:
hai toolchain khác nhau ghi vào cùng một thư mục artifact và cái nào chạy sau
thì thắng.

Đổi lại, lần build đầu trong container là build từ đầu. Chuyện đó chỉ xảy ra
một lần.

## Image chạy thật

`docker/server.Dockerfile` build **`mow-cli`** ở chế độ release — không phải
`mow-server`, vì binary đó chưa tồn tại (xem mục trên). Bản trước của file đó
build `--bin mow-server --bin mow-worker` và vì thế chưa bao giờ build được.

Nó có một bước không phải ai cũng nghĩ tới:

```dockerfile
RUN if strings /out/mow-cli | grep -q 'mow_devtool'; then exit 1; fi
```

`§P10.5` yêu cầu devtool không có trong bản phát hành. Feature flag đã lo phần
đó, nhưng feature flag là thứ có thể bị bật nhầm qua một phụ thuộc bắc cầu và
không ai nhận ra. Dòng trên biến "chúng tôi tin là không có" thành "build fail
nếu có" — đúng tinh thần `P0-11`.

## Khi Docker không có

`./mow doctor` sẽ nói rõ thiếu gì. Không có Docker thì mọi thứ vẫn chạy được
bằng `./mow native <lệnh>`, chỉ mất phần cách ly và phần bảo đảm toolchain
giống nhau.
