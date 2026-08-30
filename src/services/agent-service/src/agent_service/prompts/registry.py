"""Sổ đăng ký prompt: YAML + Jinja2 + version + filter ``untrusted`` + leak guard.

`plan.md §P6.2`. Bốn quy tắc, và mỗi quy tắc là phản ứng với một lỗi cụ thể:

1. **Mọi prompt có version.** ``(prompt_id, version)`` được ghi vào
   ``CognitionEvent`` (`§22.15`). Không có version thì sửa một template hôm nay
   sẽ làm mọi log của tháng trước trở nên không diễn giải được — ta không còn
   biết nhân vật đã *thật sự* được hỏi gì.

2. **``untrusted_slots`` bắt buộc đi qua filter.** Renderer **từ chối render**
   nếu một slot khai báo untrusted lại được nội suy trực tiếp (`§22.18`). Nội
   dung hội thoại, sách và ký ức đều do người chơi hoặc mô hình sinh ra; đưa
   thẳng vào prompt hệ thống là mở cửa cho prompt injection.

3. **Phòng thủ chính là ACL lúc truy xuất, không phải quét chuỗi lúc render.**
   Điều này quan trọng tới mức phải nói ra: hàm lấy observation, tri thức và ký
   ức phải lọc sạch thứ entity không được biết **trước khi** dữ liệu chạm vào
   biến template. Quét chuỗi bắt được khẩu quyết và tên riêng — thứ tồn tại
   dưới dạng chuỗi cố định — nhưng **không bao giờ** bắt được rò rỉ ngữ nghĩa
   kiểu *"kẻ phản bội là người mặc áo xanh"*.

4. **Leak guard là lưới cuối.** Nó bắt lỗi cài đặt của tầng ACL, không thay thế
   tầng đó. Vi phạm ném exception chứ không cảnh báo — một cảnh báo trong log
   là thứ không ai đọc, và một bí mật đã gửi đi thì không rút lại được.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml
from jinja2 import Environment, StrictUndefined, TemplateError
from jinja2.sandbox import SandboxedEnvironment

__all__ = [
    "PromptDef",
    "PromptRegistry",
    "PromptLeak",
    "UntrustedSlotNotWrapped",
    "untrusted",
    "OPEN_DELIM",
    "CLOSE_DELIM",
]

# Delimiter cố định bao quanh dữ liệu không tin cậy. Cố định chứ không ngẫu
# nhiên: prompt phải giống nhau giữa hai lần chạy để `request_hash` ổn định và
# chế độ REPLAY còn hoạt động.
OPEN_DELIM = "<<<UNTRUSTED"
CLOSE_DELIM = "UNTRUSTED>>>"

# Ký tự điều khiển và các chuỗi hay bị dùng để thoát khỏi khối.
_DELIM_PATTERN = re.compile(
    r"(<<<\s*/?\s*UNTRUSTED|UNTRUSTED\s*>>>)", re.IGNORECASE
)
_CONTROL = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]")


class PromptError(Exception):
    """Lỗi nền của mọi lỗi prompt."""


class UntrustedSlotNotWrapped(PromptError):
    """Một slot khai báo untrusted nhưng được nội suy trực tiếp."""


class PromptLeak(PromptError):
    """Leak guard bắt được một bí mật trong prompt sắp gửi.

    Đây **luôn** là bug nghiêm trọng (`§22.40`): nó nghĩa là tầng ACL đã để lọt
    một thứ mà thực thể không được biết.
    """

    def __init__(self, prompt_id: str, secrets: list[str]) -> None:
        # Không in ra chính bí mật: thông báo lỗi đi vào log, và log thì được
        # thu thập. Chỉ in độ dài và vài ký tự đầu để truy được nguồn.
        dau_moi = ", ".join(f"{s[:3]}…({len(s)} ký tự)" for s in secrets)
        super().__init__(
            f"prompt `{prompt_id}` chứa {len(secrets)} bí mật mà người quan sát "
            f"không được biết: {dau_moi}. Đây là lỗi của tầng ACL lúc truy xuất, "
            f"không phải của renderer — sửa ở chỗ lấy dữ liệu, không phải ở đây."
        )
        self.prompt_id = prompt_id
        self.secret_count = len(secrets)


def untrusted(value: Any) -> str:
    """Bọc dữ liệu không tin cậy trong delimiter cố định và vô hiệu hóa thoát.

    Ba việc, theo thứ tự:

    1. Xóa ký tự điều khiển — chúng vô hình với người đọc log nhưng không vô
       hình với mô hình.
    2. **Vô hiệu hóa mọi chuỗi trông giống delimiter** trong chính nội dung.
       Không có bước này, một cuốn sách trong game chứa ``UNTRUSTED>>>`` sẽ tự
       đóng khối và phần sau nó được đọc như chỉ thị hệ thống.
    3. Bọc trong delimiter.
    """
    text = "" if value is None else str(value)
    text = _CONTROL.sub("", text)
    text = _DELIM_PATTERN.sub(lambda m: m.group(0).replace("<", "‹").replace(">", "›"), text)
    return f"{OPEN_DELIM}\n{text}\n{CLOSE_DELIM}"


@dataclass(frozen=True)
class PromptDef:
    """Một prompt đã nạp."""

    id: str
    version: int
    template: str
    path: Path
    model_hint: dict[str, Any] = field(default_factory=dict)
    output_schema: str | None = None
    vars_model: str | None = None
    untrusted_slots: tuple[str, ...] = ()

    @property
    def key(self) -> str:
        """Khóa tra cứu, gồm cả version."""
        return f"{self.id}@v{self.version}"


@dataclass
class RenderResult:
    """Kết quả render, kèm siêu dữ liệu để ghi vào ``CognitionEvent``."""

    text: str
    prompt_id: str
    prompt_version: int


class PromptRegistry:
    """Nạp, kiểm tra và render prompt."""

    def __init__(self, root: Path, *, strict_undefined: bool = True) -> None:
        self.root = Path(root)
        self._defs: dict[str, PromptDef] = {}
        # `SandboxedEnvironment` chứ không phải `Environment`: template là dữ
        # liệu, và content pack của cộng đồng sẽ đóng góp template. Một template
        # không sandbox có thể gọi tới thuộc tính nội bộ của object Python được
        # truyền vào.
        self.env: Environment = SandboxedEnvironment(
            undefined=StrictUndefined if strict_undefined else None,
            autoescape=False,
            keep_trailing_newline=True,
        )
        self.env.filters["untrusted"] = untrusted

    # ── Nạp ─────────────────────────────────────────────────────────────────

    def load_dir(self, subdir: str = "") -> int:
        """Nạp mọi ``*.yaml`` trong một thư mục con. Trả số prompt đã nạp."""
        base = self.root / subdir if subdir else self.root
        n = 0
        # `sorted` để thứ tự nạp xác định — nó ảnh hưởng thông báo lỗi và, nếu
        # có prompt trùng id, ảnh hưởng cái nào bị báo là trùng.
        for p in sorted(base.rglob("*.yaml")):
            if p.name == "registry.yaml" or "golden" in p.parts:
                continue
            self.load_file(p)
            n += 1
        return n

    def load_file(self, path: Path) -> PromptDef:
        """Nạp một file prompt."""
        raw = yaml.safe_load(path.read_text(encoding="utf-8"))
        if not isinstance(raw, dict):
            raise PromptError(f"{path}: phải là một ánh xạ YAML")

        thieu = [k for k in ("id", "version", "template") if k not in raw]
        if thieu:
            raise PromptError(f"{path}: thiếu trường bắt buộc {thieu}")

        d = PromptDef(
            id=str(raw["id"]),
            version=int(raw["version"]),
            template=str(raw["template"]),
            path=path,
            model_hint=raw.get("model_hint") or {},
            output_schema=raw.get("output_schema"),
            vars_model=raw.get("vars_model"),
            untrusted_slots=tuple(raw.get("untrusted_slots") or ()),
        )

        self._check_untrusted_slots(d)

        if d.key in self._defs:
            raise PromptError(
                f"{path}: trùng `{d.key}` với {self._defs[d.key].path}. "
                f"Sửa template thì phải bump version, không được ghi đè."
            )
        self._defs[d.key] = d
        return d

    @staticmethod
    def _check_untrusted_slots(d: PromptDef) -> None:
        """Từ chối nạp nếu một slot untrusted được nội suy trực tiếp.

        Kiểm ở **lúc nạp**, không phải lúc render. Một prompt sai phải làm tiến
        trình không khởi động được, chứ không phải chạy êm rồi rò rỉ ở lần render
        thứ một nghìn — lúc đó dữ liệu đã gửi đi rồi.
        """
        vi_pham = []
        for slot in d.untrusted_slots:
            # Tìm `{{ slot }}` hoặc `{{ slot | filter_khac }}` mà không có
            # `untrusted` trong chuỗi filter.
            for m in re.finditer(
                r"\{\{\s*" + re.escape(slot) + r"\b([^}]*)\}\}", d.template
            ):
                if "untrusted" not in m.group(1):
                    vi_pham.append(f"`{{{{ {slot}{m.group(1)}}}}}`")
        if vi_pham:
            raise UntrustedSlotNotWrapped(
                f"{d.path}: slot khai báo untrusted nhưng nội suy trực tiếp: "
                f"{', '.join(vi_pham)}. Thêm ` | untrusted`. "
                f"(§22.18 — nội dung hội thoại, sách và ký ức là dữ liệu không "
                f"tin cậy đối với prompt hệ thống.)"
            )

    # ── Tra cứu ─────────────────────────────────────────────────────────────

    def get(self, prompt_id: str, version: int) -> PromptDef:
        """Lấy một prompt theo id và version."""
        key = f"{prompt_id}@v{version}"
        if key not in self._defs:
            co = sorted(k for k in self._defs if k.startswith(f"{prompt_id}@"))
            raise PromptError(
                f"không có `{key}`. Các phiên bản đã nạp của prompt này: {co or 'không có'}"
            )
        return self._defs[key]

    def latest(self, prompt_id: str) -> PromptDef:
        """Phiên bản mới nhất của một prompt."""
        cac_ban = [d for d in self._defs.values() if d.id == prompt_id]
        if not cac_ban:
            raise PromptError(f"không có prompt `{prompt_id}`")
        return max(cac_ban, key=lambda d: d.version)

    def ids(self) -> list[str]:
        """Mọi khóa đã nạp, đã sắp xếp."""
        return sorted(self._defs)

    def __len__(self) -> int:
        return len(self._defs)

    # ── Render ──────────────────────────────────────────────────────────────

    def render(
        self,
        prompt_id: str,
        version: int,
        variables: dict[str, Any],
        *,
        secrets: list[str] | None = None,
    ) -> RenderResult:
        """Render một prompt.

        ``secrets`` là tập bí mật dạng chuỗi mà **người quan sát này không được
        biết** — khẩu quyết, tên thật của kẻ cải trang, mật khẩu của một vật
        phẩm. Leak guard so nội dung sắp gửi với tập đó.

        Nhắc lại vì nó quan trọng: guard này là **lưới cuối**. Nó bắt được chuỗi
        cố định, không bắt được rò rỉ ngữ nghĩa. Nếu nó kêu, chỗ cần sửa là hàm
        truy xuất, không phải chỗ này.
        """
        d = self.get(prompt_id, version)
        try:
            text = self.env.from_string(d.template).render(**variables)
        except TemplateError as e:
            # Thiếu biến hoặc sai kiểu là lỗi **lúc render**, không phải lý do
            # để mô hình trả lời lung tung (`§P6.2`).
            raise PromptError(f"{d.key}: render thất bại: {e}") from e

        if secrets:
            self.leak_guard(d.id, text, secrets)

        return RenderResult(text=text, prompt_id=d.id, prompt_version=d.version)

    @staticmethod
    def leak_guard(prompt_id: str, text: str, secrets: list[str]) -> None:
        """Ném [`PromptLeak`] nếu bất kỳ bí mật nào xuất hiện trong prompt."""
        thap = text.casefold()
        # Bỏ qua chuỗi quá ngắn: một "bí mật" ba ký tự sẽ khớp ngẫu nhiên trong
        # mọi văn bản đủ dài, và một guard hay báo nhầm là một guard bị tắt.
        tim_thay = [s for s in secrets if len(s) >= 4 and s.casefold() in thap]
        if tim_thay:
            raise PromptLeak(prompt_id, tim_thay)
