//! The syllable-by-syllable breakdown cards.

use yew::prelude::*;

use crate::audio::SyllableCache;
use crate::data::phonetics::Analysis;

use super::chart;
use super::chart::{ChartModel, Zoom};
use super::{inline_md, Icon};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub analysis: Analysis,
    pub model: ChartModel,
    pub cache: SyllableCache,
    pub on_play_syllable: Callback<String>,
}

/// The mini Chao grid with one tone drawn on it.
fn mini_contour(tone: u8, spoken_tone: u8, han: &str) -> Html {
    let (from, to) = crate::dsp::contour::tone_levels(spoken_tone);
    let path = crate::dsp::contour::shape_path(from, to, 18.0, 122.0);
    let lifted = spoken_tone != tone;
    let under = if lifted {
        let (uf, ut) = crate::dsp::contour::tone_levels(tone);
        Some(crate::dsp::contour::shape_path(uf, ut, 18.0, 122.0))
    } else {
        None
    };

    html! {
        <svg class="contour" viewBox="0 0 140 80" role="img"
             aria-label={format!("Tone contour for {han}")}>
            { for crate::dsp::contour::grid_lines().iter().map(|(_, y)| {
                let y = format!("{y:.1}");
                html! { <line x1="8" y1={y.clone()} x2="132" y2={y} class="gl" /> }
            })}
            if let Some(u) = under {
                <path d={u} class="tone-under" />
            }
            <path d={path} class="tone-path" />
        </svg>
    }
}

#[function_component(SyllableCards)]
pub fn syllable_cards(props: &Props) -> Html {
    html! {
        <section class="cards-section">
            <h2 class="section-title">{ "Syllable by syllable" }</h2>
            <div class="cards">
                { for props.analysis.syllables.iter().filter(|s| !s.foreign).enumerate().map(|(tidx, s)| {
                    let jp = s.jyutping.clone();
                    let play = {
                        let cb = props.on_play_syllable.clone();
                        let jp = jp.clone();
                        Callback::from(move |_: MouseEvent| cb.emit(jp.clone()))
                    };
                    let zoom = {
                        let model = props.model.clone();
                        // `focus` indexes the chart's target vector, which only
                        // holds the real (non-foreign) syllables.
                        let idx = tidx;
                        Callback::from(move |_: MouseEvent| {
                            {
                                let mut m = model.borrow_mut();
                                m.focus = Some(idx);
                                m.zoom = Zoom::Syllable;
                            }
                            chart::bump(&model);
                        })
                    };

                    let badge_class = if s.lifted { "num lift" } else { "num" };
                    let badge_text = if s.lifted {
                        format!("tone {} → {}", s.tone, s.spoken_tone)
                    } else {
                        format!("tone {}", s.tone)
                    };

                    html! {
                        <article class="scard" key={s.index} onclick={zoom} tabindex="0"
                                 role="button"
                                 aria-label={format!("Zoom the chart onto {han}", han = s.han)}>
                            <header class="scard-head">
                                <span class="hz">{ s.han.clone() }</span>
                                <span class="jp">{ s.jyutping.clone() }</span>
                                <button type="button" class="speak" title={format!("Play {}", s.jyutping)}
                                        aria-label={format!("Play {}", s.jyutping)}
                                        onclick={play}>
                                    { Icon::Play.svg() }
                                </button>
                            </header>

                            if !s.en.is_empty() {
                                <p class="gloss" lang="en">{ s.en.clone() }</p>
                            }

                            <div class="scard-contour">
                                { mini_contour(s.tone, s.spoken_tone, &s.han) }
                                <span class={badge_class}>{ badge_text }</span>
                            </div>
                            <p class="movement">{ s.movement.clone() }</p>

                            <p class="say">
                                { for inline_md(&s.anchor_sentence) }
                            </p>

                            if !s.traps.is_empty() {
                                <div class="traps">
                                    { for s.traps.iter().map(|t| {
                                        html! {
                                            <div class="trap" key={t.title.clone()}>
                                                <p class="trap-title">{ t.title.clone() }</p>
                                                <p class="trap-body">{ for inline_md(&t.body) }</p>
                                            </div>
                                        }
                                    })}
                                </div>
                            }

                            if !s.minimal_pairs.is_empty() {
                                <div class="pairs">
                                    { for s.minimal_pairs.iter().map(|p| {
                                        let jy = p.jyutping.clone();
                                        let play_pair = {
                                            let cb = props.on_play_syllable.clone();
                                            let jy = jy.clone();
                                            Callback::from(move |_: MouseEvent| {
                                                cb.emit(jy.clone())
                                            })
                                        };
                                        html! {
                                            <button type="button" class="link-play" key={jy.clone()}
                                                    title={p.note.clone()}
                                                    onclick={play_pair}>
                                                <span class="lp-han">{ p.han.clone() }</span>
                                                <span class="lp-jp">{ p.jyutping.clone() }</span>
                                            </button>
                                        }
                                    })}
                                </div>
                            }
                        </article>
                    }
                })}
            </div>
        </section>
    }
}
