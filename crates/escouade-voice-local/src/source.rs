//! Where the sound comes from: the microphone through cpal, or a WAV file (the end-to-end tests,
//! which have no microphone), or what a test hands it.

use crate::audio;
use crate::VoiceError;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// What takes the samples as they come.
pub type Samples = Box<dyn FnMut(&[f32]) + Send>;
/// What takes what goes wrong once it listens.
pub type Failure = Box<dyn FnMut(VoiceError) + Send>;

/// What a listening hands what it hears to, as it comes: mono samples, at the rate the source
/// said when it opened; and what goes wrong once it listens (a microphone unplugged).
pub struct Sink {
    pub samples: Samples,
    pub error: Failure,
}

/// A listening under way: dropped, it stops, and nothing more reaches its sink.
pub trait Capture: Send {
    /// The rate of the samples it hands out.
    fn rate(&self) -> u32;
}

/// A microphone, as the settings name it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// What the settings keep (cpal's id, stable across runs where the system allows).
    pub id: String,
    pub name: String,
    /// The system's own choice.
    pub default: bool,
}

pub trait Source: Send + Sync {
    /// Starts listening to `device` (empty: the system's default microphone).
    fn open(&self, device: &str, sink: Sink) -> Result<Box<dyn Capture>, VoiceError>;
    /// The microphones there are.
    fn devices(&self) -> Vec<Device>;
}

// ---------------------------------------------------------------------------------------------
// The microphone, through cpal.

pub struct Microphone;

impl From<cpal::Error> for VoiceError {
    fn from(e: cpal::Error) -> Self {
        use cpal::ErrorKind as K;
        match e.kind() {
            K::PermissionDenied => VoiceError::Denied,
            K::DeviceNotAvailable => VoiceError::NoDevice,
            K::DeviceBusy => VoiceError::Busy,
            K::HostUnavailable => VoiceError::NoAudioSystem,
            _ => VoiceError::Other(e.to_string()),
        }
    }
}

/// The cpal stream lives on a thread of its own, from its making to its end: some systems want
/// it dropped where it was made.
struct MicCapture {
    rate: u32,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Capture for MicCapture {
    fn rate(&self) -> u32 {
        self.rate
    }
}

impl Drop for MicCapture {
    fn drop(&mut self) {
        self.stop.take();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn input_device(host: &cpal::Host, id: &str) -> Option<cpal::Device> {
    use cpal::traits::HostTrait;
    if !id.is_empty() {
        if let Some(d) = id
            .parse::<cpal::DeviceId>()
            .ok()
            .and_then(|id| host.device_by_id(&id))
        {
            return Some(d);
        }
        // Unplugged since it was chosen: the system's own goes on in its place.
        log::info!("voice: the microphone chosen is not there, the default one listens");
    }
    host.default_input_device()
}

fn build_stream(device: &cpal::Device, sink: Sink) -> Result<(cpal::Stream, u32), VoiceError> {
    use cpal::traits::{DeviceTrait, StreamTrait};
    use cpal::{FromSample, SampleFormat, SizedSample};
    let config = device.default_input_config()?;
    let rate = config.sample_rate();
    let channels = config.channels() as usize;
    let format = config.sample_format();
    let error = std::sync::Arc::new(parking_lot::Mutex::new(sink.error));
    let samples = sink.samples;
    fn make<T: SizedSample>(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        channels: usize,
        mut samples: Samples,
        error: std::sync::Arc<parking_lot::Mutex<Failure>>,
    ) -> Result<cpal::Stream, cpal::Error>
    where
        f32: FromSample<T>,
    {
        let mut buf = Vec::new();
        device.build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                buf.clear();
                buf.extend(data.iter().map(|s| cpal::Sample::to_sample::<f32>(*s)));
                samples(&audio::to_mono(&buf, channels));
            },
            move |e: cpal::Error| {
                // A glitch, or the route that changed by itself: the listening goes on.
                if matches!(
                    e.kind(),
                    cpal::ErrorKind::Xrun
                        | cpal::ErrorKind::DeviceChanged
                        | cpal::ErrorKind::RealtimeDenied
                ) {
                    log::debug!("voice: {e}");
                    return;
                }
                (error.lock())(e.into());
            },
            None,
        )
    }
    let config: cpal::StreamConfig = config.into();
    let stream = match format {
        SampleFormat::F32 => make::<f32>(device, config, channels, samples, error),
        SampleFormat::I16 => make::<i16>(device, config, channels, samples, error),
        SampleFormat::U16 => make::<u16>(device, config, channels, samples, error),
        SampleFormat::I32 => make::<i32>(device, config, channels, samples, error),
        SampleFormat::U32 => make::<u32>(device, config, channels, samples, error),
        SampleFormat::I8 => make::<i8>(device, config, channels, samples, error),
        SampleFormat::U8 => make::<u8>(device, config, channels, samples, error),
        SampleFormat::F64 => make::<f64>(device, config, channels, samples, error),
        other => {
            return Err(VoiceError::Other(format!(
                "sample format {other} not supported"
            )));
        }
    }?;
    stream.play()?;
    Ok((stream, rate))
}

impl Source for Microphone {
    fn open(&self, device: &str, sink: Sink) -> Result<Box<dyn Capture>, VoiceError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32, VoiceError>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let id = device.to_string();
        let thread = std::thread::Builder::new()
            .name("voice-capture".into())
            .spawn(move || {
                let host = cpal::default_host();
                let Some(device) = input_device(&host, &id) else {
                    let _ = ready_tx.send(Err(VoiceError::NoDevice));
                    return;
                };
                match build_stream(&device, sink) {
                    Ok((stream, rate)) => {
                        let _ = ready_tx.send(Ok(rate));
                        // Until the capture is dropped (its sender with it).
                        let _ = stop_rx.recv();
                        drop(stream);
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
            })
            .map_err(|e| VoiceError::Other(e.to_string()))?;
        // A system that asks the user first (macOS's dialog) answers once they did.
        let rate = ready_rx
            .recv()
            .map_err(|_| VoiceError::Other("capture thread ended".into()))??;
        Ok(Box::new(MicCapture {
            rate,
            stop: Some(stop_tx),
            thread: Some(thread),
        }))
    }

    fn devices(&self) -> Vec<Device> {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let default = host
            .default_input_device()
            .and_then(|d| d.id().ok())
            .map(|id| id.to_string());
        let Ok(list) = host.input_devices() else {
            return Vec::new();
        };
        list.filter_map(|d| {
            let id = d.id().ok()?.to_string();
            let name = d
                .description()
                .map(|x| x.name().to_string())
                .unwrap_or_else(|_| d.to_string());
            Some(Device {
                default: default.as_deref() == Some(id.as_str()),
                id,
                name,
            })
        })
        .collect()
    }
}

// ---------------------------------------------------------------------------------------------
// A WAV file, played as a microphone would hear it: 20 ms at a time, in real time, then silence.

pub struct WavFile(pub PathBuf);

/// What plays a file: stops when it is dropped.
struct Playback {
    rate: u32,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Capture for Playback {
    fn rate(&self) -> u32 {
        self.rate
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.stop.take();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The samples of a WAV file, mono, in [-1, 1], and their rate.
pub fn read_wav(path: &std::path::Path) -> Result<(Vec<f32>, u32), VoiceError> {
    let mut r = hound::WavReader::open(path).map_err(|e| VoiceError::Other(e.to_string()))?;
    let spec = r.spec();
    let data: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().filter_map(Result::ok).collect(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>()
                .filter_map(Result::ok)
                .map(|s| s as f32 / scale)
                .collect()
        }
    };
    Ok((
        audio::to_mono(&data, spec.channels as usize),
        spec.sample_rate,
    ))
}

impl Source for WavFile {
    fn open(&self, _device: &str, mut sink: Sink) -> Result<Box<dyn Capture>, VoiceError> {
        let (samples, rate) = read_wav(&self.0)?;
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let chunk = (rate as usize / 50).max(1);
        let thread = std::thread::spawn(move || {
            let mut chunks = samples.chunks(chunk);
            let silence = vec![0f32; chunk];
            let started = std::time::Instant::now();
            let mut sent = 0u32;
            loop {
                match stop_rx.recv_timeout(Duration::from_millis(20)) {
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    _ => return,
                }
                // What the clock says is due, as a microphone would have heard it by now: a
                // timer that wakes late (a busy machine) is caught up, never fallen behind.
                let due = (started.elapsed().as_millis() / 20) as u32;
                while sent < due {
                    (sink.samples)(chunks.next().unwrap_or(&silence));
                    sent += 1;
                }
            }
        });
        Ok(Box::new(Playback {
            rate,
            stop: Some(stop_tx),
            thread: Some(thread),
        }))
    }

    fn devices(&self) -> Vec<Device> {
        vec![Device {
            id: String::new(),
            name: self
                .0
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            default: true,
        }]
    }
}
