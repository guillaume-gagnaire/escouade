//! What the conversation hears, frame by frame (20 ms): when a sentence starts and when it ends (the silence after it),
//! and, while the agent speaks, whether the user speaks over it (barge-in) rather than the microphone hearing the
//! agent's own voice from the speakers.
//!
//! Both go by the loudness of each frame (its RMS) against what is expected there: the room's own noise, learnt while
//! nobody speaks; and, while the agent speaks, its echo, learnt from what is played and what comes back.

/// The length of a frame, in milliseconds.
pub const FRAME_MS: u32 = 20;

/// How loud a frame is: its RMS.
pub fn rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt()
}

/// Under this, a frame is silence whatever the room's noise (about -46 dBFS), at the default sensitivity.
const FLOOR: f32 = 0.005;
/// A frame is speech when it is this many times the room's noise (about +10 dB), at the default sensitivity.
const OVER_NOISE: f32 = 3.2;
/// « Sensibilité du micro » (ESC-665): from `-SENSITIVITY_MAX` (less sensitive) to `SENSITIVITY_MAX` (more), 0 being
/// the detector as it always was. Each step moves the threshold over the room's noise, and the floor with it, by
/// `STEP_DB`: in a quiet room the floor is what decides, in a noisy one the noise.
pub const SENSITIVITY_MAX: i32 = 5;
const STEP_DB: f32 = 1.5;
/// Never closer to the room's noise than this (about +6 dB): its own ups and downs, from one frame to the next, are
/// not speech. The more sensitive steps then lower the floor only.
const MIN_OVER_NOISE: f32 = 2.0;

/// What a sensitivity makes of the detector: how many times the room's noise a frame must be (`over_noise`), and the
/// level under which it is silence whatever the noise (`floor`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levels {
    pub over_noise: f32,
    pub floor: f32,
}

impl Levels {
    pub fn of(sensitivity: i32) -> Self {
        let s = sensitivity.clamp(-SENSITIVITY_MAX, SENSITIVITY_MAX);
        if s == 0 {
            return Self {
                over_noise: OVER_NOISE,
                floor: FLOOR,
            };
        }
        let factor = 10f32.powf(-(s as f32) * STEP_DB / 20.0);
        Self {
            over_noise: (OVER_NOISE * factor).max(MIN_OVER_NOISE),
            floor: FLOOR * factor,
        }
    }

    /// The level over which a frame is speech, in a room of `noise`.
    pub fn threshold(&self, noise: f32) -> f32 {
        (noise * self.over_noise).max(self.floor)
    }
}
/// The speech that starts a sentence: this long, in the last `START_WINDOW` frames.
const START_FRAMES: usize = 8;
const START_WINDOW: usize = 12;
/// The frames the room's noise is learnt from (1.5 s).
const ROOM_FRAMES: usize = 75;
/// A sentence that goes on this long ends anyway (a noise that never stops).
const MAX_SENTENCE_MS: u32 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heard {
    /// A sentence started: it began this many frames ago (what to keep from before).
    Started { frames_ago: usize },
    /// It ended: the silence after it was long enough.
    Ended,
}

/// Finds a sentence's start and its end (the silence after it) in a stream of frames.
#[derive(Debug, Clone)]
pub struct Endpointer {
    /// The silence that ends a sentence, in milliseconds (« Délai avant l’envoi »).
    pub silence_ms: u32,
    sensitivity: i32,
    levels: Levels,
    noise: f32,
    /// The levels of the last frames outside a sentence: the room's noise is the quietest of them.
    room: std::collections::VecDeque<f32>,
    recent: std::collections::VecDeque<bool>,
    speaking: bool,
    quiet_ms: u32,
    spoken_ms: u32,
}

impl Endpointer {
    pub fn new(silence_ms: u32) -> Self {
        Self {
            silence_ms,
            sensitivity: 0,
            levels: Levels::of(0),
            noise: 0.003,
            room: Default::default(),
            recent: Default::default(),
            speaking: false,
            quiet_ms: 0,
            spoken_ms: 0,
        }
    }

    /// « Sensibilité du micro »: the threshold it hears speech over (nothing else changes).
    pub fn with_sensitivity(mut self, sensitivity: i32) -> Self {
        self.set_sensitivity(sensitivity);
        self
    }

    /// The sensitivity changed while it listens (the settings' tester): the noise learnt is kept.
    pub fn set_sensitivity(&mut self, sensitivity: i32) {
        self.sensitivity = sensitivity.clamp(-SENSITIVITY_MAX, SENSITIVITY_MAX);
        self.levels = Levels::of(self.sensitivity);
    }

    pub fn sensitivity(&self) -> i32 {
        self.sensitivity
    }

    /// The level over which a frame is speech now.
    pub fn threshold(&self) -> f32 {
        self.levels.threshold(self.noise)
    }

    /// Whether it is within a sentence: from its start to the silence that ends it.
    pub fn in_sentence(&self) -> bool {
        self.speaking
    }

    /// The room's noise, as learnt.
    pub fn noise(&self) -> f32 {
        self.noise
    }

    /// How long it has been quiet within the sentence under way (0 while the user speaks, or outside a sentence).
    pub fn quiet_ms(&self) -> u32 {
        if self.speaking {
            self.quiet_ms
        } else {
            0
        }
    }

    /// Speech had already started when the endpointer was told to listen (a barge-in): it is in a sentence.
    pub fn start_in_sentence(&mut self) {
        self.speaking = true;
        self.quiet_ms = 0;
        self.spoken_ms = 0;
    }

    /// The sentence under way is ended by the window, not by its silence (« appuyer pour parler »): the next speech
    /// starts a new one. What it learnt of the room stays.
    pub fn end_sentence(&mut self) {
        self.speaking = false;
        self.quiet_ms = 0;
        self.spoken_ms = 0;
        self.recent.clear();
    }

    /// The next frame, by its RMS.
    pub fn push(&mut self, level: f32) -> Option<Heard> {
        if !self.speaking {
            // The room's noise: the quietest frame of the last 1.5 s outside a sentence. A fan that never stops
            // is the noise; words, with their gaps, are not.
            self.room.push_back(level);
            if self.room.len() > ROOM_FRAMES {
                self.room.pop_front();
            }
            self.noise = self.room.iter().fold(f32::MAX, |m, x| m.min(*x));
        }
        let speech = level > self.threshold();
        if self.speaking {
            self.spoken_ms += FRAME_MS;
            self.quiet_ms = if speech { 0 } else { self.quiet_ms + FRAME_MS };
            if self.quiet_ms >= self.silence_ms || self.spoken_ms >= MAX_SENTENCE_MS {
                self.speaking = false;
                self.recent.clear();
                return Some(Heard::Ended);
            }
            return None;
        }
        self.recent.push_back(speech);
        if self.recent.len() > START_WINDOW {
            self.recent.pop_front();
        }
        if self.recent.iter().filter(|s| **s).count() >= START_FRAMES {
            self.speaking = true;
            self.quiet_ms = 0;
            self.spoken_ms = 0;
            let frames_ago = self.recent.len();
            self.recent.clear();
            return Some(Heard::Started { frames_ago });
        }
        None
    }
}

/// While the agent speaks: whether the user speaks over it. The microphone hears the agent's voice too, from the
/// speakers: how much of it comes back (the coupling) is learnt from the first moments of the playback, and only what
/// is clearly louder than that echo, and than the room, for long enough, is the user.
#[derive(Debug, Clone)]
pub struct BargeIn {
    /// The microphone's level over the agent's played level, as learnt.
    coupling: f32,
    learnt_ms: u32,
    /// What was played lately, loudest first: the echo comes back late (the output's and the room's delay).
    played: std::collections::VecDeque<f32>,
    noise: f32,
    levels: Levels,
    over: std::collections::VecDeque<bool>,
}

/// How long the coupling is learnt at the start of a playback (the user seldom speaks over its first words).
const LEARN_MS: u32 = 400;
/// The echo of a frame is looked for this long after it was played.
const ECHO_FRAMES: usize = 15;
/// The user, over the echo expected: this many times louder (about +9.5 dB)…
const OVER_ECHO: f32 = 3.0;
/// …in this many of the last `BARGE_WINDOW` frames (250 ms of speech).
const BARGE_FRAMES: usize = 12;
const BARGE_WINDOW: usize = 15;

impl BargeIn {
    /// `noise`: the room's, as the endpointer learnt it.
    pub fn new(noise: f32) -> Self {
        Self {
            coupling: 0.0,
            learnt_ms: 0,
            played: Default::default(),
            noise: noise.max(0.001),
            levels: Levels::of(0),
            over: Default::default(),
        }
    }

    /// « Sensibilité du micro »: how far over the room the user must be (never what it knows of the echo).
    pub fn with_sensitivity(mut self, sensitivity: i32) -> Self {
        self.levels = Levels::of(sensitivity);
        self
    }

    /// The level the microphone must pass now to be the user.
    pub fn threshold(&self) -> f32 {
        let loudest = self.played.iter().fold(0f32, |m, x| m.max(*x));
        (self.coupling * loudest * OVER_ECHO)
            .max(self.noise * self.levels.over_noise * 1.5)
            .max(self.levels.floor * 2.0)
    }

    /// The next frame: what the microphone heard (`mic`, RMS) and what was played meanwhile (`played`, RMS). True when
    /// the user speaks over the agent.
    pub fn push(&mut self, mic: f32, played: f32) -> bool {
        self.played.push_front(played);
        self.played.truncate(ECHO_FRAMES);
        let loudest = self.played.iter().fold(0f32, |m, x| m.max(*x));
        if self.learnt_ms < LEARN_MS {
            // Learnt from what is played: the first sentence may still be in the making.
            if loudest > 0.01 {
                self.learnt_ms += FRAME_MS;
                self.coupling = self.coupling.max(mic / loudest);
            }
            return false;
        }
        let over = mic > self.threshold();
        if !over && loudest > 0.01 {
            // The echo grows (the volume turned up): followed, slowly, by frames that are not the user.
            let c = mic / loudest;
            if c > self.coupling {
                self.coupling += (c - self.coupling) * 0.05;
            }
        }
        self.over.push_back(over);
        if self.over.len() > BARGE_WINDOW {
            self.over.pop_front();
        }
        self.over.iter().filter(|o| **o).count() >= BARGE_FRAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames of a voice-like sound (`amp` peak) for `ms`.
    fn voice(amp: f32, ms: u32) -> Vec<f32> {
        (0..ms / FRAME_MS)
            .map(|i| amp * (0.55 + 0.15 * ((i as f32) * 0.7).sin()))
            .collect()
    }

    fn quiet(level: f32, ms: u32) -> Vec<f32> {
        (0..ms / FRAME_MS)
            .map(|i| level * (1.0 + 0.3 * ((i as f32) * 1.3).sin()))
            .collect()
    }

    fn run(e: &mut Endpointer, frames: &[f32]) -> Vec<(usize, Heard)> {
        frames
            .iter()
            .enumerate()
            .filter_map(|(i, l)| e.push(*l).map(|h| (i, h)))
            .collect()
    }

    #[test]
    fn the_default_sensitivity_is_the_detector_as_it_was() {
        assert_eq!(
            Levels::of(0),
            Levels {
                over_noise: 3.2,
                floor: 0.005
            }
        );
        let e = Endpointer::new(800);
        assert_eq!(e.sensitivity(), 0);
        assert_eq!(e.threshold(), (0.003f32 * 3.2).max(0.005));
        assert_eq!(
            Endpointer::new(800).with_sensitivity(0).threshold(),
            e.threshold()
        );
        // Beyond the scale, its ends.
        assert_eq!(Levels::of(40), Levels::of(SENSITIVITY_MAX));
        assert_eq!(Levels::of(-40), Levels::of(-SENSITIVITY_MAX));
    }

    #[test]
    fn more_sensitive_lowers_the_threshold_less_sensitive_raises_it() {
        let mut last = f32::MAX;
        for s in -SENSITIVITY_MAX..=SENSITIVITY_MAX {
            let l = Levels::of(s);
            // In a quiet room (the floor decides) and a noisy one (the noise) alike.
            assert!(l.threshold(0.0005) < last, "{s}");
            last = l.threshold(0.0005);
            assert!(
                l.threshold(0.02) <= Levels::of(s - 1).threshold(0.02),
                "{s}"
            );
            assert!(l.over_noise >= MIN_OVER_NOISE);
        }
    }

    #[test]
    fn a_low_voice_is_heard_when_more_sensitive_and_ignored_when_less() {
        // A quiet room, a voice said low or far from the microphone (about -44 dBFS).
        let mut frames = quiet(0.0008, 1500);
        frames.extend(voice(0.012, 1500));
        frames.extend(quiet(0.0008, 2000));
        let heard = |s: i32| run(&mut Endpointer::new(800).with_sensitivity(s), &frames);
        let more = heard(SENSITIVITY_MAX);
        assert_eq!(more.len(), 2, "{more:?}");
        assert!(matches!(more[0].1, Heard::Started { .. }));
        assert_eq!(more[1].1, Heard::Ended);
        // Today's detector hears it (over -46 dBFS)…
        assert_eq!(heard(0).len(), 2);
        // …less sensitive, it is the room.
        assert!(heard(-SENSITIVITY_MAX).is_empty());
        // Lower still (about -50 dBFS): only the more sensitive hears it.
        let mut frames = quiet(0.0008, 1500);
        frames.extend(voice(0.006, 1500));
        frames.extend(quiet(0.0008, 2000));
        assert_eq!(
            run(
                &mut Endpointer::new(800).with_sensitivity(SENSITIVITY_MAX),
                &frames
            )
            .len(),
            2
        );
        assert!(run(&mut Endpointer::new(800), &frames).is_empty());
    }

    #[test]
    fn the_sensitivity_changes_nothing_but_the_threshold() {
        // The end of a sentence comes after the same silence whatever the sensitivity.
        for s in [-SENSITIVITY_MAX, 0, SENSITIVITY_MAX] {
            let mut e = Endpointer::new(800).with_sensitivity(s);
            let mut frames = quiet(0.004, 1000);
            frames.extend(voice(0.2, 1500));
            frames.extend(quiet(0.004, 2000));
            let heard = run(&mut e, &frames);
            assert_eq!(heard.len(), 2, "{s}: {heard:?}");
            assert_eq!(heard[1], (124 + 40, Heard::Ended), "{s}");
        }
        // Moved while it listens: the room's noise learnt is kept.
        let mut e = Endpointer::new(800);
        run(&mut e, &quiet(0.02, 2000));
        let noise = e.noise();
        e.set_sensitivity(3);
        assert_eq!(e.noise(), noise);
        assert_eq!(e.threshold(), Levels::of(3).threshold(noise));
    }

    #[test]
    fn a_sentence_starts_with_the_voice_and_ends_after_the_silence() {
        let mut e = Endpointer::new(800);
        let mut frames = quiet(0.004, 1000);
        frames.extend(voice(0.2, 1500));
        frames.extend(quiet(0.004, 2000));
        let heard = run(&mut e, &frames);
        assert_eq!(heard.len(), 2, "{heard:?}");
        // Started within 200 ms of the voice (frame 50), knowing how far back it began.
        assert!(
            matches!(heard[0], (i, Heard::Started { frames_ago }) if (57..=60).contains(&i) && frames_ago >= 8)
        );
        // Ended 800 ms after it (frame 125 + 40).
        assert_eq!(heard[1], (124 + 40, Heard::Ended));
    }

    #[test]
    fn a_short_pause_inside_a_sentence_does_not_end_it() {
        let mut e = Endpointer::new(1000);
        let mut frames = quiet(0.003, 500);
        frames.extend(voice(0.2, 800));
        frames.extend(quiet(0.003, 600));
        frames.extend(voice(0.2, 800));
        frames.extend(quiet(0.003, 1200));
        let heard = run(&mut e, &frames);
        assert_eq!(
            heard.iter().filter(|(_, h)| *h == Heard::Ended).count(),
            1,
            "{heard:?}"
        );
    }

    #[test]
    fn a_click_or_a_cough_is_not_a_sentence() {
        let mut e = Endpointer::new(800);
        let mut frames = quiet(0.003, 500);
        frames.extend(voice(0.3, 100));
        frames.extend(quiet(0.003, 1500));
        assert!(run(&mut e, &frames).is_empty());
    }

    #[test]
    fn a_noisy_room_is_learnt_and_is_not_speech() {
        let mut e = Endpointer::new(800);
        // A fan at -32 dBFS: never a sentence, however long.
        assert!(run(&mut e, &quiet(0.025, 5000)).is_empty());
        let mut frames = voice(0.25, 1000);
        frames.extend(quiet(0.025, 1500));
        let heard = run(&mut e, &frames);
        assert_eq!(heard.len(), 2, "{heard:?}");
    }

    #[test]
    fn the_echo_of_the_agent_is_not_the_user() {
        // Laptop speakers: a third of the played level comes back, 100 ms late.
        let played: Vec<f32> = voice(0.3, 6000);
        let mut b = BargeIn::new(0.003);
        for (i, p) in played.iter().enumerate() {
            let echo = if i >= 5 { played[i - 5] * 0.33 } else { 0.0 };
            assert!(!b.push(echo + 0.003, *p), "frame {i}");
        }
    }

    #[test]
    fn the_user_speaking_over_the_agent_is_heard_within_300_ms() {
        let played: Vec<f32> = voice(0.3, 4000);
        let mut b = BargeIn::new(0.003);
        let mut at = None;
        for (i, p) in played.iter().enumerate() {
            let echo = if i >= 5 { played[i - 5] * 0.33 } else { 0.0 };
            // From 2 s on, the user speaks, close to the microphone.
            let user = if i >= 100 { 0.35 } else { 0.0 };
            if b.push(echo + user + 0.003, *p) {
                at = Some(i);
                break;
            }
        }
        let at = at.expect("barge-in heard");
        assert!((111..=115).contains(&at), "{at}");
    }

    #[test]
    fn the_echo_is_learnt_from_the_first_sound_played_not_from_the_wait_for_it() {
        // The first sentence takes 600 ms to make: nothing is played meanwhile, then the echo comes.
        let mut played = vec![0.0; 30];
        played.extend(voice(0.3, 4000));
        let mut b = BargeIn::new(0.003);
        for (i, p) in played.iter().enumerate() {
            let echo = if i >= 5 { played[i - 5] * 0.4 } else { 0.0 };
            assert!(!b.push(echo + 0.003, *p), "frame {i}");
        }
    }

    #[test]
    fn the_echo_is_never_the_user_however_sensitive() {
        let played: Vec<f32> = voice(0.3, 6000);
        let mut b = BargeIn::new(0.003).with_sensitivity(SENSITIVITY_MAX);
        for (i, p) in played.iter().enumerate() {
            let echo = if i >= 5 { played[i - 5] * 0.33 } else { 0.0 };
            assert!(!b.push(echo + 0.003, *p), "frame {i}");
        }
    }

    #[test]
    fn with_headphones_a_low_voice_barges_in_by_the_sensitivity() {
        let played: Vec<f32> = voice(0.3, 3000);
        let barges = |s: i32, user: f32| {
            let mut b = BargeIn::new(0.001).with_sensitivity(s);
            played
                .iter()
                .enumerate()
                .any(|(i, p)| b.push(if i >= 50 { user } else { 0.001 }, *p))
        };
        assert!(barges(SENSITIVITY_MAX, 0.006));
        assert!(!barges(0, 0.006));
        assert!(barges(0, 0.012));
        assert!(!barges(-SENSITIVITY_MAX, 0.012));
    }

    #[test]
    fn with_headphones_a_low_voice_still_barges_in() {
        let played: Vec<f32> = voice(0.3, 3000);
        let mut b = BargeIn::new(0.003);
        let heard = played
            .iter()
            .enumerate()
            .any(|(i, p)| b.push(if i >= 50 { 0.05 } else { 0.003 }, *p));
        assert!(heard);
    }
}

#[cfg(test)]
mod wav_tests {
    use super::*;

    /// The frames of a WAV of speech, a little noise of a room added (a microphone never gives digital silence).
    fn frames(name: &str, noise: f32) -> (Vec<f32>, f32, f32) {
        let path = crate::testdata(name);
        let (samples, rate) = crate::source::read_wav(&path).unwrap();
        let first = samples.iter().position(|s| s.abs() > 0.02).unwrap() as f32 / rate as f32;
        let last = samples.iter().rposition(|s| s.abs() > 0.02).unwrap() as f32 / rate as f32;
        let mut seed = 7u32;
        let noisy: Vec<f32> = samples
            .iter()
            .map(|s| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                s + noise * ((seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 2.0
            })
            .collect();
        let per = (rate * FRAME_MS / 1000) as usize;
        (noisy.chunks(per).map(rms).collect(), first, last)
    }

    #[test]
    fn a_spoken_question_in_a_wav_starts_with_its_first_word_and_ends_after_the_silence() {
        for noise in [0.0005, 0.01] {
            let (levels, first, last) = frames("question-fr.wav", noise);
            let mut e = Endpointer::new(1000);
            let heard: Vec<(f32, Heard)> = levels
                .iter()
                .enumerate()
                .filter_map(|(i, l)| e.push(*l).map(|h| (i as f32 * FRAME_MS as f32 / 1000.0, h)))
                .collect();
            assert_eq!(heard.len(), 2, "noise {noise}: {heard:?}");
            let (start, Heard::Started { frames_ago }) = heard[0] else {
                panic!("{heard:?}")
            };
            let began = start - frames_ago as f32 * FRAME_MS as f32 / 1000.0;
            assert!(
                (began - first).abs() < 0.3,
                "noise {noise}: began {began}, first word {first}"
            );
            assert_eq!(heard[1].1, Heard::Ended);
            // Its pause between the two clauses (« réglages, puis ») did not end it: the end is the silence's.
            let end = heard[1].0;
            assert!(
                end > last + 0.6 && end < last + 1.3,
                "noise {noise}: ended {end}, last word {last}"
            );
        }
    }
}
