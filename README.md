# SingSin 聲線

[![CI](https://github.com/watchcat/singsin/actions/workflows/ci.yml/badge.svg)](https://github.com/watchcat/singsin/actions/workflows/ci.yml)

**Find your Cantonese voice.** A single-page Rust + WebAssembly application for
learning and practising everyday Hong Kong Cantonese: a live pitch-tracking
practice studio fused with a deep pedagogical pronunciation guide, on top of an
editable phrasebook.

> 聲線 (*sing1 sin3*) — “voice line / vocal timbre”. The name is the
> application: every phrase is reduced to the line your voice traces through
> the air, and you learn to draw the same line yourself.

**Try it live:** https://singsin-app.watchcat.workers.dev/
**Source:** https://github.com/watchcat/singsin

---

## Running it

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk          # or: brew install trunk

cd singsin
trunk serve                  # dev server on http://127.0.0.1:8080
trunk build --release        # static site in dist/
```

Requirements: a Rust toolchain with the `wasm32-unknown-unknown` target, and
Trunk. Nothing else — the app is a single static bundle with no server side.

### Scripts

Two scripts do the boring parts. Both are dependency-free (just `bash`,
`curl` and `python3`).

**`scripts/dev.sh`** — build, run the CORS relay, serve the bundle and open
it in your browser:

```bash
./scripts/dev.sh                    # build + proxy + serve + open the browser
./scripts/dev.sh --no-build         # reuse the existing dist/
./scripts/dev.sh --no-open          # don't open a browser
./scripts/dev.sh --port 9000        # serve somewhere else
./scripts/dev.sh --relay-port 9001
```

It starts `tools/proxy.py` on `:8081` and serves `dist/` on `:8099`, waits for
both to answer, then opens <http://127.0.0.1:8099/index.html>. If either port
is already taken it **reuses the server that is already running** instead of
failing. Ctrl-C stops everything it started.

**`scripts/deploy.sh`** — build and deploy to Cloudflare:

```bash
export CLOUDFLARE_API_TOKEN=...     # a token with Workers + R2 edit
./scripts/deploy.sh                 # test, build, upload, verify
./scripts/deploy.sh --dry-run       # build and print the plan, write nothing
./scripts/deploy.sh --no-build      # deploy the existing dist/
./scripts/deploy.sh --skip-tests    # skip the test suite
```

It runs the tests, builds `dist/`, uploads every asset to the R2 bucket
` singsin-site`, deletes objects left behind by older builds, uploads both
Workers, enables their `workers.dev` subdomains, and finally checks that the
public URLs answer. `index.html` is uploaded **last**, so the page never
points at an asset that isn't in the bucket yet.

Overridable with `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_BUCKET`,
`CLOUDFLARE_APP_WORKER`, `CLOUDFLARE_RELAY_WORKER` and `CLOUDFLARE_SUBDOMAIN`.

A `justfile` wraps both if you have [`just`](https://github.com/casey/just)
installed:

```bash
just build      # trunk build --release
just dev        # scripts/dev.sh
just deploy     # scripts/deploy.sh
just test       # cargo test --test phonetics
just check      # cargo check --target wasm32-unknown-unknown
```

### Tests

```bash
cargo test              # everything: the engine, the DSP and the chart logic
cargo test --test phonetics   # just the integration suite
```

The suite covers Jyutping parsing, the Han ⇄ Jyutping alignment (including
Latin names and digit sequences), the tone-4 sandhi lift, analysis of all 100
seed phrases, the tone-key coverage, YIN on a synthetic 220 Hz tone and the
VAD trimmer. It also checks that **every character used by the seed library
carries an English gloss**, that the glosses are short enough to read on a
card, and that the **syllable-zoom window follows the playhead** across a
phrase, falls back to the chosen syllable when the audio stops, and always
shows the whole phrase in phrase-zoom.

---

## What it does

### 1. A deterministic Cantonese phonetic engine (`src/data/phonetics.rs`)

The engine parses any Jyutping string — initial + nucleus + coda + tone — and
derives, for every syllable:

- a **Chao tone contour** (`55`, `25`, `33`, `21`, `13`, `22`) as an SVG path;
- an **English sound anchor** (*“Sound like “ngo” — **ng** as in *sing*, then
  **aw** as in *law. Say it low and flat.”*);
- the **phonetic traps** a Mandarin speaker actually falls into;
- **minimal pairs** with clickable audio chips (`喺 hai2` vs `係 hai6`);
- the **colloquial sandhi** — a run of tone-4 syllables lifts the second and
  later ones to a high tone 1, so 荷蘭 is spoken *ho4 laan1*, not *ho4 laan4*.
- a short **English gloss** for the character (`src/data/gloss.rs`), so you can
  recognise it later: 你 → *you*, 好 → *good / very*, 嘅 → *(possessive)*.

Because all of it is derived from the Jyutping itself, **a phrase you add or
edit in the UI gets a complete deep-dive immediately** — no network round-trip,
no content pipeline. Curated verbatim overrides ship for the headline phrases
(`我住喺荷蘭`, `你好，我叫 Jacob`, `我今年五十二歲`).

The glosses are a built-in dictionary rather than a seed-only lookup, so they
also work for phrases you type yourself. Cantonese is morphosyllabic, so the
gloss is per *character*: every character carries its own meaning, and the
sentence-level English sits above the phrase as well.

### 2. A real DSP pipeline in Wasm (`src/dsp/`)

- **YIN / McLeod pitch tracking** over `f32` PCM, 75–500 Hz, with parabolic
  interpolation.
- **RMS envelopes** for the reference recording and for your take.
- **Voice-activity detection** that trims the dead silence from the front and
  back of your recording, so your `t = 0 %` lands on the same two points of
  speech as the reference phrase.
- Contours are stored in **semitones relative to the speaker's own median**,
  which makes two different voices directly comparable — the *shape* carries
  the information, not the register.

### 3. The comparison studio (`src/components/studio.rs`, `chart.rs`)

One canvas, four layers, one live playhead:

| Layer | Colour | What it is |
|---|---|---|
| Target | coral `#d48164`, **dashed** | the schematic Chao tone melody to aim for |
| Dictionary tone | grey `#aab2ab`, thin dashed | the tone *before* a sandhi lift — only drawn when the lift changes it |
| Original | clay `#c97b3f`, **solid** | the reference recording's measured pitch and energy envelope |
| Mine | green `#246c5e`, **solid** | your measured pitch and envelope |
| Playhead | ink | sweeps in real time while audio plays |

Dashed means *schematic* (an ideal shape to aim at); solid means *measured* (a
real voice). The two are separated by colour **and** by stroke style, so the
Target can never be mistaken for the Original even on a monochrome screen. The
legend names the three layers, and adds the dictionary-tone entry only when the
current phrase actually contains a sandhi lift.

Four playback modes:

1. **Play Original** — the native recording.
2. **Play Mine** — your trimmed take.
3. **Play Both Together** — both scheduled at the same `AudioContext` time, so
   they land in chorus. Optional **stereo split** (original left, yours right)
   and a **balance** slider.
4. **Compare: Original → Mine** — sequential A/B with a 300 ms gap.

The running mode is highlighted in the bar, so you always know which voice
you are hearing.

Plus a **Time-Sync** toggle that rescales your take's `playbackRate` so both
voices have identical duration, and an axis that can switch between
**normalised %** and **real seconds**, or zoom into a single syllable.

#### Syllable zoom follows the playhead

In **Single Syllable Zoom** the visible window tracks whatever the playhead is
over: as the recording plays, the chart scrolls from one syllable to the next
and names it underneath (the character and its Jyutping). Stop the audio and it
falls back to the syllable you picked. The axis labels follow the window too, so
a zoomed-in axis reads as real positions along the phrase (`39% … 46%`) rather
than always printing `0% … 100%`.

The follow logic lives in `chart.rs` and takes the audio clock as an argument,
so it is covered by deterministic unit tests (`cargo test`) rather than by
eyeballing a canvas.

### 4. An editable phrasebook

Add, edit, delete, import and export phrases in the UI. Everything persists in
`localStorage` under `singsin_phrases`, `singsin_practiced` and
`singsin_settings`. The seed library (`assets/phrases.json`, 100 phrases) is
embedded at compile time so the app works offline out of the box.

### 5. Optional MiniMax enrichment (`src/minimax.rs`)

When you add or edit a phrase you can ask the MiniMax chat API for bespoke
colloquial sandhi notes, a custom mistakes table and natural sentence-particle
drills. Your API key lives only in your browser's `localStorage`.

---

## The reference audio, and a note on CORS

The native recordings come from [words.hk](https://words.hk), one MP3 per
syllable, which the app fetches, trims, stitches with a natural 120 ms
inter-syllable transition, and analyses.

**`words.hk` does not send an `Access-Control-Allow-Origin` header**, so a
browser cannot `fetch()` those bytes directly.

The app therefore does not trust a single relay. When a phrase loads it races
every candidate source **in parallel** and keeps whichever answers first:

| # | Source | Notes |
|---|---|---|
| 1 | the direct URL | works if CORS is ever enabled, or behind your own proxy |
| 2 | `https://singsin-relay.watchcat.workers.dev/?url=…` | the deployed Cloudflare Worker (below) |
| 3 | `http://127.0.0.1:8081/?url=…` | the bundled local relay (below) |
| 4 | your own proxy prefix | from **Audio Settings** |
| 5 | `https://cors.eu.org/…` | public relay |
| 6 | `https://api.allorigins.win/raw?url=…` | public relay |
| 7 | `https://api.codetabs.com/v1/proxy/?quest=…` | public relay |

Each source gets a **7-second budget** (`AbortController`), so a hanging relay
can never stall the lesson. The winner is remembered for the rest of the
session, so the remaining syllables are fetched straight from it.

### The bundled local relay

`tools/proxy.py` is a dependency-free, standard-library-only CORS relay:

```bash
python3 tools/proxy.py                 # http://127.0.0.1:8081
python3 tools/proxy.py --port 9000     # somewhere else
```

It is the most reliable option, because it does not depend on anybody else's
uptime. Run it in a second terminal before `trunk serve` and the app will find
it automatically — no configuration needed.

If no source answers, the studio says so plainly and falls back to the
browser's own Cantonese voice (`yue-HK` / `zh-HK` only — never Mandarin).

### The browser's Cantonese voice

The browser voice is only a *fallback* for when a phrase's reference MP3s
cannot be reached, but it is worth getting right. `speechSynthesis.getVoices()`
returns an **empty array** until the browser has finished loading its voices,
and Chrome on macOS regularly takes well over a second to do that — it only
then fires `voiceschanged`. An empty list therefore means *"not ready yet"*,
not *"this browser has no Cantonese voice"*.

So `src/audio/tts.rs` keeps the two apart:

- `speak_when_ready()` waits for the list (up to four seconds) before deciding
  anything, so one click always does the right thing;
- `VoicesNotReady` and `NoCantoneseVoice` are different errors, and the
  messages say which one you are looking at;
- the **Audio Settings** panel watches for the list too, so it never claims
  there is no Cantonese voice while the browser is still loading.

On macOS the Cantonese voice is called **Sinji** (`lang="yue-HK"`); it is
listed in the Voice dropdown once the browser has finished loading.

### The deployed Cloudflare relay

`tools/worker-relay.js` is the same relay as a Cloudflare Worker, deployed at
`https://singsin-relay.watchcat.workers.dev/?url=<encoded-url>`.

It is deliberately **not an open proxy**: only `words.hk` and `www.words.hk`
are allowed through, anything else gets a `403`. It buffers the upstream body
and drops `content-encoding` / `content-length` so the browser never receives
mismatched headers, and it answers `Access-Control-Allow-Origin: *`.

```bash
# deploy the relay
wrangler deploy tools/worker-relay.js --name singsin-relay
# or with the Cloudflare API: PUT /accounts/{id}/workers/scripts/singsin-relay
```

Because the Worker now sits second in the candidate list, the hosted app pulls
its reference audio without asking the visitor to run anything locally.

---

## The hosted instance

The app is deployed and publicly reachable at:

> **https://singsin-app.watchcat.workers.dev/**

The source lives at **https://github.com/watchcat/singsin**, and CI runs the
test suite plus a full WebAssembly release build on every push.

It is served by a Cloudflare Worker (`singsin-app`) backed by an **R2 bucket**
(`singsin-site`), with the four built assets — `index.html`, the hashed `.js`,
the hashed `.wasm` and the hashed `.css` — stored as R2 objects with their
correct `content-type` metadata. On a cache miss the Worker writes the asset
through to R2, so the bucket heals itself on the first visit and the site keeps
working without any origin server.

The hashed assets are served with `cache-control: public, max-age=3600` —
they are immutable. `index.html` is the pointer to them, so it is served with
`no-cache, must-revalidate` instead: a redeploy is then visible to everyone on
the very next page load rather than after the cache window.

Both URLs are plain Cloudflare `workers.dev` endpoints, so there is nothing to
configure and no custom domain to point at.

### Deploying it yourself

The static bundle in `dist/` is plain files, so any static host will do. For
the Worker + R2 setup, `scripts/deploy.sh` does all of it — see
[Scripts](#scripts). The steps it performs, if you ever need to drive them by
hand:

1. `trunk build --release` → `dist/`
2. Create the bucket: `PUT /accounts/{account_id}/r2/buckets/{bucket_name}`
3. Upload `dist/index.html`, `dist/*.js`, `dist/*.wasm` and `dist/*.css` to
   `r2/buckets/{bucket_name}/objects/{key}`
4. Publish the Worker with an R2 binding (`{ name: "SITE", type: "r2_bucket",
   bucket_name: "singsin-site" }`) and the script in `tools/worker-app.js`
5. Enable the subdomain: `POST /accounts/{account_id}/workers/scripts/{name}/subdomain`
   with `{ "enabled": true }` — without this, Cloudflare returns error 1042.

Note that the binary `.wasm` cannot be pushed through a JSON-only API client,
which is why the assets are stored in R2 rather than inlined into the Worker.

The Worker pulls anything it is missing from the static origin configured as
`ORIGIN` in `tools/worker-app.js`, writes it through to R2 and serves it. That
origin injects its own analytics snippet and a floating widget into every HTML
page it serves, so the Worker **strips that block** before the page is stored
or returned — what the visitor receives is byte-for-byte the `index.html` that
`trunk` built.

---

## Layout

```
singsin/
├── Cargo.toml
├── index.html
├── justfile                   just build / dev / deploy / test / check
├── style.css                  the whole design system
├── assets/phrases.json        the seed phrasebook
├── scripts/
│   ├── dev.sh                 build + proxy + serve + open the browser
│   └── deploy.sh              build + upload + deploy to Cloudflare
├── tools/
│   ├── proxy.py               a local CORS relay for words.hk
│   ├── worker-relay.js        the same relay, as a Cloudflare Worker
│   └── worker-app.js          serves the built bundle from R2 at the edge
└── src/
    ├── main.rs
    ├── app.rs                 shell: library, workspace, modal stack
    ├── storage.rs             localStorage
    ├── minimax.rs             optional LLM enrichment
    ├── data/
    │   ├── phrases.rs         Phrase model, seed loader, Han ⇄ Jyutping alignment
    │   ├── phonetics.rs       the deterministic phonetic engine
    │   └── gloss.rs           the built-in character ⇄ English dictionary
    ├── dsp/
    │   ├── pitch.rs           YIN pitch tracking + RMS
    │   ├── vad.rs             voice activity detection / trimming
    │   ├── contour.rs         Chao grid geometry
    │   └── track.rs           the measured-contour model
    ├── audio/
    │   ├── engine.rs          fetching, stitching, playback scheduling
    │   ├── recorder.rs        microphone capture into Wasm memory
    │   └── tts.rs             Cantonese speech-synthesis fallback
    └── components/            header, banner, sidebar, hero, studio,
                               chart, cards, tonelist, mistakes, drill,
                               takeaway, modals
```

## Design

Warm editorial palette — paper `#f6f5ef`, ink `#263b37`, green `#246c5e` for
*you*, coral `#d48164` / clay `#c97b3f` for the *original*, lifted tone badges
and phonetic traps. Headings in `DM Serif Display` + `Noto Serif HK`, body in
`Outfit` + `Inter` + `Noto Sans HK`, romanisation in `JetBrains Mono`.

Desktop is a two-column grid (a 260 px sticky library beside the practice
column); at ≤ 760 px the library becomes a slide-over drawer.

## Licence

MIT. The reference recordings belong to words.hk.
