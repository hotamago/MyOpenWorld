"""Sinh bộ bóng nguyên thủy SVG cho hệ icon hợp thành (PA-14).

Chạy một lần để tạo `content/core/icons/primitives.json`. Sau đó file JSON là
nguồn; script này giữ lại để dễ thêm bóng mới theo cùng phong cách.

Nguyên tắc vẽ, và cả ba đều là ràng buộc thật:

1. **Đọc được ở 16px.** Đó là kích thước icon trong túi đồ ở mức zoom thường.
2. **Không chi tiết nhỏ hơn 2px.** Chúng biến mất khi thu nhỏ, và một icon mất
   chi tiết trông giống một icon khác — người chơi sẽ nhặt nhầm đồ.
3. **Một hình bóng, không phải một bức tranh.** Bóng nguyên thủy được xếp
   chồng; nếu mỗi cái đã là một tác phẩm thì chồng lên nhau sẽ thành mớ hỗn độn.
"""

import json
import os
from collections import Counter


def p_(d: str) -> str:
    return f'<path d="{d}" fill="currentColor"/>'


def s_(d: str, w: float = 2) -> str:
    return (
        f'<path d="{d}" fill="none" stroke="currentColor" '
        f'stroke-width="{w}" stroke-linecap="round"/>'
    )


def fill(color: str, opacity: float) -> str:
    return f'<rect x="0" y="0" width="32" height="32" fill="{color}" opacity="{opacity}"/>'


SILHOUETTE = {
    # ── công cụ và vũ khí ────────────────────────────────────────────────────
    "axe": s_("M10 24 L20 8") + p_("M18 4 h8 v8 h-8 z"),
    "sword": s_("M8 24 L24 8") + s_("M20 4 h6 v6"),
    "spear": s_("M6 26 L26 6") + p_("M22 4 l6 0 l0 6 z"),
    "bow": s_("M12 5 A11 11 0 0 1 12 27") + s_("M12 5 L12 27", 1),
    "hammer": s_("M16 28 L16 14") + p_("M9 6 h14 v8 h-14 z"),
    "pick": s_("M16 28 L16 14") + s_("M5 12 A16 16 0 0 1 27 12"),
    "shovel": s_("M16 28 L16 16") + p_("M11 4 h10 v12 h-10 z"),
    "knife": s_("M11 25 L19 13") + s_("M19 13 L23 7", 3),
    "staff": s_("M16 29 L16 5") + p_("M16 3 a4 4 0 1 0 0.1 0 z"),
    "wand": s_("M10 22 L22 10") + p_("M23 6 l1 3 l3 1 l-3 1 l-1 3 l-1 -3 l-3 -1 l3 -1 z"),
    "shield": p_("M16 3 L27 8 v9 a11 13 0 0 1 -11 12 a11 13 0 0 1 -11 -12 V8 z"),
    "helmet": p_("M5 19 a11 11 0 0 1 22 0 v6 h-22 z"),
    "armor": p_("M9 7 l7 -3 l7 3 v13 a7 7 0 0 1 -14 0 z"),
    "boot": p_("M10 4 h7 v15 h9 v8 h-16 z"),
    # ── đồ dùng ─────────────────────────────────────────────────────────────
    "book": p_("M6 5 h20 v22 h-20 z") + s_("M16 5 v22", 1),
    "scroll": p_("M8 7 h16 v18 h-16 z") + s_("M8 7 a3 3 0 0 0 0 6", 1),
    "map": p_("M4 7 l8 -2 l8 2 l8 -2 v20 l-8 2 l-8 -2 l-8 2 z"),
    "letter": p_("M4 9 h24 v14 h-24 z") + s_("M4 9 l12 9 l12 -9", 1),
    "tablet": p_("M8 4 h16 v24 h-16 z"),
    "bottle": p_("M13 3 h6 v6 l4 5 v15 h-14 v-15 l4 -5 z"),
    "pot": p_("M7 11 h18 v16 h-18 z") + s_("M7 11 h18", 3),
    "bowl": p_("M5 13 h22 a11 11 0 0 1 -22 0 z"),
    "chest": p_("M4 12 h24 v15 h-24 z") + s_("M4 12 a12 6 0 0 1 24 0", 2),
    "sack": p_("M11 7 h10 l4 20 h-18 z"),
    "basket": p_("M7 11 h18 l-3 16 h-12 z"),
    "barrel": p_("M8 5 h16 v22 h-16 z") + s_("M8 12 h16", 1) + s_("M8 20 h16", 1),
    "key": p_("M8 16 a6 6 0 1 1 12 0 a6 6 0 1 1 -12 0 z") + s_("M20 16 h9") + s_("M26 16 v4"),
    "lock": p_("M8 15 h16 v13 h-16 z") + s_("M12 15 v-4 a4 4 0 0 1 8 0 v4"),
    "coin": p_("M16 4 a12 12 0 1 0 0.1 0 z"),
    "gem": p_("M16 3 l11 9 l-11 17 l-11 -17 z"),
    "ring": s_("M16 17 m-9 0 a9 9 0 1 0 18 0 a9 9 0 1 0 -18 0", 3),
    "amulet": s_("M8 5 a11 9 0 0 0 16 0") + p_("M16 13 a6 6 0 1 0 0.1 0 z"),
    # ── thức ăn ─────────────────────────────────────────────────────────────
    "bread": p_("M4 11 a12 6 0 0 1 24 0 v10 h-24 z"),
    "meat": p_("M7 14 a9 7 0 0 1 18 0 v7 h-18 z") + s_("M25 17 h5"),
    "fish": p_("M4 16 l10 -7 l10 7 l-10 7 z") + p_("M24 11 l5 5 l-5 5 z"),
    "fruit": p_("M16 8 a9 9 0 1 0 0.1 0 z") + s_("M16 8 V3"),
    "grain": s_("M16 29 V6") + s_("M16 11 l6 -5") + s_("M16 18 l-6 -5") + s_("M16 24 l6 -5"),
    "vegetable": p_("M11 9 h10 v17 h-10 z") + s_("M16 9 V3"),
    "cheese": p_("M4 21 l12 -11 h12 v11 z"),
    "egg": p_("M16 4 a8 11 0 1 0 0.1 0 z"),
    "water": p_("M16 3 l8 13 a8 8 0 1 1 -16 0 z"),
    # ── sinh vật ────────────────────────────────────────────────────────────
    "humanoid": (
        p_("M16 4 a4 4 0 1 0 0.1 0 z")
        + p_("M12 11 h8 v11 h-8 z")
        + s_("M13 22 v7")
        + s_("M19 22 v7")
    ),
    "beast": (
        p_("M6 13 h15 v9 h-15 z")
        + p_("M21 10 h6 v8 h-6 z")
        + s_("M8 22 v6")
        + s_("M19 22 v6")
    ),
    "bird": p_("M9 14 a8 6 0 0 1 14 0 l5 -4 l-3 8 z") + s_("M9 20 v4"),
    "insect": p_("M13 9 h6 v15 h-6 z") + s_("M13 13 h-7") + s_("M19 13 h7") + s_("M13 19 h-6"),
    "fish_creature": p_("M4 16 l11 -8 l11 8 l-11 8 z") + s_("M26 12 v8"),
    "reptile": p_("M5 19 h17 a5 5 0 0 0 0 -7 h-17 z") + s_("M5 15 h-3"),
    "undead": p_("M16 5 a7 8 0 1 0 0.1 0 z") + p_("M12 20 h8 v8 h-8 z"),
    "spirit": s_("M9 25 a7 13 0 0 1 14 0", 2) + p_("M16 5 a7 7 0 1 0 0.1 0 z"),
    "dragon": p_("M4 20 l9 -9 l9 5 l7 -7 v15 h-25 z") + s_("M27 6 v4"),
    "plant": (
        s_("M16 29 V11")
        + p_("M16 11 a7 7 0 0 0 -9 7 a9 9 0 0 0 9 -7 z")
        + p_("M16 11 a7 7 0 0 1 9 7 a9 9 0 0 1 -9 -7 z")
    ),
    "tree": s_("M16 29 V17", 3) + p_("M16 3 a11 11 0 1 0 0.1 0 z"),
    "mushroom": p_("M5 15 a11 9 0 0 1 22 0 z") + p_("M13 15 h6 v13 h-6 z"),
    # ── công trình ──────────────────────────────────────────────────────────
    "house": p_("M3 16 l13 -11 l13 11 v13 h-26 z"),
    "tower": p_("M10 5 h12 v24 h-12 z") + p_("M8 5 h16 v3 h-16 z"),
    "wall": p_("M3 11 h26 v16 h-26 z") + s_("M3 19 h26", 1),
    "gate": p_("M5 28 V13 a11 11 0 0 1 22 0 v15 z"),
    "well": p_("M8 13 h16 v15 h-16 z") + s_("M16 13 V4") + s_("M9 6 h14"),
    "forge": p_("M5 16 h22 v12 h-22 z") + s_("M11 16 V6") + s_("M21 16 V9"),
    "altar": p_("M7 19 h18 v9 h-18 z") + p_("M11 9 h10 v10 h-10 z"),
    "farm": s_("M3 23 h26") + s_("M8 23 V14") + s_("M16 23 V11") + s_("M24 23 V14"),
    "market": p_("M4 11 h24 v5 h-24 z") + s_("M7 16 v12") + s_("M25 16 v12"),
    "road": s_("M3 28 L29 4", 4),
    "bridge": s_("M3 19 a13 9 0 0 1 26 0", 3) + s_("M8 19 v7") + s_("M24 19 v7"),
    "portal": s_("M16 16 m-11 0 a11 13 0 1 0 22 0 a11 13 0 1 0 -22 0", 3),
    # ── trừu tượng ──────────────────────────────────────────────────────────
    "flame": p_("M16 3 c7 9 9 11 9 15 a9 9 0 1 1 -18 0 c0 -4 2 -6 9 -15 z"),
    "star": p_("M16 3 l4 10 l11 1 l-8 7 l3 11 l-10 -6 l-10 6 l3 -11 l-8 -7 l11 -1 z"),
    "eye": p_("M3 16 a15 10 0 0 1 26 0 a15 10 0 0 1 -26 0 z") + p_("M16 16 a4 4 0 1 0 0.1 0 z"),
    "hand": p_("M11 28 v-13 h2 v-7 h2.5 v7 h2 v-8 h2.5 v8 h2 v-5 h2 v18 z"),
    "footprint": p_("M12 7 a5 7 0 1 0 0.1 0 z") + p_("M12 21 a4 5 0 1 0 0.1 0 z"),
    "skull": p_("M16 3 a11 10 0 1 0 0.1 0 z") + p_("M12 21 h8 v7 h-8 z"),
    "heart": p_("M16 29 l-11 -11 a7 7 0 0 1 11 -8 a7 7 0 0 1 11 8 z"),
    "clock": (
        s_("M16 16 m-12 0 a12 12 0 1 0 24 0 a12 12 0 1 0 -24 0")
        + s_("M16 16 V8")
        + s_("M16 16 h6")
    ),
    "scale": s_("M16 28 V5") + s_("M5 9 h22") + p_("M2 9 l3 9 l3 -9 z") + p_("M24 9 l3 9 l3 -9 z"),
    "banner": p_("M8 3 h16 v20 l-8 -6 l-8 6 z"),
    "question": s_("M11 11 a5 5 0 1 1 5 6 v3", 2.5) + p_("M14 24 h4 v4 h-4 z"),
    "cloud": p_("M7 21 a7 7 0 0 1 2 -12 a9 9 0 0 1 17 3 a6 6 0 0 1 -1 9 z"),
    "wave": s_("M3 18 q6 -7 13 0 t13 0", 3) + s_("M3 24 q6 -7 13 0 t13 0", 2),
    "mountain": p_("M2 27 l11 -18 l7 10 l4 -6 l6 14 z"),
    "sun": (
        p_("M16 9 a7 7 0 1 0 0.1 0 z")
        + s_("M16 1 v4")
        + s_("M16 27 v4")
        + s_("M1 16 h4")
        + s_("M27 16 h4")
    ),
    "moon": p_("M21 3 a13 13 0 1 0 0 26 a11 13 0 0 1 0 -26 z"),
    "rune": s_("M16 3 V29") + s_("M16 11 l8 -8") + s_("M16 21 l-8 -8"),
}

MATERIAL = {
    "wood": fill("#8a5a2b", 0.55),
    "stone": fill("#7d7f82", 0.55),
    "iron": fill("#5c6470", 0.55),
    "steel": fill("#9aa4b0", 0.55),
    "brass": fill("#b08d3f", 0.55),
    "copper": fill("#a75c34", 0.55),
    "silver": fill("#c3c8cc", 0.55),
    "gold": fill("#d4a220", 0.55),
    "bone": fill("#ddd6c0", 0.55),
    "leather": fill("#7a4a24", 0.55),
    "cloth": fill("#b8a98c", 0.55),
    "glass": fill("#8fc4cc", 0.40),
    "clay": fill("#a06848", 0.55),
    "obsidian": fill("#2b2b33", 0.60),
    "crystal": fill("#9fd0e8", 0.50),
}

STATE = {
    # Trạng thái nằm ở rìa để không che hình bóng — người chơi phải nhận ra
    # *cái gì* trước, rồi mới tới *nó đang thế nào*.
    "chipped": s_("M22 4 l5 5", 2),
    "broken": s_("M7 7 L25 25", 3) + s_("M25 7 L7 25", 3),
    "worn": s_("M5 29 h22", 1),
    "burnt": p_("M3 29 h26 v3 h-26 z"),
    "wet": p_("M5 3 a2 3 0 1 0 0.1 0 z") + p_("M26 5 a2 3 0 1 0 0.1 0 z"),
    "frozen": s_("M3 3 L10 10", 1) + s_("M29 3 L22 10", 1),
    "cursed": s_("M2 16 q4 -5 8 0 t8 0 t8 0 t6 0", 1),
    "blessed": s_("M28 3 v6", 1.5) + s_("M25 6 h6", 1.5),
    "poisoned": p_("M26 25 a3 3 0 1 0 0.1 0 z"),
    "enchanted": p_("M27 3 l1 3.5 l3.5 1 l-3.5 1 l-1 3.5 l-1 -3.5 l-3.5 -1 l3.5 -1 z"),
    "sealed": s_("M4 4 h7 v7 h-7 z", 1),
    "hidden": s_("M1 31 L31 1", 1),
}

MARKER = {
    # Góc dưới phải, luôn luôn. Vị trí cố định là thứ khiến mắt tìm được nó mà
    # không phải quét.
    "faction_blue": '<circle cx="27" cy="27" r="4.5" fill="#1a4fa0"/>',
    "faction_amber": '<circle cx="27" cy="27" r="4.5" fill="#b5651d"/>',
    "faction_slate": '<circle cx="27" cy="27" r="4.5" fill="#3d4b52"/>',
    "owned": '<rect x="23" y="23" width="8" height="8" fill="currentColor"/>',
    "stolen": '<path d="M23 31 l8 -8 v8 z" fill="currentColor"/>',
    "claimed": '<path d="M23 23 h8 v8 z" fill="currentColor"/>',
    "unowned": (
        '<circle cx="27" cy="27" r="4" fill="none"'
        ' stroke="currentColor" stroke-width="1.5"/>'
    ),
}

ANNOTATION = {
    "quality_poor": s_("M2 29 h5", 2),
    "quality_fine": s_("M2 29 h5", 2) + s_("M2 25 h5", 2),
    "quality_master": s_("M2 29 h5", 2) + s_("M2 25 h5", 2) + s_("M2 21 h5", 2),
    "quality_legendary": p_(
        "M5 21 l1.6 3.8 l4 0.6 l-2.8 2.8 l0.6 4"
        " l-3.4 -2.1 l-3.4 2.1 l0.6 -4 l-2.8 -2.8 l4 -0.6 z"
    ),
    "stack_small": '<circle cx="5" cy="5" r="2.5" fill="currentColor"/>',
    "stack_many": (
        '<circle cx="5" cy="5" r="2.5" fill="currentColor"/>'
        '<circle cx="11" cy="5" r="2.5" fill="currentColor"/>'
    ),
    # `§18.14.5`: chưa thẩm định thì hiện dấu hỏi. Đây là lý do lỗi hợp thành
    # KHÔNG được vẽ dấu hỏi — nó đã có nghĩa khác rồi.
    "unappraised": (
        s_("M2 8 a3.5 3.5 0 1 1 3.5 4 v2", 1.5)
        + '<rect x="4.5" y="16" width="2.5" height="2.5" fill="currentColor"/>'
    ),
    "quantity_low": (
        '<rect x="2" y="26" width="5" height="5"'
        ' fill="currentColor" opacity="0.4"/>'
    ),
    "quantity_full": '<rect x="2" y="26" width="5" height="5" fill="currentColor"/>',
}


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    out_dir = os.path.join(here, "content", "core", "icons")
    os.makedirs(out_dir, exist_ok=True)

    rows = []
    for layer, table in (
        ("silhouette", SILHOUETTE),
        ("material", MATERIAL),
        ("state", STATE),
        ("marker", MARKER),
        ("annotation", ANNOTATION),
    ):
        # `sorted` để thứ tự trong file ổn định — nó đi vào content hash của pack.
        for name in sorted(table):
            rows.append({"id": f"core.{name}", "layer": layer, "svg": table[name]})

    path = os.path.join(out_dir, "primitives.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(rows, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"{len(rows)} bóng nguyên thủy → {path}")
    for layer, n in sorted(Counter(r["layer"] for r in rows).items()):
        print(f"  {layer:<12} {n}")


if __name__ == "__main__":
    main()
