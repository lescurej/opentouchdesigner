use super::super::{PendingLaunch, *};

impl OtdApp {
    pub(crate) fn run_pending_launch(&mut self) {
        let launch = std::mem::replace(&mut self.document.pending_launch, PendingLaunch::None);
        match launch {
            PendingLaunch::None => {}
            PendingLaunch::Startup => self.apply_startup(),
            PendingLaunch::Open(path) => {
                #[cfg(target_os = "macos")]
                crate::macos_open::skip_startup();
                if path.is_file() {
                    self.load_project_file(path);
                } else {
                    self.load_blank();
                    self.document.status = format!("File not found: {}", path.display());
                }
            }
        }
    }

    pub fn apply_new_project_defaults(&mut self) {
        self.set_project_fps(self.settings.default_project_fps);
        self.transport.playing = self.settings.start_playing;
        self.transport.looping = self.settings.default_looping;
        let start = self.settings.default_loop_start;
        let end = if self.settings.default_loop_end > self.settings.default_loop_start {
            self.settings.default_loop_end
        } else {
            self.settings.default_loop_start + 0.1
        };
        self.transport.loop_range = (start, end);
    }

    pub fn apply_startup(&mut self) {
        use crate::settings::StartupMode;
        match self.settings.startup_mode {
            StartupMode::DefaultExample => {
                let name = if otd_engine::demo::NAMES
                    .contains(&self.settings.startup_demo.as_str())
                {
                    self.settings.startup_demo.clone()
                } else {
                    "tunnel".to_string()
                };
                self.load_demo(&name);
            }
            StartupMode::Blank => self.load_blank(),
            StartupMode::CustomFile => {
                let path = self.settings.startup_custom_path.clone();
                if path.as_os_str().is_empty() {
                    self.load_blank();
                    self.document.status = "Custom startup file not set — blank project".into();
                } else if path.is_file() {
                    self.open(Some(path));
                } else {
                    self.load_blank();
                    self.document.status = format!("Startup file not found: {}", path.display());
                }
            }
            StartupMode::LastOpened => {
                let path = self.settings.last_opened_path.clone();
                if path.as_os_str().is_empty() || !path.is_file() {
                    self.load_blank();
                    if !path.as_os_str().is_empty() {
                        self.document.status = format!("Last opened file not found: {}", path.display());
                    }
                } else {
                    self.load_project_file(path);
                }
            }
        }
    }

    pub fn export_bundle(&mut self) {
        let name = self
            .document.project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "show".into());
        let dialog = rfd::AsyncFileDialog::new()
            .set_file_name(&name)
            .set_title("Export bundle into a new folder");
        crate::app::file_dialog::request(
            self,
            dialog,
            crate::app::file_dialog::Kind::PickFolder,
            crate::app::file_dialog::Action::ExportBundle { name },
        );
    }

    pub fn import_component(&mut self) {
        let dialog =
            rfd::AsyncFileDialog::new().add_filter("OpenTouchDesigner component", &["otdc"]);
        crate::app::file_dialog::request(
            self,
            dialog,
            crate::app::file_dialog::Kind::PickFile,
            crate::app::file_dialog::Action::ImportComponent,
        );
    }

    pub fn import_component_at(&mut self, path: &std::path::Path) -> Option<NodeId> {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "component".into());
        let def = self.registry.get(otd_engine::comp::BASE).cloned()?;
        let parent = self.canvas.current;
        self.edit("import component");
        self.document.history.end_gesture();
        let id = self.graph.create(parent, &def, Some(&name)).ok()?;
        match self
            .graph
            .attach_external(id, &path.to_string_lossy(), &self.registry)
        {
            Ok(()) => {
                self.select_only(id);
                self.document.status = format!("Imported {}", path.display());
                Some(id)
            }
            Err(e) => {
                let _ = self.graph.remove(id);
                self.document.status = format!("Import failed: {e}");
                None
            }
        }
    }

    pub fn import_isf(&mut self, id: NodeId) {
        let dialog =
            rfd::AsyncFileDialog::new().add_filter("ISF shader", &["fs", "isf", "frag"]);
        crate::app::file_dialog::request(
            self,
            dialog,
            crate::app::file_dialog::Kind::PickFile,
            crate::app::file_dialog::Action::ImportIsf { id },
        );
    }

    pub fn import_isf_from(&mut self, id: NodeId, path: &std::path::Path) {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.document.status = format!("Import failed: {e}");
                return;
            }
        };
        match otd_gpu::isf::import(&source) {
            Ok(isf) => {
                self.edit("import ISF");
                self.document.history.end_gesture();
                let count = isf.params.len();
                otd_gpu::isf::apply(&mut self.graph, id, &isf);
                let name = path.file_stem().unwrap_or_default().to_string_lossy();
                self.document.status = format!("Imported {name} — {count} parameter(s)");
            }
            Err(e) => self.document.status = format!("Import failed: {e}"),
        }
    }

    pub fn load_blank(&mut self) {
        self.assistant.reset_context();
        self.graph = Graph::new();
        self.engine.reset_cook_state();
        self.thumbs.clear();
        self.document.history.clear();
        self.document.project_path = None;
        self.document.document_dirty = false;
        self.time = CookContext::default();
        self.canvas.current = self.graph.root();
        self.canvas.view = CanvasView::default();
        self.canvas.network_views.clear();
        self.canvas.viewer = None;
        self.clear_selection();
        self.document.status = "blank project".into();
        self.apply_new_project_defaults();
    }

    pub fn load_demo(&mut self, name: &str) {
        let Some((graph, _out)) = otd_engine::demo::by_name(name, &self.registry) else {
            self.load_blank();
            self.document.status = format!("no built-in patch called `{name}`");
            return;
        };
        self.assistant.reset_context();
        self.graph = graph;
        self.engine.reset_cook_state();
        self.thumbs.clear();
        self.document.history.clear();
        self.document.project_path = None;
        self.document.document_dirty = false;
        self.time = CookContext::default();
        self.canvas.current = self.graph.root();
        self.canvas.view = CanvasView::default();
        self.canvas.network_views.clear();
        self.canvas.viewer = self
            .graph
            .walk()
            .into_iter()
            .rev()
            .find(|&id| self.graph.node(id).family == otd_core::Family::Comp);
        self.clear_selection();
        if let Some(first) = self.graph.walk().into_iter().nth(1) {
            self.select_only(first);
        }
        self.document.status = format!("{name} patch");
        self.apply_new_project_defaults();
    }

    pub fn new_project(&mut self) {
        self.load_demo("starter");
    }

    pub fn open(&mut self, path: Option<PathBuf>) {
        let Some(path) = path else {
            let dialog =
                rfd::AsyncFileDialog::new().add_filter("OpenTouchDesigner project", &["otd"]);
            crate::app::file_dialog::request(
                self,
                dialog,
                crate::app::file_dialog::Kind::PickFile,
                crate::app::file_dialog::Action::OpenProject,
            );
            return;
        };
        self.load_project_file(path);
    }

    pub fn save(&mut self, path: Option<PathBuf>) {
        match path.or_else(|| self.document.project_path.clone()) {
            Some(p) => self.write_project_file(&p),
            None => self.ask_save_project(false),
        }
    }

    pub fn save_as(&mut self) {
        let suggested = self
            .document.project_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled.otd".to_string());
        let mut dialog = rfd::AsyncFileDialog::new()
            .add_filter("OpenTouchDesigner project", &["otd"])
            .set_file_name(&suggested);
        if let Some(dir) = self.document.project_path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        crate::app::file_dialog::request(
            self,
            dialog,
            crate::app::file_dialog::Kind::SaveFile,
            crate::app::file_dialog::Action::SaveProject { then_close: false },
        );
    }

    pub fn save_component(&mut self) {
        let Some(id) = self.canvas.selected.filter(|s| self.graph.contains(*s)) else {
            return;
        };
        if self.graph.node(id).family != otd_core::Family::Comp {
            self.document.status = "select a component first".into();
            return;
        }
        let name = self.graph.node(id).name.clone();
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter("OpenTouchDesigner component", &["otdc"])
            .set_file_name(format!("{name}.otdc"));
        crate::app::file_dialog::request(
            self,
            dialog,
            crate::app::file_dialog::Kind::SaveFile,
            crate::app::file_dialog::Action::SaveComponent { id },
        );
    }

    pub fn set_project_fps(&mut self, fps: f64) {
        let fps = fps.clamp(1.0, 240.0);
        self.time.fps = fps;
        self.time.frame = (self.time.time * fps).round() as i64;
    }
}