//! The top bar — brand mark, language tag and the two modal triggers.

use yew::prelude::*;

use super::Icon;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub on_open_tone_guide: Callback<()>,
    pub on_open_settings: Callback<()>,
    pub on_toggle_menu: Callback<()>,
    pub menu_open: bool,
}

#[function_component(Header)]
pub fn header(props: &Props) -> Html {
    let tone_guide = {
        let cb = props.on_open_tone_guide.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let settings = {
        let cb = props.on_open_settings.clone();
        Callback::from(move |_| cb.emit(()))
    };
    let menu = {
        let cb = props.on_toggle_menu.clone();
        Callback::from(move |_| cb.emit(()))
    };

    html! {
        <header class="site-header">
            <button type="button" class="menu-toggle" onclick={menu}
                    aria-expanded={props.menu_open.to_string()} aria-label="Toggle the phrase library">
                <span /><span /><span />
            </button>

            <a class="brand" href="#top" aria-label="SingSin home">
                <svg class="brand-bars" viewBox="0 0 34 24" aria-hidden="true" focusable="false">
                    <path d="M3.5 20 L9 4 L12.6 4 L7.1 20 Z" fill="var(--green)" />
                    <path d="M12.5 20 L18 4 L21.6 4 L16.1 20 Z" fill="var(--green)" />
                    <path d="M21.5 20 L27 4 L30.6 4 L25.1 20 Z" fill="var(--coral)" />
                </svg>
                <span class="brand-word">{"singsin"}<span class="brand-dot">{"."}</span></span>
            </a>

            <span class="lang-tag">
                    {"Cantonese "}<span class="sep">{"/"}</span>{" 廣東話 "}
                    <span class="dot-sep">{"\u{b7}"}</span>{" 聲線"}
                </span>

            <div class="header-actions">
                <button type="button" class="btn ghost" onclick={tone_guide}>
                    { Icon::Gauge.svg() }
                    <span>{ "Tone Guide" }</span>
                </button>
                <button type="button" class="btn ghost" onclick={settings}>
                    { Icon::Layers.svg() }
                    <span>{ "Audio Settings" }</span>
                </button>
            </div>
        </header>
    }
}
