//! Native open/save panels that do not nest a modal run loop inside winit.
//!
//! `rfd`'s blocking `save_file` / `pick_file` calls `NSSavePanel.runModal()`.
//! That spins the Cocoa run loop while winit is still inside the event that
//! opened the panel (Cmd+S, a menu click). Cmd+Q during the panel is delivered
//! into that nested loop, and winit aborts with "tried to handle event while
//! another event is currently being handled".
//!
//! The async API attaches the panel as a sheet and returns immediately. The
//! path comes back on a later frame. While the sheet is up, a close request is
//! cancelled so quit waits until the panel is done.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use otd_core::NodeId;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

use super::OtdApp;

#[derive(Default)]
pub(crate) struct FileDialogState {
    pending: Option<Pending>,
    host: Option<HostWindow>,
}

struct Pending {
    future: Pin<Box<dyn Future<Output = Option<PathBuf>> + Send>>,
    action: Action,
}

#[derive(Clone, Copy)]
struct HostWindow {
    window: RawWindowHandle,
    display: RawDisplayHandle,
}

impl FileDialogState {
    pub(crate) fn is_open(&self) -> bool {
        self.pending.is_some()
    }
}

pub(crate) enum Kind {
    SaveFile,
    PickFile,
    PickFolder,
}

pub(crate) enum Action {
    SaveProject { then_close: bool },
    OpenProject,
    SaveComponent { id: NodeId },
    ImportComponent,
    ExportBundle { name: String },
    ImportIsf { id: NodeId },
    SettingsStartupFile,
}

/// Remember the root window so the sheet is parented to it. Without a parent,
/// rfd falls back to `runModal()` — the call that panics.
pub(crate) fn note_host(app: &mut OtdApp, frame: &eframe::Frame) {
    let Ok(window) = raw_window_handle::HasWindowHandle::window_handle(frame) else {
        return;
    };
    let Ok(display) = raw_window_handle::HasDisplayHandle::display_handle(frame) else {
        return;
    };
    app.file_dialogs.host = Some(HostWindow {
        window: window.as_raw(),
        display: display.as_raw(),
    });
}

pub(crate) fn request(
    app: &mut OtdApp,
    mut dialog: rfd::AsyncFileDialog,
    kind: Kind,
    action: Action,
) {
    if app.file_dialogs.pending.is_some() {
        app.document.status = "Close the file dialog first".into();
        return;
    }
    if let Some(host) = app.file_dialogs.host {
        dialog = dialog.set_parent(&host);
    }
    app.file_dialogs.pending = Some(Pending {
        future: boxed(dialog, kind),
        action,
    });
}

pub(crate) fn poll(app: &mut OtdApp, ctx: &egui::Context) {
    let path = {
        let Some(pending) = app.file_dialogs.pending.as_mut() else {
            return;
        };
        let mut cx = Context::from_waker(std::task::Waker::noop());
        match pending.future.as_mut().poll(&mut cx) {
            Poll::Ready(path) => path,
            Poll::Pending => return,
        }
    };
    let action = app
        .file_dialogs
        .pending
        .take()
        .expect("dialog future just became ready")
        .action;
    finish(app, ctx, action, path);
}

fn boxed(
    dialog: rfd::AsyncFileDialog,
    kind: Kind,
) -> Pin<Box<dyn Future<Output = Option<PathBuf>> + Send>> {
    match kind {
        Kind::SaveFile => Box::pin(async move {
            dialog
                .save_file()
                .await
                .map(|handle| handle.path().to_path_buf())
        }),
        Kind::PickFile => Box::pin(async move {
            dialog
                .pick_file()
                .await
                .map(|handle| handle.path().to_path_buf())
        }),
        Kind::PickFolder => Box::pin(async move {
            dialog
                .pick_folder()
                .await
                .map(|handle| handle.path().to_path_buf())
        }),
    }
}

fn finish(app: &mut OtdApp, ctx: &egui::Context, action: Action, path: Option<PathBuf>) {
    let Some(path) = path else {
        return;
    };
    match action {
        Action::SaveProject { then_close } => {
            app.write_project_file(&path);
            if then_close && !app.document.document_dirty {
                app.document.show_close_save_prompt = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        Action::OpenProject => app.load_project_file(path),
        Action::SaveComponent { id } => app.write_component_file(id, &path),
        Action::ImportComponent => {
            app.import_component_at(&path);
        }
        Action::ExportBundle { name } => app.export_bundle_into(&path, &name),
        Action::ImportIsf { id } => app.import_isf_from(id, &path),
        Action::SettingsStartupFile => {
            if let Some(draft) = app.settings_draft.as_mut() {
                draft.startup_custom_path = path;
            }
        }
    }
}

impl HasWindowHandle for HostWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // The pointer is the root window's NSView, refreshed every frame, and
        // only read on the main thread while that window is still open.
        Ok(unsafe { WindowHandle::borrow_raw(self.window) })
    }
}

impl HasDisplayHandle for HostWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe { DisplayHandle::borrow_raw(self.display) })
    }
}
