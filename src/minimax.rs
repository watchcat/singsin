//! Optional MiniMax LLM enrichment.
//!
//! When a learner adds or edits a phrase, the deterministic Rust engine
//! already produces a complete deep-dive. This module is the optional
//! upgrade on top of it: bespoke colloquial notes, a custom mistakes table
//! and natural sentence-particle drills, written by the MiniMax chat API
//! using the learner's own API key.

use serde_json::{json, Value};

use crate::data::phrases::RichNotes;
use crate::storage::Settings;

const ENDPOINTS: [&str; 2] = [
    "https://api.minimax.chat/v1/text/chatcompletion_v2",
    "https://api.minimax.io/v1/chat/completions",
];

const MODEL: &str = "MiniMax-Text-01";

/// The instruction sent to the model.
fn system_prompt() -> String {
    format!(
        "You are a Cantonese (Hong Kong, 粵語) phonetics teacher writing for a {} \
         speaker who already speaks Mandarin. \
         Reply with STRICT JSON only — no prose, no markdown fences. \
         Schema: {{\"sandhi\": [string], \"mistakes\": [{{\"wrong\": string, \"right\": string, \"why\": string}}], \
         \"drills\": [{{\"n\": number, \"title\": string, \"body\": string, \"focus\": [string]}}], \"takeaway\": string}}. \
         Use Traditional Chinese characters (香港正體字). \
         \"mistakes\" compares the Mandarin-flavoured pronunciation against the correct Cantonese one. \
         \"sandhi\" lists the real spoken tone changes in the sentence. \
         \"drills\" must contain exactly 5 passes with n = 1..5 in this order: vowels/sound anchors only, \
         add tone shapes, put consonants back, full phrase slowly, real sentence with a final particle. \
         \"takeaway\" is one punchy paragraph of Markdown that names the pitch sequence and the checkpoints.",
        "Mandarin"
    )
}

/// The user prompt for one phrase.
fn user_prompt(han: &str, jyutping: &str, en: &str) -> String {
    format!(
        "Phrase (Traditional Chinese): {han}\nJyutping: {jyutping}\nEnglish: {en}\n\n\
         Write the JSON described above."
    )
}

/// Ask MiniMax for rich notes about one phrase.
pub async fn enrich(
    settings: &Settings,
    han: &str,
    jyutping: &str,
    en: &str,
) -> Result<RichNotes, String> {
    if settings.mini_max_api_key.trim().is_empty() {
        return Err("No MiniMax API key saved — add one in Audio Settings.".into());
    }

    let body = json!({
        "model": MODEL,
        "messages": [
            { "role": "system", "content": system_prompt() },
            { "role": "user",   "content": user_prompt(han, jyutping, en) },
        ],
        "stream": false,
    });

    let mut last_error = String::from("no endpoint answered");
    for endpoint in ENDPOINTS {
        match call(endpoint, &settings.mini_max_api_key, &body).await {
            Ok(text) => match parse_notes(&text) {
                Ok(notes) => return Ok(notes),
                Err(e) => last_error = e,
            },
            Err(e) => last_error = e,
        }
    }
    Err(last_error)
}

/// Perform one chat-completion request.
async fn call(endpoint: &str, api_key: &str, body: &Value) -> Result<String, String> {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let window = web_sys::window().ok_or("No window")?;

    let headers = web_sys::Headers::new().map_err(|e| format!("{e:?}"))?;
    headers.set("Content-Type", "application/json").ok();
    headers.set("Authorization", &format!("Bearer {api_key}")).ok();

    let init = RequestInit::new();
    init.set_method("POST");
    init.set_mode(RequestMode::Cors);
    init.set_headers(&headers);
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    init.set_body(&JsValue::from_str(&json));

    let request = Request::new_with_str_and_init(endpoint, &init).map_err(|e| format!("{e:?}"))?;
    let promise = window.fetch_with_request(&request);
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| "network error".to_string())?;
    let response: Response = value.dyn_into().map_err(|_| "bad response".to_string())?;

    let text = JsFuture::from(response.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|_| "read failed".to_string())?
        .as_string()
        .unwrap_or_default();

    if !response.ok() {
        return Err(format!("MiniMax HTTP {}: {}", response.status(), text));
    }

    let json: Value = serde_json::from_str(&text).map_err(|e| format!("bad JSON: {e}"))?;
    let content = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or("MiniMax response had no content")?;

    // The v2 endpoint sometimes returns the JSON as a string inside `content`.
    Ok(content.to_string())
}

/// Turn the model's reply into structured notes, tolerating markdown fences
/// and stray prose around the JSON.
fn parse_notes(text: &str) -> Result<RichNotes, String> {
    let trimmed = text.trim();
    let cleaned = strip_fences(trimmed);

    let candidate = match serde_json::from_str::<Value>(&cleaned) {
        Ok(v) => v,
        Err(_) => {
            // Try to pull the first balanced JSON object out of the reply.
            let start = cleaned
                .find('{')
                .ok_or("the reply contained no JSON object")?;
            let end = cleaned
                .rfind('}')
                .ok_or("the reply contained no JSON object")?;
            serde_json::from_str::<Value>(&cleaned[start..=end])
                .map_err(|e| format!("could not read model JSON: {e}"))?
        }
    };

    let mut notes = RichNotes::default();

    if let Some(items) = candidate.get("sandhi").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(s) = item.as_str() {
                notes.sandhi.push(s.to_string());
            }
        }
    }

    if let Some(items) = candidate.get("mistakes").and_then(|v| v.as_array()) {
        for item in items {
            let wrong = item.get("wrong").and_then(|v| v.as_str()).unwrap_or("");
            let right = item.get("right").and_then(|v| v.as_str()).unwrap_or("");
            let why = item.get("why").and_then(|v| v.as_str()).unwrap_or("");
            if wrong.is_empty() && right.is_empty() {
                continue;
            }
            notes.mistakes.push(crate::data::phonetics::MistakeRow {
                wrong: wrong.to_string(),
                right: right.to_string(),
                why: why.to_string(),
            });
        }
    }

    if let Some(items) = candidate.get("drills").and_then(|v| v.as_array()) {
        for item in items {
            let n = item.get("n").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            if n == 0 || n > 5 {
                continue;
            }
            let title = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Practice")
                .to_string();
            let body = item
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let focus = item
                .get("focus")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            notes.drills.push(crate::data::phonetics::DrillPass {
                n,
                title,
                body,
                focus,
            });
        }
    }

    if let Some(t) = candidate.get("takeaway").and_then(|v| v.as_str()) {
        notes.takeaway = Some(t.to_string());
    }

    if notes.sandhi.is_empty()
        && notes.mistakes.is_empty()
        && notes.drills.is_empty()
        && notes.takeaway.is_none()
    {
        return Err("MiniMax returned no usable notes".into());
    }

    Ok(notes)
}

/// Remove markdown code fences from a reply.
fn strip_fences(text: &str) -> String {
    let trimmed = text.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let without_open = trimmed.trim_start_matches("```");
    let without_open = match without_open.find('\n') {
        Some(i) => &without_open[i + 1..],
        None => without_open,
    };
    without_open
        .trim_end()
        .trim_end_matches("```")
        .trim()
        .to_string()
}
