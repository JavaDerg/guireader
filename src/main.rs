mod talker;

use eframe::epi::{App, Frame};
use egui::text::LayoutJob;
use egui::{Color32, CtxRef, RichText, ScrollArea, TextEdit, TextFormat, TextStyle};
use std::mem::swap;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use tts::Tts;

enum Main {
    Init {
        handle: JoinHandle<Result<Tts, tts::Error>>,
        signal: Arc<AtomicBool>,
        r#try: usize,
    },
    Error(String),
    Panicked,
    Running {
        tts: Tts,
        text: String,
        position: Range<usize>,
        running: Arc<AtomicBool>,
    },
}

impl Main {
    pub fn new() -> Self {
        Self::inited(0)
    }

    fn inited(r#try: usize) -> Self {
        let atomic = Arc::new(AtomicBool::new(false));
        let signal = atomic.clone();
        Self::Init {
            handle: thread::spawn(move || {
                let tts = Tts::default();
                signal.store(true, Ordering::Relaxed);
                tts
            }),
            signal: atomic,
            r#try,
        }
    }
}

impl App for Main {
    fn update(&mut self, ctx: &CtxRef, _frame: &Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self {
                Main::Init { signal: flag, .. } => {
                    if flag.load(Ordering::Relaxed) {
                        let mut switch = Main::Panicked;
                        swap(self, &mut switch);

                        let (handle, r#try) = match switch {
                            Main::Init { handle, r#try, .. } => (handle.join(), r#try),
                            _ => unreachable!(),
                        };
                        if handle.is_err() {
                            if r#try >= 3 {
                                return;
                            }
                            *self = Self::inited(r#try + 1);
                        }
                        let tts = handle.unwrap();
                        *self = match tts {
                            Ok(tts) => {
                                let running = Arc::new(AtomicBool::new(false));
                                let (c1, c2, c3) = (running.clone(), running.clone(), running.clone());

                                let failed =
                                    tts.on_utterance_begin(Some(Box::new(move |_| c1.store(true, Ordering::Relaxed)))).is_err()
                                    || tts.on_utterance_stop(Some(Box::new(move |_| c2.store(false, Ordering::Relaxed)))).is_err()
                                    || tts.on_utterance_end(Some(Box::new(move |_| c3.store(false, Ordering::Relaxed)))).is_err();
                                if failed {
                                    *self = Main::Panicked;
                                    return;
                                }

                                Main::Running {
                                    tts,
                                    text: String::new(),
                                    position: 0..0,
                                    running,
                                }
                            },
                            Err(err) => Main::Error(format!("{}", err)),
                        };

                        ctx.request_repaint();
                    } else {
                        ui.vertical_centered_justified(|ui| {
                            ui.heading("Loading TTS...");
                        });
                    }
                }
                Main::Error(err) => {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Error:");
                        ui.code(RichText::new(&*err).color(Color32::RED));
                    });
                }
                Main::Panicked => {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Panicked while or failed to load TTS 3 times, aborting.");
                    });
                }
                Main::Running {
                    tts,
                    text,
                    position,
                    running: arun,
                } => {
                    let running = arun.load(Ordering::Relaxed);
                    let horizontal = ui.horizontal(|ui| {
                        if ui
                            .button(match running {
                                true => "Pause",
                                false => "Play",
                            })
                            .clicked()
                        {
                            if running && tts.stop().is_err() {
                                return false;
                            }
                            if !running && tts.speak(&*text, true).is_err() {
                                return false;
                            }

                            ctx.request_repaint();
                        }

                        ui.separator();

                        ui.label("Status: ");
                        ui.label(match running {
                            true => "Running",
                            false => "Paused",
                        });

                        true
                    });
                    if !horizontal.inner {
                        *self = Main::Panicked;
                        return;
                    }
                    ui.separator();

                    let mut tc = text.clone();
                    // TODO: Doesn't work
                    ScrollArea::vertical().show(ui, |ui| {
                        let changed = ui.add(
                            TextEdit::multiline(text)
                                .desired_width(f32::INFINITY)
                                .hint_text({
                                    let mut job = LayoutJob::default();
                                    let slice = tc.as_mut_str();
                                    let (first, second) = slice.split_at_mut(position.start);
                                    let (second, third) = second.split_at_mut(position.start);
                                    job.append(first, 0.0, TextFormat::default());
                                    job.append(second, 0.0, TextFormat {
                                        background: Color32::DARK_BLUE,
                                        ..TextFormat::default()
                                    });
                                    job.append(third, 0.0, TextFormat::default());
                                    job
                                }),
                        ).changed();
                        if changed {
                            *position = 0.min(text.len())..text.len().min(text.len().saturating_sub(1));
                        }
                    });
                }
            }
        });
    }

    fn name(&self) -> &str {
        "GuiReader"
    }
}

fn main() {
    eframe::run_native(Box::new(Main::new()), eframe::NativeOptions::default());
}
