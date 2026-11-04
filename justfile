# SingSin 聲線 — build, run and deploy.
#
#   just          list the recipes
#   just build    build the release bundle into dist/
#   just dev      build, run the CORS relay, serve dist/ and open the browser
#   just deploy   build and deploy the app + relay Workers to Cloudflare
#   just test     run the phonetic engine, parser and DSP tests
#   just check    type-check for the wasm target

# List the available recipes.
default:
    @just --list

# Build the release bundle into dist/
build:
    trunk build --release

# Build, run the CORS relay, serve dist/ and open the browser
dev:
    ./scripts/dev.sh

# Build and deploy the app + relay Workers to Cloudflare
deploy:
    ./scripts/deploy.sh

# Run the phonetic engine, Jyutping parser and DSP tests
test:
    cargo test --test phonetics

# Type-check for the wasm target
check:
    cargo check --target wasm32-unknown-unknown
