// « Sensibilité du micro » and its tester (ESC-665): the microphone scripted in real time. Included in `tests.rs` after
// `conv_tests.rs`, whose `hear` it uses.

/// The tester's events so far.
fn meters(h: &Harness) -> Vec<(f32, f32, f32, bool)> {
    h.events
        .lock()
        .iter()
        .filter_map(|e| match e {
            VoiceEvent::Meter { level, noise, threshold, speech, .. } => Some((*level, *noise, *threshold, *speech)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_tester_shows_the_levels_the_threshold_moved_by_the_slider_and_the_voice_heard() {
    let h = harness("meter-levels", "jamais transcrit", 16_000);
    let id = h.voice.meter_start("mic-1", 0, 600).unwrap();
    assert_eq!(*h.script.opened.lock(), vec!["mic-1".to_string()]);
    assert_eq!(h.voice.metering(), Some(id));
    // A quiet room: its noise learnt, the threshold that of today's detector (its floor, about -46 dBFS).
    hear(&h, 1.6, 0.0);
    let (_, noise, threshold, speech) = *meters(&h).last().unwrap();
    assert!(noise > 0.0005 && noise < 0.002, "{noise}");
    assert_eq!(threshold, vad::Levels::of(0).threshold(noise));
    assert!(!speech);
    // More sensitive: the line comes down at once, the noise learnt kept.
    h.voice.meter_set(id, 5);
    hear(&h, 0.2, 0.0);
    let (_, noise, threshold, _) = *meters(&h).last().unwrap();
    assert_eq!(threshold, vad::Levels::of(5).threshold(noise));
    assert!(threshold < 0.005 * 0.6, "{threshold}");
    // A voice: « Voix détectée » lights, and goes out after the silence that ends its sentence.
    hear(&h, 0.8, 0.3);
    let (level, _, _, speech) = *meters(&h).last().unwrap();
    assert!(speech && level > 0.1, "{level}");
    hear(&h, 1.0, 0.0);
    assert!(!meters(&h).last().unwrap().3);
    // Nothing of it was transcribed, nor kept.
    assert!(h.fake.calls.lock().is_empty());
    // « Arrêter »: the microphone is let go, the events stop.
    h.voice.meter_stop(id);
    assert!(h.script.dropped.load(Ordering::Acquire));
    assert_eq!(h.voice.metering(), None);
    let n = meters(&h).len();
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(meters(&h).len(), n);
}

#[test]
fn a_dictation_takes_the_microphone_from_the_tester_and_a_conversation_keeps_it_from_it() {
    let h = harness("meter-taken", "bonjour", 16_000);
    put_model(&h.voice.store, "base");
    let id = h.voice.meter_start("", 0, 1200).unwrap();
    h.voice.meter_set(id, 3);
    let session = match h.voice.start(&choice("base", "fr")).unwrap() {
        Started::Listening { session } => session,
        other => panic!("{other:?}"),
    };
    assert_eq!(h.voice.metering(), None);
    // While it dictates, the tester waits.
    assert_eq!(h.voice.meter_start("", 0, 1200).unwrap_err(), VoiceError::AlreadyListening);
    h.voice.cancel();
    assert_ne!(session, 0);
    // The window gone (`cancel`): the tester stops too.
    let id = h.voice.meter_start("", 0, 1200).unwrap();
    h.voice.cancel();
    assert_eq!(h.voice.metering(), None);
    // An old id stops nothing.
    let now = h.voice.meter_start("", 0, 1200).unwrap();
    h.voice.meter_stop(id);
    assert_eq!(h.voice.metering(), Some(now));
}

#[test]
fn a_low_voice_starts_a_sentence_of_the_conversation_only_when_more_sensitive() {
    for (sensitivity, heard) in [(0, false), (5, true)] {
        let c = conv_harness(&format!("conv-sensitivity-{sensitivity}"), "doucement");
        let h = &c.h;
        put_model(&h.voice.store, "base");
        put_voice(&h.voice, "fr_FR-siwis-medium");
        let choice = ConvChoice { sensitivity, ..conv_choice() };
        listening(h.voice.conversation_start(choice).unwrap());
        hear(h, 1.6, 0.0);
        // Said low, or far from the microphone (about -47 dBFS).
        hear(h, 1.0, 0.006);
        hear(h, 0.3, 0.0);
        let started = conv_events(h).iter().any(|e| matches!(e, VoiceEvent::ConvSpeech { .. }));
        assert_eq!(started, heard, "sensitivity {sensitivity}: {:?}", conv_events(h));
        h.voice.conversation_stop();
    }
}
