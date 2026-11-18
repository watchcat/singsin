//! Building the library the learner actually sees.
//!
//! Three things go into the library: the bundled phrasebook, the phrases
//! built from the learner's own details, and whatever the learner has
//! edited, added or deleted themselves. The rule that keeps them from
//! fighting each other is simple — **the learner's own version always
//! wins**:
//!
//! * a bundled phrase the learner has edited keeps their text, even if
//!   the profile would have filled it in differently;
//! * a bundled phrase the learner has deleted stays deleted;
//! * a generated phrase the learner has edited keeps their version.
//!
//! The learner's own phrases live in the browser's storage and never
//! contain the personalised text: the personalisation is applied on the
//! way to the screen, so changing a detail in the profile changes the
//! sentences, and changing an unrelated phrase does not.

use super::phrases::Phrase;
use super::profile::Profile;
use super::templates::{self, GENERATED_ID_BASE};

/// Whether a phrase id belongs to the generated personal section.
pub fn is_generated(id: i64) -> bool {
    id >= GENERATED_ID_BASE
}

/// The library, ready to be shown.
pub struct Library {
    /// The phrases the learner sees and practises.
    pub display: Vec<Phrase>,
}

/// Build the library: the bundled phrases, with the learner's details
/// filled in and their own edits applied.
pub fn personalise(seed: &[Phrase], own: Option<&[Phrase]>, profile: &Profile) -> Library {
    let base: Vec<Phrase> = match own {
        Some(list) => list.to_vec(),
        None => seed.to_vec(),
    };

    // A phrase the learner has touched is theirs. The comparison is
    // against the bundled text, and also against the personalised text:
    // a phrase that merely happens to say what the profile would have
    // said is not an edit.
    let personalised = templates::build_all(profile);
    let edited: Vec<i64> = match own {
        None => Vec::new(),
        Some(list) => seed
            .iter()
            .filter(|s| match list.iter().find(|o| o.id == s.id) {
                None => false,
                Some(o) => {
                    o != &**s && personalised.iter().find(|g| g.id == s.id) != Some(o)
                }
            })
            .map(|s| s.id)
            .collect(),
    };

    let mut display = base;

    // 1. The bundled phrases that were about somebody else.
    for template in templates::SEED {
        if edited.contains(&template.id) {
            continue;
        }
        let Some(value) = profile.value(template.slot) else {
            continue;
        };
        let built = template.build(&value);
        if let Some(slot) = display.iter_mut().find(|p| p.id == template.id) {
            *slot = built;
        }
    }

    // 2. The personal section.
    for template in templates::PERSONAL {
        // A generated phrase the learner has their own version of stays
        // theirs.
        if own.map(|l| l.iter().any(|p| p.id == template.id)).unwrap_or(false) {
            continue;
        }
        let Some(value) = profile.value(template.slot) else {
            continue;
        };
        display.push(template.build(&value));
    }

    Library { display }
}

/// The list of phrases that belongs in the browser's storage.
///
/// The learner's own phrases, with the generated ones left out: those are
/// rebuilt from the profile every time, and keeping them would freeze the
/// sentences at whatever the details said when they were first built. The
/// personalised text is never stored either — the profile is the source
/// of truth for the four library phrases it fills in.
pub fn for_storage(own: &Option<Vec<Phrase>>) -> Option<Vec<Phrase>> {
    let list = own.as_ref()?;
    Some(list.iter().filter(|p| !is_generated(p.id)).cloned().collect())
}
