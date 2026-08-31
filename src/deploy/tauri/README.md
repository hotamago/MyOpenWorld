# `deploy/tauri/` — ban desktop

## PA-09 la mot SMOKE BUILD, khong phai san pham

`progress.md PA-09` noi ro dieu nay va no dang duoc nhac lai o day, vi day la
cho de hieu nham nhat:

> Day la **smoke build trong CI**, khong phai be mat san pham phai bao tri; no
> ton tai de bat som cac loi dong goi, duong dan file va quyen he thong.

Ba lop loi do chi lo ra khi dong goi that, va chung re de sua o Giai doan A,
dat de sua o Giai doan F:

- **Duong dan**: `content/` va `config/` nam canh binary trong ban dong goi,
  khong nam o thu muc lam viec. Code doc chung bang duong dan tuong doi se chay
  o dev va hong o ban phat hanh.
- **Quyen**: ghi save vao thu muc du lieu cua ung dung, khong vao thu muc cai
  dat. Windows tu choi cai thu hai, va no chi tu choi sau khi dong goi.
- **CSP**: WebView chan `connect-src` mac dinh. WebSocket toi backend nhung se
  bi chan im lang neu khong khai bao — va "im lang" la phan te nhat.

## Bat dau lam viec

```bash
pnpm --filter web build          # dung frontend truoc
cd deploy/tauri/src-tauri
cargo tauri dev                  # can `cargo install tauri-cli`
```

## Bao mat: devtool khong bao gio vao day

Ban desktop dong goi `mow-server` **khong** co feature `devtool`
(`plan.md §P10.5`). Ranh gioi do duoc kiem o ba lop, xem
`crates/mow-devtool/tests/khong_co_trong_release.rs`.
