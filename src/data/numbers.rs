//! Han numerals, with their Jyutping, for the numbers a learner puts in
//! a sentence about themselves.
//!
//! The app already knows how a number behaves in *speech* — its sandhi
//! rules turn 一 into jat6 before a fourth tone, so 一年 comes out as
//! *jat6 nin4* rather than *jat1 nin4*. What this module adds is the
//! writing and the reading of the number itself.
//!
//! Two forms exist for the twenties and are worth keeping apart:
//!
//! * **written** — 二十一, the form on a page and in a dictionary;
//! * **spoken** — 廿一, what a Hong Kong speaker actually says. 廿
//!   (*ng6*) stands for twenty, so 廿一 is *ng6 jat1* and not
//!   *ji6 sap6 jat1*.
//!
//! The ten is also spoken as 廿 in the twenties only. From thirty up the
//! two forms agree, and the older spoken short forms (二五 for twenty-five)
//! are left alone: for an age, a Hong Kong speaker says 三十五, never
//! 三五.

/// The digits, in Han and in Jyutping.
const DIGITS: [(&str, &str); 10] = [
    ("零", "ling4"),
    ("一", "jat1"),
    ("二", "ji6"),
    ("三", "saam1"),
    ("四", "sei3"),
    ("五", "ng5"),
    ("六", "luk6"),
    ("七", "cat1"),
    ("八", "baat3"),
    ("九", "gau2"),
];

/// Ten.
const TEN: (&str, &str) = ("十", "sap6");

/// Twenty, in the short form the twenty-something use in speech.
const TWENTY: (&str, &str) = ("廿", "ng6");

/// Hundred.
const HUNDRED: (&str, &str) = ("百", "baak3");

/// The written form of a number: 52 → ("五十二", "ng5 sap6 ji6").
///
/// The Jyutping is the dictionary reading of each character. The spoken
/// form of the sentence around it — 一 turning into jat6 before a fourth
/// tone, for instance — is the engine's business, not this module's.
pub fn han_number(n: u32) -> (String, String) {
    let mut han = String::new();
    let mut jyutping = String::new();

    // The Han form is written without any separator — 五十二, never 五 十二 —
    // because that is how the number appears on a page, and how the
    // library's own phrase 我今年五十二歲。 writes it. The Jyutping keeps
    // its spaces: that is what makes it readable, and it is what the
    // engine counts when it pairs each syllable with its character.
    let push = |h: &str, j: &str, han: &mut String, jyutping: &mut String| {
        if !jyutping.is_empty() {
            jyutping.push(' ');
        }
        han.push_str(h);
        jyutping.push_str(j);
    };

    let hundreds = n / 100;
    let rest = n % 100;
    if hundreds > 0 {
        if hundreds > 1 {
            push(DIGITS[hundreds as usize].0, DIGITS[hundreds as usize].1, &mut han, &mut jyutping);
        }
        push(HUNDRED.0, HUNDRED.1, &mut han, &mut jyutping);
    }

    if rest > 0 {
        let tens = rest / 10;
        let units = rest % 10;
        if tens == 1 {
            push(TEN.0, TEN.1, &mut han, &mut jyutping);
        } else if tens > 1 {
            push(DIGITS[tens as usize].0, DIGITS[tens as usize].1, &mut han, &mut jyutping);
            push(TEN.0, TEN.1, &mut han, &mut jyutping);
        }
        if units > 0 {
            push(DIGITS[units as usize].0, DIGITS[units as usize].1, &mut han, &mut jyutping);
        }
    }

    if han.is_empty() {
        push(DIGITS[0].0, DIGITS[0].1, &mut han, &mut jyutping);
    }

    (han, jyutping)
}

/// The spoken form of a number: 21 → ("廿一", "ng6 jat1").
///
/// The differences from the written form are the ones a listener would
/// notice:
///
/// * 10 to 19 are spoken 十X, never 一十X — 十五 is *sap6 ng5*;
/// * 20 to 29 are spoken 廿X — 廿一 is *ng6 jat1*, not *ji6 sap6 jat1*;
/// * everything else is the same in speech and on the page.
pub fn han_number_spoken(n: u32) -> (String, String) {
    let (han, jyutping) = han_number(n);
    if n < 10 || n > 99 {
        return (han, jyutping);
    }

    let tens = n / 10;
    let units = n % 10;

    if tens == 1 {
        // 一十 → 十
        let (unit_han, unit_jp) = if units > 0 {
            (DIGITS[units as usize].0.to_string(), DIGITS[units as usize].1.to_string())
        } else {
            (String::new(), String::new())
        };
        let han = if unit_han.is_empty() {
            TEN.0.to_string()
        } else {
            format!("{}{}", TEN.0, unit_han)
        };
        let jyutping = if unit_jp.is_empty() {
            TEN.1.to_string()
        } else {
            format!("{} {}", TEN.1, unit_jp)
        };
        return (han, jyutping);
    }

    if tens == 2 {
        let (unit_han, unit_jp) = if units > 0 {
            (DIGITS[units as usize].0.to_string(), DIGITS[units as usize].1.to_string())
        } else {
            (String::new(), String::new())
        };
        let han = if unit_han.is_empty() {
            TWENTY.0.to_string()
        } else {
            format!("{}{}", TWENTY.0, unit_han)
        };
        let jyutping = if unit_jp.is_empty() {
            TWENTY.1.to_string()
        } else {
            format!("{} {}", TWENTY.1, unit_jp)
        };
        return (han, jyutping);
    }

    (han, jyutping)
}
