use crossterm::event::{Event as CrosstermEvent, KeyEvent, MouseEvent};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
}

pub struct EventHandler {
    pub rx: mpsc::UnboundedReceiver<Event>,
    _task: JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut interval = tokio::time::interval(tick_rate);

            loop {
                let tick_delay = interval.tick();
                let crossterm_event = reader.next();

                tokio::select! {
                    _ = tick_delay => {
                        if tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                let event_to_send = match evt {
                                    CrosstermEvent::Key(key) => Event::Key(key),
                                    CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                                    CrosstermEvent::Resize(w, h) => Event::Resize(w, h),
                                    _ => continue,
                                };
                                if tx.send(event_to_send).is_err() {
                                    break;
                                }
                            }
                            Some(Err(_)) => {
                                break;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Self { rx, _task: task }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
