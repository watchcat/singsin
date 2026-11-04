//! The phrase library — the sticky left-hand column.

use yew::prelude::*;

use crate::data::phrases::Phrase;

use super::Icon;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub phrases: Vec<Phrase>,
    pub selected_id: i64,
    pub practiced: Vec<i64>,
    pub on_select: Callback<i64>,
    pub on_edit: Callback<i64>,
    pub on_add: Callback<()>,
    pub on_import: Callback<()>,
    pub on_export: Callback<()>,
    pub on_toggle_practiced: Callback<i64>,
    pub on_close: Callback<()>,
}

#[function_component(Sidebar)]
pub fn sidebar(props: &Props) -> Html {
    let query = use_state(String::new);
    let filter = use_state(String::new);

    let on_input = {
        let search = query.clone();
        Callback::from(move |e: InputEvent| {
            let target: web_sys::HtmlInputElement = e.target_unchecked_into();
            search.set(target.value());
        })
    };

    // Group the library by category, keeping the author's order.
    let mut groups: Vec<(String, Vec<&Phrase>)> = Vec::new();
    for phrase in &props.phrases {
        let needle = query.to_lowercase();
        let matches = needle.is_empty()
            || phrase.han.contains(query.as_str())
            || phrase.en.to_lowercase().contains(&needle)
            || phrase.jyutping.to_lowercase().contains(&needle);
        let category_ok = filter.is_empty() || *filter == phrase.category;
        if !matches || !category_ok {
            continue;
        }
        match groups.iter_mut().find(|(c, _)| *c == phrase.category) {
            Some((_, list)) => list.push(phrase),
            None => groups.push((phrase.category.clone(), vec![phrase])),
        }
    }

    let mut categories: Vec<String> = Vec::new();
    for p in &props.phrases {
        if !categories.contains(&p.category) {
            categories.push(p.category.clone());
        }
    }

    let on_select = {
        let cb = props.on_select.clone();
        let close = props.on_close.clone();
        Callback::from(move |id: i64| {
            cb.emit(id);
            close.emit(());
        })
    };

    html! {
        <aside class="sidebar" aria-label="Phrase library">
            <div class="sidebar-head">
                <h2>{ "Phrase library" }</h2>
                <span class="count">{ format!("{} phrases", props.phrases.len()) }</span>
            </div>

            <div class="search">
                <svg class="icon search-icon" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M10 4a6 6 0 104.3 10.2l4.5 4.5 1.4-1.4-4.5-4.5A6 6 0 0010 4zm0 2a4 4 0 110 8 4 4 0 010-8z" fill="currentColor" />
                </svg>
                <input type="search" placeholder="Search phrases…"
                       value={(*query).clone()} oninput={on_input}
                       aria-label="Search phrases" />
            </div>

            <div class="chips" role="group" aria-label="Filter by category">
                <button type="button"
                        class={if filter.is_empty() { "chip on" } else { "chip" }}
                        onclick={{
                            let f = filter.clone();
                            Callback::from(move |_| f.set(String::new()))
                        }}>{ "All" }</button>
                { for categories.iter().map(|c| {
                    let c = c.clone();
                    let cls = if *filter == *c { "chip on" } else { "chip" };
                    let f = filter.clone();
                    let label = c.clone();
                    let onclick = Callback::from(move |_| {
                        if *f == label { f.set(String::new()) } else { f.set(label.clone()) }
                    });
                    html! { <button type="button" class={cls} {onclick}>{ c.clone() }</button> }
                })}
            </div>

            <div class="phrase-list">
                if groups.is_empty() {
                    <p class="empty">{ "No phrase matches that search." }</p>
                }
                { for groups.iter().map(|(category, list)| {
                    html! {
                        <div class="phrase-group" key={category.clone()}>
                            <h3 class="group-name">{ category.clone() }</h3>
                            { for list.iter().map(|p| {
                                let id = p.id;
                                let is_selected = id == props.selected_id;
                                let is_practiced = props.practiced.contains(&id);

                                let select = { let cb = on_select.clone(); Callback::from(move |_| cb.emit(id)) };
                                let edit = {
                                    let cb = props.on_edit.clone();
                                    Callback::from(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        cb.emit(id);
                                    })
                                };
                                let toggle = {
                                    let cb = props.on_toggle_practiced.clone();
                                    Callback::from(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        cb.emit(id);
                                    })
                                };

                                html! {
                                    <div class={if is_selected { "phrase on" } else { "phrase" }}
                                         key={id}
                                         onclick={select}
                                         role="button" tabindex="0"
                                         aria-current={if is_selected { "true" } else { "false" }}>
                                        <span class="phrase-han">{ p.han.clone() }</span>
                                        <span class="phrase-en">{ p.en.clone() }</span>
                                        <span class="phrase-jp">{ p.jyutping.clone() }</span>
                                        <span class="phrase-tools">
                                            <button type="button" class={if is_practiced { "dot on" } else { "dot" }}
                                                    title={if is_practiced { "Practised" } else { "Mark as practised" }}
                                                    aria-label={if is_practiced { "Mark as not practised" } else { "Mark as practised" }}
                                                    onclick={toggle}>
                                                { if is_practiced { Icon::Check.svg() } else { Icon::Record.svg() } }
                                            </button>
                                            <button type="button" class="mini" title="Edit phrase"
                                                    aria-label={format!("Edit {}", p.han)}
                                                    onclick={edit}>
                                                { Icon::Edit.svg() }
                                            </button>
                                        </span>
                                    </div>
                                }
                            })}
                        </div>
                    }
                })}
            </div>

            <div class="sidebar-foot">
                <button type="button" class="btn primary" onclick={{
                    let cb = props.on_add.clone();
                    Callback::from(move |_| cb.emit(()))
                }}>
                    { Icon::Plus.svg() }
                    <span>{ "Add / Edit Phrase" }</span>
                </button>
                <div class="foot-row">
                    <button type="button" class="btn tiny" onclick={{
                        let cb = props.on_import.clone();
                        Callback::from(move |_| cb.emit(()))
                    }}>
                        { Icon::Upload.svg() }<span>{ "Import" }</span>
                    </button>
                    <button type="button" class="btn tiny" onclick={{
                        let cb = props.on_export.clone();
                        Callback::from(move |_| cb.emit(()))
                    }}>
                        { Icon::Download.svg() }<span>{ "Export" }</span>
                    </button>
                </div>
            </div>
        </aside>
    }
}
