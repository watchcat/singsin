//! The curated vocabulary behind the "My details" form.
//!
//! Every entry here is hand-picked and its Jyutping was checked against
//! words.hk — the same source the app plays its reference recordings
//! from. Nothing in this file is guessed: a word that is not in the
//! tables is not offered to the learner, because a wrong tone is the one
//! mistake this app must never make.
//!
//! The English label carries its article ("a teacher", "the
//! Netherlands"), so the phrase built from it reads the way a person
//! would say it.

/// One word the learner can pick for themselves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    /// Stable identifier, stored in the profile.
    pub key: &'static str,
    /// How the word appears in the English phrase.
    pub label: &'static str,
    /// The Chinese.
    pub han: &'static str,
    /// Its Jyutping.
    pub jyutping: &'static str,
}

/// Countries.
///
/// Where the learner is from. A country is spoken as one word, and the
/// spelling is the one Hong Kong uses — the English name is never
/// transliterated letter by letter.
pub const COUNTRIES: &[Entry] = &[
    Entry { key: "netherlands", label: "the Netherlands", han: "荷蘭", jyutping: "ho4 laan4" },
    Entry { key: "germany", label: "Germany", han: "德國", jyutping: "dak1 gwok3" },
    Entry { key: "france", label: "France", han: "法國", jyutping: "faat3 gwok3" },
    Entry { key: "uk", label: "the United Kingdom", han: "英國", jyutping: "jing1 gwok3" },
    Entry { key: "usa", label: "the United States", han: "美國", jyutping: "mei5 gwok3" },
    Entry { key: "russia", label: "Russia", han: "俄羅斯", jyutping: "ngo4 lo4 si1" },
    Entry { key: "china", label: "China", han: "中國", jyutping: "zung1 gwok3" },
    Entry { key: "japan", label: "Japan", han: "日本", jyutping: "jat6 bun2" },
    Entry { key: "south-korea", label: "South Korea", han: "韓國", jyutping: "hon4 gwok3" },
    Entry { key: "canada", label: "Canada", han: "加拿大", jyutping: "gaa1 naa4 daai6" },
    Entry { key: "australia", label: "Australia", han: "澳洲", jyutping: "ou3 zau1" },
    Entry { key: "new-zealand", label: "New Zealand", han: "新西蘭", jyutping: "san1 sai1 laan4" },
    Entry { key: "spain", label: "Spain", han: "西班牙", jyutping: "sai1 baan1 ngaa4" },
    Entry { key: "italy", label: "Italy", han: "意大利", jyutping: "ji3 daai6 lei6" },
    Entry { key: "thailand", label: "Thailand", han: "泰國", jyutping: "taai3 gwok3" },
    Entry { key: "vietnam", label: "Vietnam", han: "越南", jyutping: "jyut6 naam4" },
    Entry { key: "india", label: "India", han: "印度", jyutping: "jan3 dou6" },
    Entry { key: "philippines", label: "the Philippines", han: "菲律賓", jyutping: "fei1 leot6 ban1" },
    Entry { key: "malaysia", label: "Malaysia", han: "馬來西亞", jyutping: "maa5 loi4 sai1 aa3" },
    Entry { key: "singapore", label: "Singapore", han: "新加坡", jyutping: "san1 gaa3 bo1" },
    Entry { key: "indonesia", label: "Indonesia", han: "印尼", jyutping: "jan3 nei4" },
    Entry { key: "sweden", label: "Sweden", han: "瑞典", jyutping: "seoi6 din2" },
    Entry { key: "norway", label: "Norway", han: "挪威", jyutping: "no4 wai1" },
    Entry { key: "denmark", label: "Denmark", han: "丹麥", jyutping: "daan1 mak6" },
    Entry { key: "finland", label: "Finland", han: "芬蘭", jyutping: "fan1 laan4" },
    Entry { key: "ireland", label: "Ireland", han: "愛爾蘭", jyutping: "oi3 ji5 laan4" },
    Entry { key: "poland", label: "Poland", han: "波蘭", jyutping: "bo1 laan4" },
    Entry { key: "austria", label: "Austria", han: "奧地利", jyutping: "ou3 dei6 lei6" },
    Entry { key: "switzerland", label: "Switzerland", han: "瑞士", jyutping: "seoi6 si6" },
    Entry { key: "israel", label: "Israel", han: "以色列", jyutping: "ji5 sik1 lit6" },
    Entry { key: "south-africa", label: "South Africa", han: "南非", jyutping: "naam4 fei1" },
    Entry { key: "argentina", label: "Argentina", han: "阿根廷", jyutping: "aa3 gan1 ting4" },
    Entry { key: "chile", label: "Chile", han: "智利", jyutping: "zi3 lei6" },
    Entry { key: "peru", label: "Peru", han: "秘魯", jyutping: "bei3 lou2" },
    Entry { key: "ukraine", label: "Ukraine", han: "烏克蘭", jyutping: "wu1 hak1 laan4" },
    Entry { key: "hungary", label: "Hungary", han: "匈牙利", jyutping: "hung1 ngaa4 lei6" },
    Entry { key: "czechia", label: "the Czech Republic", han: "捷克", jyutping: "zit3 hak1" },
    Entry { key: "portugal", label: "Portugal", han: "葡萄牙", jyutping: "pou4 tou4 ngaa4" },
];

/// Cities.
///
/// Where the learner lives. A city name is a proper noun with a
/// conventional Chinese form: Berlin is 柏林 (*paak3 lam4*), not a
/// transliteration of the English letters.
pub const CITIES: &[Entry] = &[
    Entry { key: "moscow", label: "Moscow", han: "莫斯科", jyutping: "mok6 si1 fo1" },
    Entry { key: "berlin", label: "Berlin", han: "柏林", jyutping: "paak3 lam4" },
    Entry { key: "london", label: "London", han: "倫敦", jyutping: "leon4 deon1" },
    Entry { key: "paris", label: "Paris", han: "巴黎", jyutping: "baa1 lai4" },
    Entry { key: "new-york", label: "New York", han: "紐約", jyutping: "nau2 joek3" },
    Entry { key: "amsterdam", label: "Amsterdam", han: "阿姆斯特丹", jyutping: "aa3 mou5 si1 dak6 daan1" },
    Entry { key: "madrid", label: "Madrid", han: "馬德里", jyutping: "maa5 dak1 lei5" },
    Entry { key: "sydney", label: "Sydney", han: "悉尼", jyutping: "sik1 nei4" },
    Entry { key: "melbourne", label: "Melbourne", han: "墨爾本", jyutping: "mak6 ji5 bun2" },
    Entry { key: "vancouver", label: "Vancouver", han: "溫哥華", jyutping: "wan1 go1 waa4" },
    Entry { key: "toronto", label: "Toronto", han: "多倫多", jyutping: "do1 leon4 do1" },
    Entry { key: "chicago", label: "Chicago", han: "芝加哥", jyutping: "zi1 gaa1 go1" },
    Entry { key: "manchester", label: "Manchester", han: "曼徹斯特", jyutping: "maan6 cit3 si1 dak6" },
    Entry { key: "zurich", label: "Zurich", han: "蘇黎世", jyutping: "sou1 lai4 sai3" },
    Entry { key: "prague", label: "Prague", han: "布拉格", jyutping: "bou3 laai1 gaak3" },
    Entry { key: "warsaw", label: "Warsaw", han: "華沙", jyutping: "waa4 saa1" },
    Entry { key: "athens", label: "Athens", han: "雅典", jyutping: "ngaa5 din2" },
    Entry { key: "cairo", label: "Cairo", han: "開羅", jyutping: "hoi1 lo4" },
    Entry { key: "dubai", label: "Dubai", han: "迪拜", jyutping: "dik6 baai3" },
    Entry { key: "vienna", label: "Vienna", han: "維也納", jyutping: "wai4 jaa5 naap6" },
    Entry { key: "lisbon", label: "Lisbon", han: "里斯本", jyutping: "lei5 si1 bun2" },
    Entry { key: "brussels", label: "Brussels", han: "布魯塞爾", jyutping: "bou3 lou5 coi3 ji5" },
    Entry { key: "budapest", label: "Budapest", han: "布達佩斯", jyutping: "bou3 daat6 pui3 si1" },
    Entry { key: "kuala-lumpur", label: "Kuala Lumpur", han: "吉隆坡", jyutping: "gat1 lung4 bo1" },
    Entry { key: "jakarta", label: "Jakarta", han: "雅加達", jyutping: "ngaa5 gaa1 daat6" },
    Entry { key: "seoul", label: "Seoul", han: "首爾", jyutping: "sau2 ji5" },
    Entry { key: "kyoto", label: "Kyoto", han: "京都", jyutping: "ging1 dou1" },
    Entry { key: "osaka", label: "Osaka", han: "大阪", jyutping: "daai6 baan2" },
    Entry { key: "bangkok", label: "Bangkok", han: "曼谷", jyutping: "maan6 guk1" },
    Entry { key: "hanoi", label: "Hanoi", han: "河內", jyutping: "ho4 noi6" },
    Entry { key: "rome", label: "Rome", han: "羅馬", jyutping: "lo4 maa5" },
];

/// Jobs.
///
/// What the learner does for a living. The words are the ones a Hong
/// Kong speaker would use — for a programmer 程式員 (*sing4 sik1
/// jyun4*), not the Mandarin 程序员. The English label carries its
/// article, so the phrase reads "as an engineer" and "as a teacher".
pub const JOBS: &[Entry] = &[
    Entry { key: "teacher", label: "a teacher", han: "教師", jyutping: "gaau3 si1" },
    Entry { key: "nurse", label: "a nurse", han: "護士", jyutping: "wu6 si6" },
    Entry { key: "doctor", label: "a doctor", han: "醫生", jyutping: "ji1 sang1" },
    Entry { key: "lawyer", label: "a lawyer", han: "律師", jyutping: "leot6 si1" },
    Entry { key: "accountant", label: "an accountant", han: "會計", jyutping: "wui6 gai3" },
    Entry { key: "engineer", label: "an engineer", han: "工程師", jyutping: "gung1 cing4 si1" },
    Entry { key: "designer", label: "a designer", han: "設計師", jyutping: "cit3 gai3 si1" },
    Entry { key: "cook", label: "a cook", han: "廚師", jyutping: "cyu4 si1" },
    Entry { key: "product-manager", label: "a product manager", han: "產品經理", jyutping: "caan2 ban2 ging1 lei5" },
    Entry { key: "reporter", label: "a reporter", han: "記者", jyutping: "gei3 ze2" },
    Entry { key: "editor", label: "an editor", han: "編輯", jyutping: "pin1 cap1" },
    Entry { key: "driver", label: "a driver", han: "司機", jyutping: "si1 gei1" },
    Entry { key: "shop-assistant", label: "a shop assistant", han: "售貨員", jyutping: "sau6 fo3 jyun4" },
    Entry { key: "dentist", label: "a dentist", han: "牙醫", jyutping: "ngaa4 ji1" },
    Entry { key: "pharmacist", label: "a pharmacist", han: "藥劑師", jyutping: "joek6 zai1 si1" },
    Entry { key: "police-officer", label: "a police officer", han: "警員", jyutping: "ging2 jyun4" },
    Entry { key: "firefighter", label: "a firefighter", han: "消防員", jyutping: "siu1 fong4 jyun4" },
    Entry { key: "architect", label: "an architect", han: "建築師", jyutping: "gin3 zuk1 si1" },
    Entry { key: "film-director", label: "a film director", han: "導演", jyutping: "dou6 jin2" },
    Entry { key: "singer", label: "a singer", han: "歌手", jyutping: "go1 sau2" },
    Entry { key: "actor", label: "an actor", han: "演員", jyutping: "jin2 jyun4" },
    Entry { key: "writer", label: "a writer", han: "作家", jyutping: "zok3 gaa1" },
    Entry { key: "painter", label: "a painter", han: "畫家", jyutping: "waa2 gaa1" },
    Entry { key: "photographer", label: "a photographer", han: "攝影師", jyutping: "sip3 jing2 si1" },
    Entry { key: "cashier", label: "a cashier", han: "收銀員", jyutping: "sau1 ngan2 jyun4" },
    Entry { key: "hairdresser", label: "a hairdresser", han: "髮型師", jyutping: "faat3 jing4 si1" },
    Entry { key: "tour-guide", label: "a tour guide", han: "導遊", jyutping: "dou6 jau4" },
    Entry { key: "flight-attendant", label: "a flight attendant", han: "空姐", jyutping: "hung1 ze2" },
    Entry { key: "sales-assistant", label: "a sales assistant", han: "業務員", jyutping: "jip6 mou6 jyun4" },
    Entry { key: "manager", label: "a manager", han: "經理", jyutping: "ging1 lei5" },
    Entry { key: "boss", label: "the boss", han: "老闆", jyutping: "lou5 baan2" },
    Entry { key: "ship-crew", label: "a crew member", han: "船員", jyutping: "syun4 jyun4" },
    Entry { key: "pilot", label: "a pilot", han: "飛行員", jyutping: "fei1 haang4 jyun4" },
    Entry { key: "courier", label: "a courier", han: "速遞員", jyutping: "suk6 dai3 jyun4" },
    Entry { key: "farmer", label: "a farmer", han: "農民", jyutping: "nung4 man4" },
    Entry { key: "merchant", label: "a merchant", han: "商人", jyutping: "soeng1 jan4" },
    Entry { key: "banker", label: "a banker", han: "銀行家", jyutping: "ngan4 haang4 gaa1" },
];

/// Hobbies.
///
/// What the learner does for fun. These are the everyday words, not
/// the formal ones: 睇戲 (*tai2 hei3*) for films, 踩單車 (*caai2
/// daan1 ce1*) for cycling.
pub const HOBBIES: &[Entry] = &[
    Entry { key: "reading", label: "reading", han: "睇書", jyutping: "tai2 syu1" },
    Entry { key: "films", label: "films", han: "睇戲", jyutping: "tai2 hei3" },
    Entry { key: "music", label: "music", han: "聽歌", jyutping: "teng1 go1" },
    Entry { key: "travelling", label: "travelling", han: "旅行", jyutping: "lei5 haang4" },
    Entry { key: "sport", label: "sport", han: "做運動", jyutping: "zou6 wun6 dung6" },
    Entry { key: "cooking", label: "cooking", han: "煮飯", jyutping: "zyu2 faan6" },
    Entry { key: "painting", label: "painting", han: "畫畫", jyutping: "waak6 waak6" },
    Entry { key: "cycling", label: "cycling", han: "踩單車", jyutping: "caai2 daan1 ce1" },
    Entry { key: "swimming", label: "swimming", han: "游水", jyutping: "jau4 seoi2" },
    Entry { key: "running", label: "running", han: "跑步", jyutping: "paau2 bou6" },
    Entry { key: "dim-sum", label: "dim sum", han: "飲茶", jyutping: "jam2 caa4" },
    Entry { key: "football", label: "football", han: "打波", jyutping: "daa2 bo1" },
    Entry { key: "skiing", label: "skiing", han: "滑雪", jyutping: "waat6 syut3" },
    Entry { key: "hiking", label: "hiking", han: "爬山", jyutping: "paa4 saan1" },
    Entry { key: "writing", label: "writing", han: "寫字", jyutping: "se2 zi6" },
];

/// Family.
///
/// The people the learner lives with. The words carry the kinship
/// itself — 老公 is the husband and 婆婆 the mother-in-law, which
/// English keeps apart.
pub const FAMILY: &[Entry] = &[
    Entry { key: "husband", label: "husband", han: "老公", jyutping: "lou5 gung1" },
    Entry { key: "wife", label: "wife", han: "老婆", jyutping: "lou5 po4" },
    Entry { key: "son", label: "son", han: "仔", jyutping: "zai2" },
    Entry { key: "daughter", label: "daughter", han: "女仔", jyutping: "neoi5 zai2" },
    Entry { key: "children", label: "children", han: "仔女", jyutping: "zai2 neoi2" },
    Entry { key: "father", label: "father", han: "爸爸", jyutping: "baa4 baa1" },
    Entry { key: "mother", label: "mother", han: "媽媽", jyutping: "maa4 maa1" },
    Entry { key: "older-brother", label: "older brother", han: "哥哥", jyutping: "go4 go1" },
    Entry { key: "older-sister", label: "older sister", han: "姐姐", jyutping: "ze4 ze1" },
    Entry { key: "younger-brother", label: "younger brother", han: "弟弟", jyutping: "di4 di2" },
    Entry { key: "younger-sister", label: "younger sister", han: "妹妹", jyutping: "mui4 mui2" },
    Entry { key: "mother-in-law", label: "mother-in-law", han: "婆婆", jyutping: "po4 po2" },
    Entry { key: "father-in-law", label: "father-in-law", han: "公公", jyutping: "gung4 gung1" },
];

/// Find an entry by its key in one of the tables.
pub fn find(table: &'static [Entry], key: &str) -> Option<&'static Entry> {
    table.iter().find(|e| e.key == key)
}
