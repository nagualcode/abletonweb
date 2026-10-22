//! Runs install/uninstall actions on a worker thread so the UI stays live.

use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Outcome {
    pub label: String,
    pub ok: bool,
    pub output: String,
}

#[derive(Default)]
pub struct Runner {
    busy: Arc<Mutex<Option<String>>>,
    last: Arc<Mutex<Option<Outcome>>>,
}

impl Runner {
    pub fn busy(&self) -> Option<String> {
        self.busy.lock().unwrap().clone()
    }

    pub fn last(&self) -> Option<Outcome> {
        self.last.lock().unwrap().clone()
    }

    /// Runs `job` unless another action is running.
    pub fn run(
        &self,
        label: &str,
        ctx: &eframe::egui::Context,
        job: impl FnOnce() -> Result<String, String> + Send + 'static,
    ) {
        let mut busy = self.busy.lock().unwrap();
        if busy.is_some() {
            return;
        }
        *busy = Some(label.to_string());
        drop(busy);

        let (busy, last, ctx, label) = (self.busy.clone(), self.last.clone(), ctx.clone(), label.to_string());
        std::thread::spawn(move || {
            let (ok, output) = match job() {
                Ok(out) => (true, out),
                Err(err) => (false, err),
            };
            *last.lock().unwrap() = Some(Outcome { label, ok, output });
            *busy.lock().unwrap() = None;
            ctx.request_repaint();
        });
    }
}

pub fn open_url(url: &str) {
    let _ = std::process::Command::new("open").arg(url).spawn();
}
