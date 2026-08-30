# Playground Gemini — Brief

Ban duoc toan quyen trong thu muc `playground-gemini/` NAY va chi trong day.
TUYET DOI KHONG duoc doc, sua, tao, xoa bat cu file nao ben ngoai `playground-gemini/`.
Khong duoc dung `git` (khong commit, khong checkout, khong add).
Khong duoc sua `../src`, `../docs`, `../.gitignore`.

## Boi canh

Du an chinh la "My Open World" — game mo phong the gioi 2D top-down, goc nhin tu tren xuong,
the gioi song dong, nhieu thuc the (entity) tu hanh dong. Dac ta o `../docs/idea.md` (chi DOC neu can cam hung, KHONG SUA).

## Nhiem vu cua ban

Xay mot **demo giao dien + do hoa + tuong tac** doc lap, chay duoc ngay tren trinh duyet,
khong phu thuoc backend that. Muc tieu la pho dien ky nang do hoa/UI/UX cua ban.

Yeu cau toi thieu:
1. Render ban do tile 2D top-down, co the pan/zoom muot.
2. Co nhieu entity di chuyen tu dong (AI don gian: doi, met, di tim thuc an, ngu).
3. Click vao entity -> panel Inspector hien trang thai that cua no.
4. Overlay ban do co the bat/tat (vd: nhiet do, do am, mat do dan so) voi legend co don vi.
5. Dieu khien thoi gian: pause / step / x1 / x4 / x16, hien dong ho trong game.
6. He thong icon + mau sac: icon phai de doc, bang mau phai an toan cho nguoi mu mau.
7. Panel inventory / item card cho entity duoc chon.

Rang buoc ky thuat:
- Tu chon tech stack (khuyen nghi: Vite + TypeScript + PixiJS hoac canvas thuan; Vue/React tuy ban).
- Phai chay duoc bang lenh don gian, ghi ro trong `playground-gemini/README.md`.
- Neu khong cai duoc dependency thi lam ban single-file HTML thuan chay duoc bang cach mo file.
- Code sach, co cau truc, co comment cho phan kho.

Khi xong, viet `playground-gemini/README.md` gom: cach chay, kien truc, nhung gi da lam,
nhung gi con thieu, va nhung quyet dinh thiet ke dang chu y.

Hay lam that tot. Day la bai kiem tra nang luc.
