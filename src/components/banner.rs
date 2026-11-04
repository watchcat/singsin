//! The editorial intro banner with the practice progress ring.

use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub practiced: usize,
    pub total: usize,
}

#[function_component(Banner)]
pub fn banner(props: &Props) -> Html {
    let pct = if props.total == 0 {
        0.0
    } else {
        (props.practiced as f64 / props.total as f64) * 100.0
    };
    let pct_text = format!("{pct:.0}");
    let done = props.total > 0 && props.practiced >= props.total;

    html! {
        <section class="intro" id="top">
            <div class="intro-copy">
                <p class="eyebrow">{ "Cantonese voice-line practice" }</p>
                <h1>{ "Find your voice." }</h1>
                <p class="intro-lede">
                    { "Every phrase in this library is broken down into its tones, its mouth shapes and its \
                       traps — then you record yourself and hear the two versions side by side." }
                </p>
            </div>
            <div class="intro-ring-wrap">
                <div class="ring" style={format!("--p: {pct_text}")} role="img"
                     aria-label={format!("{} of {} phrases practiced", props.practiced, props.total)}>
                    <span class="ring-hole">
                        <span class="ring-num">{ props.practiced.to_string() }</span>
                        <span class="ring-of">{ format!("of {} phrases practiced", props.total) }</span>
                    </span>
                </div>
                if done {
                    <p class="ring-done">{ "Every phrase practised — nice work." }</p>
                }
            </div>
        </section>
    }
}
