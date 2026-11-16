#[cfg(target_os = "macos")]
use std::sync::Mutex;

pub struct PlaybackActivity {
    #[cfg(target_os = "macos")]
    token: Mutex<Option<objc2::rc::Retained<objc2_foundation::NSObject>>>,
}

impl PlaybackActivity {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            token: Mutex::new(None),
        }
    }

    pub fn set_playing(&self, playing: bool) {
        #[cfg(target_os = "macos")]
        {
            let mut slot = self.token.lock().expect("playback activity");
            if playing == slot.is_some() {
                return;
            }
            let info = objc2_foundation::NSProcessInfo::processInfo();
            if playing {
                let reason = objc2_foundation::NSString::from_str("OpenTouchDesigner playback");
                let activity = unsafe {
                    info.beginActivityWithOptions_reason(
                        objc2_foundation::NSActivityOptions::NSActivityUserInteractive,
                        &reason,
                    )
                };
                *slot = Some(activity);
            } else if let Some(activity) = slot.take() {
                unsafe { info.endActivity(&activity) };
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = playing;
        }
    }
}
