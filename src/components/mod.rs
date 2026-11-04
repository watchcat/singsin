//! The SingSin interface.

pub mod banner;
pub mod cards;
pub mod chart;
pub mod drill;
pub mod header;
pub mod hero;
pub mod mistakes;
pub mod modals;
pub mod sidebar;
pub mod studio;
pub mod tonelist;
pub mod takeaway;

use yew::prelude::*;
use yew::virtual_dom::VNode;

/// A piece of inline Markdown.
#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
}

/// Split a string into inline-Markdown segments.
///
/// Supports `**bold**`, `*italic*` and `` `code` `` — everything else is
/// plain text. Deliberately tiny: the notes in this app are written by hand
/// and by an LLM, so a full Markdown parser would be overkill.
pub fn parse_inline(text: &str) -> Vec<Segment> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<Segment> = Vec::new();
    let mut plain = String::new();
    let mut i = 0;

    macro_rules! flush {
        () => {
            if !plain.is_empty() {
                out.push(Segment::Text(std::mem::take(&mut plain)));
            }
        };
    }

    while i < chars.len() {
        // `**bold**`
        if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if let Some(end) = find_marker(&chars, i + 2, "**") {
                flush!();
                out.push(Segment::Bold(chars[i + 2..end].iter().collect()));
                i = end + 2;
                continue;
            }
        }
        // `*italic*`
        if chars[i] == '*' {
            if let Some(end) = find_marker(&chars, i + 1, "*") {
                flush!();
                out.push(Segment::Italic(chars[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        }
        // `` `code` ``
        if chars[i] == '`' {
            if let Some(end) = find_char(&chars, i + 1, '`') {
                flush!();
                out.push(Segment::Code(chars[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        }
        plain.push(chars[i]);
        i += 1;
    }
    flush!();
    out
}

fn find_marker(chars: &[char], from: usize, marker: &str) -> Option<usize> {
    let m: Vec<char> = marker.chars().collect();
    let mut i = from;
    while i + m.len() <= chars.len() {
        if chars[i..i + m.len()] == m[..] {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_char(chars: &[char], from: usize, needle: char) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == needle)
}

/// Render inline Markdown as VNodes.
pub fn inline_md(text: &str) -> Vec<VNode> {
    parse_inline(text)
        .into_iter()
        .map(|seg| match seg {
            Segment::Text(t) => html! { <>{t}</> },
            Segment::Bold(t) => html! { <strong>{t}</strong> },
            Segment::Italic(t) => html! { <em>{t}</em> },
            Segment::Code(t) => html! { <code class="md-code">{t}</code> },
        })
        .collect()
}

/// A small inline SVG icon set, drawn with the current text colour.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Icon {
    Play,
    Stop,
    Record,
    Mic,
    Edit,
    Trash,
    Plus,
    Download,
    Upload,
    Check,
    Close,
    Wave,
    Music,
    Gauge,
    Layers,
}

impl Icon {
    /// The SVG path data for this icon (a 24×24 viewBox).
    pub fn path(self) -> &'static str {
        match self {
            Icon::Play => "M8 5.5v13l11-6.5z",
            Icon::Stop => "M6.5 6.5h11v11h-11z",
            Icon::Record => "M12 5a7 7 0 110 14 7 7 0 010-14z",
            Icon::Mic => "M12 15a3.5 3.5 0 003.5-3.5V6a3.5 3.5 0 10-7 0v5.5A3.5 3.5 0 0012 15zm6-3.5a6 6 0 11-12 0H4a8 8 0 008 8v3h3v-3a8 8 0 008-8h-2z",
            Icon::Edit => "M4 17.2V20h2.8L18 8.8 15.2 6 4 17.2zM20.7 7.1a1 1 0 000-1.4l-2.4-2.4a1 1 0 00-1.4 0l-1.8 1.8L19 8.9l1.7-1.8z",
            Icon::Trash => "M7 7h10v12a2 2 0 01-2 2H9a2 2 0 01-2-2V7zm3-4h4l1 1h3v2H6V4h3l1-1z",
            Icon::Plus => "M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6V5z",
            Icon::Download => "M11 4h2v8.2l3-3 1.4 1.4L12 16 6.6 10.6 8 9.2l3 3V4zM5 18h14v2H5z",
            Icon::Upload => "M12 4l5.4 5.4L16 11l-3-3v8h-2V8l-3 3-1.4-1.4L12 4zM5 18h14v2H5z",
            Icon::Check => "M9.5 16.2L5.3 12l-1.4 1.4 5.6 5.6L20.1 8.4 18.7 7 9.5 16.2z",
            Icon::Close => "M6.2 5L12 10.8 17.8 5 19 6.2 13.2 12 19 17.8 17.8 19 12 13.2 6.2 19 5 17.8 10.8 12 5 6.2z",
            Icon::Wave => "M3 12h2.5l2-6 3 15 3-11 2 7 1.5-5H21",
            Icon::Music => "M9 18a3 3 0 11-2-2.8V5l12-2v11.2a3 3 0 11-2-2.8V6.2L9 7.5z",
            Icon::Gauge => "M12 4a9 9 0 00-9 9 1 1 0 001 1h16a1 1 0 001-1 9 9 0 00-9-9zm-4 8a1 1 0 112 0 1 1 0 01-2 0zm3.6 4.9l-1.8-1.8A4 4 0 0112 10a4 4 0 013.2 1.6l-1.8 1.8A2 2 0 0012 12a2 2 0 00-.4 1.9z",
            Icon::Layers => "M12 3l9 5-9 5-9-5 9-5zm-7.5 8.7L12 15.4l7.5-3.7 1.8.9L12 16.6 2.7 12.6l1.8-.9z",
        }
    }

    /// Render the icon.
    pub fn svg(self) -> Html {
        html! {
            <svg class="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path d={self.path()} fill="currentColor" />
            </svg>
        }
    }
}

/// A labelled toggle switch.
#[derive(Properties, PartialEq)]
pub struct ToggleProps {
    pub label: AttrValue,
    #[prop_or_default]
    pub checked: bool,
    pub on_toggle: Callback<bool>,
    #[prop_or_default]
    pub hint: Option<AttrValue>,
}

#[function_component(Toggle)]
pub fn toggle(props: &ToggleProps) -> Html {
    let onclick = {
        let on_toggle = props.on_toggle.clone();
        let checked = props.checked;
        Callback::from(move |_| on_toggle.emit(!checked))
    };
    html! {
        <label class="toggle">
            <input type="checkbox" checked={props.checked} {onclick} />
            <span class="toggle-track" aria-hidden="true"><span class="toggle-knob" /></span>
            <span class="toggle-label">{ props.label.clone() }</span>
            if let Some(hint) = props.hint.clone() {
                <span class="toggle-hint">{ hint }</span>
            }
        </label>
    }
}

/// A segmented control with two or three options.
#[derive(Properties, PartialEq)]
pub struct SegmentedProps {
    pub options: Vec<AttrValue>,
    pub active: AttrValue,
    pub on_select: Callback<AttrValue>,
    #[prop_or_default]
    pub label: Option<AttrValue>,
}

#[function_component(Segmented)]
pub fn segmented(props: &SegmentedProps) -> Html {
    html! {
        <div class="segmented" role="group">
            if let Some(label) = props.label.clone() {
                <span class="segmented-label">{ label }</span>
            }
            { for props.options.iter().map(|opt| {
                let on_select = props.on_select.clone();
                let value = opt.clone();
                let onclick = Callback::from(move |_| on_select.emit(value.clone()));
                let cls = if *opt == props.active { "seg on" } else { "seg" };
                html! { <button type="button" class={cls} {onclick} aria-pressed={(*opt == props.active).to_string()}>{ opt.clone() }</button> }
            })}
        </div>
    }
}
