# `config/`

Thu tu lop, **sau ghi de truoc**:

```
base.yaml  ->  <env>.yaml  ->  bien moi truong MOW_*  ->  tham so dong lenh
```

- `base.yaml` la lop nen, luon duoc nap. Moi truong khac chi ghi de phan can doi.
- `<env>.yaml` la tuy chon. Khong can tao file rong chi de ton tai.
- Bien moi truong dung `__` lam dau phan cap: `MOW_LLM__MODE=LIVE` -> `llm.mode`.

## Bi mat khong nam o day

Moi file trong thu muc nay **duoc commit**. API key, DSN co mat khau, chung chi
— tat ca nam o `.env` hoac secret manager cua moi truong (`plan.md §P10.6`).

`AppConfig::validate` co mot buoc quet bat nhung gia tri trong giong bi mat va
**tu choi khoi dong**. No khong chi canh bao, vi mot canh bao trong log khoi
dong la thu khong ai doc.

## Doi config anh huong mo phong

`sim.*`, `budget.*` va `llm.cognitive_latency_ticks` deu anh huong ket qua mo
phong. Doi chung giua chung phai ghi vao event log (`§8.4`), neu khong replay se
lech ma khong co gi trong lich su giai thich tai sao.
