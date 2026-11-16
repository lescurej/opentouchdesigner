#!/usr/bin/env python3
from __future__ import annotations

import re
from pathlib import Path

OPS = Path(__file__).resolve().parents[1] / "src" / "ops"
TABLE = OPS / "table.rs"

SLUG_BY_TYPE = {
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


def extract_entries(text: str) -> dict[str, str]:
    entries: dict[str, str] = {}
    patterns = [
        r"spec_animated\(",
        r"spec\(",
        r"converter_spec\(",
        r"let mut s = converter_spec\(",
    ]
    i = 0
    while i < len(text):
        matched = None
        for p in patterns:
            m = re.search(p, text[i:])
            if m and (matched is None or m.start() < matched.start()):
                matched = m
                pat = p
        if not matched:
            break
        start = i + matched.start()
        depth = 0
        j = start
        while j < len(text):
            if text[j] == "(":
                depth += 1
            elif text[j] == ")":
                depth -= 1
                if depth == 0:
                    block = text[start : j + 1]
                    type_m = re.search(r'"(\w+CHOP)"|(PARAMETER|INFO)', block)
                    if type_m:
                        type_name = type_m.group(1) or (
                            "parameterCHOP" if type_m.group(2) == "PARAMETER" else "infoCHOP"
                        )
                        slug = SLUG_BY_TYPE.get(type_name)
                        if slug:
                            inner = block
                            inner = re.sub(r"params_\w+", "params", inner)
                            inner = re.sub(r"cook_\w+", "cook", inner)
                            inner = inner.replace("params_info", "params")
                            inner = inner.replace("params_parameter", "params")
                            inner = inner.replace("cook_info", "cook")
                            inner = inner.replace("cook_parameter", "cook")
                            inner = inner.replace("PARAMETER", "TYPE_NAME").replace(
                                "INFO", "TYPE_NAME"
                            )
                            inner = inner.replace('"parameterCHOP"', "TYPE_NAME").replace(
                                '"infoCHOP"', "TYPE_NAME"
                            )
                            for tn in SLUG_BY_TYPE:
                                inner = inner.replace(f'"{tn}"', "TYPE_NAME")
                            if inner.startswith("let mut s"):
                                spec_body = f"""pub const TYPE_NAME: &str = "{type_name}";

pub fn spec() -> ChopSpec {{
    {inner};
    s
}}"""
                            else:
                                fn = inner.split("(", 1)[0]
                                spec_body = f"""pub const TYPE_NAME: &str = "{type_name}";

pub fn spec() -> ChopSpec {{
    super::{inner}
}}"""
                            entries[slug] = spec_body
                    i = j + 1
                    break
            j += 1
        else:
            break
    entries["parameter"] = """pub const TYPE_NAME: &str = "parameterCHOP";
pub const PARAMETER: &str = TYPE_NAME;

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Parameter",
        &[],
        "Another operator's parameter values as channels (does not cook that operator).",
        params,
        cook,
    )
}"""
    entries["info"] = """pub const TYPE_NAME: &str = "infoCHOP";
pub const INFO: &str = TYPE_NAME;

pub fn spec() -> ChopSpec {
    super::spec_animated(
        TYPE_NAME,
        "Info",
        &[],
        "Another operator's info channels (TouchDesigner Info CHOP).",
        params,
        cook,
    )
}"""
    entries["merge"] = entries.get(
        "merge",
        """pub const TYPE_NAME: &str = "mergeCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Merge",
        &["a", "b"],
        "Put two CHOPs' channels side by side.",
        super::helpers::no_params,
        cook,
    )
}""",
    )
    return entries


def main() -> None:
    table = TABLE.read_text()
    specs = extract_entries(table)
    for slug, spec_tail in specs.items():
        mod_path = OPS / slug / "mod.rs"
        if not mod_path.is_file():
            continue
        text = mod_path.read_text()
        cut = re.search(r"\npub const TYPE_NAME:", text)
        if not cut:
            cut = re.search(r"\n// missing spec", text)
        if cut:
            text = text[: cut.start()] + "\n\n" + spec_tail + "\n"
        else:
            text = text.rstrip() + "\n\n" + spec_tail + "\n"
        mod_path.write_text(text)
    print("fixed", len(specs), "specs")


if __name__ == "__main__":
    main()