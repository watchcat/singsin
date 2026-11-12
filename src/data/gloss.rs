//! Per-syllable English glosses.
//!
//! Cantonese is largely morphosyllabic, so a gloss for every character gives
//! a learner something concrete to hold on to: 荷蘭 becomes *"lotus" +
//! "orchid"*, while the phrase-level English above it says *"the
//! Netherlands"*. That pairing is what makes the characters recognisable the
//! next time they turn up.
//!
//! The table is keyed by character and consulted per syllable, so it covers
//! the bundled phrasebook **and** anything the user types into the editor.
//! A character that is not in the table simply has no gloss — the rest of the
//! analysis is unaffected.
//!
//! Particles carry their meaning in the grammar rather than in a word, so
//! they are glossed as such (嘅 = *"(possessive)"* rather than a bogus noun).

/// One English gloss per Chinese character.
///
/// These sit under the character on a card, so keep them short.
const GLOSSES: &[(&str, &str)] = &[
    ("一", "one"),
    ("下", "down / below"),
    ("中", "middle / China"),
    ("二", "two"),
    ("五", "five"),
    ("今", "now / this"),
    ("介", "introduces a person"),
    ("以", "using / by"),
    ("件", "classifier for things"),
    ("休", "rest"),
    ("位", "classifier for people"),
    ("低", "low / put down"),
    ("住", "live / stay"),
    ("你", "you"),
    ("使", "use / let"),
    ("係", "is / to be"),
    ("俾", "give / for"),
    ("個", "classifier (one, a)"),
    ("借", "borrow"),
    ("做", "make / do"),
    ("停", "stop"),
    ("充", "charge / fill"),
    ("兩", "two / both"),
    ("冇", "not / don't have"),
    ("再", "again"),
    ("冰", "ice"),
    ("出", "out"),
    ("到", "arrive / until"),
    ("十", "ten"),
    ("卡", "card"),
    ("去", "go"),
    ("口", "mouth"),
    ("叫", "call / be called"),
    ("可", "can / may"),
    ("名", "name"),
    ("呢", "this"),
    ("咖", "coffee"),
    ("咩", "what"),
    ("咪", "is it (question)"),
    ("哋", "we / they"),
    ("唔", "not"),
    ("問", "ask"),
    ("啡", "coffee"),
    ("啦", "casual particle"),
    ("啫", "(only) particle"),
    ("啲", "some"),
    ("喇", "emphatic particle"),
    ("單", "single / bill"),
    ("喺", "at / in"),
    ("嗎", "question particle"),
    ("嗰", "that"),
    ("嘅", "possessive"),
    ("器", "device"),
    ("嚟", "come / from"),
    ("圖", "map / picture"),
    ("地", "ground / place"),
    ("埋", "together / the bill"),
    ("報", "report / newspaper"),
    ("士", "person (as in 的士)"),
    ("外", "outside / other"),
    ("多", "many / more"),
    ("太", "too"),
    ("好", "good / very"),
    ("家", "home / shop"),
    ("密", "dense / close"),
    ("寫", "write"),
    ("對", "right / correct"),
    ("少", "few / less"),
    ("就", "then / exactly"),
    ("巴", "bus (as in 巴士)"),
    ("幫", "help"),
    ("年", "year"),
    ("幾", "how many"),
    ("店", "shop"),
    ("度", "place / times"),
    ("廣", "wide / Guangdong"),
    ("張", "classifier for flat things"),
    ("得", "can / get"),
    ("思", "think"),
    ("息", "rest / breath"),
    ("想", "think / want"),
    ("意", "meaning / idea"),
    ("感", "feel"),
    ("慢", "slow"),
    ("我", "I / me"),
    ("手", "hand"),
    ("指", "finger"),
    ("推", "push"),
    ("搭", "take / match with"),
    ("據", "according to"),
    ("收", "receive"),
    ("敏", "quick / clever"),
    ("救", "rescue / save"),
    ("文", "text / writing"),
    ("日", "day / sun"),
    ("明", "bright / tomorrow"),
    ("晚", "evening"),
    ("最", "most"),
    ("會", "can / will"),
    ("有", "have"),
    ("服", "clothes / serve"),
    ("杯", "cup"),
    ("東", "east"),
    ("梯", "stairs"),
    ("次", "time (occasion)"),
    ("歲", "years old"),
    ("水", "water"),
    ("洗", "wash"),
    ("港", "harbour / Hong Kong"),
    ("渴", "thirsty"),
    ("熱", "hot"),
    ("牌", "card / sign"),
    ("班", "class / shift"),
    ("現", "now / show"),
    ("環", "around / ring"),
    ("生", "raw / life"),
    ("用", "use"),
    ("痛", "painful"),
    ("白", "white"),
    ("的", "possessive"),
    ("睇", "look"),
    ("知", "know"),
    ("碼", "number / code"),
    ("站", "station / stand"),
    ("第", "ordinal prefix"),
    ("糖", "sugar"),
    ("約", "about / arrange"),
    ("素", "vegetarian / plain"),
    ("細", "small / thin"),
    ("緊", "tight / progressive"),
    ("而", "and / but"),
    ("聽", "listen"),
    ("肉", "meat"),
    ("肚", "belly"),
    ("自", "from / self"),
    ("興", "interest"),
    ("舒", "comfortable"),
    ("舖", "shop"),
    ("色", "colour"),
    ("花", "flower / spend"),
    ("英", "English"),
    ("茶", "tea"),
    ("荷", "lotus"),
    ("落", "fall / drop"),
    ("蘭", "orchid"),
    ("行", "walk / OK / row"),
    ("袋", "bag"),
    ("要", "want / need"),
    ("見", "see"),
    ("試", "try"),
    ("話", "speech / say"),
    ("該", "should"),
    ("講", "speak"),
    ("謝", "thank"),
    ("識", "know / recognise"),
    ("警", "police"),
    ("護", "nurse / protect"),
    ("貴", "expensive"),
    ("賣", "sell"),
    ("走", "walk / leave"),
    ("車", "car / vehicle"),
    ("辣", "spicy"),
    ("近", "near"),
    ("返", "return / back"),
    ("遠", "far"),
    ("遲", "late"),
    ("邊", "side / which"),
    ("酒", "alcohol / wine"),
    ("醫", "doctor / medicine"),
    ("金", "gold / money"),
    ("錢", "money"),
    ("鐵", "iron"),
    ("開", "open"),
    ("閒", "free / idle"),
    ("間", "between / room"),
    ("院", "institution"),
    ("隻", "classifier for animals"),
    ("雨", "rain"),
    ("電", "electricity"),
    ("需", "need"),
    ("頭", "head"),
    ("題", "question / topic"),
    ("飛", "fly"),
    ("食", "eat"),
    ("飯", "meal / rice"),
    ("餐", "meal"),
    ("餓", "hungry"),
    ("高", "tall / high"),
    ("點", "how / what time"),
    ("齊", "together / neat"),
    ("稱", "call / address"),
    ("呼", "call out / address"),
];

/// Glosses for a character that changes meaning with its reading.
///
/// 呢 is the case in the bundled library: *ni1* is the demonstrative "this"
/// (呢個, 呢度), but *ne1* is the question particle that closes a question.
/// The character table cannot tell the two apart, so the readings that mean
/// something different are listed here and matched on the Jyutping.
const BY_READING: &[(&str, &str, &str)] = &[
    ("呢", "ne1", "question particle"),
];

/// The English gloss for one Chinese character, if it is known.
///
/// Pass the whole unit, not just its first byte, so a multi-byte character
/// matches exactly.
pub fn gloss(han: &str) -> Option<&'static str> {
    GLOSSES
        .iter()
        .find(|(c, _)| *c == han)
        .map(|(_, g)| *g)
}

/// The gloss for one character *as it is read here*.
///
/// The reading-specific table wins where it applies — 呢 is "question
/// particle" as *ne1* and "this" as *ni1* — and everything else falls back
/// to the character table.
pub fn gloss_for(han: &str, jyutping: &str) -> Option<&'static str> {
    if let Some((_, _, g)) = BY_READING.iter().find(|(c, j, _)| *c == han && *j == jyutping) {
        return Some(*g);
    }
    gloss(han)
}

/// The gloss for every character of `han`, in order.
///
/// Characters that are not in the table produce an empty string, so the
/// result always lines up with the characters. The readings are not
/// consulted: this walks the characters alone, so a character whose meaning
/// depends on its reading gets the commonest one.
pub fn gloss_all(han: &str) -> Vec<String> {
    han.chars()
        .map(|c| gloss(&c.to_string()).unwrap_or("").to_string())
        .collect()
}
