//! The sound as Whisper takes it: mono, 16 kHz, f32 in [-1, 1]; and the level the window shows
//! while it listens.

/// The rate Whisper's models are trained on.
pub const WHISPER_RATE: u32 = 16_000;

/// The frames of `data` (`channels` samples each, interleaved) as one channel: their mean.
pub fn to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

pub use escouade_voice::resample;

/// How loud `samples` are, from 0 (silence, or under -60 dBFS) to 1 (full scale): their RMS on a
/// decibel scale, which follows a voice better than a straight one.
pub fn level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms <= 0.0 {
        return 0.0;
    }
    let db = 20.0 * rms.log10();
    ((db + 60.0) / 60.0).clamp(0.0, 1.0)
}

/// Not one sample but zero: what a microphone the system keeps from the app gives (macOS gives
/// silence rather than a refusal).
pub fn all_zero(samples: &[f32]) -> bool {
    samples.iter().all(|s| *s == 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, secs: f32) -> Vec<f32> {
        let n = (rate as f32 * secs) as usize;
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.5)
            .collect()
    }

    /// The frequency of a sine, by its zero crossings.
    fn freq_of(samples: &[f32], rate: u32) -> f32 {
        let crossings = samples
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        crossings as f32 * rate as f32 / samples.len() as f32
    }

    #[test]
    fn stereo_becomes_the_mean_of_its_channels() {
        assert_eq!(
            to_mono(&[1.0, 0.0, 0.5, 0.5, -1.0, 1.0], 2),
            vec![0.5, 0.5, 0.0]
        );
        assert_eq!(to_mono(&[0.1, 0.2], 1), vec![0.1, 0.2]);
        // A frame cut short at the end is dropped.
        assert_eq!(to_mono(&[0.3, 0.3, 0.3, 0.9], 3), vec![0.3]);
    }

    #[test]
    fn a_48k_voice_comes_down_to_16k_with_its_pitch_and_its_length() {
        let s = sine(440.0, 48_000, 1.0);
        let out = resample(&s, 48_000, WHISPER_RATE);
        assert_eq!(out.len(), 16_000);
        assert!((freq_of(&out, WHISPER_RATE) - 440.0).abs() < 3.0);
        // The amplitude stays (0.5 peak, RMS 0.354).
        let rms = (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt();
        assert!((rms - 0.354).abs() < 0.01, "{rms}");
    }

    #[test]
    fn a_44k1_rate_that_is_no_multiple_of_16k_comes_down_too() {
        let s = sine(300.0, 44_100, 2.0);
        let out = resample(&s, 44_100, WHISPER_RATE);
        assert_eq!(out.len(), 32_000);
        assert!((freq_of(&out, WHISPER_RATE) - 300.0).abs() < 3.0);
    }

    #[test]
    fn a_tone_above_the_new_nyquist_is_damped_not_folded_into_the_voice() {
        // 15 kHz at 48 kHz: plain decimation would fold it to 1 kHz at full strength.
        let s = sine(15_000.0, 48_000, 0.5);
        let out = resample(&s, 48_000, WHISPER_RATE);
        let rms = (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt();
        assert!(rms < 0.2, "{rms}");
    }

    #[test]
    fn an_8k_rate_goes_up_to_16k() {
        let s = sine(200.0, 8_000, 1.0);
        let out = resample(&s, 8_000, WHISPER_RATE);
        assert_eq!(out.len(), 16_000);
        assert!((freq_of(&out, WHISPER_RATE) - 200.0).abs() < 3.0);
        assert_eq!(resample(&s, 16_000, 16_000), s);
        assert!(resample(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn the_level_is_0_in_silence_and_grows_with_the_voice() {
        assert_eq!(level(&[]), 0.0);
        assert_eq!(level(&[0.0; 100]), 0.0);
        // -60 dBFS and under: nothing shown.
        assert_eq!(level(&[0.0005; 100]), 0.0);
        let quiet = level(
            &sine(200.0, 16_000, 0.1)
                .iter()
                .map(|s| s * 0.02)
                .collect::<Vec<_>>(),
        );
        let loud = level(&sine(200.0, 16_000, 0.1));
        assert!(quiet > 0.0 && quiet < loud && loud < 1.0, "{quiet} {loud}");
        assert_eq!(level(&[1.0; 10]), 1.0);
    }

    #[test]
    fn a_microphone_kept_from_the_app_gives_only_zeros() {
        assert!(all_zero(&[0.0; 50]));
        assert!(!all_zero(&[0.0, 0.0, 1e-6]));
    }
}
