use std::sync::Arc;

use super::super::decode::{ffmpeg_command, missing_ffmpeg};
use super::super::Frame;
#[cfg(target_os = "macos")]
use super::super::macos;
use super::reader::Source;
use super::{spawn_ffmpeg, FIRST_FRAME_GRACE, MovieDecodeBackend, Reader};

impl Source {
    pub(crate) fn spawn_file(&self, path: &str, start_index: i64) -> Result<Reader, String> {
        #[cfg(target_os = "macos")]
        if self.prefer_hw {
            if let Ok(reader) = macos::spawn_file_reader(
                path,
                &self.info,
                start_index,
                self.preread,
                self.problem.clone(),
            ) {
                return Ok(reader);
            }
        }
        let mut command = ffmpeg_command().ok_or_else(missing_ffmpeg)?;
        command.args(["-v", "error"]);
        if self.max_decode_cpus > 0 {
            command.args(["-threads", &self.max_decode_cpus.to_string()]);
        } else {
            command.args(["-threads", "0"]);
        }
        if start_index > 0 {
            // Before -i, so ffmpeg seeks the container rather than decoding
            // and discarding everything up to the seek point.
            command.args(["-ss", &format!("{}", start_index as f64 / self.info.fps)]);
        }
        command
            .args(["-i", path])
            // A fixed output rate makes frame index and time the same fact,
            // which is what lets a seek be computed rather than searched for.
            .args(["-r", &format!("{}", self.info.fps)]);
        spawn_ffmpeg(
            command,
            self.info.width,
            self.info.height,
            start_index,
            self.problem.clone(),
            self.preread,
        )
    }

    pub(crate) fn route_incoming_frame(
        &mut self,
        wanted: i64,
        index: i64,
        frame: Arc<Frame>,
        pick: &mut Option<(i64, Arc<Frame>)>,
    ) -> bool {
        if self.snap_index {
            if index != wanted {
                return true;
            }
            *pick = Some((index, frame));
            return true;
        }
        if index <= wanted {
            if pick.as_ref().is_none_or(|(i, _)| index >= *i) {
                *pick = Some((index, frame));
            }
            return true;
        }
        if self.push_pending(index, frame) {
            return true;
        }
        false
    }

    pub(crate) fn drain_reader(&mut self, wanted: i64, pick: &mut Option<(i64, Arc<Frame>)>) {
        loop {
            if let Some((index, frame)) = self.blocked_frame.take() {
                if !self.route_incoming_frame(wanted, index, frame.clone(), pick) {
                    self.blocked_frame = Some((index, frame));
                    return;
                }
                continue;
            }
            let Some(reader) = &self.reader else {
                return;
            };
            match reader.frames.try_recv() {
                Ok((index, frame)) => {
                    if !self.route_incoming_frame(wanted, index, frame.clone(), pick) {
                        self.blocked_frame = Some((index, frame));
                        return;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.reader = None;
                    return;
                }
            }
        }
    }

    pub fn advance(&mut self, path: &str, wanted: i64) -> Option<&Arc<Frame>> {
        let wanted = wanted.max(0);
        let prev_wanted = self.last_wanted;
        self.last_wanted = wanted;

        if self.live {
            while let Some(reader) = &self.reader {
                match reader.frames.try_recv() {
                    Ok((index, frame)) => {
                        self.current = index;
                        self.frame = Some(frame);
                        self.awaiting_seek = false;
                        self.opened = None;
                        self.revision += 1;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.reader = None;
                        break;
                    }
                }
            }
            return self.frame.as_ref();
        }

        if self.info.fps > 0.0 {
            if self.snap_index {
                if wanted != prev_wanted {
                    self.restart_reader(path, wanted);
                }
            } else if self.needs_seek(wanted, prev_wanted) {
                self.restart_reader(path, wanted);
            }
        }
        if !self.snap_index {
            if self.info.fps > 0.0 && self.reader.is_none() && self.frame.is_none() {
                self.restart_reader(path, wanted);
            } else if self.reader.is_none() && self.movie_playing && wanted > self.current.max(0) {
                self.restart_reader(path, self.current.max(0));
            }
        } else if self.info.fps > 0.0 && self.reader.is_none() && self.frame.is_none() {
            self.restart_reader(path, wanted);
        }

        let mut pick = if self.snap_index {
            None
        } else {
            self.best_pending_upto(wanted)
        };
        if self.snap_index {
            pick = self
                .pending_frames
                .iter()
                .find(|(i, _)| *i == wanted)
                .map(|(i, f)| (*i, f.clone()));
            if pick.is_none() && self.current == wanted {
                if let Some(f) = self.frame.clone() {
                    pick = Some((self.current, f));
                }
            }
        } else if pick.is_none() && self.current >= 0 && self.current <= wanted {
            if let Some(f) = self.frame.clone() {
                pick = Some((self.current, f));
            }
        }

        self.drain_reader(wanted, &mut pick);

        if let Some((index, frame)) = pick {
            if self.current != index || !self.frame.as_ref().is_some_and(|f| Arc::ptr_eq(f, &frame))
            {
                if self.movie_playing && self.current >= 0 && index > self.current + 1 {
                    self.dropped_frames = self
                        .dropped_frames
                        .saturating_add((index - self.current - 1) as u64);
                }
                self.current = index;
                self.frame = Some(frame);
                self.awaiting_seek = false;
                self.opened = None;
                self.revision += 1;
                if self.info.width == 0 {
                    self.info.width = self.frame.as_ref().unwrap().width;
                    self.info.height = self.frame.as_ref().unwrap().height;
                }
            }
        }

        self.frame.as_ref()
    }

    /// Whether the decode has finished and nothing more will arrive.
    pub fn ended(&self) -> bool {
        self.reader.is_none()
    }

    pub fn frame(&self) -> Option<&Arc<Frame>> {
        self.frame.as_ref()
    }

    /// What went wrong on the decode thread, if anything did.
    ///
    /// ffmpeg's own words first — when it has any, they are better than
    /// anything guessed from the outside. A camera that was refused has none:
    /// see [`silent_camera`].
    pub fn problem(&self) -> Option<String> {
        let said = self.problem.lock().ok().and_then(|p| p.clone());
        said.or_else(|| self.silent_camera())
    }

    pub fn movie_decode_backend(&self) -> Option<MovieDecodeBackend> {
        if self.live {
            return None;
        }
        self.reader.as_ref()?.file_backend
    }

    /// A live source that has produced nothing for too long.
    ///
    /// This exists because the failure it names is otherwise invisible. On
    /// macOS a denied camera is not an error anywhere: ffmpeg starts, opens
    /// the AVFoundation session, writes nothing to stderr and never exits,
    /// and the node cooks a black texture every frame for as long as you care
    /// to watch it. Somebody looking at that has no way to learn that the
    /// answer is one checkbox in System Settings.
    ///
    /// Worded as a suspicion rather than a diagnosis. Access is only the
    /// likeliest cause — a camera held by another app, or one that has been
    /// unplugged since the mode probe, look identical from here — so it says
    /// what it observed first and what to try second.
    fn silent_camera(&self) -> Option<String> {
        let opened = self.opened?;
        if !self.live || opened.elapsed() < FIRST_FRAME_GRACE {
            return None;
        }
        Some(if cfg!(target_os = "macos") {
            "the camera has not produced a frame — check System Settings ▸ \
             Privacy & Security ▸ Camera, and that nothing else is using it"
                .to_string()
        } else {
            "the camera has not produced a frame — check that it is connected \
             and that nothing else is using it"
                .to_string()
        })
    }
}
