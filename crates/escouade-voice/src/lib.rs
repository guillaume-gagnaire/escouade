#![doc = include_str!("../README.md")]

pub mod catalog;
pub mod host;
pub mod live;
pub mod online;
pub mod serve;
pub mod speech;
pub mod wire;

pub use escouade_plugin_kit::protocol;

/// `samples` taken from `from` Hz to `to` Hz. Down, each output sample is the mean of the input ones it covers (what keeps
/// the frequencies above the new Nyquist from folding back into the voice), read between two of them by a straight line;
/// up, the straight line alone.
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() || from == 0 || to == 0 {
        return samples.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let len = ((samples.len() as f64) / ratio).floor() as usize;
    let mut out = Vec::with_capacity(len);
    if ratio > 1.0 {
        // The running sum: the mean of any span costs two reads.
        let mut sum = Vec::with_capacity(samples.len() + 1);
        let mut total = 0f64;
        sum.push(total);
        for s in samples {
            total += *s as f64;
            sum.push(total);
        }
        let at = |x: f64| -> f64 {
            // The sum up to the fractional position x, the last sample counted in part.
            let i = (x.floor() as usize).min(samples.len());
            let frac = x - i as f64;
            let part = if i < samples.len() {
                samples[i] as f64 * frac
            } else {
                0.0
            };
            sum[i] + part
        };
        for k in 0..len {
            let a = k as f64 * ratio;
            let b = ((k + 1) as f64 * ratio).min(samples.len() as f64);
            out.push(((at(b) - at(a)) / (b - a)) as f32);
        }
    } else {
        for k in 0..len {
            let x = k as f64 * ratio;
            let i = x.floor() as usize;
            let frac = (x - i as f64) as f32;
            let a = samples[i];
            let b = *samples.get(i + 1).unwrap_or(&a);
            out.push(a + (b - a) * frac);
        }
    }
    out
}
