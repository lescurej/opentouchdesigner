use egui::{Color32, RichText, Ui};
use otd_core::indexmap::IndexMap;
use otd_core::{NodeId, Param};

use crate::app::OtdApp;

const TAB_ORDER: &[&str] = &["Play", "Image", "Trim", "Tune", "Common", "Parameters"];

const PANEL_TAB_ORDER: &[&str] = &[
    "Button",
    "Slider",
    "Field",
    "Text",
    "Layout",
    "Panel",
    "Look",
    "Children",
    "Drag/Drop",
    "Extensions",
    "Common",
];

pub fn sort_pages(pages: &mut Vec<String>, panel: bool) {
    let order = if panel { PANEL_TAB_ORDER } else { TAB_ORDER };
    pages.sort_by(|a, b| {
        let rank = |name: &str| {
            if !panel && name == "Common" {
                return 10_000;
            }
            order.iter().position(|&p| p == name).unwrap_or(500)
        };
        rank(a).cmp(&rank(b)).then_with(|| a.cmp(b))
    });
    pages.dedup();
}

pub fn collect_pages(params: &IndexMap<String, Param>, panel: bool, primary: &str) -> Vec<String> {
    let mut pages: Vec<String> = Vec::new();
    let mut unpaged = false;
    for param in params.values() {
        let page = param.page();
        if page.is_empty() {
            unpaged = true;
            continue;
        }
        if !pages.iter().any(|existing| existing == page) {
            pages.push(page.to_string());
        }
    }
    if unpaged {
        let name = if primary.is_empty() {
            "Parameters"
        } else {
            primary
        };
        if !pages.iter().any(|existing| existing == name) {
            pages.insert(0, name.to_string());
        }
    }
    if panel {
        for extra in ["Extensions", "Common"] {
            if !pages.iter().any(|p| p == extra) {
                pages.push(extra.into());
            }
        }
    }
    if pages.is_empty() {
        return vec!["Parameters".into()];
    }
    sort_pages(&mut pages, panel);
    if !panel && !primary.is_empty() {
        if let Some(index) = pages.iter().position(|page| page == primary) {
            let page = pages.remove(index);
            pages.insert(0, page);
        }
    }
    pages
}

pub fn ordered_param_keys(
    op_type: &str,
    params: &IndexMap<String, Param>,
    page: &str,
) -> Vec<String> {
    let table: &[&str] = match op_type {
        otd_engine::panel::BUTTON if page == "Button" => &["label", "value", "mode"],
        otd_engine::panel::SLIDER if page == "Slider" => &["value", "min", "max", "orientation"],
        otd_engine::panel::FIELD if page == "Field" => &["text"],
        otd_engine::panel::TEXT if page == "Text" => &["text", "fontsize"],
        _ if page == "Layout" => &[
            "x",
            "y",
            "w",
            "h",
            "hmode",
            "leftanchor",
            "leftoffset",
            "rightanchor",
            "rightoffset",
            "horigin",
            "hfillweight",
            "vmode",
            "bottomanchor",
            "bottomoffset",
            "topanchor",
            "topoffset",
            "vorigin",
            "vfillweight",
            "fixedaspect",
            "aspect",
            "layer",
            "postoffsetx",
            "postoffsety",
            "alignallow",
            "alignorder",
        ],
        _ if page == "Panel" => &["display", "enable", "clickthrough"],
        _ if page == "Look" => &[
            "opacity",
            "bgcolorr",
            "bgcolorg",
            "bgcolorb",
            "bgalpha",
            "topfill",
            "composite",
            "borderover",
        ],
        _ if page == "Children" => &[
            "align",
            "spacing",
            "alignmax",
            "marginl",
            "marginr",
            "margint",
            "marginb",
            "justifymethod",
            "justifyh",
            "justifyv",
            "fit",
            "scalex",
            "scaley",
            "offsetx",
            "offsety",
            "crop",
        ],
        _ => &[],
    };
    let mut keys: Vec<String> = table
        .iter()
        .filter(|k| params.contains_key(**k))
        .map(|s| (*s).to_string())
        .collect();
    if keys.is_empty() {
        keys = params
            .iter()
            .filter(|(_, p)| p.page() == page || (page == "Parameters" && p.page().is_empty()))
            .map(|(k, _)| k.clone())
            .collect();
        keys.sort();
    }
    keys
}

pub fn page_tabs(app: &mut OtdApp, ui: &mut Ui, id: NodeId, pages: &[String]) -> String {
    let active = app
        .param_inspector_page
        .get(&id)
        .cloned()
        .filter(|p| pages.contains(p))
        .unwrap_or_else(|| {
            pages
                .first()
                .cloned()
                .unwrap_or_else(|| "Parameters".into())
        });

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        for page in pages {
            let selected = *page == active;
            let label = RichText::new(page).size(13.0).color(if selected {
                Color32::from_rgb(235, 236, 240)
            } else {
                Color32::from_rgb(120, 122, 132)
            });
            let resp = ui.selectable_label(selected, label);
            if resp.clicked() {
                app.param_inspector_page.insert(id, page.clone());
            }
            if selected {
                let r = resp.rect;
                ui.painter().hline(
                    r.x_range(),
                    r.bottom() + 2.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(220, 222, 230)),
                );
            }
        }
    });
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
    app.param_inspector_page.get(&id).cloned().unwrap_or(active)
}

pub fn param_on_page(param: &Param, page: &str, has_pages: bool, primary: &str) -> bool {
    if !has_pages || (page == "Parameters" && primary.is_empty()) {
        return true;
    }
    let own = param.page();
    if own.is_empty() {
        return !primary.is_empty() && page == primary;
    }
    own == page
}