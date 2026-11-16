#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::Mutex;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(target_os = "macos")]
static OPEN_QUEUE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
#[cfg(target_os = "macos")]
static LAUNCH: AtomicU8 = AtomicU8::new(0);

#[cfg(target_os = "macos")]
pub enum Poll {
    Pending,
    Startup,
    Files(Vec<PathBuf>),
}

#[cfg(target_os = "macos")]
pub fn poll() -> Poll {
    let files = take_pending_opens();
    if !files.is_empty() {
        LAUNCH.store(2, Ordering::Relaxed);
        return Poll::Files(files);
    }
    match LAUNCH.load(Ordering::Relaxed) {
        0 => {
            LAUNCH.store(1, Ordering::Relaxed);
            Poll::Pending
        }
        1 => {
            LAUNCH.store(2, Ordering::Relaxed);
            Poll::Startup
        }
        _ => Poll::Pending,
    }
}

#[cfg(target_os = "macos")]
fn take_pending_opens() -> Vec<PathBuf> {
    OPEN_QUEUE
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

#[cfg(target_os = "macos")]
pub fn skip_startup() {
    LAUNCH.store(2, Ordering::Relaxed);
}

#[cfg(target_os = "macos")]
fn push_path(path: PathBuf) {
    let Ok(mut q) = OPEN_QUEUE.lock() else {
        return;
    };
    if !q.iter().any(|p| p == &path) {
        q.push(path);
    }
}

#[cfg(target_os = "macos")]
pub fn install_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(install);
}

#[cfg(not(target_os = "macos"))]
pub fn install_once() {}

#[cfg(target_os = "macos")]
pub fn install() {
    use objc2::runtime::{AnyClass, AnyObject, Bool, Sel};

    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(class) = AnyClass::get("WinitApplicationDelegate") else {
            log::warn!("macOS open-document hooks: WinitApplicationDelegate not registered");
            return;
        };
        let cls = std::ptr::from_ref(class)
            .cast::<objc2::ffi::objc_class>()
            .cast_mut();
        unsafe {
            add_if_missing(
                cls,
                objc2::sel!(application:openURLs:),
                imp(open_urls
                    as extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject)),
                c"v@:@@",
            );
            add_if_missing(
                cls,
                objc2::sel!(application:openFile:),
                imp(open_file
                    as extern "C-unwind" fn(
                        *mut AnyObject,
                        Sel,
                        *mut AnyObject,
                        *mut AnyObject,
                    ) -> Bool),
                open_file_types(),
            );
            add_if_missing(
                cls,
                objc2::sel!(application:openFiles:),
                imp(open_files
                    as extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject)),
                c"v@:@@",
            );
        }
    });
}

#[cfg(target_os = "macos")]
fn open_file_types() -> &'static CStr {
    if cfg!(target_arch = "aarch64") {
        c"B@:@@"
    } else {
        c"c@:@@"
    }
}

#[cfg(target_os = "macos")]
fn imp<T: Copy>(func: T) -> objc2::runtime::Imp {
    debug_assert_eq!(
        std::mem::size_of::<T>(),
        std::mem::size_of::<objc2::runtime::Imp>()
    );
    unsafe { std::mem::transmute_copy(&func) }
}

#[cfg(target_os = "macos")]
unsafe fn add_if_missing(
    cls: *mut objc2::ffi::objc_class,
    sel: objc2::runtime::Sel,
    imp: objc2::runtime::Imp,
    types: &CStr,
) {
    unsafe {
        if !objc2::ffi::class_getInstanceMethod(cls, sel.as_ptr()).is_null() {
            return;
        }
        let ok = objc2::ffi::class_addMethod(cls, sel.as_ptr(), Some(imp), types.as_ptr());
        if (ok as i32) == 0 {
            log::warn!("macOS open-document hooks: failed to add {sel:?}");
        }
    }
}

#[cfg(target_os = "macos")]
fn push_nsstring(s: &objc2_foundation::NSString) {
    push_path(PathBuf::from(s.to_string()));
}

#[cfg(target_os = "macos")]
fn push_url(url: &objc2_foundation::NSURL) {
    if let Some(path) = unsafe { url.path() } {
        push_nsstring(&path);
    }
}

#[cfg(target_os = "macos")]
extern "C-unwind" fn open_urls(
    _: *mut objc2::runtime::AnyObject,
    _: objc2::runtime::Sel,
    _: *mut objc2::runtime::AnyObject,
    urls: *mut objc2::runtime::AnyObject,
) {
    if urls.is_null() {
        return;
    }
    let urls = unsafe { &*urls.cast::<objc2_foundation::NSArray<objc2_foundation::NSURL>>() };
    for i in 0..urls.count() {
        let url = unsafe { urls.objectAtIndex(i) };
        push_url(&url);
    }
}

#[cfg(target_os = "macos")]
extern "C-unwind" fn open_file(
    _: *mut objc2::runtime::AnyObject,
    _: objc2::runtime::Sel,
    _: *mut objc2::runtime::AnyObject,
    filename: *mut objc2::runtime::AnyObject,
) -> objc2::runtime::Bool {
    if !filename.is_null() {
        let name = unsafe { &*filename.cast::<objc2_foundation::NSString>() };
        push_nsstring(name);
    }
    objc2::runtime::Bool::YES
}

#[cfg(target_os = "macos")]
extern "C-unwind" fn open_files(
    _: *mut objc2::runtime::AnyObject,
    _: objc2::runtime::Sel,
    _: *mut objc2::runtime::AnyObject,
    filenames: *mut objc2::runtime::AnyObject,
) {
    if filenames.is_null() {
        return;
    }
    let names =
        unsafe { &*filenames.cast::<objc2_foundation::NSArray<objc2_foundation::NSString>>() };
    for i in 0..names.count() {
        let name = unsafe { names.objectAtIndex(i) };
        push_nsstring(&name);
    }
}

#[cfg(target_os = "macos")]
impl crate::app::OtdApp {
    pub(crate) fn poll_macos_open_files(&mut self) {
        if let Poll::Files(paths) = poll() {
            for path in paths {
                self.open(Some(path));
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl crate::app::OtdApp {
    pub(crate) fn poll_macos_open_files(&mut self) {}
}

