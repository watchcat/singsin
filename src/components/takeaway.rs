//! The closing editorial summary.

use yew::prelude::*;

use crate::data::phonetics::Analysis;
use crate::data::phrases::Phrase;

use super::inline_md;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub phrase: Phrase,
    pub analysis: Analysis,
}

#[function_component(Takeaway)]
pub fn takeaway(props: &Props) -> Html {
    html! {
        <section class="takeaway">
            <p class="takeaway-eyebrow">{ "Key takeaway" }</p>
            <p class="takeaway-body">{ for inline_md(&props.analysis.takeaway) }</p>
            <p class="takeaway-meta">
                <span class="mono">{ props.phrase.jyutping.clone() }</span>
                <span class="arrow">{ "→" }</span>
                <span class="mono spoken">{ props.analysis.spoken_jyutping.clone() }</span>
            </p>
        </section>
    }
}
