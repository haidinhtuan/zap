use crossterm::event::{
    EventStream, KeyEvent, MouseEvent,
    Event as CrosstermEvent,
};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Application-level events forwarded from the terminal or generated internally.
#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    Render,
    FocusGained,
    FocusLost,
    Paste(String),
    Error,
}

/// Asynchronous event handler that reads crossterm events and produces tick/render
/// events at configurable intervals.
pub struct EventHandler {
    _tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
    task: Option<JoinHandle<()>>,
}

impl EventHandler {
    /// Create a new EventHandler that polls for terminal events and emits
    /// Tick / Render events at the given rates.
    pub fn new(tick_rate: Duration, frame_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        let task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);
            let mut render_interval = tokio::time::interval(frame_rate);

            loop {
                tokio::select! {
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                let app_event = crossterm_to_app_event(evt);
                                if tx_clone.send(app_event).is_err() {
                                    break;
                                }
                            }
                            Some(Err(_)) => {
                                let _ = tx_clone.send(Event::Error);
                            }
                            None => break,
                        }
                    }
                    _ = tick_interval.tick() => {
                        if tx_clone.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    _ = render_interval.tick() => {
                        if tx_clone.send(Event::Render).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            _tx: tx,
            rx,
            task: Some(task),
        }
    }

    /// Receive the next event. Returns an error if the channel is closed.
    pub async fn next(&mut self) -> color_eyre::Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| color_eyre::eyre::eyre!("Event channel closed"))
    }

    /// Check whether the background task has stopped.
    pub fn is_stopped(&self) -> bool {
        match &self.task {
            Some(handle) => handle.is_finished(),
            None => true,
        }
    }

    /// Stop the background task.
    pub fn stop(&mut self) {
        if let Some(handle) = self.task.take() {
            handle.abort();
        }
    }
}

/// Convert a crossterm event into our application event type.
fn crossterm_to_app_event(evt: CrosstermEvent) -> Event {
    match evt {
        CrosstermEvent::Key(key) => Event::Key(key),
        CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
        CrosstermEvent::Resize(w, h) => Event::Resize(w, h),
        CrosstermEvent::FocusGained => Event::FocusGained,
        CrosstermEvent::FocusLost => Event::FocusLost,
        CrosstermEvent::Paste(text) => Event::Paste(text),
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_send_and_receive() {
        // Test the core channel mechanism without requiring a real terminal.
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(Event::Tick).unwrap();
        tx.send(Event::Render).unwrap();
        let first = rx.recv().await.unwrap();
        assert!(matches!(first, Event::Tick));
        let second = rx.recv().await.unwrap();
        assert!(matches!(second, Event::Render));
    }

    #[tokio::test]
    async fn test_channel_closed_returns_none() {
        let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
        drop(tx);
        let result = rx.recv().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stop_marks_finished() {
        // Create an event handler, stop it, and verify is_stopped.
        // We use a long interval so the task won't panic before we stop it.
        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            // Simply wait forever; will be aborted.
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        let mut handler = EventHandler {
            _tx: tx,
            rx,
            task: Some(task),
        };
        assert!(!handler.is_stopped());
        handler.stop();
        assert!(handler.is_stopped());
    }

    #[tokio::test]
    async fn test_drop_stops_task() {
        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        let handler = EventHandler {
            _tx: tx,
            rx,
            task: Some(task),
        };
        drop(handler);
        // If we get here, drop didn't panic.
    }

    #[test]
    fn test_event_variants_exist() {
        // Ensure all event variants can be constructed.
        let _ = Event::Tick;
        let _ = Event::Render;
        let _ = Event::FocusGained;
        let _ = Event::FocusLost;
        let _ = Event::Paste("hello".to_string());
        let _ = Event::Error;
        let _ = Event::Resize(80, 24);
    }

    #[test]
    fn test_crossterm_to_app_event_focus() {
        let evt = crossterm_to_app_event(CrosstermEvent::FocusGained);
        assert!(matches!(evt, Event::FocusGained));
        let evt = crossterm_to_app_event(CrosstermEvent::FocusLost);
        assert!(matches!(evt, Event::FocusLost));
    }

    #[test]
    fn test_crossterm_to_app_event_resize() {
        let evt = crossterm_to_app_event(CrosstermEvent::Resize(120, 40));
        assert!(matches!(evt, Event::Resize(120, 40)));
    }

    #[test]
    fn test_crossterm_to_app_event_paste() {
        let evt = crossterm_to_app_event(CrosstermEvent::Paste("clipboard".to_string()));
        if let Event::Paste(text) = evt {
            assert_eq!(text, "clipboard");
        } else {
            panic!("expected Paste event");
        }
    }
}
