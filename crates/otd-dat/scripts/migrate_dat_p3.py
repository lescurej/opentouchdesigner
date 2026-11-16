#!/usr/bin/env python3
from pathlib import Path

OPS = Path(__file__).resolve().parents[1] / "src" / "ops"

OPS12 = [
    ("table", "TABLE", "Table", "[]", "A table of text, stored in the project file.", "params_table", "cook_table", False),
    ("text", "TEXT", "Text", "[]", "A block of text.", "params_text", "cook_text", False),
    ("info", '"infoDAT"', "Info", "[]", "Table of name/value rows for another operator.", "params_info", "cook_info", False),
    ("panel_table_text", "PANEL_TABLE_TEXT", "Panel Table Text", "[]", "Formats a list/table COMP's table DAT as text for a textTOP.", "params_panel_table_text", "cook_panel_table_text", False),
    ("select", '"selectDAT"', "Select", '["dat"]', "Pick rows and columns, by name or index.", "params_select", "cook_select", False),
    ("merge", '"mergeDAT"', "Merge", '["dat"]', "Join two DATs by rows or by columns.", "params_merge", "cook_merge", False),
    ("json", '"jsonDAT"', "JSON", '["in"]', "Parse JSON text into a path/value table.", "params_json", "cook_json", False),
    ("null", "NULL", "Null", '["in"]', "Pass-through. A stable name to reference.", "no_params", "cook_null", False),
    ("missing", "otd_core::MISSING_DAT", "Missing", '["in"]', "Placeholder for an unknown or removed DAT. Passes input 0 through.", "params_missing", "cook_null", False),
    ("udpin", '"udpinDAT"', "UDP In", "[]", "Listens for datagrams; table columns address, port, message.", "params_udp_in", "cook_udp_in", True),
    ("udpout", '"udpoutDAT"', "UDP Out", '["in"]', "Sends with send() / optional Send Input On Change; table shows incoming replies.", "params_udp_out", "cook_udp_out", True),
    ("script", "SCRIPT", "Script", '["in"]', "Rows produced by a Python script.", "params_script", "cook_script", True),
]

TEMPLATE = '''use super::{{DatCtx, DatSpec, spec}};
use super::constants::*;
use super::helpers::{{no_params, params_missing}};
use super::operators::{{{params}, {cook}}};

pub const TYPE_NAME: &str = {type_const};

pub fn spec() -> DatSpec {{
    let mut s = spec(
        TYPE_NAME,
        "{label}",
        &{inputs},
        "{summary}",
        {params},
        {cook},
    );
{time_dependent}    s
}}
'''

for slug, type_const, label, inputs, summary, params, cook, td in OPS12:
    td_line = '    s.def.time_dependent = true;\n' if td else ''
    body = TEMPLATE.format(
        type_const=type_const,
        label=label,
        inputs=inputs,
        summary=summary,
        params=params,
        cook=cook,
        time_dependent=td_line,
    )
    dest = OPS / slug
    dest.mkdir(exist_ok=True)
    (dest / "mod.rs").write_text(body)
    print(slug)