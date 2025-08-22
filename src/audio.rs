use rodio::{Decoder, Sink, Source}; // OutputStream removido del import
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};

pub struct AudioManager {
    // Mantenemos los tipos plenamente calificados para evitar imports extra
    _stream: Option<rodio::OutputStream>,
    stream_handle: Option<rodio::OutputStreamHandle>,
    music_sink: Arc<Mutex<Option<Sink>>>,
}

impl AudioManager {
    pub fn new() -> Self {
        let stream = rodio::OutputStream::try_default().ok();
        let handle = stream.as_ref().map(|s| s.1.clone());
        Self {
            _stream: stream.map(|s| s.0),
            stream_handle: handle,
            music_sink: Arc::new(Mutex::new(None)),
        }
    }

    pub fn play_music_loop(&self, path: &str) {
        if let Some(handle) = &self.stream_handle {
            if let Ok(file) = File::open(path) {
                let sink = Sink::try_new(handle).ok();
                if let Some(sink) = sink {
                    let source = Decoder::new(BufReader::new(file)).unwrap();
                    sink.append(source.repeat_infinite());
                    sink.play();
                    if let Ok(mut s) = self.music_sink.lock() {
                        if let Some(old) = s.take() {
                            old.stop();
                        }
                        *s = Some(sink);
                    }
                }
            } else {
                // Silencioso si falta archivo
            }
        }
    }

    pub fn play_sfx(&self, path: &str) {
        if let Some(handle) = &self.stream_handle {
            if let Ok(file) = File::open(path) {
                if let Ok(dec) = Decoder::new(BufReader::new(file)) {
                    if let Ok(sink) = Sink::try_new(handle) {
                        sink.append(dec.amplify(0.8));
                        sink.detach();
                    }
                }
            }
        }
    }
}