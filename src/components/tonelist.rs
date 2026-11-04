//! The tone key — one card per tone used in the active phrase.

use yew::prelude::*;

use crate::data::phonetics::Analysis;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub analysis: Analysis,
}

#[function_component(ToneList)]
pub fn tone_list(props: &Props) -> Html {
    html! {
        <section class="tonelist-section">
            <h2 class="section-title">{ "The tones you're using" }</h2>
            <div class="tonelist">
                { for props.analysis.tones.iter().map(|t| {
                    html! {
                        <article class="tone-card" key={t.tone}>
                            <div class="tone-card-head">
                                <span class="chao">{ t.chao.clone() }</span>
                                <span class="tone-name">{ t.name.clone() }</span>
                            </div>
                            <svg class="contour" viewBox="0 0 140 80" role="img"
                                 aria-label={format!("Tone {} contour, Chao {}", t.tone, t.chao)}>
                                { for crate::dsp::contour::grid_lines().iter().map(|(_, y)| {
                                    let y = format!("{y:.1}");
                                    html! { <line x1="8" y1={y.clone()} x2="132" y2={y} class="gl" /> }
                                })}
                                <path d={t.shape.clone()} class="tone-path" />
                            </svg>
                            <p class="tone-example">
                                <span class="ex-han">{ t.example_han.clone() }</span>
                                <span class="ex-jp">{ t.example_jyutping.clone() }</span>
                            </p>
                        </article>
                    }
                })}
            </div>
        </section>
    }
}
