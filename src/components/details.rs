//! The "My details" modal — the learner's own details, and the sentences
//! built from them.

use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::data::profile::Profile;
use crate::data::tables::{self, Entry};
use crate::data::templates;
use crate::storage::Settings;

use super::Icon;

#[derive(Properties, PartialEq)]
pub struct DetailsProps {
    /// The details as they stand now.
    pub profile: Profile,
    /// The settings, for the LLM key.
    pub settings: Settings,
    /// Called with the new details when the learner saves.
    pub on_save: Callback<Profile>,
    pub on_close: Callback<()>,
}

/// One `<option>` per entry of a table.
fn options(table: &'static [Entry]) -> Vec<Html> {
    table
        .iter()
        .map(|e| {
            let key = e.key.to_string();
            html! { <option value={key}>{ e.label }</option> }
        })
        .collect()
}

/// The properties of the [`Picker`].
#[derive(Properties, PartialEq)]
pub struct PickerProps {
    /// The question above the control.
    pub label: &'static str,
    /// The table of words to choose from.
    pub table: &'static [Entry],
    /// The key that is chosen, if any.
    pub selected: Option<String>,
    /// Called with the new key when the learner chooses something.
    pub on_change: Callback<String>,
}

/// A `<select>` for one of the tables.
///
/// The picker's own value is kept in step with the profile through the DOM
/// *property* (`select.value = …`) rather than through a `value` attribute
/// on the element. The difference matters: Yew applies a `value` attribute
/// to an `<input>` or a `<textarea>` as a property, but a `<select>` gets
/// the plain content attribute — and a `<select>` reads a content attribute
/// only as its *default* option. Left to that, the element drifts away from
/// the profile: a learner who types their name into the field above makes
/// the form re-render, and the pickers come back showing somebody else's
/// answers.
///
/// Setting the property is the call that actually moves the selection, and
/// — unlike the attribute — it fires no `change` event, so the picker's own
/// handler is never called back with a choice that nobody made.
#[function_component(Picker)]
pub fn picker(props: &PickerProps) -> Html {
    let select_ref = use_node_ref();

    let handler = {
        let on_change = props.on_change.clone();
        Callback::from(move |e: Event| {
            let target: web_sys::HtmlSelectElement = e.target_unchecked_into();
            on_change.emit(target.value());
        })
    };

    // The element follows the profile: once when the picker appears, and
    // again whenever the profile's answer for this field changes.
    {
        let select_ref = select_ref.clone();
        let value = props.selected.clone().unwrap_or_default();
        use_effect_with(value, move |value| {
            if let Some(select) = select_ref.cast::<web_sys::HtmlSelectElement>() {
                let _ = select.set_value(value);
            }
            || ()
        });
    }

    html! {
        <label class="field">
            <span>{ props.label }</span>
            <select ref={select_ref} onchange={handler}>
                <option value="">{ "Not set" }</option>
                { for options(props.table) }
            </select>
        </label>
    }
}

/// A text field bound to one string of the profile.
fn text_field(
    label: &'static str,
    placeholder: &'static str,
    value: String,
    on_input: Callback<String>,
) -> Html {
    let handler = Callback::from(move |e: InputEvent| {
        let target: web_sys::HtmlInputElement = e.target_unchecked_into();
        on_input.emit(target.value());
    });
    html! {
        <label class="field">
            <span>{ label }</span>
            <input type="text" placeholder={placeholder} value={value} oninput={handler} />
        </label>
    }
}

/// A number field bound to one optional number of the profile.
fn number_field(
    label: &'static str,
    placeholder: &'static str,
    value: Option<u32>,
    on_input: Callback<Option<u32>>,
) -> Html {
    let handler = Callback::from(move |e: InputEvent| {
        let target: web_sys::HtmlInputElement = e.target_unchecked_into();
        let parsed = target.value().trim().parse::<u32>().ok();
        on_input.emit(parsed);
    });
    html! {
        <label class="field">
            <span>{ label }</span>
            <input type="number" min="0" max="120" placeholder={placeholder}
                   value={value.map(|v| v.to_string()).unwrap_or_default()}
                   oninput={handler} />
        </label>
    }
}

#[function_component(MyDetails)]
pub fn my_details(props: &DetailsProps) -> Html {
    let profile = use_state(|| props.profile.clone());
    let status = use_state(|| Option::<String>::None);
    let busy = use_state(|| false);

    // Every change edits a copy of the profile and puts it back, so that
    // the preview below always shows what the learner is about to get.
    let set_name_latin = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.name_latin = v;
            profile.set(next);
        })
    };
    let set_name_han = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.name_han = v;
            profile.set(next);
        })
    };
    let set_name_jyutping = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.name_jyutping = v;
            profile.set(next);
        })
    };
    let set_age = {
        let profile = profile.clone();
        Callback::from(move |v: Option<u32>| {
            let mut next = (*profile).clone();
            next.age = v;
            profile.set(next);
        })
    };
    let set_years = {
        let profile = profile.clone();
        Callback::from(move |v: Option<u32>| {
            let mut next = (*profile).clone();
            next.years_learning = v;
            profile.set(next);
        })
    };
    // A table picker: an empty choice means "not set".
    let set_city = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.city = if v.is_empty() { None } else { Some(v) };
            profile.set(next);
        })
    };
    let set_country = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.country = if v.is_empty() { None } else { Some(v) };
            profile.set(next);
        })
    };
    let set_job = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.job = if v.is_empty() { None } else { Some(v) };
            profile.set(next);
        })
    };
    let set_hobby = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.hobby = if v.is_empty() { None } else { Some(v) };
            profile.set(next);
        })
    };
    let set_family = {
        let profile = profile.clone();
        Callback::from(move |v: String| {
            let mut next = (*profile).clone();
            next.family = if v.is_empty() { None } else { Some(v) };
            profile.set(next);
        })
    };

    // The LLM may suggest a reading for a Chinese name. It is a
    // suggestion: it lands in the field, where the learner can see it and
    // change it before it is saved.
    let guess = {
        let profile = profile.clone();
        let status = status.clone();
        let busy = busy.clone();
        let settings = props.settings.clone();
        Callback::from(move |_: MouseEvent| {
            let current = (*profile).clone();
            if current.name_han.trim().is_empty() {
                status.set(Some("Type the Chinese name first.".into()));
                return;
            }
            let profile = profile.clone();
            let status = status.clone();
            let busy = busy.clone();
            let settings = settings.clone();
            busy.set(true);
            status.set(Some("Asking MiniMax…".into()));
            spawn_local(async move {
                match crate::minimax::guess_jyutping(&settings, &current.name_han).await {
                    Ok(jp) => {
                        let mut next = (*profile).clone();
                        next.name_jyutping = jp.clone();
                        profile.set(next);
                        status.set(Some(format!(
                            "The model suggests {jp}. Read it before you save it — a wrong tone \
                             is the one thing this app must not teach."
                        )));
                    }
                    Err(e) => status.set(Some(e)),
                }
                busy.set(false);
            });
        })
    };

    let save = {
        let profile = profile.clone();
        let on_save = props.on_save.clone();
        let on_close = props.on_close.clone();
        Callback::from(move |_: MouseEvent| {
            on_save.emit((*profile).clone());
            on_close.emit(());
        })
    };

    // "Forget them" clears the form as well as the saved profile: leaving
    // the old answers on screen would suggest they had been kept.
    let clear = {
        let profile = profile.clone();
        let on_save = props.on_save.clone();
        Callback::from(move |_: MouseEvent| {
            profile.set(Profile::default());
            on_save.emit(Profile::default());
        })
    };

    // What the learner will get, with the details they have just typed.
    let preview = templates::build_all(&(*profile).clone());

    html! {
        <>
            <p class="lead">
                { "Tell the app about yourself and the library says it about you. Every sentence \
                   below is built from these details and then goes through the same phonetic engine \
                   as the rest of the library — tones, traps, the mistakes to watch for, the five \
                   drill passes and the pitch chart." }
            </p>

            <section class="settings-block">
                <h3>{ "Your name" }</h3>
                <p class="hint">
                    { "Cantonese says an English name in English, so a name in Latin letters needs no \
                       romanisation at all. A Chinese name needs its Jyutping: type it, or let the \
                       model suggest one and check it." }
                </p>
                <div class="editor-grid">
                    { text_field("Name (Latin letters)", "Jacob", (*profile).name_latin.clone(), set_name_latin) }
                    { text_field("Chinese name (optional)", "家明", (*profile).name_han.clone(), set_name_han) }
                    { text_field("Its Jyutping (optional)", "gaai1 ming4", (*profile).name_jyutping.clone(), set_name_jyutping) }
                </div>
                <button type="button" class="btn tiny" disabled={*busy} onclick={guess}>
                    { Icon::Wave.svg() }
                    <span>{ if *busy { "Asking…" } else { "Guess the Jyutping with MiniMax" } }</span>
                </button>
                if let Some(message) = status.as_ref() {
                    <p class="hint">{ message.clone() }</p>
                }
            </section>

            <section class="settings-block">
                <h3>{ "Your details" }</h3>
                <p class="hint">
                    { "The age and the years are written out in Chinese for you. The places and the \
                       job come from a list that was checked against words.hk — the same dictionary \
                       the reference recordings come from — so the Jyutping is never a guess. A \
                       detail that is not in the list is simply not offered." }
                </p>
                <div class="editor-grid">
                    { number_field("Age", "35", (*profile).age, set_age) }
                    { number_field("Years learning Cantonese", "2", (*profile).years_learning, set_years) }
                    <Picker label="City" table={tables::CITIES} selected={(*profile).city.clone()} on_change={set_city} />
                    <Picker label="Country" table={tables::COUNTRIES} selected={(*profile).country.clone()} on_change={set_country} />
                    <Picker label="Job" table={tables::JOBS} selected={(*profile).job.clone()} on_change={set_job} />
                    <Picker label="Hobby" table={tables::HOBBIES} selected={(*profile).hobby.clone()} on_change={set_hobby} />
                    <Picker label="You live with" table={tables::FAMILY} selected={(*profile).family.clone()} on_change={set_family} />
                </div>
            </section>

            <section class="settings-block">
                <h3>{ "What you will say" }</h3>
                if preview.is_empty() {
                    <p class="hint">
                        { "Fill in a name, an age, a place or a job and the sentences appear here." }
                    </p>
                } else {
                    <ul class="detail-preview">
                        { for preview.iter().map(|p| {
                            let han = p.han.clone();
                            let jp = p.jyutping.clone();
                            let en = p.en.clone();
                            html! {
                                <li key={p.id}>
                                    <span class="dp-han">{ han }</span>
                                    <span class="dp-jp">{ jp }</span>
                                    <span class="dp-en">{ en }</span>
                                </li>
                            }
                        })}
                    </ul>
                }
            </section>

            <div class="modal-actions">
                <button type="button" class="btn primary" onclick={save}>
                    <span>{ "Save my details" }</span>
                </button>
                <button type="button" class="btn" onclick={clear}>
                    <span>{ "Forget them" }</span>
                </button>
            </div>
        </>
    }
}
