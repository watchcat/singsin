//! The application shell: library, phrase workspace and the modal stack.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen_futures::spawn_local;
use web_sys::AudioBuffer;
use yew::prelude::*;

use crate::audio::engine;
use crate::audio::SyllableCache;
use crate::components::chart::{ChartData, ChartModel};
use crate::components::{banner, cards, chart, drill, header, hero, mistakes, modals, sidebar, studio, tonelist, takeaway};
use crate::data::phonetics;
use crate::data::phrases::{self, Phrase};
use crate::storage::{self, Settings};

/// Which modal is open.
#[derive(Clone, PartialEq, Debug)]
pub enum ModalKind {
    ToneGuide,
    AudioSettings,
    /// `None` inside means "add a new phrase".
    Editor(Option<i64>),
    ImportExport,
}

/// Play a decoded syllable.
fn play_buffer(buffer: &AudioBuffer) {
    if let Ok(ctx) = engine::ctx() {
        engine::resume(&ctx);
        let _ = engine::play_oneshot(&ctx, buffer);
    }
}

/// Mount the application into the document body.
pub fn run() {
    yew::Renderer::<App>::new().render();
}

#[function_component(App)]
pub fn app() -> Html {
    // ── state ────────────────────────────────────────────────────────────
    let phrases = use_state(|| storage::load_phrases().unwrap_or_else(phrases::seed_phrases));
    let selected_id = use_state(|| 1i64);
    let practiced = use_state(storage::load_practiced);
    let settings = use_state(storage::load_settings);
    let modal = use_state(|| Option::<ModalKind>::None);
    let menu_open = use_state(|| false);
    let cache = use_state(SyllableCache::default);

    let model_state = use_state(|| {
        Rc::new(RefCell::new(ChartData {
            show_target: true,
            show_original: true,
            show_mine: true,
            time_mode: chart::TimeMode::Percent,
            zoom: chart::Zoom::Phrase,
            ..Default::default()
        }))
    });
    let model: ChartModel = (*model_state).clone();

    // ── derived ──────────────────────────────────────────────────────────
    let phrase: Phrase = phrases
        .iter()
        .find(|p| p.id == *selected_id)
        .cloned()
        .or_else(|| phrases.first().cloned())
        .unwrap_or_else(|| phrases::seed_phrases().remove(0));
    let analysis = phonetics::analyze(&phrase);

    // Apply the saved axis/zoom preferences once, on start-up. Later changes
    // to the settings must not yank the view back while the learner is
    // working — the studio owns the live view from then on.
    {
        let model = model.clone();
        let initial = (*settings).clone();
        use_effect_with((), move |_| {
            {
                let mut m = model.borrow_mut();
                m.time_mode = if initial.time_mode == "seconds" {
                    chart::TimeMode::Seconds
                } else {
                    chart::TimeMode::Percent
                };
                m.zoom = if initial.zoom == "syllable" {
                    chart::Zoom::Syllable
                } else {
                    chart::Zoom::Phrase
                };
            }
            chart::bump(&model);
            || ()
        });
    }

    // ── phrase library handlers ──────────────────────────────────────────
    let on_select = {
        let selected_id = selected_id.clone();
        let menu_open = menu_open.clone();
        let model = model.clone();
        Callback::from(move |id: i64| {
            selected_id.set(id);
            menu_open.set(false);
            {
                let mut m = model.borrow_mut();
                m.focus = None;
                m.zoom = chart::Zoom::Phrase;
            }
            chart::bump(&model);
        })
    };

    let on_toggle_practiced = {
        let practiced = practiced.clone();
        Callback::from(move |id: i64| {
            let mut next = (*practiced).clone();
            if next.contains(&id) {
                next.retain(|x| *x != id);
            } else {
                next.push(id);
            }
            next.sort_unstable();
            storage::save_practiced(&next);
            practiced.set(next);
        })
    };

    let on_add = {
        let modal = modal.clone();
        Callback::from(move |_| modal.set(Some(ModalKind::Editor(None))))
    };

    let on_edit = {
        let modal = modal.clone();
        Callback::from(move |id: i64| modal.set(Some(ModalKind::Editor(Some(id)))))
    };

    let on_save = {
        let phrases = phrases.clone();
        let selected_id = selected_id.clone();
        let modal = modal.clone();
        Callback::from(move |p: Phrase| {
            let mut list = (*phrases).clone();
            match list.iter_mut().find(|x| x.id == p.id) {
                Some(slot) => *slot = p.clone(),
                None => list.push(p.clone()),
            }
            storage::save_phrases(&list);
            phrases.set(list);
            selected_id.set(p.id);
            modal.set(None);
        })
    };

    let on_delete = {
        let phrases = phrases.clone();
        let selected_id = selected_id.clone();
        let modal = modal.clone();
        Callback::from(move |id: i64| {
            let mut list: Vec<Phrase> = (*phrases).clone().into_iter().filter(|p| p.id != id).collect();
            if list.is_empty() {
                list = phrases::seed_phrases();
            }
            storage::save_phrases(&list);
            let fallback = list[0].id;
            phrases.set(list);
            if *selected_id == id {
                selected_id.set(fallback);
            }
            modal.set(None);
        })
    };

    let on_import = {
        let phrases = phrases.clone();
        let modal = modal.clone();
        Callback::from(move |list: Vec<Phrase>| {
            storage::save_phrases(&list);
            phrases.set(list);
            modal.set(None);
        })
    };

    let on_reset = {
        let phrases = phrases.clone();
        let modal = modal.clone();
        Callback::from(move |_| {
            storage::clear_phrases();
            phrases.set(phrases::seed_phrases());
            modal.set(None);
        })
    };

    let on_import_export = {
        let modal = modal.clone();
        Callback::from(move |_| modal.set(Some(ModalKind::ImportExport)))
    };

    // ── settings ─────────────────────────────────────────────────────────
    let on_settings = {
        let settings = settings.clone();
        Callback::from(move |s: Settings| {
            storage::save_settings(&s);
            settings.set(s);
        })
    };

    // ── syllable playback ────────────────────────────────────────────────
    let on_play_syllable = {
        let cache = cache.clone();
        let settings = settings.clone();
        Callback::from(move |jyutping: String| {
            if let Some(buffer) = cache.get(&jyutping) {
                play_buffer(&buffer);
                return;
            }
            let cache = cache.clone();
            let proxy = settings.audio_proxy.clone();
            spawn_local(async move {
                if let Ok(buffer) = engine::fetch_syllable(&jyutping, &proxy).await {
                    cache.put(&jyutping, buffer.clone());
                    play_buffer(&buffer);
                }
            });
        })
    };

    // ── modal plumbing ───────────────────────────────────────────────────
    let close_modal = {
        let modal = modal.clone();
        Callback::from(move |_| modal.set(None))
    };

    let open_tone_guide = {
        let modal = modal.clone();
        Callback::from(move |_| modal.set(Some(ModalKind::ToneGuide)))
    };

    let open_settings = {
        let modal = modal.clone();
        Callback::from(move |_| modal.set(Some(ModalKind::AudioSettings)))
    };

    let toggle_menu = {
        let menu_open = menu_open.clone();
        Callback::from(move |_| {
            let next = !*menu_open;
            menu_open.set(next);
        })
    };

    let close_menu = {
        let menu_open = menu_open.clone();
        Callback::from(move |()| menu_open.set(false))
    };

    let backdrop_click = {
        let menu_open = menu_open.clone();
        Callback::from(move |_: MouseEvent| menu_open.set(false))
    };

    let practiced_count = practiced.len();
    let total_count = phrases.len();

    // The `html!` macro has no `match` support, so the modal stack is built
    // as a plain Rust value first.
    let modal_view: Option<Html> = match modal.as_ref() {
        Some(ModalKind::ToneGuide) => Some(html! {
            <modals::ModalShell title="Tone guide" subtitle="The six Cantonese tones"
                on_close={close_modal.clone()}>
                <modals::ToneGuide />
            </modals::ModalShell>
        }),
        Some(ModalKind::AudioSettings) => Some(html! {
            <modals::ModalShell title="Audio settings"
                subtitle="Reference audio, voices and the LLM key"
                on_close={close_modal.clone()}>
                <modals::AudioSettings settings={(*settings).clone()}
                    on_settings={on_settings.clone()} on_close={close_modal.clone()} />
            </modals::ModalShell>
        }),
        Some(ModalKind::Editor(id)) => {
            let editing = id.and_then(|i| phrases.iter().find(|p| p.id == i).cloned());
            let editor_key = editing.as_ref().map(|p| p.id).unwrap_or(0);
            Some(html! {
                <modals::ModalShell
                    title={if editing.is_some() { "Edit phrase" } else { "Add a phrase" }}
                    subtitle="The phonetic engine rebuilds every section the moment you save"
                    on_close={close_modal.clone()}>
                    <modals::PhraseEditor
                        key={editor_key}
                        phrase={editing} settings={(*settings).clone()}
                        on_save={on_save.clone()} on_delete={on_delete.clone()}
                        on_close={close_modal.clone()} />
                </modals::ModalShell>
            })
        }
        Some(ModalKind::ImportExport) => Some(html! {
            <modals::ModalShell title="Import & export" subtitle="Your phrasebook as JSON"
                on_close={close_modal.clone()}>
                <modals::ImportExport phrases={(*phrases).clone()}
                    on_import={on_import.clone()} on_reset={on_reset.clone()}
                    on_close={close_modal.clone()} />
            </modals::ModalShell>
        }),
        None => None,
    };

    html! {
        <div class="app">
            <header::Header
                on_open_tone_guide={open_tone_guide}
                on_open_settings={open_settings}
                on_toggle_menu={toggle_menu}
                menu_open={*menu_open} />

            <banner::Banner practiced={practiced_count} total={total_count} />

            <div class="layout">
                <div class={if *menu_open { "drawer open" } else { "drawer" }}>
                    if *menu_open {
                        <div class="drawer-backdrop" onclick={backdrop_click.clone()} />
                    }
                    <sidebar::Sidebar
                        phrases={(*phrases).clone()}
                        selected_id={*selected_id}
                        practiced={(*practiced).clone()}
                        on_select={on_select.clone()}
                        on_edit={on_edit.clone()}
                        on_add={on_add.clone()}
                        on_import={on_import_export.clone()}
                        on_export={on_import_export.clone()}
                        on_toggle_practiced={on_toggle_practiced.clone()}
                        on_close={close_menu.clone()} />
                </div>

                <main class="main">
                    <hero::Hero
                        phrase={phrase.clone()}
                        analysis={analysis.clone()}
                        practiced={practiced.contains(&phrase.id)}
                        on_toggle_practiced={{
                            let cb = on_toggle_practiced.clone();
                            let id = phrase.id;
                            Callback::from(move |_| cb.emit(id))
                        }}
                        on_edit={{
                            let cb = on_edit.clone();
                            let id = phrase.id;
                            Callback::from(move |_| cb.emit(id))
                        }} />

                    <studio::Studio
                        phrase={phrase.clone()}
                        analysis={analysis.clone()}
                        model={model.clone()}
                        cache={(*cache).clone()}
                        settings={(*settings).clone()}
                        on_settings={on_settings.clone()} />

                    <cards::SyllableCards
                        analysis={analysis.clone()}
                        model={model.clone()}
                        cache={(*cache).clone()}
                        on_play_syllable={on_play_syllable.clone()} />

                    <tonelist::ToneList analysis={analysis.clone()} />

                    <mistakes::Mistakes analysis={analysis.clone()} />

                    <drill::Drill analysis={analysis.clone()} model={model.clone()} />

                    <takeaway::Takeaway phrase={phrase.clone()} analysis={analysis.clone()} />

                    <footer class="site-footer">
                        <p>
                            { "SingSin 聲線 — reference recordings from words.hk. \
                               Pitch tracking, voice-activity detection and the phonetic engine are Rust, \
                               compiled to WebAssembly." }
                        </p>
                    </footer>
                </main>
            </div>

            { modal_view }
        </div>
    }
}
