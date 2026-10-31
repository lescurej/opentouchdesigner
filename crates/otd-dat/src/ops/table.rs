use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, Param};

use crate::DatData;

use super::constants::*;
use super::helpers::no_params;
use super::operators::*;
use super::{DatCtx, DatSpec, spec};

pub(super) fn core_specs() -> Vec<DatSpec> {
    let mut v = vec![
            spec(
                TABLE,
                "Table",
                &[],
                "A table of text, stored in the project file.",
                params_table,
                cook_table,
            ),
            spec(
                TEXT,
                "Text",
                &[],
                "A block of text.",
                params_text,
                cook_text,
            ),
            spec(
                "selectDAT",
                "Select",
                &["in"],
                "Pick rows and columns, by name or index.",
                params_select,
                cook_select,
            ),
            spec(
                "mergeDAT",
                "Merge",
                &["a", "b"],
                "Join two DATs by rows or by columns.",
                params_merge,
                cook_merge,
            ),
            spec(
                "jsonDAT",
                "JSON",
                &["in"],
                "Parse JSON text into a path/value table.",
                params_json,
                cook_json,
            ),
            spec(
                NULL,
                "Null",
                &["in"],
                "Pass-through. A stable name to reference.",
                no_params,
                cook_null,
            ),
        ];
        // UDP In re-cooks every frame: messages arrive whether or not
        // anything in the graph changed.
        let mut udp_in = spec(
            "udpinDAT",
            "UDP In",
            &[],
            "Listens for datagrams; table columns address, port, message.",
            params_udp_in,
            cook_udp_in,
        );
        udp_in.def.time_dependent = true;
        v.push(udp_in);
        let mut udp_out = spec(
            "udpoutDAT",
            "UDP Out",
            &["in"],
            "Sends with send() / optional Send Input On Change; table shows incoming replies.",
            params_udp_out,
            cook_udp_out,
        );
        udp_out.def.time_dependent = true;
        v.push(udp_out);

        // A Script DAT re-runs every frame: its source may read time, and
        // there is no way to know it doesn't.
        let mut script = spec(
            SCRIPT,
            "Script",
            &["in"],
            "Rows produced by a Python script.",
            params_script,
            cook_script,
        );
        script.def.time_dependent = true;
        v.push(script);

        let mut webclient = spec(
            WEBCLIENT,
            "Web Client",
            &[],
            "HTTP client: pulse Fetch to request in a background thread; response body is the DAT contents. WebSocket is not implemented yet.",
            params_webclient,
            cook_webclient,
        );
        webclient.def.time_dependent = true;
        v.push(webclient);

        let mut webserver = spec(
            WEBSERVER,
            "Web Server",
            &[],
            "HTTP server: onHTTPRequest and WebSocket callbacks in the Callbacks DAT; HTTP and WebSocket on the same port.",
            params_webserver,
            cook_webserver,
        );
        webserver.def.time_dependent = true;
        v.push(webserver);

        for (type_name, label, summary, params, cook) in [
            (
                EXECUTE,
                "Execute",
                "Python callbacks at the start and end of a frame.",
                params_execute as fn() -> IndexMap<String, Param>,
                cook_execute as fn(&mut DatCtx) -> DatData,
            ),
            (
                "chopexecuteDAT",
                "CHOP Execute",
                "Python callbacks when a watched channel changes or crosses a threshold.",
                params_chop_execute,
                cook_chop_execute,
            ),
            (
                "parameterexecuteDAT",
                "Parameter Execute",
                "Python callbacks when a watched parameter changes.",
                params_par_execute,
                cook_par_execute,
            ),
        ] {
            let mut ex = spec(type_name, label, &[], summary, params, cook);
            // A callback that only ran when something downstream wanted it
            // would not be a callback. These cook every frame, and the hosts
            // treat them as roots.
            ex.def.time_dependent = true;
            v.push(ex);
        }

        let mut panel_ex = spec(
            PANEL_EXECUTE,
            "Panel Execute",
            &[],
            "Python callbacks when a panel value changes.",
            params_panel_execute,
            cook_panel_execute,
        );
        panel_ex.def.time_dependent = true;
        v.push(panel_ex);

        v.push(spec(
            "sortDAT",
            "Sort",
            &["in"],
            "Sort rows by a column, numerically or as text.",
            params_sort,
            cook_sort,
        ));
        v.push(spec(
            "transposeDAT",
            "Transpose",
            &["in"],
            "Swap rows and columns.",
            no_params,
            cook_transpose,
        ));
        v.push(spec(
            "substituteDAT",
            "Substitute",
            &["in"],
            "Fill $name placeholders from a two-column lookup table.",
            params_substitute,
            cook_substitute,
        ));
        v.push(spec(
            "convertDAT",
            "Convert",
            &["in"],
            "Between a table and a block of text.",
            params_convert,
            cook_convert,
        ));

        let mut to_dat = spec(
            "choptodatDAT",
            "CHOP to DAT",
            &["in"],
            "Channels as a table of numbers.",
            params_chop_to,
            cook_chop_to_dat,
        );
        to_dat.def.input_families = &[Family::Chop];
        v.push(to_dat);

        let mut in_dat = spec(
            IN,
            "In",
            &[],
            "A data input on this component's node.",
            no_params,
            cook_null,
        );
        in_dat.def.connector = Connector::In;
        v.push(in_dat);

        let mut out_dat = spec(
            OUT,
            "Out",
            &["in"],
            "This component's data output.",
            no_params,
            cook_null,
        );
        out_dat.def.connector = Connector::Out;
        v.push(out_dat);
        v
}
