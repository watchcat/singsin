//! Microphone capture.
//!
//! Raw `f32` PCM comes straight out of a `ScriptProcessorNode` into the Wasm
//! heap, where the Rust DSP (YIN pitch tracking, RMS envelope, VAD trimming)
//! runs on it. The same buffer is later replayed through an
//! `AudioBufferSourceNode`, so what you compare is exactly what you recorded.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AudioContext, GainNode, MediaDevices, MediaStream, MediaStreamAudioSourceNode,
    AudioProcessingEvent, ScriptProcessorNode,
};

use crate::dsp::pitch::{analyse, WIN};

/// Size of each ScriptProcessor callback, in samples.
const CHUNK: usize = 4096;

/// A live microphone capture.
pub struct Recorder {
    stream: MediaStream,
    source: MediaStreamAudioSourceNode,
    processor: ScriptProcessorNode,
    sink: GainNode,
    /// Kept alive for as long as the capture runs.
    _closure: Option<Closure<dyn FnMut(AudioProcessingEvent)>>,
    /// Every captured sample, flat mono.
    pub samples: Rc<RefCell<Vec<f32>>>,
    /// Live pitch track: `(seconds, hz)` — `hz == 0` means unvoiced.
    pub live: Rc<RefCell<Vec<(f64, f32)>>>,
    /// Milliseconds captured so far.
    pub elapsed_ms: Rc<Cell<f64>>,
    /// Sample rate of the capture context.
    pub sample_rate: f32,
}

impl Recorder {
    /// Ask for the microphone and start capturing.
    pub async fn start(context: &AudioContext) -> Result<Recorder, String> {
        let navigator = web_sys::window()
            .ok_or("No window")?
            .navigator();
        let devices: MediaDevices = navigator
            .media_devices()
            .map_err(|e| format!("mediaDevices: {e:?}"))?;

        let constraints = web_sys::MediaStreamConstraints::new();
        constraints.set_audio_bool(true);
        constraints.set_video_bool(false);

        let promise = devices
            .get_user_media_with_constraints(&constraints)
            .map_err(|e| format!("microphone denied: {e:?}"))?;
        let value = JsFuture::from(promise)
            .await
            .map_err(|_| "the microphone request was refused".to_string())?;
        let stream: MediaStream = value
            .dyn_into()
            .map_err(|_| "the microphone returned an unexpected stream".to_string())?;

        let source = context
            .create_media_stream_source(&stream)
            .map_err(|e| format!("stream source: {e:?}"))?;

        let processor = context
            .create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
                CHUNK as u32,
                1,
                1,
            )
            .map_err(|e| format!("script processor: {e:?}"))?;

        // The processor has to reach the destination for `onaudioprocess` to
        // fire, but we must not feed the microphone back into the speakers.
        let sink = context.create_gain().map_err(|e| format!("gain: {e:?}"))?;
        sink.gain().set_value(0.0);

        source.connect_with_audio_node(&processor).ok();
        processor.connect_with_audio_node(&sink).ok();
        sink.connect_with_audio_node(&context.destination()).ok();

        let samples: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(Vec::new()));
        let live: Rc<RefCell<Vec<(f64, f32)>>> = Rc::new(RefCell::new(Vec::new()));
        let elapsed_ms: Rc<Cell<f64>> = Rc::new(Cell::new(0.0));
        let sample_rate = context.sample_rate();

        let s2 = samples.clone();
        let l2 = live.clone();
        let e2 = elapsed_ms.clone();
        let sr = sample_rate;

        let closure = Closure::wrap(Box::new(move |event: AudioProcessingEvent| {
            let buffer = match event.input_buffer() {
                Ok(b) => b,
                Err(_) => return,
            };
            let chunk: Vec<f32> = buffer.get_channel_data(0).unwrap_or_default();

            // Live pitch tracking on the most recent window.
            let mut voiced: Vec<(f64, f32)> = Vec::new();
            {
                let mut guard = s2.borrow_mut();
                let total = guard.len() + chunk.len();
                guard.extend_from_slice(&chunk);
                e2.set(total as f64 * 1000.0 / sr as f64);

                if total >= WIN {
                    let start = total - WIN;
                    let window = &guard[start..total];
                    for frame in analyse(window, sr) {
                        voiced.push((frame.time, frame.f0));
                    }
                }
            }

            if !voiced.is_empty() {
                let mut guard = l2.borrow_mut();
                let base = (e2.get() / 1000.0) - (WIN as f64 / sr as f64);
                for (t, f) in voiced {
                    guard.push((base + t, f));
                }
                // Keep the live track bounded.
                if guard.len() > 20_000 {
                    let drop = guard.len() - 20_000;
                    guard.drain(..drop);
                }
            }
        }) as Box<dyn FnMut(AudioProcessingEvent)>);

        processor.set_onaudioprocess(Some(closure.as_ref().unchecked_ref()));

        Ok(Recorder {
            stream,
            source,
            processor,
            sink,
            _closure: Some(closure),
            samples,
            live,
            elapsed_ms,
            sample_rate,
        })
    }

    /// Take the captured samples out of the recorder.
    pub fn take_samples(&self) -> Vec<f32> {
        std::mem::take(&mut *self.samples.borrow_mut())
    }

}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.processor.set_onaudioprocess(None);
        let _ = self.processor.disconnect();
        let _ = self.source.disconnect();
        let _ = self.sink.disconnect();
        for track in self.stream.get_audio_tracks().iter() {
            if let Some(t) = track.dyn_ref::<web_sys::MediaStreamTrack>() {
                t.stop();
            }
        }
    }
}
