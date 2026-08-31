# Viết content pack cho My Open World

> Tài liệu cho người viết mod. `PF-13`.
>
> Mọi thứ ở đây đúng với **cả** `content/core`. Không có đường đặc quyền cho
> nội dung chính thức — `plan.md §P10.7` chốt điều đó từ Giai đoạn 0, và
> `content/core/pack.yaml` đi qua đúng `Registry::add_from_dir` mà pack của bạn
> đi qua. Nếu một chỗ nào đó trong tài liệu này sai, nó sai với cả nội dung gốc,
> và đó là một lỗi sẽ được phát hiện hằng ngày.

Pack mẫu đầy đủ: [`src/content/example-thirdparty/`](../src/content/example-thirdparty).

---

## 1. Bảo đảm quan trọng nhất

**Cài thêm một pack không làm hỏng save cũ.**

Một world tạo bằng `core` thôi vẫn mở được sau khi bạn cài mười mod, vì content
hash của một pack chỉ phụ thuộc vào **nội dung của chính nó** — không phụ thuộc
vào những gì nằm cạnh nó, không phụ thuộc thứ tự thư mục trên đĩa, không phụ
thuộc hệ điều hành.

Điều đó được kiểm tự động ở
[`crates/mow-plugin/tests/thirdparty.rs`](../src/crates/mow-plugin/tests/thirdparty.rs),
chạy mỗi PR, trên chính pack mẫu trong repo.

Chiều ngược lại cũng đúng và cũng được kiểm: một save **có** dùng pack của bạn
sẽ **từ chối mở** nếu pack đó biến mất hoặc đổi nội dung. Nó từ chối hẳn chứ
không nạp một nửa — một world nạp một nửa sẽ tham chiếu tới những định nghĩa
không tồn tại, và nó hỏng rải rác, khó truy hơn nhiều so với một lỗi rõ ràng
lúc mở file (`§22.30`).

---

## 2. Cấu trúc một pack

```text
my-pack/
├── pack.yaml          # manifest — bắt buộc
├── content/           # vật phẩm, công thức, bảng tra
├── laws/              # luật DSL Tier 0
├── modules/           # module WASM
├── prompts/           # persona, policy
└── generators/        # địa hình, khí hậu, loài
```

**Tên thư mục quyết định quyền cần xin.** Đó là chủ đích: nếu quyền suy từ một
trường bạn khai trong file, thì lời khai đó chính là thứ đang cần kiểm.

---

## 3. `pack.yaml`

```yaml
id: example_thirdparty          # cũng là NAMESPACE của mọi id trong pack
version: "1.0.0"
name: "Lò bánh mì Veskar"
description: "Thêm nghề làm bánh và một luật ủ bột."

requires:
  - id: core
    version: ">=0.1"

overrides: []                   # id của pack khác mà bạn CỐ Ý ghi đè

capabilities:                   # xin ít nhất có thể
  - define_law

tests:
  - smoke/genesis.yaml          # kịch bản `mow-cli pack test` sẽ chạy
```

### `id` là namespace

Chữ thường, gạch dưới, **không có dấu chấm**. Dấu chấm là dấu phân cấp của id
nội dung (`core.apple`), nên cho phép nó trong namespace sẽ làm `a.b` với id
`c` và `a` với id `b.c` không phân biệt được.

Mọi id bạn đăng ký phải bắt đầu bằng `<id-pack>.` — `§22.29`. Điều này được thi
hành lúc nạp, không phải bằng quy ước.

---

## 4. Quyền (`capabilities`)

**Mặc định là không có quyền gì** ngoài dữ liệu tĩnh. Đại đa số pack không cần
khai dòng nào.

| Quyền | Cho phép | Cảnh báo hiện cho người cài |
|---|---|---|
| *(mặc định)* | `content/`, bảng tra | thêm dữ liệu tĩnh — không đổi cách thế giới vận hành |
| `define_law` | `laws/` | viết luật mới: đổi kết quả mô phỏng |
| `define_module` | `modules/` | chạy code trong sandbox: có fuel và trần bộ nhớ |
| `define_prompt` | `prompts/` | chạm vào đường LLM: đổi cách nhân vật nghĩ |
| `define_generator` | `generators/` | đổi generator: hai world cùng seed sẽ khác nhau |
| `override_foreign` | ghi đè id pack khác | ghi đè nội dung của pack khác |

**Quyền được kiểm bằng nội dung thật, không bằng lời khai.** Khai
`capabilities: []` mà thư mục có `laws/` thì pack bị **từ chối nạp** — và bị từ
chối trước khi bất kỳ định nghĩa nào của nó vào sổ.

### Ghi đè cần **hai** thứ

Khai id trong `overrides` **và** xin `override_foreign`. Nếu chỉ cần khai
`overrides` thì quyền ghi đè tự cấp được bằng một dòng YAML, và nó không còn là
quyền nữa.

Xung đột không khai báo là **lỗi**, không phải "ai load sau thì thắng" — vì
"ai load sau thì thắng" biến thứ tự nạp thành một phần vô hình của luật chơi mà
không ai gỡ được.

---

## 5. Vòng lặp phát triển

```bash
mow-cli pack validate content/core content/my-pack   # manifest, namespace, quyền
mow-cli pack test content/my-pack                    # chạy kịch bản đã khai
mow-cli pack watch content/my-pack                   # kế hoạch nạp nóng (chỉ dev)
```

`pack validate` nhận **nhiều** thư mục vì phụ thuộc chỉ giải được khi cả bộ có
mặt: kiểm riêng pack của bạn sẽ luôn báo thiếu `core`.

### `pack test` không báo xanh cho pack không có test

Một pack không khai `tests` bị coi là **chưa ai kiểm**, không phải "đã đạt".
Báo xanh cho nó là cách nhanh nhất để cả một thư viện mod không có test mà ai
cũng tin là đã kiểm.

### Nạp nóng đi qua migration, không ghi đè tại chỗ

Chỉ ở dev build. Đổi nội dung thì **phải tăng version** — event log ghi *"dùng
định nghĩa `my_pack.bread` phiên bản 1"*, và nếu v1 đổi nội dung tại chỗ thì
replay cùng log đó ra kết quả khác. Thế giới vẫn chạy, save vẫn mở được, chỉ là
hash không còn tái lập, và **không có gì báo**.

Ba loại thay đổi, ba cách xử lý:

| Thay đổi | Xử lý |
|---|---|
| thêm id mới | nạp thẳng — không event nào tham chiếu nó |
| sửa id đang có | version mới, **bản cũ ở lại** để replay |
| xóa id đang có | tombstone, không xóa — thế giới đang tham chiếu nó |

---

## 6. Schema có `$id` version

Schema JSON của dự án mang `$id` có version, ví dụ:

```json
"$id": "https://myopenworld.dev/schemas/config/app_config.v1.json"
```

Version nằm **trong** định danh chứ không cạnh nó. Muốn đổi hình dạng thì phát
hành `v2`; `v1` còn diễn giải được mãi mãi. Cùng quy tắc với id nội dung ở
`§19.7.2` và với ký hiệu hình ảnh ở `§18.14.6`: **không bao giờ đổi nghĩa của
một thứ đã phát hành.**

---

## 7. Những giới hạn không vượt được

Không phải để làm khó — chúng là những chỗ mà vượt qua sẽ phá một bảo đảm mà
mọi thứ khác dựa vào.

- **Không có float trên đường commit** (`§P10.2.1`). Luật của bạn chạy trên
  fixed-point và số nguyên có đơn vị. Một `f64` ở đây làm hai máy ra hai kết
  quả, và `INV-22-9` hỏng.
- **Không `eval`, không code do LLM sinh chạy trực tiếp** (`§15.3`). Luật là
  một cây biểu thức đã phân tích, tập phép toán **đóng**, mọi giá trị mang đơn
  vị.
- **Module WASM không có WASI**: không tệp, không mạng, và **không hỏi được
  giờ**. Cái cuối dễ quên nhất và là cái phá determinism.
- **Module chạy ở context `Agent` không đọc được state authoritative**
  (`§13.9.6`). Một module xin quyền đó bị **từ chối nạp**, không phải bị bỏ qua
  phần vi phạm — nhầm lẫn giữa hai context là con đường ngắn nhất tạo ra lỗ
  hổng toàn tri.
- **Mọi state change đi qua transaction handler** (`INV-22-1`). Không có API
  nào cho pack ghi thẳng.

---

## 8. Đọc thêm

| Chủ đề | Chỗ đọc |
|---|---|
| Bất biến phải giữ | [`idea.md §22`](idea.md) |
| Registry, namespace, load order | [`crates/mow-plugin`](../src/crates/mow-plugin) |
| DSL luật Tier 0 | [`crates/mow-magic/src/dsl.rs`](../src/crates/mow-magic/src/dsl.rs) |
| Sandbox và capability | [`crates/mow-magic/src/sandbox.rs`](../src/crates/mow-magic/src/sandbox.rs) |
| Vòng lặp phát triển pack | [`plan.md §P10.7`](plan.md) |
