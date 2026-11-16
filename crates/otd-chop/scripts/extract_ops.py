#!/usr/bin/env python3
"""Split operators.rs by // ---- section markers into ops/<slug>/mod.rs files."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OPS = ROOT / "src" / "ops"
OPERATORS = OPS / "operators.rs"
TABLE = OPS / "table.rs"

SLUG_FOR_TYPE = {
    "particleCHOP": "particles",
    "parameterCHOP": "parameter",
    "infoCHOP": "info",
    "lfoCHOP": "lfo",
    "animationCHOP": "animation",
    "noiseCHOP": "noise",
    "patternCHOP": "pattern",
    "mathCHOP": "math",
    "lagCHOP": "lag",
    "filterCHOP": "filter",
    "logicCHOP": "logic",
    "triggerCHOP": "trigger",
    "timerCHOP": "timer",
    "speedCHOP": "speed",
    "countCHOP": "count",
    "selectCHOP": "select",
    "mergeCHOP": "merge",
    "switchCHOP": "switch",
    "delayCHOP": "delay",
    "trailCHOP": "trail",
    "springCHOP": "spring",
    "fanCHOP": "fan",
    "expressionCHOP": "expression",
    "limitCHOP": "limit",
    "slopeCHOP": "slope",
    "holdCHOP": "hold",
    "shuffleCHOP": "shuffle",
    "renameCHOP": "rename",
    "analyzeCHOP": "analyze",
    "crossCHOP": "cross",
    "resampleCHOP": "resample",
    "panelCHOP": "panel",
    "clockCHOP": "clock",
    "beatCHOP": "beat",
    "dattochopCHOP": "dattochop",
    "soptochopCHOP": "soptochop",
    "blobtrackCHOP": "blobtrack",
    "toptochopCHOP": "toptochop",
}

COOK_SUFFIX = {
    "dattochop": "dat_to_chop",
    "soptochop": "sop_to_chop",
    "toptochop": "top_to_chop",
}

PARAM_SUFFIX = {
    "dattochop": "dat_to",
    "soptochop": "sop_to",
    "toptochop": "top_to",
}


def slug_from_title(title: str) -> str:
    t = title.strip().lower()
    if "dat to chop" in t:
        return "__dat_block__"
    return re.sub(r"[^a-z0-9]+", "", t)


def parse_sections(text: str) -> list[tuple[str, str]]:
    lines = text.splitlines(keepends=True)
    sections: list[tuple[str, int]] = []
    for i, line in enumerate(lines):
        if line.startswith("// ----"):
            title = re.sub(r"^//\s*-+\s*", "", line.strip()).strip()
            sections.append((title, i))
    out: list[tuple[str, str]] = []
    for idx, (title, start) in enumerate(sections):
        end = sections[idx + 1][1] if idx + 1 < len(sections) else len(lines)
        body = "".join(lines[start + 1 : end])
        out.append((slug_from_title(title), body))
    return out


def split_info_parameter(body: str) -> tuple[str, str]:
    idx = body.find("pub(crate) fn params_parameter")
    if idx < 0:
        raise RuntimeError("parameter split failed")
    return body[:idx].strip() + "\n", body[idx:].strip() + "\n"


def split_dat_block(body: str) -> dict[str, str]:
    keys = [
        ("particles", "params_particles"),
        ("blobtrack", "params_blobtrack"),
        ("dattochop", "params_dat_to"),
        ("soptochop", "params_sop_to"),
        ("toptochop", "params_top_to"),
    ]
    positions = [(body.index(f"pub(crate) fn {k}"), name) for name, k in keys if f"pub(crate) fn {k}" in body]
    positions.sort()
    parts: dict[str, str] = {}
    for i, (start, name) in enumerate(positions):
        end = positions[i + 1][0] if i + 1 < len(positions) else len(body)
        chunk = body[start:end].strip() + "\n"
        if name == "dattochop":
            san = re.search(
                r"/// A channel name has to survive.*?\nfn sanitise\(raw: &str\) -> String \{.*?\n\}\n",
                chunk,
                re.S,
            )
            if san:
                OPS.mkdir(parents=True, exist_ok=True)
                shared_dir = OPS / "shared"
                shared_dir.mkdir(exist_ok=True)
                san_body = san.group(0).replace("fn sanitise", "pub(crate) fn sanitise")
                (shared_dir / "sanitise.rs").write_text(san_body + "\n")
                if not (shared_dir / "mod.rs").exists():
                    (shared_dir / "mod.rs").write_text("pub(crate) mod sanitise;\n")
                chunk = chunk[: san.start()] + chunk[san.end() :]
        parts[name] = chunk
    return parts


def rename_body(slug: str, body: str) -> str:
    ps = PARAM_SUFFIX.get(slug)
    if ps:
        body = body.replace(f"params_{ps}", "params")
    elif "params_" in body:
        body = re.sub(r"pub\(crate\) fn params_\w+", "pub(crate) fn params", body, count=1)

    cs = COOK_SUFFIX.get(slug, slug)
    for old in (f"cook_{cs}", f"cook_{slug}"):
        if f"pub(crate) fn {old}" in body:
            body = body.replace(f"pub(crate) fn {old}", "pub(crate) fn cook")
            break

    if slug == "pattern":
        body = re.sub(r"(?<!::)\bwave\(", "super::lfo::wave(", body)
    if slug == "filter":
        body = body.replace("lag_alpha(", "crate::ops::lag::lag_alpha(")
    if slug in ("panel", "dattochop"):
        body = body.replace("sanitise(", "crate::ops::shared::sanitise::sanitise(")
    if slug == "lag":
        body = body.replace("fn lag_alpha", "pub(crate) fn lag_alpha")
    if slug == "lfo":
        body = body.replace("pub(crate) fn wave", "pub(crate) fn wave")
    if slug == "select":
        body = body.replace("pub fn name_matches", "pub(crate) fn name_matches")
    return body


def imports_for(slug: str, body: str) -> str:
    need_crossing = "Crossing" in body
    need_eval = "EvalContext" in body
    lines = ["use otd_core::indexmap::IndexMap;", "use otd_core::Param;"]
    if need_eval:
        lines.append("use otd_core::EvalContext;")
    if need_crossing:
        lines.append("use otd_core::Crossing;")
    data = ["CONTROL_RATE", "Channel", "ChopData"]
    if "slice_len" in body:
        data.append("slice_len")
    if "slice_times" in body:
        data.append("slice_times")
    lines.append(f"use crate::data::{{{', '.join(data)}}};")
    lines.append("")
    lines.append("use super::helpers::*;")
    lines.append("use super::{ChopCtx, ChopSpec};")
    if slug == "info":
        lines = [
            "use otd_core::indexmap::IndexMap;",
            "use otd_core::Param;",
            "",
            "use crate::data::{CONTROL_RATE, Channel, ChopData};",
            "",
            "use super::{ChopCtx, ChopSpec};",
        ]
    if slug == "clock":
        lines = [
            "use otd_core::indexmap::IndexMap;",
            "",
            "use crate::data::ChopData;",
            "",
            "use super::{ChopCtx, ChopSpec};",
        ]
    if slug == "blobtrack":
        lines[-2] = "use otd_core::Crossing;"
        lines.insert(2, "use otd_core::Param;")
    return "\n".join(lines) + "\n\n"


def parse_table_specs() -> dict[str, str]:
    text = TABLE.read_text()
    specs: dict[str, str] = {}
    for m in re.finditer(
        r"(spec(?:_animated)?|converter_spec)\(\s*"
        r'(?:"([^"]+)"|([A-Z_]+))\s*,\s*"([^"]*)"',
        text,
    ):
        kind, type_lit, const_lit, label = m.groups()
        type_name = type_lit or {"PARAMETER": "parameterCHOP", "INFO": "infoCHOP"}.get(const_lit, const_lit)
        slug = SLUG_FOR_TYPE.get(type_name)
        if not slug:
            continue
        start = m.start()
        depth = 0
        i = start
        while i < len(text):
            if text[i] == "(":
                depth += 1
            elif text[i] == ")":
                depth -= 1
                if depth == 0:
                    block = text[start : i + 1]
                    break
            i += 1
        else:
            continue
        block = re.sub(r"params_\w+", "params", block)
        block = re.sub(r"cook_\w+", "cook", block)
        fn = {"spec_animated": "spec_animated", "converter_spec": "converter_spec", "spec": "spec"}[kind.split("(")[0] if "(" in kind else kind]
        if fn == "spec":
            fn_name = "spec_animated" if "spec_animated" in block[:20] else ("converter_spec" if "converter_spec" in block[:25] else "spec")
        else:
            fn_name = fn
        block = block.split("(", 1)[0] + "(" + block.split("(", 1)[1]
        inner = block[block.index("(") + 1 : block.rindex(")")]
        parts = []
        cur = ""
        depth = 0
        for ch in inner:
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
            if ch == "," and depth == 0:
                parts.append(cur.strip())
                cur = ""
            else:
                cur += ch
        parts.append(cur.strip())
        while len(parts) < 6:
            parts.append("")
        type_arg = parts[0]
        if type_lit:
            type_const = f'pub const TYPE_NAME: &str = "{type_lit}";'
        else:
            type_const = f"pub const TYPE_NAME: &str = {const_lit};"
        inputs = parts[2]
        summary = parts[3]
        params_ref = parts[4].strip()
        cook_ref = parts[5].strip()
        if params_ref == "no_params":
            params_ref = "super::helpers::no_params"
        if fn_name == "converter_spec":
            spec_fn = f"""{type_const}

pub fn spec() -> ChopSpec {{
    super::converter_spec(
        TYPE_NAME,
        {parts[1]},
        {summary},
        params,
        cook,
    )
}}"""
        elif fn_name == "spec_animated":
            spec_fn = f"""{type_const}

pub fn spec() -> ChopSpec {{
    super::spec_animated(
        TYPE_NAME,
        {parts[1]},
        {inputs},
        {summary},
        params,
        cook,
    )
}}"""
        else:
            spec_fn = f"""{type_const}

pub fn spec() -> ChopSpec {{
    super::spec(
        TYPE_NAME,
        {parts[1]},
        {inputs},
        {summary},
        {params_ref},
        cook,
    )
}}"""
        specs[slug] = spec_fn
    blob = re.search(
        r"converter_spec\(\s*\"blobtrackCHOP\".*?params_blobtrack, cook_blobtrack,\s*\);\s*s\.def\.time_dependent = true;\s*s",
        text,
        re.S,
    )
    if blob:
        specs["blobtrack"] = '''pub const TYPE_NAME: &str = "blobtrackCHOP";

pub fn spec() -> ChopSpec {
    let mut s = super::converter_spec(
        TYPE_NAME,
        "Blob Track",
        "Largest connected bright region. Channels: present, count, x, y, area, width, height, vx, vy. \\
         Coordinates and sizes are 0..1, origin top-left; velocity is frame-fractions/second. \\
         CPU readback of last frame; downsample to 160x90 first. \\
         Not person recognition or persistent identity: switching largest regions can jump. \\
         A missing/empty input returns zero channels values.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}'''
    top = re.search(
        r"converter_spec\(\s*\"toptochopCHOP\".*?params_top_to,\s*cook_top_to_chop,\s*\);\s*s\.def\.time_dependent = true;\s*s",
        text,
        re.S,
    )
    if top:
        specs["toptochop"] = '''pub const TYPE_NAME: &str = "toptochopCHOP";

pub fn spec() -> ChopSpec {
    let mut s = super::converter_spec(
        TYPE_NAME,
        "TOP to CHOP",
        "Pixels as channels r g b a. `image` reads every pixel (top row first, at most \\
         128x128) and adds u/v positions, centred, v up — instance a Geometry COMP with \\
         tx=u ty=v tz=g for a camera point cloud. Reads last frame; costs a GPU sync, \\
         so put a Resolution TOP in front.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}'''
    specs["particles"] = '''pub const TYPE_NAME: &str = "particleCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
        TYPE_NAME,
        "Particle",
        &[],
        "Stateful 2D particle emitter with velocity, gravity, lifetime and respawn. \\
         tx/ty/tz are instance positions (6 scene units across); size/life fade to zero. \\
         Emitter coordinates are normalized, origin top-left. Connect via geometryCOMP.instancechop; \\
         set sx/sy/sz to size. Active stops births; existing particles finish. \\
         Reset clears the simulation. CPU, at most 4096 particles; dt capped at 0.1s; no collisions or flocking.",
        params,
        cook,
    )
}'''
    specs["parameter"] = f"""pub const TYPE_NAME: &str = PARAMETER;
pub const PARAMETER: &str = TYPE_NAME;

pub fn spec() -> ChopSpec {{
    super::spec(
        TYPE_NAME,
        "Parameter",
        &[],
        "Another operator's parameter values as channels (does not cook that operator).",
        params,
        cook,
    )
}}"""
    specs["info"] = f"""pub const TYPE_NAME: &str = INFO;
pub const INFO: &str = TYPE_NAME;

pub fn spec() -> ChopSpec {{
    super::spec_animated(
        TYPE_NAME,
        "Info",
        &[],
        "Another operator's info channels (TouchDesigner Info CHOP).",
        params,
        cook,
    )
}}"""
    specs["merge"] = '''pub const TYPE_NAME: &str = "mergeCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Merge",
        &["a", "b"],
        "Put two CHOPs' channels side by side.",
        super::helpers::no_params,
        cook,
    )
}'''
    return specs


def write_mod(slug: str, body: str, table_specs: dict[str, str]) -> None:
    d = OPS / slug
    d.mkdir(parents=True, exist_ok=True)
    renamed = rename_body(slug, body)
    tail = table_specs.get(slug, f"// missing spec for {slug}\n")
    if slug == "parameter" and "pub const PARAMETER" not in renamed:
        renamed = "pub const PARAMETER: &str = \"parameterCHOP\";\n\n" + renamed
    if slug == "info" and "pub const INFO" not in renamed:
        renamed = "pub const INFO: &str = \"infoCHOP\";\n\n" + renamed
    content = imports_for(slug, body) + renamed + "\n" + tail + "\n"
    (d / "mod.rs").write_text(content)


def main() -> None:
    text = OPERATORS.read_text()
    table_specs = parse_table_specs()
    for slug, body in parse_sections(text):
        if slug == "parameter":
            info_b, param_b = split_info_parameter(body)
            write_mod("info", info_b, table_specs)
            write_mod("parameter", param_b, table_specs)
            continue
        if slug == "__dat_block__":
            for name, part in split_dat_block(body).items():
                write_mod(name, part, table_specs)
            continue
        write_mod(slug, body, table_specs)
    print(f"extracted {len(list(OPS.glob('*/mod.rs')))} modules")


if __name__ == "__main__":
    main()