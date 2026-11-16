use std::sync::{Arc, Mutex};

use otd_core::{CookContext, MoviePcm};

use crate::data::{Channel, ChopData, slice_len};

use super::speakers::{channel_names, write_speakers};

/// A jump larger than this is a seek (cue, scrub). A video frame step is not.
const SEEK_SECONDS: f64 = 0.15;
const SEEK_FADE: u32 = 128;

pub(crate) struct AudioMovieState {
    pub(crate) file: String,
    /// Identity of the soundtrack buffer, so a finished decode replaces a pending one.
    pub(crate) pcm_id: usize,
    pub transport: Arc<Mutex<MovieTransport>>,
}

pub(crate) struct MovieTransport {
    pub pcm: Arc<MoviePcm>,
    channels: usize,
    rate: f64,
    /// Next source frame to read.
    pos: f64,
    speed: f64,
    playing: bool,
    volume: f32,
    last_media: f64,
    started: bool,
    /// Playhead is past the samples read so far. Snap once they arrive.
    waiting_on_decode: bool,
    fade: u32,
    fade_from: Vec<f32>,
    last_out: Vec<f32>,
    scratch: Vec<f32>,
    /// Audiodeviceout is pulling. The cook must not also advance `pos`.
    device_driven: bool,
}

impl MovieTransport {
    pub(crate) fn from_pcm(pcm: Arc<MoviePcm>) -> Self {
        MovieTransport {
            channels: pcm.channels.max(1),
            rate: pcm.sample_rate.max(1.0),
            pcm,
            pos: 0.0,
            speed: 1.0,
            playing: false,
            volume: 1.0,
            last_media: 0.0,
            started: false,
            waiting_on_decode: false,
            fade: 0,
            fade_from: Vec::new(),
            last_out: Vec::new(),
            scratch: Vec::new(),
            device_driven: false,
        }
    }

    fn ensure_bufs(&mut self) {
        let ch = self.channels.max(1);
        if self.scratch.len() != ch {
            self.scratch.resize(ch, 0.0);
            self.fade_from.resize(ch, 0.0);
            self.last_out.resize(ch, 0.0);
        }
    }

    fn frame_count(&self) -> usize {
        self.pcm.frames()
    }

    /// Picture transport for this cook. Does not walk the cursor unless no
    /// device is attached (then the CHOP itself is the consumer).
    pub(crate) fn sync(&mut self, media_sec: f64, speed: f64, movie_playing: bool, volume: f32) {
        let frames = self.frame_count() as f64;
        let end = (frames - 1.0).max(0.0);
        let want = (media_sec * self.rate).max(0.0);
        let target = want.min(end);
        let pos_sec = self.pos / self.rate;
        let media_step = (media_sec - self.last_media).abs();
        let held = self.started && media_step < 1e-4;
        let jumped =
            self.started && media_step > SEEK_SECONDS && (media_sec - pos_sec).abs() > SEEK_SECONDS;
        let caught_up = self.waiting_on_decode && want <= end;
        if !self.started || held || jumped || caught_up || !movie_playing {
            if self.started && (target - self.pos).abs() > 2.0 {
                self.ensure_bufs();
                self.fade = SEEK_FADE;
                self.fade_from.clone_from(&self.last_out);
            }
            self.pos = target;
        }
        self.waiting_on_decode = want > end + 1.0;
        self.playing = movie_playing && !held && speed.abs() > 1e-6;
        self.speed = speed;
        self.volume = volume;
        self.last_media = media_sec;
        self.started = true;
    }

    pub(crate) fn set_device_driven(&mut self, driven: bool) {
        self.device_driven = driven;
    }

    pub(crate) fn channel_count(&self) -> usize {
        self.channels.max(1)
    }

    /// Fill the hardware buffer.
    ///
    /// When the device has at least as many channels as the file, each source
    /// channel goes to the matching speaker (left, right, center, LFE, back,
    /// side). A stereo device playing 5.1 or 7.1 folds the extra channels into
    /// left and right so the center and surrounds are still heard.
    pub(crate) fn render(&mut self, device_rate: f64, out: &mut [f32], hw_channels: usize) {
        let hw = hw_channels.max(1);
        self.ensure_bufs();
        for frame in out.chunks_mut(hw) {
            self.read_frame();
            self.apply_fade();
            for s in self.scratch.iter_mut() {
                *s *= self.volume;
            }
            write_speakers(&self.scratch, frame);
            self.advance(device_rate);
        }
    }

    fn read_frame(&mut self) {
        self.pcm.read_frame(self.pos, &mut self.scratch);
    }

    fn apply_fade(&mut self) {
        if self.fade == 0 {
            self.last_out.copy_from_slice(&self.scratch);
            return;
        }
        let t = 1.0 - self.fade as f32 / SEEK_FADE as f32;
        self.fade -= 1;
        for (sample, from) in self.scratch.iter_mut().zip(self.fade_from.iter()) {
            *sample = *from + (*sample - *from) * t;
        }
        self.last_out.copy_from_slice(&self.scratch);
    }

    fn advance(&mut self, consume_rate: f64) {
        if self.playing && consume_rate > 0.0 {
            let step = self.speed * self.rate / consume_rate;
            let end = (self.frame_count() as f64 - 1.0).max(0.0);
            self.pos = (self.pos + step).clamp(0.0, end);
        }
    }

    /// Viewer / downstream CHOP slice. One channel per soundtrack channel.
    /// Advances only when no device is pulling.
    pub(crate) fn slice(&mut self, ctx: &CookContext) -> ChopData {
        let n = slice_len(self.rate, ctx);
        let ch = self.channels.max(1);
        self.ensure_bufs();
        let saved_pos = self.pos;
        let saved_fade = self.fade;
        let saved_playing = self.playing;
        let saved_from = self.fade_from.clone();
        let saved_last = self.last_out.clone();
        if self.device_driven {
            self.playing = false;
        }
        let mut cols = vec![Vec::with_capacity(n); ch];
        for _ in 0..n {
            self.read_frame();
            self.apply_fade();
            for (col, sample) in cols.iter_mut().zip(self.scratch.iter()) {
                col.push(*sample * self.volume);
            }
            self.advance(self.rate);
        }
        if self.device_driven {
            self.pos = saved_pos;
            self.fade = saved_fade;
            self.playing = saved_playing;
            self.fade_from = saved_from;
            self.last_out = saved_last;
        }
        let names = channel_names(ch);
        let channels = cols
            .into_iter()
            .enumerate()
            .map(|(i, samples)| Channel::new(&names[i], samples))
            .collect();
        ChopData::new(channels, self.rate, true)
    }
}
