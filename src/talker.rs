use std::sync::{Arc, Mutex};
use tts::Tts;

#[derive(Clone)]
pub struct Talker(Arc<Mutex<InnerTalker>>);

struct InnerTalker {
    tts: Tts,

    actually_running: bool,
    running: bool,
}

impl Talker {
    pub fn running(&self) -> bool {
        self.0.lock().unwrap().running
    }
}
