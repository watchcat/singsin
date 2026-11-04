//! The four modals: tone guide, audio settings, phrase editor and
//! import / export.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use yew::prelude::*;

use crate::audio::tts;
use crate::data::phrases::Phrase;
use crate::storage::Settings;

use super::Icon;

// ── shell ───────────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct ShellProps {
    pub title: AttrValue,
    #[prop_or_default]
    pub subtitle: Option<AttrValue>,
    pub on_close: Callback<()>,
    pub children: Children,
}

#[function_component(ModalShell)]
pub fn modal_shell(props: &ShellProps) -> Html {
    let on_close = props.on_close.clone();

    let close_click = {
        let on_close = on_close.clone();
        Callback::from(move |_: MouseEvent| on_close.emit(()))
    };

    let backdrop = {
        let on_close = on_close.clone();
        Callback::from(move |e: MouseEvent| {
            if let Some(target) = e.target_dyn_into::<web_sys::HtmlElement>() {
                if target.class_name().contains("modal-backdrop") {
                    on_close.emit(());
                }
            }
        })
    };

    {
        let on_close = on_close.clone();
        use_effect_with((), move |_| {
            let closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if e.key() == "Escape" {
                    on_close.emit(());
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
            if let Some(w) = web_sys::window() {
                let _ =
                    w.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            }
            move || {
                if let Some(w) = web_sys::window() {
                    let _ = w.remove_event_listener_with_callback(
                        "keydown",
                        closure.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    html! {
        <div class="modal-backdrop" onclick={backdrop} role="presentation">
            <div class="modal" role="dialog" aria-modal="true" aria-label={props.title.clone()}>
                <header class="modal-head">
                    <div>
                        <h2>{ props.title.clone() }</h2>
                        if let Some(sub) = props.subtitle.clone() {
                            <p class="modal-sub">{ sub }</p>
                        }
                    </div>
                    <button type="button" class="modal-close" aria-label="Close"
                            onclick={close_click}>
                        { Icon::Close.svg() }
                    </button>
                </header>
                <div class="modal-body">{ props.children.clone() }</div>
            </div>
        </div>
    }
}

// ── tone guide ─────────────────────────────────────────────────────────────

/// The six Cantonese tones, with their Chao numbers and a spoken example.
const TONE_TABLE: [(u8, &str, &str, &str, &str); 6] = [
    (1, "55", "high level", "詩 si1", "Hold the note high and completely flat."),
    (2, "25", "rising to high", "史 si2", "Start at mid height, then rise all the way to the top."),
    (3, "33", "mid level", "試 si3", "Steady mid-level — Cantonese tone 3 does *not* dip."),
    (4, "21", "falling to low", "時 si4", "Start high and fall to the bottom of your range."),
    (5, "13", "rising from low", "市 si5", "Start at the bottom and float upward."),
    (6, "22", "low level", "是 si6", "Low and completely flat — no rise, no fall."),
];

#[function_component(ToneGuide)]
pub fn tone_guide() -> Html {
    html! {
        <>
            <p class="lead">
                { "Cantonese has six tones, written with the numbers 1–6 after each syllable in Jyutping. \
                   The Chao numbers below describe the shape of the pitch: the first digit is where the \
                   syllable starts and the second is where it ends." }
            </p>
            <div class="tone-guide">
                { for TONE_TABLE.iter().map(|(tone, chao, name, example, tip)| {
                    let (from, to) = crate::dsp::contour::tone_levels(*tone);
                    let path = crate::dsp::contour::shape_path(from, to, 18.0, 122.0);
                    html! {
                        <article class="tg" key={*tone}>
                            <div class="tg-head">
                                <span class="tg-num">{ tone.to_string() }</span>
                                <span class="tg-chao">{ chao.to_string() }</span>
                                <span class="tg-name">{ name.to_string() }</span>
                            </div>
                            <svg class="contour" viewBox="0 0 140 80" role="img"
                                 aria-label={format!("Tone {tone}, Chao {chao}")}>
                                { for crate::dsp::contour::grid_lines().iter().map(|(_, y)| {
                                    let y = format!("{y:.1}");
                                    html! { <line x1="8" y1={y.clone()} x2="132" y2={y} class="gl" /> }
                                })}
                                <path d={path} class="tone-path" />
                            </svg>
                            <p class="tg-example mono">{ example.to_string() }</p>
                            <p class="tg-tip">{ tip.to_string() }</p>
                        </article>
                    }
                })}
            </div>
            <div class="callout">
                <p>
                    <strong>{ "Sandhi — when the tone changes." }</strong>
                    { " In real speech two tones can combine. Two tone-3 syllables in a row become tone 2 then tone 3. \
                       A tone 4 followed by another tone 4 lifts the second one to a high tone 1, so 荷蘭 sounds \
                       ho4 laan1, not ho4 laan4." }
                </p>
            </div>
            <div class="callout clay">
                <p>
                    <strong>{ "Tone 3 is the big one." }</strong>
                    { " Mandarin speakers dip their third tone. Cantonese tone 3 is a flat mid level — if your \
                       third tone sounds like a question, you are speaking Mandarin, not Cantonese." }
                </p>
            </div>
        </>
    }
}

// ── audio settings ─────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct SettingsProps {
    pub settings: Settings,
    pub on_settings: Callback<Settings>,
    pub on_close: Callback<()>,
}

#[function_component(AudioSettings)]
pub fn audio_settings(props: &SettingsProps) -> Html {
    let settings = props.settings.clone();
    let on_settings = props.on_settings.clone();

    let set_key = {
        let on_settings = on_settings.clone();
        let settings = settings.clone();
        Callback::from(move |e: InputEvent| {
            let target: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut next = settings.clone();
            next.mini_max_api_key = target.value();
            on_settings.emit(next);
        })
    };

    let set_proxy = {
        let on_settings = on_settings.clone();
        let settings = settings.clone();
        Callback::from(move |e: InputEvent| {
            let target: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut next = settings.clone();
            next.audio_proxy = target.value();
            on_settings.emit(next);
        })
    };

    let set_voice = {
        let on_settings = on_settings.clone();
        let settings = settings.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let mut next = settings.clone();
            next.tts_voice = target.value();
            on_settings.emit(next);
        })
    };

    let toggle_split = {
        let on_settings = on_settings.clone();
        let settings = settings.clone();
        Callback::from(move |value: bool| {
            let mut next = settings.clone();
            next.stereo_split = value;
            on_settings.emit(next);
        })
    };

    let toggle_sync = {
        let on_settings = on_settings.clone();
        let settings = settings.clone();
        Callback::from(move |value: bool| {
            let mut next = settings.clone();
            next.time_sync = value;
            on_settings.emit(next);
        })
    };

    let toggle_slow = {
        let on_settings = on_settings.clone();
        let settings = settings.clone();
        Callback::from(move |value: bool| {
            let mut next = settings.clone();
            next.slow_tts = value;
            on_settings.emit(next);
        })
    };

    // The browser's voice list arrives asynchronously — an empty list at this
    // point means "not loaded yet", not "no Cantonese voice". Watch for it.
    let voice_status = use_state(|| tts::VoiceStatus::Loading);
    {
        let voice_status = voice_status.clone();
        use_effect_with((), move |_| {
            let cb = Callback::from(move |st: tts::VoiceStatus| voice_status.set(st));
            tts::watch_voices(cb);
            || ()
        });
    }

    let voices = match &*voice_status {
        tts::VoiceStatus::Loading => None,
        tts::VoiceStatus::Ready(v) => Some(v.clone()),
    };
    let voices_loading = voices.is_none();
    let voices_empty = voices.as_ref().map(|v| v.is_empty()).unwrap_or(false);

    html! {
        <>
            <section class="settings-block">
                <h3>{ "Reference audio" }</h3>
                <p class="hint">
                    { "The native recordings come from words.hk. That site does not send CORS headers, so the app \
                       races several relays and uses whichever answers first. A local relay ships with the \
                       project — run `python3 tools/proxy.py` and it is tried first. If you run your own \
                       proxy, put its prefix here and it will be tried too." }
                </p>
                <label class="field">
                    <span>{ "CORS proxy prefix (optional)" }</span>
                    <input type="text" placeholder="https://your-proxy.example/?url="
                           value={settings.audio_proxy.clone()} oninput={set_proxy} />
                </label>
            </section>

            <section class="settings-block">
                <h3>{ "Browser Cantonese voice" }</h3>
                { if voices_loading {
                    html! {
                        <p class="hint">
                            { "Checking the browser's voices… they arrive a moment after the page loads." }
                        </p>
                    }
                } else if voices_empty {
                    html! {
                        <p class="hint">
                            { "This browser exposes no Cantonese (yue-HK / zh-HK) voice. The native words.hk recordings \
                               are used instead wherever they can be reached." }
                        </p>
                    }
                } else {
                    let list = voices.clone().unwrap_or_default();
                    html! {
                        <label class="field">
                            <span>{ "Voice" }</span>
                            <select value={settings.tts_voice.clone()} onchange={set_voice}>
                                <option value="">{ "Automatic (best Cantonese voice)" }</option>
                                { for list.iter().map(|v| {
                                    let uri = v.uri.clone();
                                    html! {
                                        <option value={uri.clone()} selected={uri == settings.tts_voice}>
                                            { format!("{} — {}", v.name, v.lang) }
                                        </option>
                                    }
                                })}
                            </select>
                        </label>
                    }
                } }
                <div class="settings-toggles">
                    <super::Toggle label="Slow TTS (0.72×)" checked={settings.slow_tts}
                                   on_toggle={toggle_slow} />
                    <super::Toggle label="Time-sync mine to the original" checked={settings.time_sync}
                                   on_toggle={toggle_sync} />
                    <super::Toggle label="Stereo split (original left, mine right)"
                                   checked={settings.stereo_split} on_toggle={toggle_split} />
                </div>
            </section>

            <section class="settings-block">
                <h3>{ "MiniMax LLM (optional)" }</h3>
                <p class="hint">
                    { "Your API key is stored only in this browser's localStorage. It is used solely to enrich \
                       phrases you add or edit with bespoke sandhi notes, a custom mistakes table and natural \
                       sentence-particle drills." }
                </p>
                <label class="field">
                    <span>{ "API key" }</span>
                    <input type="password" placeholder="sk-…" autocomplete="off"
                           value={settings.mini_max_api_key.clone()} oninput={set_key} />
                </label>
            </section>
        </>
    }
}

// ── phrase editor ──────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    /// `None` when adding a new phrase.
    pub phrase: Option<Phrase>,
    pub settings: Settings,
    pub on_save: Callback<Phrase>,
    pub on_delete: Callback<i64>,
    pub on_close: Callback<()>,
}

#[function_component(PhraseEditor)]
pub fn phrase_editor(props: &EditorProps) -> Html {
    let existing = props.phrase.clone();
    let next_id = existing
        .as_ref()
        .map(|p| p.id)
        .unwrap_or_else(|| crate::data::phrases::seed_phrases().len() as i64 + 1000);

    let category = use_state(|| existing.as_ref().map(|p| p.category.clone()).unwrap_or_default());
    let en = use_state(|| existing.as_ref().map(|p| p.en.clone()).unwrap_or_default());
    let han = use_state(|| existing.as_ref().map(|p| p.han.clone()).unwrap_or_default());
    let jyutping = use_state(|| existing.as_ref().map(|p| p.jyutping.clone()).unwrap_or_default());
    let spoken = use_state(|| {
        existing
            .as_ref()
            .and_then(|p| p.spoken_jyutping.clone())
            .unwrap_or_default()
    });
    let notes = use_state(|| existing.as_ref().and_then(|p| p.notes.clone()).unwrap_or_default());
    let generating = use_state(|| false);
    let gen_error = use_state(|| Option::<String>::None);
    let rich = use_state(|| existing.as_ref().and_then(|p| p.rich.clone()));

    // The editor can be re-used for a different phrase, and Yew keeps the
    // component's state alive across those swaps. Without this reset the form
    // would keep the *previous* phrase's spoken Jyutping and quietly write it
    // onto the next phrase the learner saves.
    {
        let phrase = props.phrase.clone();
        let category = category.clone();
        let en = en.clone();
        let han = han.clone();
        let jyutping = jyutping.clone();
        let spoken = spoken.clone();
        let notes = notes.clone();
        let rich = rich.clone();
        let key = phrase.as_ref().map(|p| p.id).unwrap_or(0);

        use_effect_with(key, move |_| {
            category.set(phrase.as_ref().map(|p| p.category.clone()).unwrap_or_default());
            en.set(phrase.as_ref().map(|p| p.en.clone()).unwrap_or_default());
            han.set(phrase.as_ref().map(|p| p.han.clone()).unwrap_or_default());
            jyutping.set(phrase.as_ref().map(|p| p.jyutping.clone()).unwrap_or_default());
            spoken.set(
                phrase
                    .as_ref()
                    .and_then(|p| p.spoken_jyutping.clone())
                    .unwrap_or_default(),
            );
            notes.set(phrase.as_ref().and_then(|p| p.notes.clone()).unwrap_or_default());
            rich.set(phrase.as_ref().and_then(|p| p.rich.clone()));
            || ()
        });
    }

    let on_save = props.on_save.clone();

    let save = {
        let category = category.clone();
        let en = en.clone();
        let han = han.clone();
        let jyutping = jyutping.clone();
        let spoken = spoken.clone();
        let notes = notes.clone();
        let rich = rich.clone();
        let on_save = on_save.clone();
        let id = next_id;
        let is_new = existing.is_none();
        Callback::from(move |_: MouseEvent| {
            let phrase = Phrase {
                id,
                category: (*category).trim().to_string(),
                en: (*en).trim().to_string(),
                han: (*han).trim().to_string(),
                jyutping: (*jyutping).trim().to_string(),
                spoken_jyutping: {
                    let s = spoken.trim();
                    if s.is_empty() || is_new {
                        None
                    } else {
                        Some(s.to_string())
                    }
                },
                notes: {
                    let n = notes.trim();
                    if n.is_empty() {
                        None
                    } else {
                        Some(n.to_string())
                    }
                },
                rich: (*rich).clone(),
            };
            on_save.emit(phrase);
        })
    };

    let generate = {
        let settings = props.settings.clone();
        let han = han.clone();
        let jyutping = jyutping.clone();
        let en = en.clone();
        let rich = rich.clone();
        let generating = generating.clone();
        let gen_error = gen_error.clone();
        Callback::from(move |_: MouseEvent| {
            let settings = settings.clone();
            let h = (*han).clone();
            let j = (*jyutping).clone();
            let e = (*en).clone();
            let rich = rich.clone();
            let generating = generating.clone();
            let gen_error = gen_error.clone();
            generating.set(true);
            gen_error.set(None);
            spawn_local(async move {
                match crate::minimax::enrich(&settings, &h, &j, &e).await {
                    Ok(notes) => {
                        let mut next = (*rich).clone().unwrap_or_default();
                        if !notes.sandhi.is_empty() {
                            next.sandhi = notes.sandhi;
                        }
                        if !notes.mistakes.is_empty() {
                            next.mistakes = notes.mistakes;
                        }
                        if !notes.drills.is_empty() {
                            next.drills = notes.drills;
                        }
                        if notes.takeaway.is_some() {
                            next.takeaway = notes.takeaway;
                        }
                        rich.set(Some(next));
                        generating.set(false);
                    }
                    Err(err) => {
                        gen_error.set(Some(err));
                        generating.set(false);
                    }
                }
            });
        })
    };

    let delete = {
        let on_delete = props.on_delete.clone();
        let id = next_id;
        Callback::from(move |_: MouseEvent| on_delete.emit(id))
    };

    let cancel_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_: MouseEvent| on_close.emit(()))
    };

    let bind = |state: UseStateHandle<String>, kind: &'static str| -> Callback<InputEvent> {
        match kind {
            "input" => Callback::from(move |e: InputEvent| {
                let target: web_sys::HtmlInputElement = e.target_unchecked_into();
                state.set(target.value());
            }),
            _ => Callback::from(move |e: InputEvent| {
                let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                state.set(target.value());
            }),
        }
    };

    html! {
        <>
            <div class="editor-grid">
                <label class="field">
                    <span>{ "Phrase number" }</span>
                    <input type="text" value={next_id.to_string()} disabled={true} />
                </label>
                <label class="field">
                    <span>{ "Category" }</span>
                    <input type="text" placeholder="Introductions" value={(*category).clone()}
                           oninput={bind(category.clone(), "input")} />
                </label>
                <label class="field wide">
                    <span>{ "English" }</span>
                    <input type="text" placeholder="Hello, I'm Jacob." value={(*en).clone()}
                           oninput={bind(en.clone(), "input")} />
                </label>
                <label class="field wide">
                    <span>{ "Traditional Chinese" }</span>
                    <input type="text" placeholder="你好，我叫 Jacob。" value={(*han).clone()}
                           oninput={bind(han.clone(), "input")} />
                </label>
                <label class="field wide">
                    <span>{ "Jyutping" }</span>
                    <input type="text" placeholder="nei5 hou2 ngo5 giu3" value={(*jyutping).clone()}
                           oninput={bind(jyutping.clone(), "input")} />
                </label>
                <label class="field wide">
                    <span>{ "Spoken Jyutping override (optional)" }</span>
                    <input type="text" placeholder="leave empty — the engine works the sandhi out"
                           value={(*spoken).clone()}
                           oninput={bind(spoken.clone(), "input")} />
                </label>
                <label class="field wide">
                    <span>{ "Your notes (optional)" }</span>
                    <textarea rows="3" value={(*notes).clone()}
                              oninput={bind(notes.clone(), "area")} />
                </label>
            </div>

            <div class="editor-llm">
                <button type="button" class="btn tiny" disabled={*generating} onclick={generate}>
                    { Icon::Wave.svg() }
                    <span>{ if *generating { "Generating…" } else { "Generate Rich Notes with MiniMax LLM" } }</span>
                </button>
                if let Some(err) = (*gen_error).clone() {
                    <span class="llm-error">{ err }</span>
                }
                if let Some(r) = rich.as_ref() {
                    <span class="llm-ok">
                        { format!("Rich notes ready — {} sandhi note(s), {} mistake row(s).", r.sandhi.len(), r.mistakes.len()) }
                    </span>
                }
            </div>

            <div class="modal-actions">
                if existing.is_some() {
                    <button type="button" class="btn danger" onclick={delete}>
                        { Icon::Trash.svg() }<span>{ "Delete phrase" }</span>
                    </button>
                }
                <span class="spacer" />
                <button type="button" class="btn ghost" onclick={cancel_click.clone()}>
                    { "Cancel" }
                </button>
                <button type="button" class="btn primary" onclick={save}>
                    { Icon::Check.svg() }<span>{ "Save phrase" }</span>
                </button>
            </div>
        </>
    }
}

// ── import / export ────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct ImportProps {
    pub phrases: Vec<Phrase>,
    pub on_import: Callback<Vec<Phrase>>,
    pub on_reset: Callback<()>,
    pub on_close: Callback<()>,
}

#[function_component(ImportExport)]
pub fn import_export(props: &ImportProps) -> Html {
    let pasted = use_state(String::new);
    let message = use_state(|| Option::<String>::None);

    let reset_click = {
        let on_reset = props.on_reset.clone();
        Callback::from(move |_: MouseEvent| on_reset.emit(()))
    };

    let export_json = {
        let phrases = props.phrases.clone();
        Callback::from(move |_: MouseEvent| {
            let json = serde_json::to_string_pretty(&phrases).unwrap_or_else(|_| "[]".to_string());
            download_json("singsin_phrases.json", &json);
        })
    };

    let apply_text = {
        let pasted = pasted.clone();
        let on_import = props.on_import.clone();
        let message = message.clone();
        Callback::from(move |_: MouseEvent| {
            match serde_json::from_str::<Vec<Phrase>>(&pasted) {
                Ok(list) if !list.is_empty() => {
                    message.set(Some(format!("Imported {} phrases.", list.len())));
                    on_import.emit(list);
                }
                Ok(_) => message.set(Some("That file contained no phrases.".to_string())),
                Err(e) => message.set(Some(format!("Could not read that JSON: {e}"))),
            }
        })
    };

    let on_file = {
        let on_import = props.on_import.clone();
        let message = message.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let file = match input.files().and_then(|f| f.get(0)) {
                Some(f) => f,
                None => return,
            };
            let on_import = on_import.clone();
            let message = message.clone();
            spawn_local(async move {
                match read_file(&file).await {
                    Ok(text) => match serde_json::from_str::<Vec<Phrase>>(&text) {
                        Ok(list) if !list.is_empty() => {
                            message.set(Some(format!("Imported {} phrases.", list.len())));
                            on_import.emit(list);
                        }
                        Ok(_) => message.set(Some("That file contained no phrases.".to_string())),
                        Err(err) => {
                            message.set(Some(format!("Could not read that JSON: {err}")))
                        }
                    },
                    Err(err) => message.set(Some(err)),
                }
            });
        })
    };

    html! {
        <>
            <section class="settings-block">
                <h3>{ "Export" }</h3>
                <p class="hint">{ "Save your current phrasebook as a JSON file you can re-import later." }</p>
                <button type="button" class="btn primary" onclick={export_json}>
                    { Icon::Download.svg() }<span>{ "Download singsin_phrases.json" }</span>
                </button>
            </section>

            <section class="settings-block">
                <h3>{ "Import" }</h3>
                <p class="hint">{ "Choose a JSON file, or paste the contents below." }</p>
                <label class="field">
                    <span>{ "From a file" }</span>
                    <input type="file" accept="application/json,.json" onchange={on_file} />
                </label>
                <label class="field">
                    <span>{ "Or paste JSON" }</span>
                    <textarea rows="6" value={(*pasted).clone()}
                              oninput={{
                                  let pasted = pasted.clone();
                                  Callback::from(move |e: InputEvent| {
                                      let target: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                                      pasted.set(target.value());
                                  })
                              }} />
                </label>
                <button type="button" class="btn" onclick={apply_text}>
                    { Icon::Upload.svg() }<span>{ "Import pasted JSON" }</span>
                </button>
                if let Some(m) = (*message).clone() {
                    <p class="io-message">{ m }</p>
                }
            </section>

            <section class="settings-block">
                <h3>{ "Reset" }</h3>
                <p class="hint">{ "Throw away your edits and go back to the bundled 100-phrase library." }</p>
                <button type="button" class="btn danger" onclick={reset_click.clone()}>
                    { Icon::Trash.svg() }<span>{ "Reset to the default phrasebook" }</span>
                </button>
            </section>
        </>
    }
}

/// Read a picked file into a string.
async fn read_file(file: &web_sys::File) -> Result<String, String> {
    let reader = web_sys::FileReader::new().map_err(|e| format!("{e:?}"))?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        reader.set_onload(Some(&resolve));
        reader.set_onerror(Some(&reject));
        let _ = reader.read_as_text(file);
    });
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| "Could not read that file.".to_string())?;
    Ok(value.as_string().unwrap_or_default())
}

/// Offer a JSON file to the browser as a download.
fn download_json(filename: &str, text: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(text));
    let Ok(blob) = web_sys::Blob::new_with_str_sequence(parts.as_ref()) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    let Ok(anchor) = document.create_element("a") else {
        return;
    };
    let _ = anchor.set_attribute("href", &url);
    let _ = anchor.set_attribute("download", filename);
    if let Ok(anchor) = anchor.dyn_into::<web_sys::HtmlElement>() {
        anchor.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}
