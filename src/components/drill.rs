//! The five-pass practice drill.

use yew::prelude::*;

use crate::data::phonetics::Analysis;

use super::chart::ChartModel;
use super::{inline_md, Icon};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub analysis: Analysis,
    pub model: ChartModel,
}

#[function_component(Drill)]
pub fn drill(props: &Props) -> Html {
    let model = props.model.clone();

    html! {
        <section class="drill-section">
            <h2 class="section-title">{ "The 5-pass practice drill" }</h2>
            <ol class="drill">
                { for props.analysis.drill.iter().map(|pass| {
                    let record = {
                        let model = model.clone();
                        Callback::from(move |_: MouseEvent| {
                            let mut m = model.borrow_mut();
                            m.record_request = m.record_request.wrapping_add(1);
                        })
                    };
                    html! {
                        <li class="pass" key={pass.n}>
                            <div class="pass-head">
                                <span class="pass-n">{ format!("Pass {}", pass.n) }</span>
                                <h3 class="pass-title">{ pass.title.clone() }</h3>
                                <button type="button" class="btn tiny" onclick={record}>
                                    { Icon::Mic.svg() }
                                    <span>{ "Record & Compare this pass" }</span>
                                </button>
                            </div>
                            <p class="pass-body">{ for inline_md(&pass.body) }</p>
                            if !pass.focus.is_empty() {
                                <ul class="pass-focus">
                                    { for pass.focus.iter().map(|f| {
                                        html! { <li>{ f.clone() }</li> }
                                    })}
                                </ul>
                            }
                        </li>
                    }
                })}
            </ol>
        </section>
    }
}
