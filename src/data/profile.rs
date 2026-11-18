//! The learner's own details.
//!
//! A profile is what the learner tells the app about themselves: their
//! name, their age, where they are from, what they do, what they do for
//! fun. The phrases in the library are then built from it, so that the
//! sentences a learner practises are *their* sentences — and every one of
//! them still goes through the same phonetic engine as the rest of the
//! library, with its tones, its traps and its five drill passes.
//!
//! Nothing here guesses. A city or a job is looked up in the curated
//! tables (`tables.rs`), which were checked against words.hk; a name and
//! an age are built from rules rather than from a dictionary. A detail
//! the app does not know is simply not offered.

use serde::{Deserialize, Serialize};

use super::numbers::{han_number, han_number_spoken};
use super::tables;

/// The parts of a profile a phrase can be built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// The learner's name.
    Name,
    /// Their age.
    Age,
    /// How many years they have been learning Cantonese.
    Years,
    /// The city they live in.
    City,
    /// The country they are from.
    Country,
    /// What they do for a living.
    Job,
    /// What they do for fun.
    Hobby,
    /// The people they live with.
    Family,
}

/// What a slot contributes to a phrase.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotValue {
    /// The English, as it appears in the phrase: "a teacher", "35".
    pub en: String,
    /// The Chinese, or the Latin name.
    pub han: String,
    /// Its Jyutping. For a Latin name this is the name itself: Cantonese
    /// says an English name in English, and the engine treats it as one
    /// foreign syllable.
    pub jyutping: String,
    /// The Chinese as it is spoken, when it differs from the written
    /// form: 廿一 for 二十一.
    pub spoken_han: String,
    /// The spoken Jyutping, when it differs from the written one. Only
    /// the numbers ever do: 21 is written 二十一 but spoken 廿一.
    pub spoken_jyutping: String,
}

impl SlotValue {
    /// A value whose spoken form is the written form.
    fn plain(en: &str, han: &str, jyutping: &str) -> Self {
        SlotValue {
            en: en.to_string(),
            han: han.to_string(),
            jyutping: jyutping.to_string(),
            spoken_han: han.to_string(),
            spoken_jyutping: jyutping.to_string(),
        }
    }

    /// A note for the phrase this value fills, when there is something
    /// worth saying about it.
    ///
    /// Only the numbers have one: the twenties are written with 十 and
    /// spoken with 廿, and a learner who has never heard 廿 will not
    /// guess it from the page. The sentence itself is written the way it
    /// is spoken, so the note is where the two forms meet.
    pub fn note(&self) -> Option<String> {
        if self.spoken_han == self.han {
            return None;
        }
        Some(format!(
            "Your own number: a page writes {} (*{}*), but Cantonese says {} (*{}*) — in the \
             twenties Cantonese says 廿 for twenty. The sentence is written the way it is said, \
             because that is the form you will hear.",
            self.han, self.jyutping, self.spoken_han, self.spoken_jyutping
        ))
    }
}

/// What the learner told the app about themselves.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// A name written in Latin letters, said in English: "Jacob".
    #[serde(default)]
    pub name_latin: String,

    /// A name written in Chinese, if the learner has one: "家明".
    #[serde(default)]
    pub name_han: String,

    /// Its Jyutping. The learner can leave this empty and let the LLM
    /// guess it; whatever comes back can be corrected before it is saved.
    #[serde(default)]
    pub name_jyutping: String,

    /// How old the learner is.
    #[serde(default)]
    pub age: Option<u32>,

    /// How many years they have been learning Cantonese.
    #[serde(default)]
    pub years_learning: Option<u32>,

    /// A key into [`tables::CITIES`].
    #[serde(default)]
    pub city: Option<String>,

    /// A key into [`tables::COUNTRIES`].
    #[serde(default)]
    pub country: Option<String>,

    /// A key into [`tables::JOBS`].
    #[serde(default)]
    pub job: Option<String>,

    /// A key into [`tables::HOBBIES`].
    #[serde(default)]
    pub hobby: Option<String>,

    /// A key into [`tables::FAMILY`].
    #[serde(default)]
    pub family: Option<String>,
}

impl Profile {
    /// Whether the learner has told us anything at all.
    pub fn is_empty(&self) -> bool {
        self.name_latin.trim().is_empty()
            && self.name_han.trim().is_empty()
            && self.age.is_none()
            && self.years_learning.is_none()
            && self.city.is_none()
            && self.country.is_none()
            && self.job.is_none()
            && self.hobby.is_none()
            && self.family.is_none()
    }

    /// The name to use in a sentence.
    ///
    /// A Chinese name is preferred when the learner gave one, because
    /// that is the name they would be called in Chinese; otherwise the
    /// Latin name is used, which Cantonese leaves in English.
    pub fn name(&self) -> Option<SlotValue> {
        let han = self.name_han.trim();
        let jp = self.name_jyutping.trim();
        if !han.is_empty() && !jp.is_empty() {
            return Some(SlotValue::plain(han, han, jp));
        }
        let latin = self.name_latin.trim();
        if !latin.is_empty() {
            return Some(SlotValue::plain(latin, latin, latin));
        }
        None
    }

    /// The age, written and spoken.
    ///
    /// The two differ only in the twenties, where the spoken form uses 廿
    /// for twenty: 二十一 is written, 廿一 is said.
    fn age(&self) -> Option<SlotValue> {
        let age = self.age?;
        let (han, jyutping) = han_number(age);
        let (spoken_han, spoken_jyutping) = han_number_spoken(age);
        Some(SlotValue {
            en: age.to_string(),
            han,
            jyutping,
            spoken_han,
            spoken_jyutping,
        })
    }

    /// The years spent learning Cantonese, written and spoken.
    fn years(&self) -> Option<SlotValue> {
        let years = self.years_learning?;
        let (han, jyutping) = han_number(years);
        let (spoken_han, spoken_jyutping) = han_number_spoken(years);
        Some(SlotValue {
            en: years.to_string(),
            han,
            jyutping,
            spoken_han,
            spoken_jyutping,
        })
    }

    /// The value a slot contributes, or `None` when the learner has not
    /// filled that part in.
    pub fn value(&self, slot: Slot) -> Option<SlotValue> {
        match slot {
            Slot::Name => self.name(),
            Slot::Age => self.age(),
            Slot::Years => self.years(),
            Slot::City => self.city.as_deref().and_then(|k| tables::find(tables::CITIES, k)).map(table_value),
            Slot::Country => self.country.as_deref().and_then(|k| tables::find(tables::COUNTRIES, k)).map(table_value),
            Slot::Job => self.job.as_deref().and_then(|k| tables::find(tables::JOBS, k)).map(table_value),
            Slot::Hobby => self.hobby.as_deref().and_then(|k| tables::find(tables::HOBBIES, k)).map(table_value),
            Slot::Family => self.family.as_deref().and_then(|k| tables::find(tables::FAMILY, k)).map(table_value),
        }
    }
}

/// A table entry as a slot value.
fn table_value(entry: &tables::Entry) -> SlotValue {
    SlotValue::plain(entry.label, entry.han, entry.jyutping)
}
