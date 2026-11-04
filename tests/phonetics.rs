//! Tests for the deterministic phonetic engine and the DSP.

use singsin::data::phonetics::{analyze, parse_syllable};
use singsin::data::phrases::{align, seed_phrases, Phrase};
use singsin::dsp::track::Contour;
use singsin::dsp::vad;

fn phrase(id: i64) -> Phrase {
    seed_phrases()
        .into_iter()
        .find(|p| p.id == id)
        .expect("phrase exists")
}

#[test]
fn alignment_keeps_a_latin_name_whole() {
    let pairs = align("你好，我叫 Jacob。", "nei5 hou2 ngo5 giu3 Jacob");
    assert_eq!(pairs.len(), 5);
    assert_eq!(pairs[0].0, "你");
    assert_eq!(pairs[4].0, "Jacob");
    assert_eq!(pairs[4].1, "Jacob");
}

#[test]
fn alignment_handles_digits_as_one_syllable() {
    let pairs = align("我今年五十二歲。", "ngo5 gam1 nin4 ng5 sap6 ji6 seoi3");
    assert_eq!(pairs.len(), 7);
    assert_eq!(pairs[6].0, "歲");
}

#[test]
fn jyutping_parses_initial_nucleus_coda_and_tone() {
    let s = parse_syllable("ngo5");
    assert_eq!(s.initial, "n");
    assert_eq!(s.nucleus, "go");
    assert_eq!(s.coda, "");
    assert_eq!(s.tone, 5);

    // `ng` is only ever syllabic, so `ngo5` must not parse as ng + o.
    let s = parse_syllable("ng5");
    assert_eq!(s.initial, "");
    assert_eq!(s.nucleus, "ng");
    assert_eq!(s.tone, 5);

    let s = parse_syllable("sap6");
    assert_eq!(s.initial, "s");
    assert_eq!(s.nucleus, "a");
    assert_eq!(s.coda, "p");
    assert_eq!(s.tone, 6);

    let s = parse_syllable("laan4");
    assert_eq!(s.initial, "l");
    assert_eq!(s.nucleus, "aa");
    assert_eq!(s.coda, "n");
    assert_eq!(s.tone, 4);

    // A Latin name is not a Jyutping syllable.
    let s = parse_syllable("Jacob");
    assert!(s.foreign);
}

#[test]
fn tone_four_run_lifts_to_tone_one() {
    let a = analyze(&phrase(2));
    assert_eq!(a.spoken_jyutping, "ngo5 zyu6 hai2 ho4 laan1");
    assert_eq!(a.syllables.len(), 5);
    let last = &a.syllables[4];
    assert_eq!(last.tone, 4);
    assert_eq!(last.spoken_tone, 1);
    assert!(last.lifted);
    assert!(a.strip_note.is_some());
}

#[test]
fn every_seed_phrase_analyses_cleanly() {
    for p in seed_phrases() {
        let a = analyze(&p);
        assert!(!a.syllables.is_empty(), "{} produced no syllables", p.id);
        assert!(!a.mistakes.is_empty(), "{} produced no mistakes", p.id);
        assert_eq!(a.drill.len(), 5, "{} produced {} passes", p.id, a.drill.len());
        assert!(!a.tones.is_empty(), "{}", p.id);
        for s in &a.syllables {
            if !s.foreign {
                assert!((1..=6).contains(&s.tone), "{} tone {}", p.id, s.tone);
                assert!(!s.anchor_sentence.is_empty());
            }
        }
    }
}

#[test]
fn tone_cards_cover_every_tone_used() {
    let a = analyze(&phrase(3));
    let tones: Vec<u8> = a.tones.iter().map(|t| t.tone).collect();
    assert!(tones.contains(&5));
    assert!(tones.contains(&1));
    assert!(tones.contains(&4));
    assert!(tones.contains(&6));
    assert!(tones.contains(&3));
}

#[test]
fn yin_tracks_a_synthetic_tone() {
    let sr = 48_000.0f32;
    let mut samples = Vec::with_capacity(sr as usize);
    for i in 0..(sr as usize) {
        let t = i as f32 / sr;
        samples.push(0.4 * (2.0 * std::f32::consts::PI * 220.0 * t).sin());
    }
    let contour = Contour::from_pcm(&samples, sr);
    assert!(
        (contour.median_f0 - 220.0).abs() < 3.0,
        "median was {}",
        contour.median_f0
    );
}

#[test]
fn vad_trims_leading_and_trailing_silence() {
    let sr = 16_000.0f32;
    let mut samples = vec![0.0f32; (sr * 0.4) as usize];
    for i in 0..(sr * 0.4) as usize {
        let t = i as f32 / sr;
        samples.push(0.4 * (2.0 * std::f32::consts::PI * 200.0 * t).sin());
    }
    samples.extend(std::iter::repeat(0.0f32).take((sr * 0.5) as usize));

    let info = vad::trim(&mut samples, sr);
    assert!(info.changed());
    assert!(info.trimmed_front_ms > 300.0);
    assert!(info.trimmed_back_ms > 400.0);
    assert!(info.kept_ms < 900.0);
}

#[test]
fn every_seed_syllable_carries_an_english_gloss() {
    // Every Chinese character used in the bundled phrasebook must have a
    // gloss, otherwise a learner meets a syllable they cannot read.
    let mut missing: Vec<String> = Vec::new();
    for p in seed_phrases() {
        for s in analyze(&p).syllables {
            if s.foreign {
                continue;
            }
            if s.en.is_empty() {
                missing.push(format!("{} ({}) in #{}", s.han, s.jyutping, p.id));
            }
        }
    }
    assert!(missing.is_empty(), "missing glosses: {missing:?}");
}

#[test]
fn glosses_are_short_enough_to_read_on_a_card() {
    for p in seed_phrases() {
        for s in analyze(&p).syllables {
            assert!(
                s.en.chars().count() <= 28,
                "gloss for {} is too long: {:?}",
                s.han,
                s.en
            );
            assert!(!s.en.contains('\n'), "gloss for {} contains a newline", s.han);
        }
    }
}
