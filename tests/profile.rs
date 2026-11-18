//! Tests for the learner's own details: the numbers, the curated tables,
//! and the sentences built from them.

use singsin::data::gloss;
use singsin::data::numbers::{han_number, han_number_spoken};
use singsin::data::personal::{for_export, for_import, for_storage, personalise};
use singsin::data::phonetics;
use singsin::data::phrases::{seed_phrases, Phrase};
use singsin::data::profile::{Profile, Slot};
use singsin::data::tables::{self, Entry};
use singsin::data::templates;

/// Every table, so that the checks below run over all of them.
fn all_tables() -> Vec<&'static [Entry]> {
    vec![
        tables::COUNTRIES,
        tables::CITIES,
        tables::JOBS,
        tables::HOBBIES,
        tables::FAMILY,
    ]
}

// ── the numbers ─────────────────────────────────────────────────────────────

#[test]
fn the_age_of_the_bundled_library_is_written_the_same_way() {
    // The bundled phrase says 我今年五十二歲。 with *ng5 sap6 ji6* for
    // 五十二. The number must be written the same way, or personalising
    // the phrase would quietly change a phrase that was already right.
    let (han, jyutping) = han_number(52);
    assert_eq!(han, "五十二");
    assert_eq!(jyutping, "ng5 sap6 ji6");
}

#[test]
fn numbers_are_written_the_way_they_are_read() {
    assert_eq!(han_number(0).0, "零");
    assert_eq!(han_number(7).1, "cat1");
    assert_eq!(han_number(10).0, "十");
    assert_eq!(han_number(15).1, "sap6 ng5");
    assert_eq!(han_number(21).1, "ji6 sap6 jat1");
    assert_eq!(han_number(35).1, "saam1 sap6 ng5");
    assert_eq!(han_number(52).1, "ng5 sap6 ji6");
    assert_eq!(han_number(99).1, "gau2 sap6 gau2");
}

#[test]
fn the_twenties_are_spoken_with_the_short_form() {
    // 廿 stands for twenty, so 廿一 is *ng6 jat1* and not *ji6 sap6 jat1*.
    let (han, jyutping) = han_number_spoken(21);
    assert_eq!(han, "廿一");
    assert_eq!(jyutping, "ng6 jat1");

    // The twenties are the only ones that differ.
    assert_eq!(han_number_spoken(20).0, "廿");
    assert_eq!(han_number_spoken(35), han_number(35));
    assert_eq!(han_number_spoken(52), han_number(52));
}

#[test]
fn a_number_never_loses_a_syllable() {
    for n in 0..=99u32 {
        let (han, jyutping) = han_number(n);
        assert_eq!(
            han.chars().count(),
            jyutping.split_whitespace().count(),
            "{n} wrote {han} / {jyutping}"
        );
    }
}

// ── the curated tables ─────────────────────────────────────────────────────

#[test]
fn every_table_entry_has_one_syllable_for_every_character() {
    for table in all_tables() {
        for entry in table.iter() {
            let syllables = entry.jyutping.split_whitespace().count();
            let characters = entry.han.chars().count();
            assert_eq!(
                characters, syllables,
                "{} ({}) has {} characters and {} syllables",
                entry.key, entry.han, characters, syllables
            );
        }
    }
}

#[test]
fn every_character_in_the_tables_has_an_english_gloss() {
    for table in all_tables() {
        for entry in table.iter() {
            for ch in entry.han.chars() {
                let c = ch.to_string();
                assert!(
                    gloss::gloss(&c).is_some(),
                    "{} ({}) has no English gloss for {c}",
                    entry.key,
                    entry.han
                );
            }
        }
    }
}

#[test]
fn every_table_entry_parses_as_jyutping() {
    use singsin::data::phonetics::parse_syllable;
    for table in all_tables() {
        for entry in table.iter() {
            for token in entry.jyutping.split_whitespace() {
                let syllable = parse_syllable(token);
                assert!(
                    !syllable.foreign && (1..=6).contains(&syllable.tone),
                    "{} has a syllable that does not parse: {token}",
                    entry.key
                );
            }
        }
    }
}

#[test]
fn the_keys_of_the_tables_are_unique() {
    for table in all_tables() {
        let mut keys: Vec<&str> = table.iter().map(|e| e.key).collect();
        let count = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), count, "a table has the same key twice");
    }
}

#[test]
fn the_tables_hold_the_words_they_should() {
    // A few anchors, so that an accidental edit to the data is caught.
    assert!(tables::find(tables::COUNTRIES, "netherlands").is_some());
    assert!(tables::find(tables::CITIES, "moscow").is_some());
    assert!(tables::find(tables::JOBS, "teacher").is_some());
    assert!(tables::find(tables::HOBBIES, "films").is_some());
    assert!(tables::find(tables::FAMILY, "wife").is_some());
    // And a word that is not in any table must not be found.
    assert!(tables::find(tables::CITIES, "atlantis").is_none());
}

// ── the sentences built from them ──────────────────────────────────────────

/// The profile the bundled library was written for.
fn default_profile() -> Profile {
    Profile {
        name_latin: "Jacob".into(),
        age: Some(52),
        country: Some("netherlands".into()),
        ..Default::default()
    }
}

fn phrase_with_id(phrases: &[Phrase], id: i64) -> Phrase {
    phrases
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("no phrase {id}"))
        .clone()
}

#[test]
fn the_bundled_library_is_unchanged_for_its_own_details() {
    // The bundled phrases were written about a Jacob in the Netherlands
    // who is 52. A profile that says the same must not change a single
    // character of the library.
    let seed = seed_phrases();
    let library = personalise(&seed, None, &default_profile());

    for original in &seed {
        let built = phrase_with_id(&library.display, original.id);
        assert_eq!(
            built.han, original.han,
            "phrase {} changed: {} became {}",
            original.id, original.han, built.han
        );
        assert_eq!(built.jyutping, original.jyutping, "phrase {} changed", original.id);
    }
}

#[test]
fn the_details_replace_the_bundled_ones() {
    let seed = seed_phrases();
    let profile = Profile {
        name_latin: "Marta".into(),
        age: Some(35),
        country: Some("poland".into()),
        ..Default::default()
    };
    let library = personalise(&seed, None, &profile);

    let hello = phrase_with_id(&library.display, 1);
    assert!(hello.han.contains("Marta"), "{}", hello.han);
    assert!(hello.en.contains("Marta"));

    let age = phrase_with_id(&library.display, 3);
    assert!(age.han.contains("三十五"), "{}", age.han);
    assert_eq!(age.jyutping, "ngo5 gam1 nin4 saam1 sap6 ng5 seoi3");

    let home = phrase_with_id(&library.display, 2);
    assert!(home.han.contains("波蘭"), "{}", home.han);
    assert_eq!(home.jyutping, "ngo5 zyu6 hai2 bo1 laan4");
}

#[test]
fn the_personal_section_only_appears_when_it_can_be_filled() {
    let seed = seed_phrases();
    let profile = Profile {
        city: Some("berlin".into()),
        job: Some("teacher".into()),
        ..Default::default()
    };
    let library = personalise(&seed, None, &profile);

    let city = library
        .display
        .iter()
        .find(|p| p.han.contains("我住喺柏林"))
        .expect("the city phrase is missing");
    assert_eq!(city.jyutping, "ngo5 zyu6 hai2 paak3 lam4");
    assert_eq!(city.category, templates::ABOUT_ME);

    let job = library
        .display
        .iter()
        .find(|p| p.han.contains("我返工做教師"))
        .expect("the job phrase is missing");
    assert_eq!(job.jyutping, "ngo5 faan1 gung1 zou6 gaau3 si1");
    assert!(job.en.contains("a teacher"), "{}", job.en);

    // No hobby, no family, no years: those phrases are simply not there.
    assert!(!library.display.iter().any(|p| p.han.contains("鍾意")));
    assert!(!library.display.iter().any(|p| p.han.contains("一齊住")));
    assert!(!library.display.iter().any(|p| p.han.contains("學咗")));
}

#[test]
fn a_learners_own_phrase_is_never_overwritten() {
    let seed = seed_phrases();
    let mut own = seed.clone();
    // The learner edits the greeting by hand.
    for p in own.iter_mut() {
        if p.id == 1 {
            p.han = "你好，我叫 Marta。".into();
            p.jyutping = "nei5 hou2 ngo5 giu3 maa5 taa1".into();
        }
    }

    let profile = Profile {
        name_latin: "Jacob".into(),
        ..Default::default()
    };
    let library = personalise(&seed, Some(&own), &profile);
    let hello = phrase_with_id(&library.display, 1);
    assert_eq!(hello.han, "你好，我叫 Marta。");
}

#[test]
fn a_deleted_phrase_stays_deleted() {
    let seed = seed_phrases();
    let own: Vec<Phrase> = seed.iter().filter(|p| p.id != 3).cloned().collect();
    let profile = default_profile();
    let library = personalise(&seed, Some(&own), &profile);
    assert!(!library.display.iter().any(|p| p.id == 3), "the deleted phrase came back");
}

#[test]
fn the_generated_phrases_are_not_stored() {
    let seed = seed_phrases();
    let profile = Profile {
        city: Some("berlin".into()),
        ..Default::default()
    };
    let library = personalise(&seed, None, &profile);

    // The learner edits something else; the list that goes into storage
    // must not carry the generated phrases, because they are rebuilt from
    // the profile every time.
    let mut own = library.display.clone();
    for p in own.iter_mut() {
        if p.id == 7 {
            p.han = "唔該。".into();
        }
    }
    let stored = for_storage(&Some(own)).expect("a list to store");
    assert!(!stored.iter().any(|p| p.id >= 10_000));
    assert!(stored.iter().any(|p| p.id == 7));
}

#[test]
fn a_chinese_name_is_used_with_the_reading_it_was_given() {
    let seed = seed_phrases();
    let profile = Profile {
        name_han: "家明".into(),
        name_jyutping: "gaai1 ming4".into(),
        ..Default::default()
    };
    let library = personalise(&seed, None, &profile);
    let hello = phrase_with_id(&library.display, 1);
    assert!(hello.han.contains("家明"), "{}", hello.han);
    assert_eq!(hello.jyutping, "nei5 hou2 ngo5 giu3 gaai1 ming4");
    // A Chinese name is not separated from the sentence by a space, the
    // way a Latin one is.
    assert!(!hello.han.contains(" 家明"), "{}", hello.han);
}

#[test]
fn a_profile_with_nothing_in_it_leaves_the_library_alone() {
    let seed = seed_phrases();
    let library = personalise(&seed, None, &Profile::default());
    assert_eq!(library.display.len(), seed.len());
    for original in &seed {
        assert_eq!(phrase_with_id(&library.display, original.id).han, original.han);
    }
}

#[test]
fn every_slot_the_profile_fills_has_a_value() {
    let profile = Profile {
        name_latin: "Jacob".into(),
        age: Some(52),
        years_learning: Some(2),
        city: Some("berlin".into()),
        country: Some("germany".into()),
        job: Some("doctor".into()),
        hobby: Some("music".into()),
        family: Some("wife".into()),
        ..Default::default()
    };
    for slot in [
        Slot::Name,
        Slot::Age,
        Slot::Years,
        Slot::City,
        Slot::Country,
        Slot::Job,
        Slot::Hobby,
        Slot::Family,
    ] {
        assert!(profile.value(slot).is_some(), "{slot:?} has no value");
    }
}

#[test]
fn a_detail_that_is_not_in_the_tables_has_no_value() {
    let profile = Profile {
        city: Some("atlantis".into()),
        ..Default::default()
    };
    assert!(profile.value(Slot::City).is_none());
}

// ── taking the phrasebook away, and bringing one back ───────────────────────

#[test]
fn the_export_carries_the_learners_own_phrasebook_and_not_the_personalisation() {
    // A file the learner takes away must not carry their details. The
    // personalised sentences belong to the profile, which lives in this
    // browser, and a file with somebody else's name in it would freeze
    // that greeting in the next learner's library.
    let seed = seed_phrases();

    // With no phrasebook of their own, the export is the bundled library.
    assert_eq!(for_export(&None).len(), seed.len());

    // With one, it is their own — and the sentences the profile fills in
    // are in the bundled wording, because that is how they are stored.
    let exported = for_export(&Some(seed.clone()));
    let greeting = exported.iter().find(|p| p.id == 1).expect("no greeting");
    assert_eq!(greeting.han, "你好，我叫 Jacob。");
}

#[test]
fn an_imported_phrasebook_does_not_freeze_somebody_elses_details() {
    // A phrasebook exported from a browser where the learner had filled the
    // form in carries that learner's name, and the generated sentences too.
    // Importing it must not put that name in the importer's greeting, and
    // must not freeze it there as if the importer had written it by hand.
    let seed = seed_phrases();

    let mut theirs = seed[0].clone();
    theirs.han = "你好，我叫 Marta。".into();
    theirs.en = "Hello, I'm Marta.".into();
    theirs.jyutping = "nei5 hou2 ngo5 giu3 Marta".into();

    let mut generated = seed[0].clone();
    generated.id = 10_001;
    generated.han = "我返工做工程師。".into();

    let imported = for_import(vec![theirs, generated]);
    assert_eq!(imported.len(), 1, "the generated phrase should not be imported");
    assert_eq!(imported[0].han, "你好，我叫 Jacob。");

    // And the importer's own details still fill the sentence in.
    let profile = Profile {
        name_latin: "Sofia".into(),
        ..Default::default()
    };
    let library = personalise(&seed, Some(&imported), &profile);
    let hello = phrase_with_id(&library.display, 1);
    assert_eq!(hello.han, "你好，我叫 Sofia。");
}

// ── the sentences as they are spoken ────────────────────────────────────────

#[test]
fn a_number_in_the_twenties_is_written_the_way_it_is_said() {
    // A page writes 二十一, but a Hong Kong speaker says 廿一 — and the
    // sentence is built with the spoken form, because that is the form the
    // learner will hear and the one they should say.
    let seed = seed_phrases();
    let profile = Profile {
        age: Some(21),
        years_learning: Some(21),
        ..Default::default()
    };
    let library = personalise(&seed, None, &profile);

    let age = phrase_with_id(&library.display, 3);
    assert_eq!(age.han, "我今年廿一歲。");
    assert_eq!(age.jyutping, "ngo5 gam1 nin4 ng6 jat1 seoi3");

    let years = library
        .display
        .iter()
        .find(|p| p.han.contains("學咗"))
        .expect("the years phrase is missing");
    assert_eq!(years.han, "我學廣東話學咗廿一年。");
    assert_eq!(
        years.jyutping,
        "ngo5 hok6 gwong2 dung1 waa6 hok6 zo2 ng6 jat1 nin4"
    );

    // The note is where the two forms meet.
    let note = age.notes.clone().expect("the age phrase has no note");
    assert!(note.contains("二十一"), "{note}");
    assert!(note.contains("廿一"), "{note}");
}

#[test]
fn the_engine_still_applies_the_sandhi_inside_the_spoken_number() {
    // The character 一 is still there in 廿一, so the rule that turns 一
    // into *jat6* before 歲 or 年 still fires. This is the whole reason
    // the sentence is written the way it is spoken.
    let seed = seed_phrases();
    let profile = Profile {
        age: Some(21),
        years_learning: Some(21),
        ..Default::default()
    };
    let library = personalise(&seed, None, &profile);

    let age = phrase_with_id(&library.display, 3);
    let analysis = phonetics::analyze(&age);
    assert_eq!(analysis.spoken_jyutping, "ngo5 gam1 nin4 ng6 jat6 seoi3");

    let years = library
        .display
        .iter()
        .find(|p| p.han.contains("學咗"))
        .expect("the years phrase is missing");
    let analysis = phonetics::analyze(years);
    assert_eq!(
        analysis.spoken_jyutping,
        "ngo5 hok6 gwong2 dung1 waa6 hok6 zo2 ng6 jat6 nin4"
    );
}

#[test]
fn a_personalised_phrase_does_not_keep_the_bundled_notes() {
    // The curated notes for phrase 2 are about the Netherlands: its spoken
    // Jyutping says *ho4 laan1* and its mistakes talk about 蘭. A learner
    // who changes the country must not be handed the Netherlands' notes.
    let seed = seed_phrases();
    let profile = Profile {
        country: Some("poland".into()),
        ..Default::default()
    };
    let library = personalise(&seed, None, &profile);

    let home = phrase_with_id(&library.display, 2);
    assert_eq!(home.han, "我住喺波蘭。");
    let analysis = phonetics::analyze(&home);
    assert!(
        !analysis.spoken_jyutping.contains("laan1"),
        "the Netherlands' spoken form leaked into {}: {}",
        home.han,
        analysis.spoken_jyutping
    );

    // The bundled phrase keeps them: the same words, the same notes.
    let bundled = phrase_with_id(&seed, 2);
    let analysis = phonetics::analyze(&bundled);
    assert_eq!(analysis.spoken_jyutping, "ngo5 zyu6 hai2 ho4 laan1");
}
