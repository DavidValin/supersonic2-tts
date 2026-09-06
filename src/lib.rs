pub mod helper;

use anyhow::{anyhow, Result};
use helper::{load_text_to_speech, load_voice_style, set_verbose, write_wav_file, TextToSpeech};
pub use helper::{compiled_gpu_backends, gpu_support_compiled, Device};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use cpal::Sample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Default voice quality (number of flow-matching denoising steps).
/// 5 is the fastest (and the historical default of this crate), 12 is the
/// best quality at the cost of more vector-estimator time.
pub const DEFAULT_VOICE_QUALITY: usize = 5;

/// Minimum allowed voice quality (fastest, lowest quality).
pub const MIN_VOICE_QUALITY: usize = 5;

/// Maximum allowed voice quality (slowest, best quality).
pub const MAX_VOICE_QUALITY: usize = 12;

/// Validate a voice quality value against the allowed range
/// [`MIN_VOICE_QUALITY`]..=[`MAX_VOICE_QUALITY`].
pub fn validate_voice_quality(voice_quality: usize) -> Result<usize> {
    if (MIN_VOICE_QUALITY..=MAX_VOICE_QUALITY).contains(&voice_quality) {
        Ok(voice_quality)
    } else {
        Err(anyhow!(
            "Invalid voice_quality value {}: must be between {} (fastest) and {} (best quality)",
            voice_quality,
            MIN_VOICE_QUALITY,
            MAX_VOICE_QUALITY
        ))
    }
}

/// Silence inserted between text chunks, in seconds.
pub const DEFAULT_SILENCE_DURATION: f32 = 0.3;

pub struct TtsEngine {
    inner: Arc<Mutex<TextToSpeech>>,
    base_path: PathBuf,
    verbose: bool,
    voice_quality: usize,
    device: Device,
}

impl TtsEngine {
    /// Create a new TTS engine running on the CPU. Loads the ONNX model files
    /// from the provided `onnx_dir`.
    pub async fn new(onnx_dir: PathBuf, base_path: PathBuf, verbose: bool) -> Result<Self> {
        Self::new_with_device(onnx_dir, base_path, verbose, Device::Cpu).await
    }

    /// Create a new TTS engine running on `device`.
    ///
    /// [`Device::Gpu`] requires the crate to be built with a GPU feature
    /// (`cuda`, `tensorrt`, `rocm`, `directml` or `coreml`) and the matching
    /// driver / runtime libraries to be installed. Otherwise an error is
    /// returned (there is no silent CPU fallback). Use
    /// [`gpu_support_compiled`] to check the build before asking for a GPU.
    pub async fn new_with_device(
        onnx_dir: PathBuf,
        base_path: PathBuf,
        verbose: bool,
        device: Device,
    ) -> Result<Self> {
        if !onnx_dir.exists() {
            std::fs::create_dir_all(&onnx_dir)?;
        }
        set_verbose(verbose);
        let tts = load_text_to_speech(onnx_dir.to_str().unwrap(), device)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(tts)),
            base_path,
            verbose,
            voice_quality: DEFAULT_VOICE_QUALITY,
            device,
        })
    }

    /// The device the models run on.
    pub fn device(&self) -> Device {
        self.device
    }

    /// Change the default voice quality (denoising steps, quality vs speed)
    /// used by `synthesize_with_options` when called with `voice_quality = None`.
    /// Allowed range is 5 (fastest) to 12 (best quality), default 5.
    pub fn with_voice_quality(mut self, voice_quality: usize) -> Result<Self> {
        self.voice_quality = validate_voice_quality(voice_quality)?;
        Ok(self)
    }

    /// Default voice quality used when none is passed.
    pub fn voice_quality(&self) -> usize {
        self.voice_quality
    }

    /// Synthesize audio from text with optional voice style, speed, gain,
    /// language and voice quality.
    ///
    /// `voice_quality`: number of denoising steps, from [`MIN_VOICE_QUALITY`]
    /// (5, fastest) to [`MAX_VOICE_QUALITY`] (12, best). `None` uses the engine
    /// default (5 unless changed with [`TtsEngine::with_voice_quality`]).
    /// Out of range values return an error.
    pub async fn synthesize_with_options(
        &self,
        text: &str,
        voice: Option<&str>,
        speed: f32,
        gain: f32,
        language: Option<&str>,
        voice_quality: Option<usize>,
    ) -> Result<Vec<f32>> {
        let voice_quality = match voice_quality {
            Some(q) => validate_voice_quality(q)?,
            None => self.voice_quality,
        };
        let base_path = match std::fs::canonicalize(&self.base_path) {
            Ok(p) => p,
            Err(_) => self.base_path.clone(),
        };
        let voice_style = if let Some(v) = voice {
            let path = if v.contains(std::path::MAIN_SEPARATOR.to_string().as_str()) {
                base_path.join(v)
            } else {
                let candidate = base_path.join(v);
                if candidate.is_file() {
                    candidate
                } else {
                    base_path.join(format!("voice_styles/{}.json", v))
                }
            };
            load_voice_style(&[path.to_str().unwrap().to_string()], self.verbose)?
        } else {
            // default to M1.json
            let path = base_path.join("voice_styles/M1.json");
            load_voice_style(&[path.to_str().unwrap().to_string()], self.verbose)?
        };

        let mut tts = self.inner.lock().await;
        let (wav, _duration) = tts.call(
            text,
            language.unwrap_or("en"),
            &voice_style,
            voice_quality,
            speed,
            DEFAULT_SILENCE_DURATION,
        )?;
        let wav: Vec<f32> = wav.iter().map(|x| x * gain).collect();
        Ok(wav)
    }

    /// Save the given audio buffer to a WAV file. The operation is performed in a blocking task.
    pub async fn save_wav(&self, path: &str, audio: &[f32]) -> Result<()> {
        let tts = self.inner.lock().await;
        let sample_rate = tts.sample_rate;
        let audio_vec = audio.to_vec();
        let path_owned = path.to_owned();
        let res = tokio::task::spawn_blocking(move || {
            write_wav_file(&path_owned, &audio_vec, sample_rate)
        })
        .await?;
        res
    }

    /// Play the given audio buffer using the CPAL audio backend.
    ///
    /// The default output device is tried first. If it cannot be opened (for
    /// example when ALSA `default` points at a sound card that PulseAudio or
    /// PipeWire holds exclusively) any output device named `pulse` or
    /// `pipewire` is tried next.
    pub async fn play_wav(&self, audio: &[f32]) -> Result<()> {
        let sample_rate = self.sample_rate().await as u32;
        let audio = Arc::new(audio.to_vec());
        let verbose = self.verbose;
        tokio::task::spawn_blocking(move || -> Result<()> {
            let host = cpal::default_host();
            let mut errors: Vec<String> = Vec::new();
            let mut tried: Vec<String> = Vec::new();
            // 1. the host default device
            if let Some(device) = host.default_output_device() {
                let name = device.name().unwrap_or_else(|_| "default".to_string());
                match play_on_device(&device, Arc::clone(&audio), sample_rate) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        if verbose {
                            eprintln!("Playback on '{}' failed: {}, trying pulse/pipewire", name, e);
                        }
                        errors.push(format!("{}: {}", name, e));
                    }
                }
                tried.push(name);
            }
            // 2. sound server devices. Enumerated lazily: probing every ALSA
            //    PCM is slow and noisy, so only do it when the default failed.
            for (name, device) in sound_server_devices(&host) {
                if tried.contains(&name) {
                    continue;
                }
                match play_on_device(&device, Arc::clone(&audio), sample_rate) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        if verbose {
                            eprintln!("Playback on '{}' failed: {}", name, e);
                        }
                        errors.push(format!("{}: {}", name, e));
                    }
                }
            }
            if errors.is_empty() {
                Err(anyhow!("No output device found"))
            } else {
                Err(anyhow!(
                    "Could not open any output device ({})",
                    errors.join("; ")
                ))
            }
        })
        .await??;
        Ok(())
    }

    /// Return the sample rate of the underlying TextToSpeech instance.
    pub async fn sample_rate(&self) -> i32 {
        let tts = self.inner.lock().await;
        tts.sample_rate
    }
}

/// Output devices named `pulse` or `pipewire`: ALSA plugins that route to the
/// sound server instead of opening the card directly, usable even when the
/// server holds the card exclusively.
fn sound_server_devices(host: &cpal::Host) -> Vec<(String, cpal::Device)> {
    let mut found: Vec<(String, cpal::Device)> = Vec::new();
    if let Ok(devices) = host.output_devices() {
        for device in devices {
            let name = match device.name() {
                Ok(n) => n,
                Err(_) => continue,
            };
            let is_server = name == "pulse"
                || name == "pipewire"
                || name.starts_with("pulse:")
                || name.starts_with("pipewire:");
            if is_server && !found.iter().any(|(n, _)| *n == name) {
                found.push((name, device));
            }
        }
    }
    found
}

/// Pick a supported output config for `sample_rate`, preferring mono, then
/// stereo, then anything with more channels.
fn pick_output_config(device: &cpal::Device, sample_rate: u32) -> Result<cpal::StreamConfig> {
    let rate = cpal::SampleRate(sample_rate);
    let mut best: Option<cpal::SupportedStreamConfig> = None;
    for range in device.supported_output_configs().map_err(|e| anyhow!(e))? {
        if let Some(cfg) = range.try_with_sample_rate(rate) {
            let better = match &best {
                None => true,
                Some(b) => cfg.channels() < b.channels(),
            };
            if better {
                best = Some(cfg);
            }
        }
    }
    let cfg = best.ok_or_else(|| anyhow!("no output config supports {} Hz", sample_rate))?;
    Ok(cfg.config())
}

/// Open an output stream on `device` and block until `audio` has been played.
fn play_on_device(device: &cpal::Device, audio: Arc<Vec<f32>>, sample_rate: u32) -> Result<()> {
    let config = pick_output_config(device, sample_rate)?;
    let channels = config.channels.max(1) as usize;
    let audio_len = audio.len();
    let index = Arc::new(std::sync::Mutex::new(0usize));
    let err_fn = |err: cpal::StreamError| {
        eprintln!("An error occurred on the output audio stream: {:?}", err);
    };
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            let mut idx = index.lock().unwrap();
            for frame in data.chunks_mut(channels) {
                let value = if *idx < audio_len { audio[*idx] } else { 0.0 };
                *idx += 1;
                for sample in frame.iter_mut() {
                    *sample = Sample::from_sample(value);
                }
            }
        },
        err_fn,
        None,
    )?;
    stream.play()?;
    let duration = std::time::Duration::from_secs_f32(audio_len as f32 / sample_rate as f32);
    std::thread::sleep(duration + std::time::Duration::from_millis(100));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_quality_range_is_validated() {
        assert!(validate_voice_quality(MIN_VOICE_QUALITY - 1).is_err());
        assert!(validate_voice_quality(MAX_VOICE_QUALITY + 1).is_err());
        assert!(validate_voice_quality(0).is_err());
        for q in MIN_VOICE_QUALITY..=MAX_VOICE_QUALITY {
            assert_eq!(validate_voice_quality(q).unwrap(), q);
        }
        assert_eq!(DEFAULT_VOICE_QUALITY, 5);
    }
}
