use crate::expr::EvalContext;

impl Param {
    pub fn new(value: impl Into<Value>) -> Self {
        Param {
            label: String::new(),
            page: String::new(),
            value: value.into(),
            mode: ParamMode::Constant,
            expression: String::new(),
            source: String::new(),
            range: None,
            menu: None,
            logarithmic: false,
            log_floor: None,
            custom: false,
            op_ref: false,
            compiled: None,
            error: None,
            needs_python: false,
            refs: Vec::new(),
            script: false,
            path_ref: false,
            file_ref: false,
            folder_ref: false,
            audio_device_list: AudioDeviceList::None,
        }
    }

    /// Mark this parameter as naming an operator. The path it holds becomes a
    /// cook dependency, so a Render TOP's camera and geometry are guaranteed
    /// to be up to date before it draws.
    pub fn as_path_ref(mut self) -> Self {
        self.path_ref = true;
        self.refs = own_path(&self.value.as_str());
        self
    }

    pub fn is_path_ref(&self) -> bool {
        self.path_ref
    }

    /// Mark this parameter as naming a file on disk.
    ///
    /// Nothing about cooking changes; this is what lets an export find the
    /// media a project depends on. A show file that copies its components but
    /// leaves the movie behind is a file that will be missing at 8pm, which is
    /// the whole reason bundles exist.
    pub fn as_file_ref(mut self) -> Self {
        self.file_ref = true;
        self
    }

    pub fn is_file_ref(&self) -> bool {
        self.file_ref
    }

    pub fn as_folder_ref(mut self) -> Self {
        self.folder_ref = true;
        self
    }

    pub fn is_folder_ref(&self) -> bool {
        self.folder_ref
    }

    pub fn with_logarithmic(mut self, floor: f64) -> Self {
        self.logarithmic = true;
        self.log_floor = Some(floor);
        self
    }

    pub fn with_audio_device_list(mut self, list: AudioDeviceList) -> Self {
        self.audio_device_list = list;
        self
    }

    pub fn with_audio_input_devices(self) -> Self {
        self.with_audio_device_list(AudioDeviceList::Input)
    }

    pub fn with_audio_output_devices(self) -> Self {
        self.with_audio_device_list(AudioDeviceList::Output)
    }

    pub fn audio_device_list(&self) -> AudioDeviceList {
        self.audio_device_list
    }

    /// Index of `chosen` in the menu (`value` or `option` match).
    pub fn menu_index(&self, chosen: &str) -> usize {
        let Some(menu) = &self.menu else {
            return 0;
        };
        menu.iter()
            .position(|o| o.value == chosen || o.option == chosen)
            .unwrap_or(0)
    }

    /// Mark this parameter as holding script source. Operator paths quoted
    /// inside it then become cook dependencies, the same as in an expression:
    /// a Script DAT reading `ch('/lfo1', ...)` must make `/lfo1` cook first.
    pub fn as_script(mut self) -> Self {
        self.script = true;
        self.refs = extract_paths(&self.value.as_str());
        self
    }

    pub fn is_script(&self) -> bool {
        self.script
    }

    /// Same as [`Param::as_script`], for a parameter already built.
    pub fn into_script(self) -> Self {
        self.as_script()
    }

    /// Same as [`Param::as_path_ref`], for a parameter already built.
    pub fn into_path_ref(self) -> Self {
        self.as_path_ref()
    }

    /// Mark this as an author-defined parameter on a component.
    pub fn as_custom(mut self) -> Self {
        self.custom = true;
        self
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn with_page(mut self, page: &str) -> Self {
        self.page = page.to_string();
        self
    }

    pub fn page(&self) -> &str {
        &self.page
    }

    pub fn with_range(mut self, lo: f64, hi: f64) -> Self {
        self.range = Some((lo, hi));
        self
    }

    pub fn with_menu(mut self, items: &[&str]) -> Self {
        self.menu = Some(items.iter().map(|s| SelectOption::same(s)).collect());
        self
    }

    /// Switch to Expression mode with the given source, compiling immediately
    /// so the artist sees a syntax error the moment they type it.
    pub fn text_for_cook(&self) -> String {
        if self.mode == ParamMode::Expression && !self.expression.trim().is_empty() {
            self.expression.clone()
        } else {
            self.value.as_str()
        }
    }

    pub fn set_expression(&mut self, src: &str) {
        self.expression = src.to_string();
        self.mode = ParamMode::Expression;
        self.recompile();
    }

    pub fn set_constant(&mut self, value: Value) {
        self.value = if let Value::Str(s) = &value {
            if self.audio_device_list != AudioDeviceList::None {
                Value::Select(s.clone())
            } else {
                value
            }
        } else {
            value
        };
        self.mode = ParamMode::Constant;
        self.refresh_refs();
    }

    /// Recompute what this parameter references from its current value.
    fn refresh_refs(&mut self) {
        if self.script {
            self.refs = extract_paths(&self.value.as_str());
        } else if self.path_ref {
            self.refs = own_path(&self.value.as_str());
        }
    }

    /// Recompile the expression. Must be called after deserialization — the
    /// compiled AST is not part of the project file.
    pub fn recompile(&mut self) {
        self.needs_python = false;
        if self.script || self.path_ref {
            self.refresh_refs();
        } else {
            self.refs.clear();
        }
        if self.mode != ParamMode::Expression || self.expression.trim().is_empty() {
            self.compiled = None;
            self.error = None;
            return;
        }
        let expr = self.expression.trim();
        if expr.contains("parent(") || expr.contains("parent().") {
            self.compiled = None;
            self.error = None;
            self.needs_python = true;
            self.refs = extract_paths(expr);
            return;
        }
        match Expr::parse(expr) {
            Ok(e) => {
                self.refs = e.referenced_paths();
                self.compiled = Some(e);
                self.error = None;
            }
            // Not an error yet: the built-in language is a fast path for the
            // common case, and anything it cannot parse is handed to Python.
            // Only if Python is absent too does this become a real error.
            Err(_) => {
                self.compiled = None;
                self.error = None;
                self.needs_python = true;
                self.refs = extract_paths(&self.expression);
            }
        }
    }

    /// The last reported evaluation or compilation error, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// True when this parameter's value can change without anyone editing it.
    /// Propagated into the cook engine's `time_dependent` flag.
    pub fn is_time_dependent(&self) -> bool {
        match self.mode {
            // A Python expression is assumed animated: proving otherwise would
            // mean analysing arbitrary Python, and cooking a static parameter
            // every frame is far cheaper than a viewer that never updates.
            ParamMode::Expression if self.needs_python => true,
            ParamMode::Expression => self
                .compiled
                .as_ref()
                .map(|e| e.is_time_dependent())
                .unwrap_or(false),
            // A CHOP export re-evaluates whenever its source CHOP cooks; that
            // is handled as a graph dependency, not as time dependence.
            _ => false,
        }
    }

    /// Resolve to a concrete value for this cook.
    ///
    /// Never fails: a broken expression holds the last good constant and
    /// records the error, because dropping frames in a live show over a typo
    /// is not acceptable behaviour.
    pub fn eval(&self, ctx: &EvalContext) -> Value {
        match self.mode {
            ParamMode::Constant => self.value.clone(),
            ParamMode::Expression => {
                if let Some(v) = eval_parent_reference(&self.expression, &self.value, ctx) {
                    return v;
                }
                match &self.compiled {
                    Some(e) => match e.eval(ctx) {
                        Ok(v) => self.value.coerce_from_f64(v),
                        Err(_) => self.value.clone(),
                    },
                    None if self.needs_python => ctx
                        .channels
                        .and_then(|c| c.eval_python(&self.expression, ctx, ctx.path.unwrap_or("")))
                        .and_then(|r| r.ok())
                        .map(|v| coerce_like(&self.value, v))
                        .unwrap_or_else(|| self.value.clone()),
                    None => self.value.clone(),
                }
            }
            ParamMode::Export => self
                .source_parts()
                .and_then(|(path, channel)| ctx.channels?.channel(path, channel))
                .map(|v| self.value.coerce_from_f64(v as f64))
                .unwrap_or_else(|| self.value.clone()),
            ParamMode::Bind => self
                .source_parts()
                .and_then(|(path, param)| ctx.channels?.param_value(path, param))
                .filter(|v| v.same_type_as(&self.value))
                .unwrap_or_else(|| self.value.clone()),
        }
    }

    /// Split `source` into the operator path and the channel or parameter
    /// name after it: `/lfo1:chan1`.
    pub fn source_parts(&self) -> Option<(&str, &str)> {
        let (path, name) = self.source.rsplit_once(':')?;
        if path.trim().is_empty() || name.trim().is_empty() {
            return None;
        }
        Some((path.trim(), name.trim()))
    }

    /// The operator this parameter reads from, if it is in Export or Bind
    /// mode. The cook engine turns these into dependencies.
    /// Operators this parameter reads from, whatever the mode: an Export or
    /// Bind source, or a path mentioned in a Python expression.
    pub fn referenced_ops(&self) -> impl Iterator<Item = &str> {
        self.source_op()
            .into_iter()
            .chain(self.refs.iter().map(|s| s.as_str()))
    }

    pub fn source_op(&self) -> Option<&str> {
        match self.mode {
            ParamMode::Export | ParamMode::Bind => self.source_parts().map(|(p, _)| p),
            _ => None,
        }
    }

    /// Point this parameter at a CHOP channel.
    pub fn set_export(&mut self, op_path: &str, channel: &str) {
        self.source = format!("{op_path}:{channel}");
        self.mode = ParamMode::Export;
    }

    /// Point this parameter at another operator's parameter.
    pub fn set_bind(&mut self, op_path: &str, param: &str) {
        self.source = format!("{op_path}:{param}");
        self.mode = ParamMode::Bind;
    }

    /// Like [`Param::eval`] but reports evaluation failures, for the UI.
    pub fn eval_checked(&self, ctx: &EvalContext) -> Result<Value, String> {
        if let Some(err) = &self.error {
            return Err(err.clone());
        }
        if self.mode == ParamMode::Expression {
            if let Some(v) = eval_parent_reference(&self.expression, &self.value, ctx) {
                return Ok(v);
            }
            match &self.compiled {
                Some(e) => e
                    .eval(ctx)
                    .map(|v| self.value.coerce_from_f64(v))
                    .map_err(|e| e.0),
                None if self.needs_python => match ctx
                    .channels
                    .and_then(|c| c.eval_python(&self.expression, ctx, ctx.path.unwrap_or("")))
                {
                    Some(Ok(v)) => Ok(coerce_like(&self.value, v)),
                    Some(Err(e)) => Err(e),
                    None => Err("no Python interpreter in this build".to_string()),
                },
                None => Ok(self.value.clone()),
            }
        } else {
            Ok(self.eval(ctx))
        }
    }
}

fn eval_parent_reference(expression: &str, declared: &Value, ctx: &EvalContext) -> Option<Value> {
    let expr = expression.trim();
    let key = expr
        .strip_prefix("parent().par.")
        .or_else(|| expr.strip_prefix("parent()."))
        .or_else(|| expr.strip_prefix("parent."))?;
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let v = ctx.channels?.parent_param(ctx.path?, key)?;
    if v.same_type_as(declared) {
        Some(v)
    } else if matches!(declared, Value::Str(_)) {
        Some(Value::Str(v.as_str()))
    } else {
        Some(declared.coerce_from_f64(v.as_f64()))
    }
}
