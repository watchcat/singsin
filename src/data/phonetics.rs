//! Deterministic Cantonese phonetic engine.
//!
//! Everything here is derived from the Jyutping string itself, so a phrase
//! that the user adds or edits in the UI immediately gets a full deep-dive:
//! Chao contours, English sound anchors, phonetic traps, minimal pairs,
//! a mistakes table and a five-pass drill — no network round-trip needed.
//!
//! A curated override table additionally supplies verbatim notes for the
//! headline phrases of the bundled library.

use serde::{Deserialize, Serialize};

use super::phrases::{align, seed_spoken_jyutping, Phrase};

/// A "watch out for this" callout attached to a syllable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trap {
    pub title: String,
    pub body: String,
}

/// A clickable minimal pair chip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MinimalPair {
    pub han: String,
    pub jyutping: String,
    pub note: String,
}

/// One row of the "mistakes to watch for" table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MistakeRow {
    pub wrong: String,
    pub right: String,
    pub why: String,
}

/// One pass of the five-pass practice drill.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrillPass {
    pub n: u8,
    pub title: String,
    pub body: String,
    pub focus: Vec<String>,
}

/// A tone key card (Chao number + shape + example).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToneInfo {
    pub tone: u8,
    pub chao: String,
    pub name: String,
    pub shape: String,
    pub example_han: String,
    pub example_jyutping: String,
}

/// A fully analysed syllable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Syllable {
    pub index: usize,
    pub han: String,
    pub jyutping: String,
    pub initial: String,
    pub onset: String,
    pub nucleus: String,
    pub coda: String,
    pub tone: u8,
    pub spoken_tone: u8,
    pub lifted: bool,
    pub chao: String,
    pub spoken_chao: String,
    pub initial_anchor: String,
    pub final_anchor: String,
    pub anchor_sentence: String,
    pub movement: String,
    pub traps: Vec<Trap>,
    pub minimal_pairs: Vec<MinimalPair>,
    pub foreign: bool,
    /// A short English gloss of this syllable — "I / me", "at / in",
    /// "orchid". Empty when the character is not in the glossary.
    pub en: String,
}

/// Everything the UI needs to render one phrase.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Analysis {
    pub syllables: Vec<Syllable>,
    pub tones: Vec<ToneInfo>,
    pub mistakes: Vec<MistakeRow>,
    pub drill: Vec<DrillPass>,
    pub takeaway: String,
    pub tagline: String,
    pub strip_note: Option<String>,
    pub spoken_jyutping: String,
}

// ─────────────────────────────���────────────────────────────────────────────
//  Jyutping parsing
// ───────────────────────────────────────────────────────────────────────────

/// Two-letter initials. `ng` is *not* here on purpose: in Jyutping `ng` only
/// ever occurs as the syllabic nasal (`ng5`), so `ngo5` must parse as
/// `n` + `go` rather than `ng` + `o`.
const INITIALS_2: &[&str] = &["ch", "gw", "kw"];
const INITIALS_1: &[&str] = &[
    "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "s", "t", "w", "z",
];

/// A parsed Jyutping syllable.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    /// The written initial, e.g. `n` in `ngo5`.
    pub initial: String,
    /// The onset as it is actually pronounced — `ng` for the back-nasal
    /// syllables such as 我 `ngo5` and 五 `ng5`, otherwise the initial.
    pub onset: String,
    pub nucleus: String,
    pub coda: String,
    pub tone: u8,
    /// Whether the onset is the back-nasal `ng`.
    pub back_nasal: bool,
    pub foreign: bool,
}

/// Split a Jyutping token into initial / nucleus / coda / tone.
pub fn parse_syllable(token: &str) -> Parsed {
    let t = token.trim();
    if t.is_empty() {
        return Parsed {
            initial: String::new(),
            onset: String::new(),
            nucleus: String::new(),
            coda: String::new(),
            tone: 0,
            back_nasal: false,
            foreign: true,
        };
    }

    // Latin / non-Jyutping token (e.g. a person's name): keep it verbatim.
    let mut body = t;
    let mut tone = 0u8;
    if let Some(last) = t.chars().last() {
        if let Some(d) = last.to_digit(10) {
            if (1..=6).contains(&d) {
                tone = d as u8;
                body = &t[..t.len() - last.len_utf8()];
            }
        }
    }

    if body.is_empty() || !body.chars().all(|c| c.is_ascii_lowercase()) {
        return Parsed {
            initial: String::new(),
            onset: String::new(),
            nucleus: body.to_string(),
            coda: String::new(),
            tone,
            back_nasal: false,
            foreign: true,
        };
    }

    // The syllabic nasals: 唔 `m4`, 五 `ng5`. These have no initial at all.
    if body == "m" || body == "ng" {
        return Parsed {
            initial: String::new(),
            onset: body.to_string(),
            nucleus: body.to_string(),
            coda: String::new(),
            tone,
            back_nasal: body == "ng",
            foreign: false,
        };
    }

    // ── initial ──────────────────────────────────────────────────────────
    let mut initial = String::new();
    if let Some(i2) = INITIALS_2.iter().find(|i| body.starts_with(**i)) {
        initial.push_str(i2);
    } else if let Some(i1) = INITIALS_1.iter().find(|i| body.starts_with(**i)) {
        initial.push_str(i1);
    }
    let rest = &body[initial.len()..];

    // ── nucleus + coda ───────────────────────────────────────────────────
    let (nucleus, coda) = split_rime(rest);

    // `ngo`, `ngaa`, `ngan` … are written with an `n` but are pronounced with
    // the back-nasal `ng` — the single most audible marker of a Cantonese
    // speaker, and the trap every Mandarin speaker falls into.
    let back_nasal = initial == "n" && nucleus.starts_with('g');

    Parsed {
        onset: if back_nasal {
            "ng".to_string()
        } else {
            initial.clone()
        },
        initial,
        nucleus,
        coda,
        tone,
        back_nasal,
        foreign: false,
    }
}

/// Split a rime into its nucleus and (optional) final consonant.
fn split_rime(rest: &str) -> (String, String) {
    if rest.is_empty() {
        return (String::new(), String::new());
    }
    // Syllabic nasals: `m` (唔 m4), `ng` (五 ng5).
    if rest == "m" || rest == "ng" {
        return (rest.to_string(), String::new());
    }
    // `ng` codas first (ang, ing, ong, eng, oeng, aang …).
    if rest.len() > 2 && rest.ends_with("ng") {
        return (rest[..rest.len() - 2].to_string(), "ng".to_string());
    }
    // Stop / nasal codas: p t k m n.
    if rest.len() > 1 {
        let last = &rest[rest.len() - 1..];
        if matches!(last, "p" | "t" | "k" | "m" | "n") {
            return (rest[..rest.len() - 1].to_string(), last.to_string());
        }
    }
    (rest.to_string(), String::new())
}

// ───────────────────────────────────────────────────────────────────────────
//  Tone data
// ───────────────────────────────────────────────────────────────────────────

/// The Chao tone number for a Cantonese tone.
pub fn chao_of(tone: u8) -> &'static str {
    match tone {
        1 => "55",
        2 => "25",
        3 => "33",
        4 => "21",
        5 => "13",
        6 => "22",
        _ => "33",
    }
}

/// A short English description of the tone's movement.
pub fn tone_name(tone: u8) -> &'static str {
    match tone {
        1 => "high level",
        2 => "rising to high",
        3 => "mid level",
        4 => "falling to low",
        5 => "rising from low",
        6 => "low level",
        _ => "mid level",
    }
}

/// A concrete instruction on how to produce the tone.
pub fn tone_instruction(tone: u8) -> &'static str {
    match tone {
        1 => "Hold it high and completely flat.",
        2 => "Start at mid height, then let it rise all the way to the top.",
        3 => "Start at mid height and keep it flat — do not dip down and back up.",
        4 => "Start high and let it fall all the way to the bottom.",
        5 => "Start at the bottom and let it float upward, like a gentle question.",
        6 => "Keep it low and flat — no rise at the end.",
        _ => "",
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  English sound anchors
// ───────────────────────────────────────────────────────────────────────────

/// Anchor the initial consonant to an English word.
pub fn initial_anchor(initial: &str) -> &'static str {
    match initial {
        "" => "no consonant at all — begin straight on the vowel",
        "b" => "**b** as in *boy*",
        "p" => "**p** as in *put* — no puff of air",
        "m" => "**m** as in *my*",
        "f" => "**f** as in *fish*",
        "d" => "**d** as in *day*",
        "t" => "**t** as in *tea* — no puff of air",
        "n" => "**n** as in *no*",
        "l" => "**l** as in *light*",
        "g" => "**g** as in *go*",
        "k" => "**k** as in *cat* — no puff of air",
        "h" => "**h** as in *hat*",
        "gw" => "**gw** as in *guide* — round your lips as you say it",
        "kw" => "**kw** as in *quick* — round your lips as you say it",
        "z" => "**z** as in *zoo*",
        "c" => "**ts** as in *cats*",
        "s" => "**s** as in *see*",
        "j" => "**y** as in *yes*",
        "ch" => "**ch** as in *church*",
        "w" => "**w** as in *we*",
        "ng" => "**ng** as in *sing* — back of the tongue onto the soft palate",
        _ => "",
    }
}

/// Anchor the rime (vowel + coda) to an English word.
pub fn final_anchor(nucleus: &str, coda: &str) -> String {
    let base = match nucleus {
        "a" => "**ah** as in *father*",
        "aa" => "**ah** as in *car* — open and long",
        "ai" => "**eye** as in *my*",
        "aai" => "**eye** as in *my*",
        "au" => "**ow** as in *now*",
        "aau" => "**ow** as in *now*",
        "ak" => "**ak** as in *lock*",
        "am" => "**am** as in *bam*",
        "an" => "**an** as in *can*",
        "ang" => "**ang** as in *long*",
        "ap" => "**ap** as in *cup*",
        "aat" => "**aat** as in *cart*",
        "aam" => "**aam** as in *farm*",
        "aan" => "**aan** as in *ban*",
        "aang" => "**aang** as in *long*",
        "e" => "**eh** as in *bed*",
        "ei" => "**ay** as in *day*",
        "eng" => "**ung** as in *sung*",
        "ek" => "**ek** as in *check*",
        "em" => "**em** as in *them*",
        "eu" => "**ew** as in *few*",
        "i" => "**ee** as in *see*",
        "iu" => "**ew** as in *few*",
        "im" => "**im** as in *him*",
        "in" => "**in** as in *pin*",
        "ing" => "**ing** as in *sing*",
        "ib" => "**ib** as in *big*",
        "id" => "**id** as in *bid*",
        "ik" => "**ick** as in *pick*",
        "it" => "**it** as in *bit*",
        "o" => "**aw** as in *law*",
        "go" => "**aw** as in *law*",
        "gou" => "**ow** as in *cow*",
        "gaa" => "**ah** as in *car*",
        "gai" => "**eye** as in *my*",
        "gaan" => "**ahn** as in *van*",
        "gaam" => "**ahm** as in *farm*",
        "gan" => "**ahn** as in *van*",
        "gang" => "**ang** as in *long*",
        "gap" => "**ap** as in *cup*",
        "gaat" => "**aat** as in *cart*",
        "gong" => "**ong** as in *song*",
        "oi" => "**oy** as in *boy*",
        "eoi" => "**oy** as in *boy*",
        "uk" => "**ook** as in *book*",
        "wai" => "**eye** as in *my*",
        "wong" => "**ong** as in *song*",
        "ou" => "**ow** as in *cow*",
        "ong" => "**ong** as in *song*",
        "ok" => "**ok** as in *bought*",
        "om" => "**om** as in *bom*",
        "ot" => "**ot** as in *hot*",
        "oek" => "**oek** as in *puck*",
        "oeng" => "**oeng** as in *sung*",
        "oet" => "**oet** as in *coat*",
        "u" => "**oo** as in *zoo*",
        "un" => "**oon** as in *soon*",
        "ut" => "**oot** as in *foot*",
        "ui" => "**oo-ee** as in *choi*",
        "yu" => "**yu** as in *yew*",
        "yut" => "**yut** as in *put*",
        "yun" => "**yoon** as in *soon*",
        "yue" => "**yweh** as in *yes* — round your lips",
        "oe" => "**oe** as in *so* — close and rounded",
        "m" => "**m** as in *hum*",
        "ng" => "**ng** as in *sing*",
        _ => "",
    };

    let coda_text = match coda {
        "p" => ". Close the lips on a **p** without releasing it.",
        "t" => ". End on a **t** without releasing it.",
        "k" => ". End on a **k** without releasing it.",
        "m" => ". Keep the **m** nasal — do not add a vowel after it.",
        "n" => ". Keep the **n** nasal — do not add a vowel after it.",
        "ng" => ". Keep the **ng** nasal — do not add a *g* after it.",
        _ => "",
    };

    if base.is_empty() {
        format!("a Cantonese rime you will recognise by ear{coda_text}")
    } else {
        format!("{base}{coda_text}")
    }
}

/// The full "sound like …" sentence for a syllable.
///
/// `tone` is the tone as it is actually spoken, so a syllable whose tone
/// lifts in colloquial speech is described the way a native says it.
pub fn anchor_sentence(onset: &str, nucleus: &str, coda: &str, tone: u8) -> String {
    let shape = if onset.is_empty() {
        nucleus.to_string()
    } else {
        format!("{onset}{nucleus}")
    };
    let ia = initial_anchor(onset);
    let fa = final_anchor(nucleus, coda);
    let fa = if fa.ends_with('.') {
        fa
    } else {
        format!("{fa}.")
    };
    let tone_txt = tone_instruction(tone);
    format!("Sound like \"{shape}\" — {ia}, then {fa} {tone_txt}")
}

// ───────────────────────────────────────────────────────────────────────────
//  Phonetic traps
// ───────────────────────────────────────────────────────────────────────────

fn traps_for(s: &Parsed, tone: u8) -> Vec<Trap> {
    let mut out: Vec<Trap> = Vec::new();

    if s.back_nasal {
        out.push(Trap {
            title: "Back-nasal ng".into(),
            body: "Mandarin speakers reach for an *n* here. Cantonese *ng* is formed at the back of the mouth — the tongue body touches the soft palate, the same place as the *ng* in English *sing*. It is much darker and heavier than a Mandarin n.".into(),
        });
    }
    if s.initial.is_empty() && s.coda.is_empty() && !s.foreign {
        out.push(Trap {
            title: "No initial consonant".into(),
            body: "This syllable starts directly on its vowel. Many learners insert an *n* or a *g* that is not in Cantonese — start clean on the vowel itself.".into(),
        });
    }
    if matches!(s.coda.as_str(), "p" | "t" | "k") {
        out.push(Trap {
            title: "Checked final — do not release".into(),
            body: "The final stop is held, not popped. If you let the air out you will hear an extra syllable (*ka-p*) instead of one syllable. Say it once, cleanly, and stop.".into(),
        });
    }
    if matches!(s.coda.as_str(), "m" | "n" | "ng") {
        out.push(Trap {
            title: "Nasal coda".into(),
            body: "The syllable ends on a nasal murmur. Do not add a schwa or a trailing vowel after it — Cantonese keeps the nucleus tight and lets the nasal carry the end of the syllable.".into(),
        });
    }
    if tone == 6 {
        out.push(Trap {
            title: "Tone 6 is level, not falling".into(),
            body: "Tone 6 sits low and stays perfectly flat. If you hear yourself sliding *down* into tone 4, you have changed the word's meaning.".into(),
        });
    }
    if tone == 3 {
        out.push(Trap {
            title: "Tone 3 does not dip".into(),
            body: "Tone 3 is a steady mid-level tone in Cantonese. There is no English-style dip in the middle — hold it flat at mid height.".into(),
        });
    }
    if tone == 2 || tone == 5 {
        out.push(Trap {
            title: "Rising tones are not the same".into(),
            body: "Tone 5 rises from a low start; tone 2 rises from a mid start and ends higher. Match the *starting* height first, then the direction.".into(),
        });
    }
    if matches!(s.initial.as_str(), "c") {
        out.push(Trap {
            title: "ts, not s".into(),
            body: "The initial written *c* is an affricate — a *ts* — not a plain *s*. Mandarin speakers often relax it into an *s*.".into(),
        });
    }
    if matches!(s.initial.as_str(), "ch") {
        out.push(Trap {
            title: "ch, not sh".into(),
            body: "Cantonese *ch* is a true affricate (*ch*), not the fricative *sh* that Mandarin uses for 吃.".into(),
        });
    }
    if matches!(s.initial.as_str(), "z") {
        out.push(Trap {
            title: "z, not j".into(),
            body: "The initial written *z* is a voiced *z*. Mandarin learners tend to say a *j* here — keep the tongue behind the teeth and let it buzz.".into(),
        });
    }
    if matches!(s.initial.as_str(), "j") {
        out.push(Trap {
            title: "y, not zh".into(),
            body: "The initial written *j* sounds like English *y*. Do not round it into a Mandarin *zh*.".into(),
        });
    }
    if matches!(s.initial.as_str(), "gw" | "kw") {
        out.push(Trap {
            title: "Round your lips".into(),
            body: "*gw* and *kw* are labialised. If your lips stay flat the syllable will sound like plain *g* or *k* and may be misheard.".into(),
        });
    }
    if s.nucleus == "i" && s.coda.is_empty() {
        out.push(Trap {
            title: "Hold the high front vowel".into(),
            body: "Cantonese *i* is a pure high front vowel — no English *ee* glide at the end. Keep the tongue high and the lips spread, and stop there.".into(),
        });
    }
    if s.nucleus.ends_with('u') && s.nucleus != "u" {
        out.push(Trap {
            title: "Rounded back vowel".into(),
            body: "This rime is rounded — push your lips forward as you say it, the way you would for English *oo*.".into(),
        });
    }
    if s.nucleus == "u" {
        out.push(Trap {
            title: "Pure u, not English 'oo'".into(),
            body: "Cantonese *u* is a tight rounded back vowel with no diphthong. English *oo* (as in *fool*) starts too far forward — round harder and keep it short.".into(),
        });
    }
    if s.nucleus == "e" || s.nucleus == "o" {
        out.push(Trap {
            title: "Shorter vowel than English".into(),
            body: "Cantonese vowels are shorter and more clipped than their English counterparts. Do not let the vowel glide into a second syllable.".into(),
        });
    }
    if s.nucleus.len() >= 2 && s.coda.is_empty() {
        out.push(Trap {
            title: "Long rime — hold it".into(),
            body: "This is a long rime. Cantonese long vowels are genuinely held; cutting them short makes the word sound clipped and childish.".into(),
        });
    }

    out.truncate(3);
    out
}

// ───────────────────────────────────────────────────────────────────────────
//  Minimal pairs
// ───────────────────────────────────────────────────────────────────────────

/// Curated minimal pairs for syllables that learners reliably confuse.
fn curated_pairs(jyutping: &str) -> Option<Vec<MinimalPair>> {
    let v: Vec<(&str, &str, &str)> = match jyutping {
        "hai2" => vec![
            ("喺", "hai2", "at / in (a place)"),
            ("係", "hai6", "is / yes — different tone, different word"),
        ],
        "hai6" => vec![
            ("係", "hai6", "is / yes"),
            ("喺", "hai2", "at / in — different tone, different word"),
        ],
        "m4" => vec![
            ("唔", "m4", "not / don't"),
            ("咩", "me1", "what — 唔該 vs 咩"),
        ],
        "goi1" => vec![
            ("該", "goi1", "should (唔該 = excuse me / thanks)"),
            ("梗", "gang2", "exactly (confused in fast speech)"),
        ],
        "ngo5" => vec![
            ("我", "ngo5", "I / me"),
            ("糯", "ngo6", "glutinous — only the tone differs"),
        ],
        "ng5" => vec![
            ("五", "ng5", "five"),
            ("吳", "ng4", "the surname Ng — only the tone differs"),
        ],
        "laan4" => vec![
            ("蘭", "laan4", "orchid, as in 荷蘭"),
            ("爛", "laan6", "rotten — only the tone differs"),
        ],
        "ho4" => vec![
            ("荷", "ho4", "lotus, as in 荷蘭"),
            ("賀", "ho6", "congratulate — only the tone differs"),
        ],
        "zeoi3" => vec![
            ("歲", "seoi3", "year"),
            ("睡", "seoi6", "to sleep — only the tone differs"),
        ],
        "zyu6" => vec![
            ("住", "zyu6", "to live / to stay"),
            ("主", "zyu2", "master / owner — only the tone differs"),
        ],
        "gau2" => vec![
            ("狗", "gau2", "dog"),
            ("搞", "gau3", "to do / to make — only the tone differs"),
        ],
        "dou6" => vec![
            ("道", "dou6", "way / road"),
            ("刀", "dou1", "knife — only the tone differs"),
        ],
        "bin1" => vec![
            ("邊", "bin1", "which side"),
            ("變", "bin3", "to change — only the tone differs"),
        ],
        "gong2" => vec![
            ("講", "gong2", "to speak"),
            ("港", "gong2", "harbour / Hong Kong — same sound, different word"),
        ],
        "gou1" => vec![
            ("高", "gou1", "tall / high"),
            ("狗", "gau2", "dog — different vowel and tone"),
        ],
        "saa1" | "zaam6" => vec![
            ("站", "zaam6", "to stand"),
            ("譚", "taam4", "surname Tam — a near miss in fast speech"),
        ],
        "wui5" => vec![
            ("會", "wui5", "will / to be able to"),
            ("未", "mei6", "not yet — easily heard as 會"),
        ],
        "sik6" => vec![
            ("食", "sik6", "to eat"),
            ("識", "sik1", "to know — only the tone differs"),
        ],
        "zyu5" => vec![
            ("住", "zyu6", "to live"),
            ("主", "zyu2", "master — the middle of the range"),
        ],
        "joek3" => vec![
            ("腳", "goek3", "foot"),
            ("雀", "zoek3", "sparrow — different initial"),
        ],
        "ngaau1" => vec![
            ("交", "gaau1", "to hand over"),
            ("咬", "ngaau5", "to bite — different initial"),
        ],
        "ngoi6" => vec![
            ("外", "ngoi6", "outside"),
            ("危", "ngai4", "dangerous — a near miss"),
        ],
        "bing1" => vec![
            ("冰", "bing1", "ice"),
            ("柄", "bing2", "handle — only the tone differs"),
        ],
        "baai1" => vec![
            ("百", "baak3", "hundred"),
            ("柏", "baak3", "cypress — only the tone differs"),
        ],
        "je1" => vec![
            ("也", "je1", "also"),
            ("野", "je5", "field / wild — only the tone differs"),
        ],
        "ce1" => vec![
            ("車", "ce1", "car"),
            ("姐", "ze2", "older sister — a close call"),
        ],
        "tou4" => vec![
            ("投", "tou4", "to throw"),
            ("頭", "tau4", "head — different initial"),
        ],
        _ => return None,
    };
    Some(
        v.into_iter()
            .map(|(han, jyutping, note)| MinimalPair {
                han: han.to_string(),
                jyutping: jyutping.to_string(),
                note: note.to_string(),
            })
            .collect(),
    )
}

/// The tone that a learner is most likely to confuse with `tone`.
fn confusable_tone(tone: u8) -> u8 {
    match tone {
        1 => 6,
        2 => 5,
        3 => 1,
        4 => 6,
        5 => 2,
        6 => 4,
        _ => 1,
    }
}

/// Minimal pairs for one syllable: curated first, then a generated tone drill.
fn minimal_pairs_for(han: &str, jyutping: &str, parsed: &Parsed) -> Vec<MinimalPair> {
    if let Some(v) = curated_pairs(jyutping) {
        return v;
    }
    if parsed.foreign || parsed.tone == 0 {
        return Vec::new();
    }

    let other = confusable_tone(parsed.tone);
    vec![
        MinimalPair {
            han: han.to_string(),
            jyutping: jyutping.to_string(),
            note: format!("tone {} — {}", parsed.tone, tone_name(parsed.tone)),
        },
        MinimalPair {
            han: han.to_string(),
            jyutping: format!("{}{}", &jyutping[..jyutping.len() - 1], other),
            note: format!("tone {} — the one you will usually get wrong", other),
        },
    ]
}

// ───────────────────────────────────────────────────────────────────────────
//  Sandhi
// ───────────────────────────────────────────────────────────────────────────

/// The spoken tone for every syllable of a phrase, applying the colloquial
/// sandhi rules that matter most for a learner.
///
/// * a run of tone-4 syllables: the **second and later** syllables lift to a
///   high tone 1 (`荷蘭` → `ho4 laan1`);
/// * tone 3 immediately before another tone 3 becomes tone 2;
/// * the numeral `一` before a tone 3 / 4 / 6 syllable becomes tone 6.
fn spoken_tones(sylls: &[(String, String, Parsed)]) -> Vec<u8> {
    let n = sylls.len();
    let mut out: Vec<u8> = sylls.iter().map(|(_, _, p)| p.tone).collect();

    // Rule 1 — tone-4 runs.
    let mut i = 0;
    while i < n {
        if sylls[i].2.tone == 4 {
            let mut j = i;
            while j + 1 < n && sylls[j + 1].2.tone == 4 {
                j += 1;
            }
            // Positions i+1 ..= j lift to tone 1.
            for k in (i + 1)..=j {
                out[k] = 1;
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }

    // Rule 2 — tone 3 before tone 3 becomes tone 2.
    for k in 0..n.saturating_sub(1) {
        if sylls[k].2.tone == 3 && sylls[k + 1].2.tone == 3 {
            out[k] = 2;
        }
    }

    // Rule 3 — 一 before tones 3 / 4 / 6 becomes tone 6.
    for k in 0..n.saturating_sub(1) {
        if sylls[k].0 == "一" && matches!(sylls[k + 1].2.tone, 3 | 4 | 6) {
            out[k] = 6;
        }
    }

    out
}

// ───────────────────────────────────────────────────────────────────────────
//  Mistakes table
// ───────────────────────────────────────────────────────────────────────────

fn mistake_for(s: &Parsed, han: &str) -> Option<MistakeRow> {
    if s.foreign {
        return None;
    }
    let row = if s.back_nasal {
        MistakeRow {
            wrong: format!("{han} — *n* as in *no*"),
            right: format!("{han} — *ng* as in *sing*"),
            why: "Cantonese *ng* is a back nasal made at the soft palate. Mandarin has no sound here, so learners flatten it into an *n*.".into(),
        }
    } else if s.initial.is_empty() {
        MistakeRow {
            wrong: format!("{han} — *n*/*g* added to the start"),
            right: format!("{han} — start straight on the vowel"),
            why: "Cantonese allows syllables with no initial consonant. Adding one turns the word into a different one.".into(),
        }
    } else if s.initial == "c" {
        MistakeRow {
            wrong: format!("{han} — *s* as in *see*"),
            right: format!("{han} — *ts* as in *cats*"),
            why: "The written *c* is an affricate. Relaxing it into *s* is one of the most audible tells of a Mandarin speaker.".into(),
        }
    } else if s.initial == "ch" {
        MistakeRow {
            wrong: format!("{han} — *sh* as in *she*"),
            right: format!("{han} — *ch* as in *church*"),
            why: "Cantonese *ch* is an affricate. Mandarin *chi* is an *sh* sound, which is exactly what learners reach for.".into(),
        }
    } else if s.initial == "z" {
        MistakeRow {
            wrong: format!("{han} — *j* as in *yes*"),
            right: format!("{han} — *z* as in *zoo*"),
            why: "The written *z* is a voiced *z*. It is not the Mandarin *j* sound.".into(),
        }
    } else if s.initial == "j" {
        MistakeRow {
            wrong: format!("{han} — *zh* as in *jeep*"),
            right: format!("{han} — *y* as in *yes*"),
            why: "The written *j* is an English *y*. Rounding it produces a Mandarin *zh* and sounds foreign.".into(),
        }
    } else if matches!(s.initial.as_str(), "gw" | "kw") {
        MistakeRow {
            wrong: format!("{han} — plain *g* / *k*"),
            right: format!("{han} — *gw* / *kw* with rounded lips"),
            why: "*gw* and *kw* are labialised. Without the rounding the syllable can be misheard as a different word.".into(),
        }
    } else if matches!(s.coda.as_str(), "p" | "t" | "k") {
        MistakeRow {
            wrong: format!("{han} — *ka-p* / *ka-t* / *ka-k* (two syllables)"),
            right: format!("{han} — one syllable, stop unreleased"),
            why: "A checked final is held, not released. Letting the burst out adds a phantom syllable.".into(),
        }
    } else if s.nucleus == "i" && s.coda.is_empty() {
        MistakeRow {
            wrong: format!("{han} — *i* with an *ee* glide at the end"),
            right: format!("{han} — pure high front *i*"),
            why: "Cantonese *i* is a steady vowel. The English diphthong makes it sound longer and lighter.".into(),
        }
    } else if s.tone == 6 {
        MistakeRow {
            wrong: format!("{han} — falling into tone 4"),
            right: format!("{han} — tone 6, low and flat"),
            why: "Tone 6 has no movement at all. A downward slide changes the meaning of the word.".into(),
        }
    } else if s.tone == 3 {
        MistakeRow {
            wrong: format!("{han} — dipping down in the middle"),
            right: format!("{han} — tone 3, flat at mid height"),
            why: "Cantonese tone 3 is steady mid-level, unlike Mandarin's dipping third tone.".into(),
        }
    } else if s.tone == 1 {
        MistakeRow {
            wrong: format!("{han} — starting too low"),
            right: format!("{han} — tone 1, high and flat"),
            why: "Tone 1 sits at the top of the range. Starting it low makes it sound like tone 6.".into(),
        }
    } else {
        return None;
    };
    Some(row)
}

// ───────────────────────────────────────────────────────────────────────────
//  Curated phrase overrides
// ───────────────────────────────────────────────────────────────────────────

/// Bespoke, verbatim notes for the headline phrases of the bundled library.
pub struct PhraseOverride {
    pub spoken_jyutping: Option<&'static str>,
    pub strip_note: Option<&'static str>,
    pub mistakes: Option<&'static [(&'static str, &'static str, &'static str)]>,
    pub takeaway: Option<&'static str>,
    pub drill_bodies: Option<&'static [&'static str; 5]>,
}

const CURATED: &[(i64, PhraseOverride)] = &[
    (
        4,
        PhraseOverride {
            spoken_jyutping: None,
            strip_note: None,
            mistakes: Some(&[
                (
                    "*nei5 dim2 cing1 fu1 ne1* read as one flat run",
                    "*nei5 dim2 cing1 fu1 ne1* — four distinct tones",
                    "Each syllable carries its own tone: 你 low-rising, 點 mid-rising, 稱 high and flat, 呼 high and flat, 呢 high and flat.",
                ),
                (
                    "*dim2* said with a falling tail",
                    "*dim2* — rising from mid to the top",
                    "點 is the *how* of *點樣* and *點講*. It starts at mid height and rises; it never falls.",
                ),
                (
                    "*cing1* as in *sing*",
                    "*cing1* — *ts* as in *cats*, then *ing*",
                    "The initial written *c* is an affricate — a *ts* — not an *s*. Mandarin speakers relax it into *s*, and 稱呼 comes out as *sing fu*.",
                ),
                (
                    "*fu1* said as *foo* with a glide",
                    "*fu1* — a tight, rounded *u*",
                    "呼 is a pure rounded back vowel with no English diphthong. Keep it short and stop there.",
                ),
                (
                    "*ne1* said as *ni1*",
                    "*ne1* — the *e* of *bed*, not the *ee* of *bead*",
                    "呢 here is the question particle, not the demonstrative *ni1* used in 呢個. The vowel drops from *i* to *e*.",
                ),
            ]),
            takeaway: Some(
                "**The shape is: low → mid → high → high → high.** 你 rises from the bottom, 點 carries it from mid to the top, and then 稱呼呢 sits flat on the top of the range. The phrase climbs and then stays high — which is exactly why it sounds courteous. The casual 你叫咩名 (*nei5 giu3 me1 meng2*) sits lower and flatter, and the difference in pitch is part of the difference in register.",
            ),
            drill_bodies: Some(&[
                "Say **nei — dim — cing — fu — ne** with no tones. *nei* = n + ay, *dim* = d + im, *cing* = ts + ing, *fu* = f + oo, *ne* = n + eh.",
                "Add the tone shapes: **nei5** (low rising) **dim2** (mid rising) **cing1** (high flat) **fu1** (high flat) **ne1** (high flat).",
                "Put the consonants back, starting with the hardest: **ts** + *ing* for 稱. Relax the *ts* into *s* and the word stops being 稱呼.",
                "Say the whole phrase slowly — *nei5 dim2 cing1 fu1 ne1* — then at natural speed. Keep the last three syllables flat and high; do not let them sag.",
                "Make it real: **請問點稱呼？** (*cing2 man6 dim2 cing1 fu1*) — *May I ask what to call you?* — is the full polite form a native actually uses.",
            ]),
        },
    ),
    (
        2,
        PhraseOverride {
            spoken_jyutping: Some("ngo5 zyu6 hai2 ho4 laan1"),
            strip_note: Some(
                "Colloquial sandhi: before a tone-4 syllable, the tone-4 rises to a high tone 1 — so 蘭 is spoken *laan1*, not *laan4*. Hold the vowel and let it float upward at the end.",
            ),
            mistakes: Some(&[
                (
                    "*wo* zyu hai ho laan",
                    "*ngo5* zyu6 *hai2* ho4 *laan1*",
                    "Three things at once: the back-nasal **ng** in 我, the flat mid **hai2** (at, not is), and the **laan1** lift at the end of 荷蘭.",
                ),
                (
                    "*hai* as in *high*",
                    "*hai2* — mid level, flat",
                    "喺 is tone 2, mid-to-flat. 係 (hai6) is the low, flat one; learners swap them constantly.",
                ),
                (
                    "*ho4 laan4* — both falling",
                    "*ho4 laan1* — the last syllable lifts",
                    "Two tone-4s in a row: the second one sandhis up to a high tone 1. Say *ho4* falling, then *laan1* rising.",
                ),
                (
                    "*ngo* as in *no*",
                    "*ngo5* — ng as in *sing*",
                    "Cantonese 我 starts with a back-nasal **ng**. A plain *n* sounds thin and slightly foreign.",
                ),
            ]),
            takeaway: Some(
                "**The shape is: low → low → mid → falling → rising.** 我 is low and flat, 住 is low and flat, 喺 is mid and flat, 荷 falls, and 蘭 lifts to a high *laan1*. The whole phrase ends going *up*, which is what makes it sound friendly rather than abrupt.",
            ),
            drill_bodies: Some(&[
                "Say **ngo — zyu — hai — ho — laan** with no tones at all. Just five clean vowels. *ngo* = ng + aw, *zyu* = z + yoo, *hai* = h + eye, *ho* = h + aw, *laan* = l + ahn.",
                "Add the tones without the consonants: **ngo5** (low flat) **zyu6** (low flat) **hai2** (mid, flat) **ho4** (fall) **laan1** (rising).",
                "Now put the consonants back, still slowly: **ng** + *aw*, **z** + *yoo*, **h** + *eye*, **h** + *aw*, **l** + *ahn*.",
                "Say the whole phrase slowly — *ngo5 zyu6 hai2 ho4 laan1* — with the last syllable clearly rising. If *laan* falls, you have drifted back to the dictionary tone.",
                "Make it a real sentence: **我住喺荷蘭** → **我住喺荷蘭㗎** or **我住喺荷蘭喇**. The sentence particle is what a native actually hears.",
            ]),
        },
    ),
    (
        1,
        PhraseOverride {
            spoken_jyutping: None,
            strip_note: None,
            mistakes: Some(&[
                (
                    "*nei hou ngo giu* — all mid",
                    "*nei5 hou2 ngo5 giu3*",
                    "Each syllable has its own tone: 你 high-flat, 好 mid-rising, 我 low-flat, 叫 mid-level.",
                ),
                (
                    "*ho2* with a falling tail",
                    "*hou2* — mid level rising to the top",
                    "好 is tone 2. It starts at mid height and rises; it never falls.",
                ),
                (
                    "*ngo* as in *no*",
                    "*ngo5* — ng as in *sing*",
                    "The back-nasal **ng** is the single most audible marker of a Cantonese speaker.",
                ),
                (
                    "*Jacob* said as *ya-ko-b*",
                    "*Jacob* — keep it in English",
                    "A Latin name in a Cantonese sentence is simply said in English, at English length.",
                ),
            ]),
            takeaway: Some(
                "**The shape is: high → mid-rising → low → mid.** The phrase opens bright with 你 and closes on the mid-level 叫, so the greeting ends settled rather than falling off a cliff.",
            ),
            drill_bodies: Some(&[
                "Say **nei — hou — ngo — giu** with no tones. *nei* = n + ay, *hou* = h + ow, *ngo* = ng + aw, *giu* = g + iu.",
                "Add the tone shapes: **nei5** (high flat) **hou2** (mid rising) **ngo5** (low flat) **giu3** (mid flat).",
                "Put the consonants back one at a time, starting with the hardest: **ng** + *aw*.",
                "Say the full phrase slowly, then at normal speed: **你好，我叫 Jacob。**",
                "Make it a greeting: **你好，我叫 Jacob 呀** — the particle softens the introduction.",
            ]),
        },
    ),
    (
        3,
        PhraseOverride {
            spoken_jyutping: None,
            strip_note: None,
            mistakes: Some(&[
                (
                    "*ng* as in *no*",
                    "*ng5* — ng as in *sing*",
                    "五 is the syllabic back-nasal **ng**. It has no vowel of its own — just a nasal hum.",
                ),
                (
                    "*sap6 ji6* — the *j* as a *y*",
                    "*sap6 ji6* — j is a *y* sound",
                    "In 二 the initial is *j*, pronounced like English *y*, not like Mandarin *r*.",
                ),
                (
                    "*sai6* with a falling tail",
                    "*seoi3* — tone 3, flat at mid height",
                    "歲 is a tone 3. Cantonese tone 3 is steady mid-level; there is no dip in the middle.",
                ),
                (
                    "Saying **52** as one word",
                    "**ng5 sap6 ji6** — three separate syllables",
                    "Cantonese reads digits individually: 五 *ng5*, 十 *sap6*, 二 *ji6*. Never *ng5 sap6 ji6* as a single fused number.",
                ),
            ]),
            takeaway: Some(
                "**The shape is: low → high → low → low → low → low → mid.** Five of the seven syllables sit flat and low — the phrase is deliberately boring in pitch so the number lands cleanly. Your only real job is keeping 五 a pure *ng* and 歲 a flat mid.",
            ),
            drill_bodies: Some(&[
                "Say **ngo — gam — nin — ng — sap — ji — seoi**. Note that *ng* has no vowel: it is a bare nasal.",
                "Add the tones: **ngo5** (low flat) **gam1** (high flat) **nin4** (falling) **ng5** (low flat) **sap6** (low flat) **ji6** (low flat) **seoi3** (mid flat).",
                "Put the consonants back — *g*, *m*, *n*, *ng*, *s-p*, *j*, *s-oi*.",
                "Say the whole phrase slowly, keeping 五 short and 歲 perfectly flat.",
                "Make it real: **我今年五十二歲** → **我今年五十二歲喇** — and use it to ask a stranger their age politely.",
            ]),
        },
    ),
];

fn curated_for(id: i64) -> Option<&'static PhraseOverride> {
    CURATED.iter().find(|(k, _)| *k == id).map(|(_, v)| v)
}

// ───────────────────────────────────────────────────────────────────────────
//  Main entry point
// ───────────────────────────────────────────────────────────────────────────

/// Analyse a phrase and produce everything the UI renders for it.
pub fn analyze(phrase: &Phrase) -> Analysis {
    let pairs = align(&phrase.han, &phrase.jyutping);
    let override_data = curated_for(phrase.id);

    // ── parse ────────────────────────────────────────────────────────────
    let parsed: Vec<(String, String, Parsed)> = pairs
        .into_iter()
        .map(|(han, jp)| {
            let p = parse_syllable(&jp);
            (han, jp, p)
        })
        .collect();

    let tones = spoken_tones(&parsed);

    // ── build syllables ──────────────────────────────────────────────────
    let mut syllables: Vec<Syllable> = Vec::with_capacity(parsed.len());
    for (i, (han, jp, p)) in parsed.iter().enumerate() {
        let spoken = if p.foreign { 0 } else { tones[i] };
        let lifted = !p.foreign && spoken != 0 && spoken != p.tone;

        let (initial_anchor_text, final_anchor_text) = if p.foreign {
            (
                "a Latin or English name — say it in English".to_string(),
                "keep the English vowels and stress".to_string(),
            )
        } else {
            (
                initial_anchor(&p.onset).to_string(),
                final_anchor(&p.nucleus, &p.coda),
            )
        };

        syllables.push(Syllable {
            index: i,
            han: han.clone(),
            jyutping: jp.clone(),
            initial: p.initial.clone(),
            onset: p.onset.clone(),
            nucleus: p.nucleus.clone(),
            coda: p.coda.clone(),
            tone: p.tone,
            spoken_tone: spoken,
            lifted,
            chao: chao_of(p.tone).to_string(),
            spoken_chao: chao_of(spoken).to_string(),
            initial_anchor: initial_anchor_text,
            final_anchor: final_anchor_text,
            anchor_sentence: if p.foreign {
                format!("Say \"{jp}\" in English, in the middle of the Cantonese sentence.")
            } else {
                anchor_sentence(&p.onset, &p.nucleus, &p.coda, spoken)
            },
            movement: if p.foreign {
                "English name".to_string()
            } else if lifted {
                format!(
                    "{} → {}",
                    tone_name(p.tone),
                    tone_name(spoken)
                )
            } else {
                tone_name(p.tone).to_string()
            },
            traps: traps_for(p, p.tone),
            minimal_pairs: minimal_pairs_for(&han, &jp, p),
            foreign: p.foreign,
            en: if p.foreign {
                String::new()
            } else {
                // The reading matters: 呢 is "this" as *ni1* but a question
                // particle as *ne1*, and the phrase decides which.
                crate::data::gloss::gloss_for(&han, &jp).unwrap_or("").to_string()
            },
        });
    }

    // ── spoken jyutping string ───────────────────────────────────────────
    let spoken_jyutping: String = syllables
        .iter()
        .map(|s| {
            if s.foreign {
                s.jyutping.clone()
            } else {
                format!("{}{}", &s.jyutping[..s.jyutping.len() - 1], s.spoken_tone)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // ── tone key cards ───────────────────────────────────────────────────
    let mut tone_cards: Vec<ToneInfo> = Vec::new();
    for s in &syllables {
        if s.foreign || s.tone == 0 {
            continue;
        }
        if tone_cards.iter().any(|t| t.tone == s.tone) {
            continue;
        }
        tone_cards.push(ToneInfo {
            tone: s.tone,
            chao: s.chao.clone(),
            name: tone_name(s.tone).to_string(),
            shape: crate::dsp::contour::shape_path(
                crate::dsp::contour::tone_levels(s.tone).0,
                crate::dsp::contour::tone_levels(s.tone).1,
                18.0,
                122.0,
            ),
            example_han: s.han.clone(),
            example_jyutping: s.jyutping.clone(),
        });
    }
    tone_cards.sort_by_key(|t| t.tone);

    // ── mistakes ─────────────────────────────────────────────────────────
    let mut mistakes: Vec<MistakeRow> = Vec::new();
    if let Some(rows) = override_data.and_then(|o| o.mistakes) {
        for (w, r, why) in rows {
            mistakes.push(MistakeRow {
                wrong: w.to_string(),
                right: r.to_string(),
                why: why.to_string(),
            });
        }
    }
    if let Some(rich) = &phrase.rich {
        for m in &rich.mistakes {
            if !mistakes.contains(m) {
                mistakes.push(m.clone());
            }
        }
    }
    if mistakes.is_empty() {
        for s in &syllables {
            if let Some(row) = mistake_for(
                &Parsed {
                    initial: s.initial.clone(),
                    onset: s.onset.clone(),
                    nucleus: s.nucleus.clone(),
                    coda: s.coda.clone(),
                    tone: s.tone,
                    back_nasal: s.onset == "ng",
                    foreign: s.foreign,
                },
                &s.han,
            ) {
                if !mistakes.iter().any(|m| m.wrong == row.wrong) {
                    mistakes.push(row);
                }
            }
        }
    }
    mistakes.truncate(5);

    // ── five-pass drill ──────────────────────────────────────────────────
    let anchor_list: Vec<String> = syllables
        .iter()
        .filter(|s| !s.foreign)
        .map(|s| {
            if s.initial.is_empty() {
                s.nucleus.clone()
            } else {
                format!("{}{}", s.initial, s.nucleus)
            }
        })
        .collect();
    let tone_list: Vec<String> = syllables
        .iter()
        .filter(|s| !s.foreign)
        .map(|s| format!("{} = {}", s.jyutping, tone_name(s.tone)))
        .collect();
    let consonant_list: Vec<String> = syllables
        .iter()
        .filter(|s| !s.foreign)
        .map(|s| {
            if s.initial.is_empty() {
                format!("{} (no initial)", s.jyutping)
            } else {
                format!("{} — {}", s.jyutping, s.initial)
            }
        })
        .collect();

    let mut drill: Vec<DrillPass> = vec![
        DrillPass {
            n: 1,
            title: "Vowels & sound anchors only".into(),
            body: "Drop every consonant and every tone. You are only hunting for the shape of the mouth.".into(),
            focus: anchor_list.clone(),
        },
        DrillPass {
            n: 2,
            title: "Add the tone shapes".into(),
            body: "Keep the vowels, add the pitch. Say each one on its own line so you can hear the contour.".into(),
            focus: tone_list.clone(),
        },
        DrillPass {
            n: 3,
            title: "Put the consonants back".into(),
            body: "Now re-attach the onsets and finals. This is where the phrase starts to sound like Cantonese.".into(),
            focus: consonant_list.clone(),
        },
        DrillPass {
            n: 4,
            title: "Full phrase, slowly".into(),
            body: "Say the whole phrase at about 60% speed. Keep the spaces between syllables even.".into(),
            focus: vec![format!("{} — slowly, evenly spaced", phrase.han)],
        },
        DrillPass {
            n: 5,
            title: "Real sentence with a final particle".into(),
            body: "A bare phrase sounds clipped. Add a sentence particle — 啦 / 喇 / 㗎 / 咩 / 呀 — and say it to somebody.".into(),
            focus: vec![
                format!("{} 啦", phrase.han),
                format!("{} 喇", phrase.han),
                format!("{} 㗎", phrase.han),
            ],
        },
    ];

    if let Some(bodies) = override_data.and_then(|o| o.drill_bodies) {
        for (i, body) in bodies.iter().enumerate() {
            if i < drill.len() {
                drill[i].body = body.to_string();
            }
        }
    }
    if let Some(rich) = &phrase.rich {
        for d in &rich.drills {
            if let Some(slot) = drill.iter_mut().find(|x| x.n == d.n) {
                *slot = d.clone();
            }
        }
    }

    // ── tagline & takeaway ───────────────────────────────────────────────
    let tagline = syllables
        .iter()
        .filter(|s| !s.foreign)
        .map(|s| tone_name(s.tone).to_string())
        .collect::<Vec<_>>()
        .join(" → ");

    let mut takeaway = override_data
        .and_then(|o| o.takeaway)
        .map(|s| s.to_string())
        .or_else(|| phrase.rich.as_ref().and_then(|r| r.takeaway.clone()))
        .unwrap_or_else(|| {
            let seq: Vec<String> = syllables
                .iter()
                .filter(|s| !s.foreign)
                .map(|s| s.spoken_tone.to_string())
                .collect();
            let checkpoints: Vec<String> = syllables
                .iter()
                .filter(|s| !s.foreign)
                .take(3)
                .map(|s| format!("{} ({})", s.han, s.spoken_chao))
                .collect();
            format!(
                "**Pitch sequence: {}.** Watch {}. The whole phrase is {}-{} syllables long — count them out loud once before you record.",
                seq.join(" – "),
                checkpoints.join(", "),
                syllables.iter().filter(|s| !s.foreign).count(),
                syllables.iter().filter(|s| !s.foreign).count()
            )
        });

    if let Some(notes) = &phrase.rich {
        if !notes.sandhi.is_empty() {
            takeaway.push_str(" ");
            takeaway.push_str(&notes.sandhi.join(" "));
        }
    }

    // ── strip note ───────────────────────────────────────────────────────
    let lifted: Vec<&Syllable> = syllables.iter().filter(|s| s.lifted).collect();
    let mut strip_note = override_data.and_then(|o| o.strip_note).map(|s| s.to_string());
    if strip_note.is_none() && !lifted.is_empty() {
        let detail = lifted
            .iter()
            .map(|s| format!("{} {} → {}", s.han, s.jyutping, {
                format!("{}{}", &s.jyutping[..s.jyutping.len() - 1], s.spoken_tone)
            }))
            .collect::<Vec<_>>()
            .join(", ");
        strip_note = Some(format!(
            "Colloquial sandhi in this phrase: {}. The written Jyutping shows the dictionary tone; the badge shows what a native actually says.",
            detail
        ));
    }

    // An explicit user override wins over everything.
    let spoken_jyutping = phrase
        .spoken_jyutping
        .clone()
        .or_else(|| seed_spoken_jyutping(phrase.id).map(|s| s.to_string()))
        .or_else(|| override_data.and_then(|o| o.spoken_jyutping).map(|s| s.to_string()))
        .unwrap_or(spoken_jyutping);

    Analysis {
        syllables,
        tones: tone_cards,
        mistakes,
        drill,
        takeaway,
        tagline,
        strip_note,
        spoken_jyutping,
    }
}
