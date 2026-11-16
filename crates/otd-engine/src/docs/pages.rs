use std::collections::BTreeMap;
use std::fmt::Write as _;

use otd_core::{Family, OpRegistry, Param, Value};

use super::notes::NOTES;


/// One operator's page.
pub fn operator_page(def: &otd_core::OpDef) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "### {} — `{}`\n", def.label, def.type_name);
    let _ = writeln!(out, "{}\n", def.summary);

    if let Some((_, note)) = NOTES.iter().find(|(name, _)| *name == def.type_name) {
        let _ = writeln!(out, "{note}\n");
    }

    if def.inputs.is_empty() {
        let _ = writeln!(out, "*No inputs — this is a generator.*\n");
    } else {
        let _ = writeln!(out, "**Inputs:** {}\n", def.inputs.join(", "));
    }
    if def.time_dependent {
        let _ = writeln!(
            out,
            "*Time dependent: cooks every frame, and everything downstream of it does too.*\n"
        );
    }

    let params = (def.params)();
    if !params.is_empty() {
        let _ = writeln!(out, "| Parameter | Name | Default | Range |");
        let _ = writeln!(out, "|---|---|---|---|");
        for (key, p) in &params {
            let _ = writeln!(
                out,
                "| {} | `{key}` | {} | {} |",
                if p.label.is_empty() { key } else { &p.label },
                describe_default(&p.value),
                describe_range(p),
            );
        }
        let _ = writeln!(out);
    }
    out
}

fn describe_default(v: &Value) -> String {
    match v {
        Value::Str(s) if s.is_empty() => "*(empty)*".into(),
        Value::Str(s) if s.contains('\n') => "*(multi-line)*".into(),
        Value::Str(s) => format!("`{s}`"),
        Value::Float(f) => format!("`{f}`"),
        Value::Int(i) => format!("`{i}`"),
        Value::Bool(b) => format!("`{b}`"),
        Value::Vec2(v) => format!("`{v:?}`"),
        Value::Vec3(v) => format!("`{v:?}`"),
        Value::Vec4(v) => format!("`{v:?}`"),
        Value::Select(s) => format!("`{s}`"),
    }
}

fn describe_range(p: &Param) -> String {
    if let Some(menu) = &p.menu {
        return menu
            .iter()
            .map(|m| format!("`{}`", m.option))
            .collect::<Vec<_>>()
            .join(" · ");
    }
    match p.range {
        Some((lo, hi)) => format!("{lo} … {hi}"),
        None => String::new(),
    }
}

/// The whole reference: an index and a page per operator, grouped by family.
pub fn reference(registry: &OpRegistry) -> String {
    let mut by_family: BTreeMap<&'static str, Vec<&otd_core::OpDef>> = BTreeMap::new();
    for def in registry.iter() {
        by_family.entry(def.family.suffix()).or_default().push(def);
    }
    for defs in by_family.values_mut() {
        defs.sort_by_key(|d| d.type_name);
    }

    let mut out = String::from(
        "# Operator reference\n\n\
         Generated from the operator registry — the same table the editor builds \
         its menus and parameter pages from, so this cannot drift from what the \
         operators actually do.\n\n",
    );

    let total: usize = by_family.values().map(|v| v.len()).sum();
    let _ = writeln!(out, "{total} operators.\n");

    // Index first: the thing somebody scanning for "is there an X" reads.
    for (family, defs) in &by_family {
        let _ = writeln!(out, "**{family}** ({})\n", defs.len());
        for def in defs {
            let _ = writeln!(
                out,
                "- [{}](#{}) — {}",
                def.type_name,
                anchor(def),
                def.summary
            );
        }
        let _ = writeln!(out);
    }

    for (family, defs) in &by_family {
        let _ = writeln!(out, "---\n\n## {family}\n");
        for def in defs {
            out.push_str(&operator_page(def));
        }
    }
    out
}

/// The GitHub-flavoured anchor for an operator's heading.
pub(crate) fn anchor(def: &otd_core::OpDef) -> String {
    // `### Feedback — `feedbackTOP`` becomes `feedback--feedbacktop`.
    format!(
        "{}--{}",
        def.label.to_lowercase().replace(' ', "-"),
        def.type_name.to_lowercase()
    )
}

/// The hand-written note for an operator, if it has one.
///
/// The editor shows this too, so the reference and the parameter panel are
/// reading one source rather than two that can disagree.
pub fn note(type_name: &str) -> Option<&'static str> {
    NOTES
        .iter()
        .find(|(name, _)| *name == type_name)
        .map(|(_, note)| *note)
}

/// Operators or parameters with nothing written about them.
///
/// Returned rather than printed so a test can be the enforcement, which is
/// the point: "enforced in PR review" holds until the week somebody is busy.
pub fn undocumented(registry: &OpRegistry) -> Vec<String> {
    let mut missing = Vec::new();
    for def in registry.iter() {
        if def.summary.trim().is_empty() {
            missing.push(format!("{}: no summary", def.type_name));
        } else if !def.summary.trim().ends_with('.') {
            // A summary is a sentence. Without this they drift into labels.
            missing.push(format!(
                "{}: summary should be a sentence ending in a full stop",
                def.type_name
            ));
        }
        if def.label.trim().is_empty() {
            missing.push(format!("{}: no label", def.type_name));
        }
        for (key, p) in (def.params)() {
            if p.label.trim().is_empty() {
                missing.push(format!("{}.{key}: no label", def.type_name));
            }
        }
    }
    missing
}

/// Family descriptions for the top of the reference.
pub fn family_summary(family: Family) -> &'static str {
    match family {
        Family::Top => "Texture operators — everything on the GPU.",
        Family::Chop => "Channel operators — control signals and audio, sampled over time.",
        Family::Sop => "Surface operators — geometry.",
        Family::Dat => "Data operators — text and tables.",
        Family::Mat => "Materials.",
        Family::Comp => "Components — containers, and the 3D scene objects.",
    }
}

