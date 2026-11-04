//! The mistakes table — what a Mandarin speaker will say, and why it is wrong.

use yew::prelude::*;

use crate::data::phonetics::Analysis;

use super::inline_md;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub analysis: Analysis,
}

#[function_component(Mistakes)]
pub fn mistakes(props: &Props) -> Html {
    if props.analysis.mistakes.is_empty() {
        return html! {};
    }

    html! {
        <section class="mistakes-section">
            <h2 class="section-title">{ "The mistakes to watch for" }</h2>
            <table class="mistakes">
                <thead>
                    <tr>
                        <th>{ "You'll say" }</th>
                        <th>{ "You should say" }</th>
                        <th>{ "Why" }</th>
                    </tr>
                </thead>
                <tbody>
                    { for props.analysis.mistakes.iter().map(|m| {
                        html! {
                            <tr key={m.wrong.clone()}>
                                <td class="wrong">{ for inline_md(&m.wrong) }</td>
                                <td class="right">{ for inline_md(&m.right) }</td>
                                <td class="why">{ for inline_md(&m.why) }</td>
                            </tr>
                        }
                    })}
                </tbody>
            </table>
        </section>
    }
}
