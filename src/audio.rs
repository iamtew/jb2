//! Windows loopback capture and a smoothed analysis frame for the overlay.
//!
//! The WASAPI callback only copies samples. FFT runs on this thread.
//! ponytail: one FFT for every listener. A second profile is another analyzer, not another capture.
//! ponytail: SSE, not WebSocket. tiny_http only hands back a writer, so it cannot answer client pings.

use crate::config::AudioVisualizer;
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const WINDOW: usize = 1024;
const TICK_HZ: f32 = 60.0;
const RING_CAP: usize = 131072;
const FLOOR_DB: f32 = -60.0;
const MAX_LISTENERS: usize = 8;
const SPEC_LO_HZ: f32 = 20.0;
const SPEC_HI_HZ: f32 = 16000.0;

struct Ring {
    buf: Box<[AtomicU32]>,
    /// Samples written. The callback is the only writer.
    w: AtomicU64,
}

impl Ring {
    fn new() -> Self {
        let mut buf = Vec::with_capacity(RING_CAP);
        for _ in 0..RING_CAP {
            buf.push(AtomicU32::new(0));
        }
        Self {
            buf: buf.into_boxed_slice(),
            w: AtomicU64::new(0),
        }
    }

    fn reset(&self) {
        self.w.store(0, Ordering::Release);
    }

    fn push(&self, samples: &[f32]) {
        let mut w = self.w.load(Ordering::Relaxed);
        for &sample in samples {
            let s = if sample.is_finite() { sample } else { 0.0 };
            let i = (w as usize) & (RING_CAP - 1);
            self.buf[i].store(s.to_bits(), Ordering::Relaxed);
            w = w.wrapping_add(1);
        }
        self.w.store(w, Ordering::Release);
    }

    /// Last `n` samples into `out`. Slots are atomics, so a lap during the copy yields a newer sample, not a tear.
    fn copy_latest(&self, n: usize, out: &mut Vec<f32>) -> bool {
        out.clear();
        if n == 0 || n >= RING_CAP {
            return false;
        }
        let w = self.w.load(Ordering::Acquire);
        if w < n as u64 {
            return false;
        }
        let start = w - n as u64;
        out.reserve(n);
        for i in 0..n {
            let idx = (start + i as u64) as usize & (RING_CAP - 1);
            out.push(f32::from_bits(self.buf[idx].load(Ordering::Relaxed)));
        }
        true
    }
}

struct BandMap {
    spectrum: Vec<(usize, usize)>,
    bass: (usize, usize),
    mid: (usize, usize),
    treble: (usize, usize),
}

pub struct Analyzer {
    rate: u32,
    band_count: usize,
    smoothing: f32,
    hz: f32,
    bass_hz: [i32; 2],
    mid_hz: [i32; 2],
    treble_hz: [i32; 2],
    window: Vec<f32>,
    window_sum: f32,
    fft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    map: BandMap,
    spectrum: Vec<f32>,
    volume: f32,
    left: f32,
    right: f32,
    bass: f32,
    mid: f32,
    treble: f32,
    beat_strength: f32,
    bass_env: f32,
    cool: u32,
    /// Slow ceiling so a quiet song can still fill 0..1 after a loud one.
    peak: f32,
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub volume: f32,
    pub left: Option<f32>,
    pub right: Option<f32>,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub spectrum: Vec<f32>,
    pub beat: bool,
    pub beat_strength: f32,
}

impl Analyzer {
    pub fn new(rate: u32, viz: &AudioVisualizer) -> Self {
        let mut analyzer = Self {
            rate: 0,
            band_count: 0,
            smoothing: viz.smoothing,
            hz: TICK_HZ,
            bass_hz: viz.bass_hz,
            mid_hz: viz.mid_hz,
            treble_hz: viz.treble_hz,
            window: Vec::new(),
            window_sum: 1.0,
            fft: FftPlanner::<f32>::new().plan_fft_forward(WINDOW),
            scratch: vec![Complex::new(0.0, 0.0); WINDOW],
            map: BandMap {
                spectrum: Vec::new(),
                bass: (1, 2),
                mid: (1, 2),
                treble: (1, 2),
            },
            spectrum: Vec::new(),
            volume: 0.0,
            left: 0.0,
            right: 0.0,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            beat_strength: 0.0,
            bass_env: 0.0,
            cool: 0,
            peak: 0.0,
        };
        analyzer.retune(rate, viz);
        analyzer
    }

    pub fn retune(&mut self, rate: u32, viz: &AudioVisualizer) {
        let rate = rate.max(8000);
        let bands = viz.spectrum_bands.clamp(1, 32) as usize;
        let edges_changed = self.bass_hz != viz.bass_hz
            || self.mid_hz != viz.mid_hz
            || self.treble_hz != viz.treble_hz;
        self.smoothing = viz.smoothing;
        self.hz = TICK_HZ;
        self.bass_hz = viz.bass_hz;
        self.mid_hz = viz.mid_hz;
        self.treble_hz = viz.treble_hz;
        if self.rate == rate
            && self.band_count == bands
            && !edges_changed
            && !self.window.is_empty()
        {
            return;
        }
        self.rate = rate;
        self.band_count = bands;
        self.window = hann(WINDOW);
        self.window_sum = self.window.iter().sum::<f32>().max(1.0);
        self.fft = FftPlanner::<f32>::new().plan_fft_forward(WINDOW);
        self.scratch = vec![Complex::new(0.0, 0.0); WINDOW];
        self.map = band_map(rate, bands, viz.bass_hz, viz.mid_hz, viz.treble_hz);
        self.spectrum = vec![0.0; bands];
    }

    pub fn push(&mut self, interleaved: &[f32], channels: usize) -> Option<Frame> {
        let ch = channels.max(1);
        let frames = interleaved.len() / ch;
        if frames < WINDOW {
            return None;
        }
        let start = frames - WINDOW;
        let mut mono = [0.0_f32; WINDOW];
        let mut acc = 0.0_f32;
        let mut acc_l = 0.0_f32;
        let mut acc_r = 0.0_f32;
        for i in 0..WINDOW {
            let base = (start + i) * ch;
            let mut sum = 0.0_f32;
            for c in 0..ch {
                sum += interleaved[base + c];
            }
            let m = sum / ch as f32;
            mono[i] = m;
            acc += m * m;
            if ch >= 2 {
                let l = interleaved[base];
                let r = interleaved[base + 1];
                acc_l += l * l;
                acc_r += r * r;
            }
        }
        for i in 0..WINDOW {
            self.scratch[i] = Complex::new(mono[i] * self.window[i], 0.0);
        }
        self.fft.process(&mut self.scratch);

        let volume_db = rms_db(acc / WINDOW as f32);
        let bass_db = band_db(&self.scratch, self.map.bass, self.window_sum);
        let mid_db = band_db(&self.scratch, self.map.mid, self.window_sum);
        let treble_db = band_db(&self.scratch, self.map.treble, self.window_sum);
        let mut spec_db = Vec::with_capacity(self.map.spectrum.len());
        let mut raw_peak = volume_db.max(bass_db).max(mid_db).max(treble_db);
        for range in &self.map.spectrum {
            let db = band_db(&self.scratch, *range, self.window_sum);
            raw_peak = raw_peak.max(db);
            spec_db.push(db);
        }
        self.follow_peak(raw_peak);

        let release = smooth_alpha(self.smoothing);
        let vol_release = (release * 1.35).min(1.0);
        self.volume = follow(self.volume, unit(volume_db, self.peak), vol_release);
        self.bass = follow(self.bass, unit(bass_db, self.peak), release);
        self.mid = follow(self.mid, unit(mid_db, self.peak), release);
        self.treble = follow(self.treble, unit(treble_db, self.peak), release);
        for (slot, db) in self.spectrum.iter_mut().zip(spec_db.iter()) {
            *slot = follow(*slot, unit(*db, self.peak), release);
        }

        let (left, right) = if ch >= 2 {
            self.left = follow(
                self.left,
                unit(rms_db(acc_l / WINDOW as f32), self.peak),
                vol_release,
            );
            self.right = follow(
                self.right,
                unit(rms_db(acc_r / WINDOW as f32), self.peak),
                vol_release,
            );
            (Some(q(self.left)), Some(q(self.right)))
        } else {
            (None, None)
        };

        let raw_bass = unit(bass_db, self.peak);
        let onset = raw_bass > self.bass_env * 1.35 && raw_bass > 0.45 && self.cool == 0;
        let strength_now = if onset {
            ((raw_bass - self.bass_env) / self.bass_env.max(0.05)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if onset {
            self.cool = (self.hz * 0.25).round().max(1.0) as u32;
        } else if self.cool > 0 {
            self.cool -= 1;
        }
        self.bass_env = ema(self.bass_env, raw_bass, 0.12);
        self.beat_strength = follow(self.beat_strength, strength_now, release);

        Some(Frame {
            volume: q(self.volume),
            left,
            right,
            bass: q(self.bass),
            mid: q(self.mid),
            treble: q(self.treble),
            spectrum: self.spectrum.iter().copied().map(q).collect(),
            beat: onset,
            beat_strength: q(self.beat_strength),
        })
    }

    fn follow_peak(&mut self, raw: f32) {
        let drop = 48.0 / self.hz.max(1.0);
        if raw > self.peak {
            self.peak = raw;
        } else {
            self.peak = (self.peak - drop).max(raw);
        }
        self.peak = self.peak.clamp(-36.0, 3.0);
    }
}

fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let phase = 2.0 * std::f32::consts::PI * i as f32 / (n.saturating_sub(1) as f32);
            0.5 * (1.0 - phase.cos())
        })
        .collect()
}

fn smooth_alpha(smoothing: f32) -> f32 {
    let s = if smoothing.is_finite() {
        smoothing.clamp(0.0, 1.0)
    } else {
        0.35
    };
    1.0 - s * 0.95
}

fn ema(prev: f32, sample: f32, alpha: f32) -> f32 {
    prev + alpha * (sample - prev)
}

/// Rise copies the sample. `release_alpha` only eases the fall (`smoothing` is that fall).
fn follow(prev: f32, sample: f32, release_alpha: f32) -> f32 {
    if sample > prev {
        sample
    } else {
        ema(prev, sample, release_alpha)
    }
}

fn rms_db(mean_square: f32) -> f32 {
    20.0 * mean_square.max(1e-12).sqrt().log10()
}

fn unit(db: f32, peak: f32) -> f32 {
    let ceil = peak.max(FLOOR_DB + 12.0);
    ((db - FLOOR_DB) / (ceil - FLOOR_DB)).clamp(0.0, 1.0)
}

fn q(x: f32) -> f32 {
    let x = if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    };
    (x * 10000.0).round() / 10000.0
}

fn band_db(scratch: &[Complex<f32>], range: (usize, usize), window_sum: f32) -> f32 {
    let (a, b) = range;
    if b <= a {
        return FLOOR_DB;
    }
    let mut power = 0.0_f32;
    for bin in &scratch[a..b] {
        let amp = bin.norm() * 2.0 / window_sum;
        power += amp * amp;
    }
    10.0 * power.max(1e-12).log10()
}

fn hz_to_bin(hz: f32, rate: f32, n: usize) -> usize {
    let nyquist = n / 2;
    let bin = (hz * n as f32 / rate).round() as isize;
    bin.clamp(1, nyquist as isize - 1) as usize
}

fn band_map(rate: u32, bands: usize, bass: [i32; 2], mid: [i32; 2], treble: [i32; 2]) -> BandMap {
    let rate_f = rate as f32;
    let hi = SPEC_HI_HZ.min(rate_f * 0.45);
    let log_lo = SPEC_LO_HZ.ln();
    let log_hi = hi.max(SPEC_LO_HZ + 1.0).ln();
    let mut spectrum = Vec::with_capacity(bands);
    for i in 0..bands {
        let a = (log_lo + (log_hi - log_lo) * (i as f32 / bands as f32)).exp();
        let b = (log_lo + (log_hi - log_lo) * ((i + 1) as f32 / bands as f32)).exp();
        spectrum.push(bin_span(a, b, rate_f));
    }
    BandMap {
        spectrum,
        bass: bin_span(bass[0] as f32, bass[1] as f32, rate_f),
        mid: bin_span(mid[0] as f32, mid[1] as f32, rate_f),
        treble: bin_span(treble[0] as f32, treble[1] as f32, rate_f),
    }
}

fn bin_span(lo_hz: f32, hi_hz: f32, rate: f32) -> (usize, usize) {
    let a = hz_to_bin(lo_hz, rate, WINDOW);
    let b = hz_to_bin(hi_hz, rate, WINDOW).max(a + 1);
    (a, b.min(WINDOW / 2))
}

#[cfg(test)]
fn spectrum_band(freq: f32, bands: usize) -> usize {
    let hi = SPEC_HI_HZ;
    let t = (freq.clamp(SPEC_LO_HZ, hi).ln() - SPEC_LO_HZ.ln()) / (hi.ln() - SPEC_LO_HZ.ln());
    ((t * bands as f32) as usize).min(bands - 1)
}

fn frame_value(frame: &Frame) -> Value {
    let mut v = json!({
        "type": "audio",
        "ts": now_ms(),
        "volume": frame.volume,
        "bass": frame.bass,
        "mid": frame.mid,
        "treble": frame.treble,
        "spectrum": frame.spectrum,
        "beat": frame.beat,
        "beatStrength": frame.beat_strength,
    });
    if let (Some(left), Some(right)) = (frame.left, frame.right) {
        v["volumeLeft"] = json!(left);
        v["volumeRight"] = json!(right);
    }
    v
}

fn status_value(state: &str, message: &str, device: &str) -> Value {
    json!({
        "type": "status",
        "state": state,
        "message": message,
        "device": device,
    })
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

struct Snap {
    gen: u64,
    value: Value,
}

struct Shared {
    snap: Mutex<Snap>,
    frame_cv: Condvar,
    settings: Mutex<AudioVisualizer>,
    ctrl: Condvar,
    listeners: Arc<AtomicUsize>,
}

pub struct ListenGuard {
    listeners: Arc<AtomicUsize>,
}

impl Drop for ListenGuard {
    fn drop(&mut self) {
        self.listeners.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
pub struct Hub {
    shared: Arc<Shared>,
}

impl Hub {
    pub fn start(initial: AudioVisualizer) -> Self {
        let shared = Arc::new(Shared {
            snap: Mutex::new(Snap {
                gen: 0,
                value: status_value("silent", "audio visualizer disabled", ""),
            }),
            frame_cv: Condvar::new(),
            settings: Mutex::new(initial),
            ctrl: Condvar::new(),
            listeners: Arc::new(AtomicUsize::new(0)),
        });
        let thread_shared = Arc::clone(&shared);
        let _: JoinHandle<()> = std::thread::Builder::new()
            .name("jb2-audio".into())
            .spawn(move || run(thread_shared))
            .expect("audio thread");
        Self { shared }
    }

    pub fn apply(&self, next: AudioVisualizer) {
        *self.shared.settings.lock().expect("audio settings") = next;
        self.shared.ctrl.notify_all();
    }

    pub fn snapshot(&self) -> Value {
        self.shared.snap.lock().expect("audio snap").value.clone()
    }

    pub fn try_listen(&self) -> Option<ListenGuard> {
        let n = self.shared.listeners.fetch_add(1, Ordering::AcqRel);
        if n >= MAX_LISTENERS {
            self.shared.listeners.fetch_sub(1, Ordering::AcqRel);
            return None;
        }
        Some(ListenGuard {
            listeners: Arc::clone(&self.shared.listeners),
        })
    }

    fn wait_frame(&self, timeout: Duration) -> (u64, Value, bool) {
        let guard = self.shared.snap.lock().expect("audio snap");
        let (guard, result) = self
            .shared
            .frame_cv
            .wait_timeout(guard, timeout)
            .expect("audio snap");
        (guard.gen, guard.value.clone(), result.timed_out())
    }
}

fn publish(shared: &Shared, value: Value) {
    let mut snap = shared.snap.lock().expect("audio snap");
    snap.gen = snap.gen.wrapping_add(1);
    snap.value = value;
    shared.frame_cv.notify_all();
}

fn run(shared: Arc<Shared>) {
    #[cfg(windows)]
    capture_loop(shared);
    #[cfg(not(windows))]
    unsupported_loop(shared);
}

#[cfg(not(windows))]
fn unsupported_loop(shared: Arc<Shared>) {
    loop {
        let settings = shared.settings.lock().expect("audio settings").clone();
        if settings.enabled {
            publish(
                &shared,
                status_value("error", "audio capture only on windows", ""),
            );
        } else {
            publish(
                &shared,
                status_value("silent", "audio visualizer disabled", ""),
            );
        }
        let guard = shared.settings.lock().expect("audio settings");
        let _ = shared
            .ctrl
            .wait_timeout(guard, Duration::from_secs(2))
            .expect("audio settings");
    }
}

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(period: u32) -> u32;
}

#[cfg(windows)]
fn capture_loop(shared: Arc<Shared>) {
    // ponytail: poll the default output name every 2s. A named device stays open until the stream errors; listing every output stalls the tick.
    // ponytail: Windows sleeps in 15.6ms slices unless this process asks for 1ms. The 60Hz tick needs it.
    unsafe { timeBeginPeriod(1) };
    let host = cpal::default_host();
    let ring = Arc::new(Ring::new());
    let mut live: Option<LiveStream> = None;
    let mut analyzer: Option<Analyzer> = None;
    let mut last_poll = Instant::now();
    let mut logged = String::new();
    let mut samples = Vec::new();

    loop {
        let tick = Instant::now();
        let settings = shared.settings.lock().expect("audio settings").clone();
        if !settings.enabled {
            live = None;
            analyzer = None;
            ring.reset();
            note(
                &shared,
                &mut logged,
                status_value("silent", "audio visualizer disabled", ""),
            );
            let guard = shared.settings.lock().expect("audio settings");
            let _ = shared
                .ctrl
                .wait_timeout(guard, Duration::from_secs(2))
                .expect("audio settings");
            continue;
        }

        let mut drop_reason: Option<Value> = None;
        if let Some(open) = live.as_ref() {
            let broken = open.device_key != settings.device || open.err.load(Ordering::Acquire);
            if broken {
                drop_reason = Some(status_value(
                    "device_changed",
                    "capture stopped",
                    &open.name,
                ));
            } else if settings.device.trim().eq_ignore_ascii_case("default")
                && last_poll.elapsed() >= Duration::from_secs(2)
            {
                last_poll = Instant::now();
                match target_name(&host, &settings.device) {
                    Ok(name) if name != open.name => {
                        drop_reason = Some(status_value(
                            "device_changed",
                            "output device changed",
                            &name,
                        ));
                    }
                    Err(err) => {
                        drop_reason = Some(status_value("error", &err, &open.name));
                    }
                    Ok(_) => {}
                }
            }
        }
        if drop_reason.is_some() {
            live = None;
        }
        if let Some(value) = drop_reason {
            note(&shared, &mut logged, value);
        }

        if live.is_none() {
            ring.reset();
            match open_stream(&host, &settings, &ring) {
                Ok(opened) => {
                    let mut next = analyzer
                        .take()
                        .unwrap_or_else(|| Analyzer::new(opened.rate, &settings));
                    next.retune(opened.rate, &settings);
                    analyzer = Some(next);
                    note(
                        &shared,
                        &mut logged,
                        status_value("capturing", "", &opened.name),
                    );
                    live = Some(opened);
                    last_poll = Instant::now();
                }
                Err(err) => {
                    analyzer = None;
                    note(
                        &shared,
                        &mut logged,
                        status_value("error", &err, &settings.device),
                    );
                    let guard = shared.settings.lock().expect("audio settings");
                    let _ = shared
                        .ctrl
                        .wait_timeout(guard, Duration::from_secs(2))
                        .expect("audio settings");
                    continue;
                }
            }
        }

        if let (Some(open), Some(engine)) = (live.as_ref(), analyzer.as_mut()) {
            engine.retune(open.rate, &settings);
            let need = WINDOW * open.channels;
            if open.ring.copy_latest(need, &mut samples) {
                if let Some(frame) = engine.push(&samples, open.channels) {
                    publish(&shared, frame_value(&frame));
                }
            }
        }

        let frame = Duration::from_secs(1) / TICK_HZ as u32;
        let period = frame.saturating_sub(tick.elapsed());
        let guard = shared.settings.lock().expect("audio settings");
        let _ = shared
            .ctrl
            .wait_timeout(guard, period)
            .expect("audio settings");
    }
}

fn note(shared: &Shared, logged: &mut String, value: Value) {
    let line = format!(
        "{} {}",
        value.get("state").and_then(|v| v.as_str()).unwrap_or(""),
        value.get("message").and_then(|v| v.as_str()).unwrap_or("")
    );
    if logged.as_str() != line {
        *logged = line;
        let state = value.get("state").and_then(|v| v.as_str()).unwrap_or("");
        let message = value.get("message").and_then(|v| v.as_str()).unwrap_or("");
        if !message.is_empty() || state == "error" || state == "device_changed" {
            crate::log(format!("audio: {state} {message}"));
        }
    }
    publish(shared, value);
}

#[cfg(windows)]
fn open_stream(
    host: &cpal::Host,
    settings: &AudioVisualizer,
    ring: &Arc<Ring>,
) -> Result<LiveStream, String> {
    use cpal::traits::{DeviceTrait, StreamTrait};
    use cpal::SampleFormat;

    let (device, name) = pick_device(host, &settings.device)?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("output format: {e}"))?;
    let channels = supported.channels() as usize;
    if channels == 0 {
        return Err("output device has no channels".into());
    }
    let rate = supported.sample_rate().0;
    let config = supported.config();
    let err = Arc::new(AtomicBool::new(false));
    let held = Arc::clone(ring);
    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            let ring = Arc::clone(&held);
            let err_flag = Arc::clone(&err);
            device.build_input_stream(
                &config,
                move |data: &[f32], _| ring.push(data),
                move |e| flag_err(&err_flag, e),
                None,
            )
        }
        SampleFormat::I16 => {
            let ring = Arc::clone(&held);
            let err_flag = Arc::clone(&err);
            device.build_input_stream(
                &config,
                move |data: &[i16], _| push_i16(&ring, data),
                move |e| flag_err(&err_flag, e),
                None,
            )
        }
        SampleFormat::U16 => {
            let ring = Arc::clone(&held);
            let err_flag = Arc::clone(&err);
            device.build_input_stream(
                &config,
                move |data: &[u16], _| push_u16(&ring, data),
                move |e| flag_err(&err_flag, e),
                None,
            )
        }
        SampleFormat::I32 => {
            let ring = Arc::clone(&held);
            let err_flag = Arc::clone(&err);
            device.build_input_stream(
                &config,
                move |data: &[i32], _| push_i32(&ring, data),
                move |e| flag_err(&err_flag, e),
                None,
            )
        }
        other => return Err(format!("unsupported sample format {other:?}")),
    }
    .map_err(|e| format!("loopback open: {e}"))?;
    // cpal turns an input stream on an output device into WASAPI shared-mode loopback.
    stream.play().map_err(|e| format!("loopback start: {e}"))?;
    Ok(LiveStream {
        stream,
        ring: held,
        channels,
        rate,
        name,
        err,
        device_key: settings.device.clone(),
    })
}

#[cfg(windows)]
fn flag_err(flag: &AtomicBool, err: cpal::StreamError) {
    crate::log(format!("audio: {err}"));
    flag.store(true, Ordering::Release);
}

#[cfg(windows)]
struct LiveStream {
    /// Dropping this stops the callback. The loop never reads it.
    #[allow(dead_code)]
    stream: cpal::Stream,
    ring: Arc<Ring>,
    channels: usize,
    rate: u32,
    name: String,
    err: Arc<AtomicBool>,
    device_key: String,
}

#[cfg(windows)]
fn push_i16(ring: &Ring, data: &[i16]) {
    let mut tmp = [0.0_f32; 512];
    for chunk in data.chunks(512) {
        for (i, sample) in chunk.iter().enumerate() {
            tmp[i] = *sample as f32 / i16::MAX as f32;
        }
        ring.push(&tmp[..chunk.len()]);
    }
}

#[cfg(windows)]
fn push_u16(ring: &Ring, data: &[u16]) {
    let mut tmp = [0.0_f32; 512];
    for chunk in data.chunks(512) {
        for (i, sample) in chunk.iter().enumerate() {
            tmp[i] = (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0;
        }
        ring.push(&tmp[..chunk.len()]);
    }
}

#[cfg(windows)]
fn push_i32(ring: &Ring, data: &[i32]) {
    let mut tmp = [0.0_f32; 512];
    for chunk in data.chunks(512) {
        for (i, sample) in chunk.iter().enumerate() {
            tmp[i] = *sample as f32 / i32::MAX as f32;
        }
        ring.push(&tmp[..chunk.len()]);
    }
}

/// Output endpoints Stage can target. `default` marks the current default output.
pub fn list_output_devices() -> Value {
    #[cfg(windows)]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let default_name = host
            .default_output_device()
            .and_then(|device| device.name().ok())
            .unwrap_or_default();
        let mut devices = Vec::new();
        if let Ok(list) = host.output_devices() {
            for device in list {
                let Ok(name) = device.name() else { continue };
                if name.trim().is_empty() {
                    continue;
                }
                let is_default = name == default_name;
                devices.push(json!({ "name": name, "default": is_default }));
            }
        }
        return json!({ "devices": devices });
    }
    #[cfg(not(windows))]
    {
        json!({ "devices": [] })
    }
}

#[cfg(windows)]
fn pick_device(host: &cpal::Host, want: &str) -> Result<(cpal::Device, String), String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let want = want.trim();
    if want.is_empty() || want.eq_ignore_ascii_case("default") {
        let device = host
            .default_output_device()
            .ok_or_else(|| "no default output device".to_string())?;
        let name = device.name().unwrap_or_else(|_| "default".into());
        return Ok((device, name));
    }
    let devices = host
        .output_devices()
        .map_err(|e| format!("output devices: {e}"))?;
    for device in devices {
        let name = device.name().unwrap_or_default();
        if name.eq_ignore_ascii_case(want) {
            return Ok((device, name));
        }
    }
    Err(format!("output device not found: {want}"))
}

#[cfg(windows)]
fn target_name(host: &cpal::Host, want: &str) -> Result<String, String> {
    pick_device(host, want).map(|(_, name)| name)
}

/// Blocks the SSE handler. Wakes on each new frame, and pings when the frame is idle.
pub fn write_sse<W: std::io::Write>(hub: &Hub, writer: &mut W) -> std::io::Result<()> {
    writer.write_all(
        b"HTTP/1.1 200 OK\r\n\
Content-Type: text/event-stream\r\n\
Cache-Control: no-cache\r\n\
Connection: keep-alive\r\n\
Transfer-Encoding: chunked\r\n\
\r\n",
    )?;
    sse_chunk(writer, b": connected\n\n")?;
    let (mut last_gen, value, _) = hub.wait_frame(Duration::from_millis(0));
    sse_chunk(writer, sse_event(&value).as_bytes())?;
    let mut last_ping = Instant::now();
    loop {
        let (gen, value, timed_out) = hub.wait_frame(Duration::from_secs(15));
        if gen != last_gen {
            last_gen = gen;
            sse_chunk(writer, sse_event(&value).as_bytes())?;
            last_ping = Instant::now();
        } else if timed_out || last_ping.elapsed() >= Duration::from_secs(15) {
            sse_chunk(writer, b": ping\n\n")?;
            last_ping = Instant::now();
        }
    }
}

fn sse_event(value: &Value) -> String {
    let mut s = format!("event: audio\ndata: {value}\n:\n");
    // ponytail: tiny_http has no set_nodelay. Windows delayed-ACK holds a short
    // write ~200ms. Pad to one full segment so the frame leaves now.
    if s.len() < 1600 {
        let extra = 1600 - s.len();
        let at = s.len() - 1;
        s.insert_str(at, &" ".repeat(extra));
    }
    s.push('\n');
    s
}

fn sse_chunk<W: std::io::Write>(writer: &mut W, data: &[u8]) -> std::io::Result<()> {
    write!(writer, "{:x}\r\n", data.len())?;
    writer.write_all(data)?;
    writer.write_all(b"\r\n")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, rate: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate).sin() * amp)
            .collect()
    }

    #[test]
    fn sine_lands_in_one_band() {
        let rate = 48000.0;
        let bands = 16;
        let viz = AudioVisualizer {
            smoothing: 0.0,
            update_hz: 30,
            ..AudioVisualizer::default()
        };
        let mut analyzer = Analyzer::new(rate as u32, &viz);
        let center = {
            let hi = SPEC_HI_HZ;
            let lo_l = SPEC_LO_HZ.ln();
            let hi_l = hi.ln();
            let mid = lo_l + (hi_l - lo_l) * (8.5 / bands as f32);
            mid.exp()
        };
        let bin = (center * WINDOW as f32 / rate).round();
        let snapped = bin * rate / WINDOW as f32;
        let expect = spectrum_band(snapped, bands);
        let frame = analyzer
            .push(&tone(snapped, rate, WINDOW, 0.9), 1)
            .expect("frame");
        assert!(frame.spectrum.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!((0.0..=1.0).contains(&frame.volume));
        assert!(frame.bass <= 1.0 && frame.mid <= 1.0 && frame.treble <= 1.0);
        let hot = frame
            .spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(hot, expect, "spectrum {:?}", frame.spectrum);
        assert!(frame.spectrum[expect] > 0.5, "{:?}", frame.spectrum);
        for (i, value) in frame.spectrum.iter().enumerate() {
            if i.abs_diff(expect) > 2 {
                assert!(*value < 0.25, "band {i} = {value}");
            }
        }

        let mut quiet = Analyzer::new(rate as u32, &viz);
        let zeros = vec![0.0_f32; WINDOW];
        let silent = quiet.push(&zeros, 1).unwrap();
        assert!(silent.spectrum.iter().all(|v| *v < 0.05));
        assert!(silent.volume < 0.05);

        let heavy = AudioVisualizer {
            smoothing: 0.95,
            ..viz
        };
        let mut slow = Analyzer::new(rate as u32, &heavy);
        let held = slow.push(&zeros, 1).unwrap();
        let jumped = slow.push(&tone(snapped, rate, WINDOW, 0.9), 1).unwrap();
        assert!(
            jumped.spectrum[expect] > held.spectrum[expect] + 0.4,
            "attack {} -> {}",
            held.spectrum[expect],
            jumped.spectrum[expect]
        );
    }

    #[test]
    fn sse_event_fills_a_segment() {
        let event = sse_event(&json!({"type": "audio", "spectrum": [0.1, 0.2]}));
        assert!(event.len() >= 1600, "{}", event.len());
        assert!(event.ends_with("\n\n"));
    }

    #[test]
    fn bass_jump_sets_beat() {
        let viz = AudioVisualizer {
            smoothing: 0.0,
            update_hz: 30,
            ..AudioVisualizer::default()
        };
        let mut analyzer = Analyzer::new(48000, &viz);
        let zeros = vec![0.0_f32; WINDOW];
        for _ in 0..8 {
            let frame = analyzer.push(&zeros, 1).unwrap();
            assert!(!frame.beat);
        }
        let kick = tone(80.0, 48000.0, WINDOW, 0.95);
        let frame = analyzer.push(&kick, 1).unwrap();
        assert!(frame.beat, "bass {} env", frame.bass);
        assert!(frame.beat_strength > 0.0);
        assert!(frame.bass > frame.treble);
    }
}
