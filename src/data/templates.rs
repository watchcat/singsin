//! The phrases that are built from the learner's own details.
//!
//! A template is a phrase with one hole in it: the name, the age, the
//! city, the job, the hobby, the years spent learning Cantonese, the
//! people they live with. Filling the hole is what turns a library of
//! a hundred generic sentences into a hundred sentences about the
//! learner — and because the filled-in phrase is still an ordinary
//! [`Phrase`], it still goes through the whole phonetic engine: the
//! tones, the traps, the mistakes to watch for, the five drill passes
//! and the pitch chart all come for free.
//!
//! Two sets of templates exist. The first rewrites four phrases the
//! bundled library already has — the ones that were about somebody else
//! (a Jacob in the Netherlands, 52 years old) — so the learner's own
//! introduction is true. The second adds a personal section: where they
//! live, where they are from, what they do, what they do for fun, who
//! they live with, and how long they have been learning.

use super::phrases::Phrase;
use super::profile::{Profile, Slot, SlotValue};

/// The category the personal phrases live in.
pub const ABOUT_ME: &str = "About me";

/// The first id used by a generated phrase.
///
/// The generated phrases sit far above the library's own ids so that they
/// can never be confused with it — and so that they can be recognised
/// and left out of what is written back to the browser's storage.
pub const GENERATED_ID_BASE: i64 = 10_000;

/// A phrase with a hole in it.
pub struct Template {
    /// The id the phrase keeps, so that "mark as practised" survives.
    pub id: i64,
    /// The category it belongs to.
    pub category: &'static str,
    /// The English, with `{slot}` where the detail goes.
    pub en: &'static str,
    /// The Chinese, with `{slot}` where the detail goes.
    pub han: &'static str,
    /// Its Jyutping, with `{slot}` where the detail goes.
    pub jyutping: &'static str,
    /// The spoken Jyutping, for the templates where it differs.
    pub spoken_jyutping: Option<&'static str>,
    /// Which detail fills the hole.
    pub slot: Slot,
}

impl Template {
    /// Build the phrase this template describes, filled with `value`.
    pub fn build(&self, value: &SlotValue) -> Phrase {
        // A number in the twenties is said 廿, not 十: a page writes
        // 二十一, but a Hong Kong speaker says 廿一. The sentence is built
        // with the spoken form, because that is the form the learner will
        // hear and the one they should say — and because it keeps the
        // engine's sandhi honest. The character 一 is still there, so the
        // rule that turns 一 into *jat6* before 歲 or 年 still fires, and
        // the note on the phrase points out the difference.
        let (han_value, jyutping_value) = if value.spoken_han == value.han {
            (&value.han, &value.jyutping)
        } else {
            (&value.spoken_han, &value.spoken_jyutping)
        };

        let mut han = self.han.replace("{slot}", han_value);
        let jyutping = self.jyutping.replace("{slot}", jyutping_value);

        // The library writes a Latin name with a space before it — "你好，
        // 我叫 Jacob。" — because the name is not part of the Chinese.
        // A Chinese name is not separated.
        //
        // Only the Chinese gets the extra space. The Jyutping already has
        // one: the template reads "giu3 {slot}", and the name's reading is
        // a separate syllable whether it is Chinese or English, so
        // "Jacob" gives "giu3 Jacob" and 家明 gives "giu3 gaai1 ming4".
        if self.slot == Slot::Name && value.han.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        {
            han = han.replace(&value.han, &format!(" {}", value.han));
        }

        let spoken = self
            .spoken_jyutping
            .map(|s| s.replace("{slot}", &value.spoken_jyutping))
            .filter(|s| *s != jyutping);

        Phrase {
            id: self.id,
            category: self.category.to_string(),
            en: self.en.replace("{slot}", &value.en),
            han,
            jyutping,
            spoken_jyutping: spoken,
            notes: value.note(),
            rich: None,
        }
    }
}

/// The four phrases of the bundled library that were about somebody else.
///
/// They keep their ids, their place in the "Introductions" group and their
/// position in the list — only the details inside them change.
pub const SEED: &[Template] = &[
    Template {
        id: 1,
        category: "Introductions",
        en: "Hello, I'm {slot}.",
        han: "你好，我叫{slot}。",
        jyutping: "nei5 hou2 ngo5 giu3 {slot}",
        spoken_jyutping: None,
        slot: Slot::Name,
    },
    Template {
        id: 2,
        category: "Introductions",
        en: "I live in {slot}.",
        han: "我住喺{slot}。",
        jyutping: "ngo5 zyu6 hai2 {slot}",
        spoken_jyutping: None,
        slot: Slot::Country,
    },
    Template {
        id: 3,
        category: "Introductions",
        en: "I'm {slot} years old.",
        han: "我今年{slot}歲。",
        jyutping: "ngo5 gam1 nin4 {slot} seoi3",
        spoken_jyutping: None,
        slot: Slot::Age,
    },
    Template {
        id: 5,
        category: "Introductions",
        en: "I'm from {slot}.",
        han: "我嚟自{slot}。",
        jyutping: "ngo5 lai4 zi6 {slot}",
        spoken_jyutping: None,
        slot: Slot::Country,
    },
];

/// The personal section: the sentences a learner actually wants to say.
pub const PERSONAL: &[Template] = &[
    Template {
        id: GENERATED_ID_BASE + 1,
        category: ABOUT_ME,
        en: "I live in {slot}.",
        han: "我住喺{slot}。",
        jyutping: "ngo5 zyu6 hai2 {slot}",
        spoken_jyutping: None,
        slot: Slot::City,
    },
    Template {
        id: GENERATED_ID_BASE + 2,
        category: ABOUT_ME,
        en: "My hometown is {slot}.",
        han: "我老家喺{slot}。",
        jyutping: "ngo5 lou5 gaa1 hai2 {slot}",
        spoken_jyutping: None,
        slot: Slot::City,
    },
    Template {
        id: GENERATED_ID_BASE + 3,
        category: ABOUT_ME,
        en: "I go to work as {slot}.",
        han: "我返工做{slot}。",
        jyutping: "ngo5 faan1 gung1 zou6 {slot}",
        spoken_jyutping: None,
        slot: Slot::Job,
    },
    Template {
        id: GENERATED_ID_BASE + 4,
        category: ABOUT_ME,
        en: "I like {slot}.",
        han: "我鍾意{slot}。",
        jyutping: "ngo5 zung1 ji3 {slot}",
        spoken_jyutping: None,
        slot: Slot::Hobby,
    },
    Template {
        id: GENERATED_ID_BASE + 5,
        category: ABOUT_ME,
        en: "My family is in {slot}.",
        han: "我屋企人喺{slot}。",
        jyutping: "ngo5 uk1 kei2 jan4 hai2 {slot}",
        spoken_jyutping: None,
        slot: Slot::Country,
    },
    Template {
        id: GENERATED_ID_BASE + 6,
        category: ABOUT_ME,
        en: "I have been learning Cantonese for {slot} years.",
        han: "我學廣東話學咗{slot}年。",
        jyutping: "ngo5 hok6 gwong2 dung1 waa6 hok6 zo2 {slot} nin4",
        spoken_jyutping: None,
        slot: Slot::Years,
    },
    Template {
        id: GENERATED_ID_BASE + 7,
        category: ABOUT_ME,
        en: "I live with my {slot}.",
        han: "我同{slot}一齊住。",
        jyutping: "ngo5 tung4 {slot} jat1 cai4 zyu6",
        spoken_jyutping: None,
        slot: Slot::Family,
    },
];

/// The template that fills a given phrase id, if there is one.
pub fn seed_template(id: i64) -> Option<&'static Template> {
    SEED.iter().find(|t| t.id == id)
}

/// Build every phrase the profile can fill.
///
/// The four library phrases come first, in their own order; the personal
/// section follows. A template whose slot the learner has not filled in
/// is left out entirely — there is no half-written sentence.
pub fn build_all(profile: &Profile) -> Vec<Phrase> {
    let mut out = Vec::new();
    for template in SEED.iter().chain(PERSONAL.iter()) {
        if let Some(value) = profile.value(template.slot) {
            out.push(template.build(&value));
        }
    }
    out
}
