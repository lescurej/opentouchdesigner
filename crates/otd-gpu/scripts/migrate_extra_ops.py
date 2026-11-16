#!/usr/bin/env python3
"""Generate ops/<slug>/mod.rs from extra.rs entries and definitions.rs bodies."""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OPS = ROOT / "src" / "ops"
DEFS = OPS / "definitions.rs"
EXTRA = OPS / "extra.rs"

TYPE_TO_SLUG = {
    "opticalflowTOP": "opticalflow",
    "monochromeTOP": "monochrome",
    "rgbtohsvTOP": "rgbtohsv",
    "hsvtorgbTOP": "hsvtorgb",
    "channelmixTOP": "channelmix",
    "reorderTOP": "reorder",
    "matteTOP": "matte",
    "rgbkeyTOP": "rgbkey",
    "lumalevelTOP": "lumalevel",
    "functionTOP": "function",
    "limitTOP": "limit",
    "embossTOP": "emboss",
    "slopeTOP": "slope",
    "normalmapTOP": "normalmap",
    "antialiasTOP": "antialias",
    "convolveTOP": "convolve",
    "cornerpinTOP": "cornerpin",
    "cropTOP": "crop",
    "fitTOP": "fit",
    "lensdistortTOP": "lensdistort",
    "remapTOP": "remap",
    "tileTOP": "tile",
    "lumablurTOP": "lumablur",
    "overTOP": "over",
    "underTOP": "under",
    "addTOP": "add",
    "subtractTOP": "subtract",
    "multiplyTOP": "multiply",
    "screenTOP": "screen",
    "differenceTOP": "difference",
    "insideTOP": "inside",
    "outsideTOP": "outside",
    "crossTOP": "cross",
}

BLEND_PACK = {
    "overTOP": "pack_over",
    "underTOP": "pack_under",
    "addTOP": "pack_add",
    "subtractTOP": "pack_subtract",
    "multiplyTOP": "pack_multiply",
    "screenTOP": "pack_screen",
    "differenceTOP": "pack_difference",
    "insideTOP": "pack_inside",
    "outsideTOP": "pack_outside",
    "crossTOP": "pack_cross",
}

NONE_PARAMS = {"rgbtohsvTOP", "hsvtorgbTOP"}

SIZING_PARAMS = {"cropTOP", "fitTOP"}


def extract_fn(defs: str, name: str) -> str:
    pat = rf"pub\(crate\) fn {name}\([^)]*\)[^{{]*\{{"
    m = re.search(pat, defs)
    if not m:
        raise SystemExit(f"missing {name}")
    start = m.start()
    depth = 0
    i = m.end() - 1
    while i < len(defs):
        if defs[i] == "{":
            depth += 1
        elif defs[i] == "}":
            depth -= 1
            if depth == 0:
                return defs[start : i + 1]
        i += 1
    raise SystemExit(f"unclosed {name}")


def extract_const(defs: str, name: str) -> str | None:
    m = re.search(rf"(pub\(crate\) )?const {name}:.*?\n;", defs, re.S)
    return m.group(0) if m else None


def parse_extra() -> list[dict]:
    text = EXTRA.read_text()
    out: list[dict] = []
    for kind in ("filter_top!", "blend_top!"):
        for block in re.finditer(
            rf"{re.escape(kind)}\(\s*\"([^\"]+)\"\s*,\s*\"([^\"]+)\"\s*,\s*\"([^\"]*)\"",
            text,
            re.S,
        ):
            type_name = block.group(1)
            label = block.group(2)
            summary = block.group(3)
            out.append(
                {
                    "type_name": type_name,
                    "label": label,
                    "summary": summary,
                    "blend": kind.startswith("blend"),
                }
            )
    return out


def shader_path(type_name: str) -> str:
    base = type_name.removesuffix("TOP")
    return f"../../shaders/{base}.wgsl"


def slug_params_key(slug: str) -> str:
    return slug.replace("_", "")


def rewrite_fn(body: str) -> str:
    body = re.sub(r"pub\(crate\) fn (\w+)", r"fn \1", body)
    return body


def gen_filter(meta: dict, defs: str) -> str:
    tn = meta["type_name"]
    slug = TYPE_TO_SLUG[tn]
    inputs = {
        "opticalflowTOP": '["current", "previous"]',
        "matteTOP": '["source", "matte"]',
        "remapTOP": '["source", "map"]',
        "lumablurTOP": '["source", "blur"]',
        "reorderTOP": '["in1", "in2"]',
    }.get(tn, '["in"]')
    sizing = "Sizing::Params" if tn in SIZING_PARAMS else "Sizing::Input0"
    two_pass = "false"
    shader = shader_path(tn)

    extra_const = ""
    prefix = ""
    if tn == "reorderTOP":
        c = extract_const(defs, "REORDER_SOURCES")
        if c:
            extra_const = c.replace("pub(crate) ", "") + "\n\n"
            prefix = "const REORDER_SOURCES: &[&str] = &[\n"

    if tn in NONE_PARAMS:
        params_block = "fn params() -> IndexMap<String, Param> {\n    merge_common(params_none())\n}\n\nfn pack(_n: &Node, _c: &EvalContext) -> PackedParams {\n    pack_none(_n, _c)\n}\n"
        uses = "use super::helpers::{merge_common, pack_none, params_none, *};\n"
    else:
        pk = slug.replace("_", "")
        if "luma_blur" in tn or slug == "lumablur":
            pk = "luma_blur"
        elif slug == "opticalflow":
            pk = "optical_flow"
        elif slug == "channelmix":
            pk = "channel_mix"
        elif slug == "rgbkey":
            pk = "rgb_key"
        elif slug == "lumalevel":
            pk = "luma_level"
        elif slug == "normalmap":
            pk = "normal_map"
        elif slug == "cornerpin":
            pk = "corner_pin"
        elif slug == "lensdistort":
            pk = "lens_distort"
        elif slug == "lumablur":
            pk = "luma_blur"
        else:
            pk = slug
        params_fn = rewrite_fn(extract_fn(defs, f"params_{pk}"))
        pack_fn = rewrite_fn(extract_fn(defs, f"pack_{pk}"))
        params_block = f"{params_fn}\n\n{pack_fn}\n"
        uses = "use otd_core::Value;\n\nuse super::helpers::*;\n" if "Value::" in params_block else "use super::helpers::*;\n"

    return f"""use otd_core::indexmap::IndexMap;
use otd_core::{{Connector, EvalContext, Family, Node, OpDef, Param}};

{uses}use super::{{PackedParams, Sizing, TopCook, TopSpec}};

pub const TYPE_NAME: &str = "{tn}";

{extra_const}{params_block}
pub fn spec() -> TopSpec {{
    TopSpec {{
        def: OpDef {{
            type_name: TYPE_NAME,
            input_families: &[],
            label: "{meta['label']}",
            family: Family::Top,
            inputs: &{inputs},
            summary: "{meta['summary']}",
            time_dependent: false,
            params,
            connector: Connector::None,
        }},
        shader: include_str!("{shader}"),
        sizing: {sizing},
        two_pass: {two_pass},
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }}
}}
"""


def gen_blend(meta: dict) -> str:
    tn = meta["type_name"]
    pack = BLEND_PACK[tn]
    return f"""use super::{{TopSpec, blend::{pack}}};
use crate::ops::composite;

pub const TYPE_NAME: &str = "{tn}";

pub fn spec() -> TopSpec {{
    composite::blend_alias_spec(
        TYPE_NAME,
        "{meta['label']}",
        "{meta['summary']}",
        {pack},
    )
}}
"""


def main() -> None:
    defs = DEFS.read_text()
    for meta in parse_extra():
        tn = meta["type_name"]
        slug = TYPE_TO_SLUG[tn]
        dest = OPS / slug
        dest.mkdir(exist_ok=True)
        body = gen_blend(meta) if meta["blend"] else gen_filter(meta, defs)
        (dest / "mod.rs").write_text(body)
        print("wrote", slug)


if __name__ == "__main__":
    main()