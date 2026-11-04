//! The phrase hero — number, category, the Chinese headline and the
//! Jyutping strip with its tone badges.

use yew::prelude::*;

use crate::data::phonetics::Analysis;
use crate::data::phrases::Phrase;

use super::{inline_md, Icon};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub phrase: Phrase,
    pub analysis: Analysis,
    pub practiced: bool,
    pub on_toggle_practiced: Callback<()>,
    pub on_edit: Callback<()>,
}

#[function_component(Hero)]
pub fn hero(props: &Props) -> Html {
    let toggle = {
        let cb = props.on_toggle_practiced.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let edit = {
        let cb = props.on_edit.clone();
        Callback::from(move |_| cb.emit(()))
    };

    html! {
        <section class="hero card">
            <div class="hero-meta">
                <span class="phrase-no">{ format!("#{:02}", props.phrase.id) }</span>
                <span class="pill cat">{ props.phrase.category.clone() }</span>
                <span class="hero-spacer" />
                <button type="button" class="btn tiny" onclick={edit}>
                    { Icon::Edit.svg() }<span>{ "Edit Phrase" }</span>
                </button>
                <button type="button"
                        class={if props.practiced { "pill practice on" } else { "pill practice" }}
                        aria-pressed={props.practiced.to_string()}
                        onclick={toggle}>
                    { if props.practiced { Icon::Check.svg() } else { Icon::Record.svg() } }
                    <span>{ if props.practiced { "Practised" } else { "Mark practised" } }</span>
                </button>
            </div>

            <h2 class="hero-han">{ props.phrase.han.clone() }</h2>
            <p class="hero-en">{ props.phrase.en.clone() }</p>

            <div class="strip-wrap">
                <div class="jyutping-strip" role="list" aria-label="Syllable by syllable">
                    { for props.analysis.syllables.iter().map(|s| {
                        let tone_text = if s.foreign {
                            // A Latin name has no Cantonese tone — say so
                            // instead of inventing "tone 0".
                            "English".to_string()
                        } else if s.lifted {
                            format!("tone {} → {}", s.tone, s.spoken_tone)
                        } else {
                            format!("tone {}", s.tone)
                        };
                        let badge_class = if s.lifted { "num lift" } else { "num" };
                        html! {
                            <div class="strip-cell" role="listitem" key={s.index}>
                                <span class="hz">{ s.han.clone() }</span>
                                <span class="jp">{ s.jyutping.clone() }</span>
                                <span class={badge_class}>{ tone_text }</span>
                                if !s.en.is_empty() {
                                    <span class="gloss">{ s.en.clone() }</span>
                                }
                            </div>
                        }
                    })}
                </div>
                if !props.analysis.spoken_jyutping.is_empty() {
                    <p class="spoken-line">
                        <span class="spoken-label">{ "spoken" }</span>
                        <span class="mono">{ props.analysis.spoken_jyutping.clone() }</span>
                    </p>
                }
            </div>

            if let Some(note) = props.analysis.strip_note.clone() {
                <p class="strip-note">
                    <span class="note-mark">{ "◆" }</span>
                    { for inline_md(&note) }
                </p>
            }

            if let Some(notes) = props.phrase.notes.clone() {
                if !notes.trim().is_empty() {
                    <p class="user-note">
                        <span class="note-mark">{ "✎" }</span>
                        { for inline_md(&notes) }
                    </p>
                }
            }
        </section>
    }
}
