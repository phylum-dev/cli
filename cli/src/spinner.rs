use std::io::{self, IsTerminal};
use std::time;

use futures::Future;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

const SPINNER_DELAY: u64 = 40;
const SPINNER_DOTS: [&str; 56] = [
    "⢀⠀", "⡀⠀", "⠄⠀", "⢂⠀", "⡂⠀", "⠅⠀", "⢃⠀", "⡃⠀", "⠍⠀", "⢋⠀", "⡋⠀", "⠍⠁", "⢋⠁", "⡋⠁", "⠍⠉", "⠋⠉",
    "⠋⠉", "⠉⠙", "⠉⠙", "⠉⠩", "⠈⢙", "⠈⡙", "⢈⠩", "⡀⢙", "⠄⡙", "⢂⠩", "⡂⢘", "⠅⡘", "⢃⠨", "⡃⢐", "⠍⡐", "⢋⠠",
    "⡋⢀", "⠍⡁", "⢋⠁", "⡋⠁", "⠍⠉", "⠋⠉", "⠋⠉", "⠉⠙", "⠉⠙", "⠉⠩", "⠈⢙", "⠈⡙", "⠈⠩", "⠀⢙", "⠀⡙", "⠀⠩",
    "⠀⢘", "⠀⡘", "⠀⠨", "⠀⢐", "⠀⡐", "⠀⠠", "⠀⢀", "⠀⡀",
];

/// A CLI spinner. All public constructors are gated behind a
/// check that stderr is a TTY. If this is not the case, every method is a
/// no-op.
pub struct Spinner {
    tx: Option<Sender<Command>>,
    handle: Option<JoinHandle<()>>,
}

enum Command {
    Stop,
    Message(Option<String>),
}

impl Spinner {
    /// Start a CLI spinner on the current cursor line. To stop it, call
    /// `stop` on the returned `Spinner`.
    pub fn new() -> Self {
        Self::new_inner(None)
    }

    /// Like `new` but also displays a message.
    pub fn new_with_message(message: impl Into<String>) -> Self {
        Self::new_inner(Some(message.into()))
    }

    fn new_inner(message: Option<String>) -> Self {
        if io::stderr().is_terminal() {
            let (tx, rx) = mpsc::channel(10);
            let handle = tokio::spawn(Self::spin(rx, message));
            Self { tx: Some(tx), handle: Some(handle) }
        } else {
            Self { tx: None, handle: None }
        }
    }

    /// Takes a future and shows a CLI spinner (if stderr is a TTY) until its
    /// output is ready.
    pub async fn wrap<F>(future: F) -> F::Output
    where
        F: Future,
    {
        Self::wrap_inner(future, None).await
    }

    /// Like `wrap` but also displays a message.
    pub async fn wrap_with_message<F>(future: F, message: impl Into<String>) -> F::Output
    where
        F: Future,
    {
        Self::wrap_inner(future, Some(message.into())).await
    }

    async fn wrap_inner<F>(future: F, message: Option<String>) -> F::Output
    where
        F: Future,
    {
        let spinner = Spinner::new_inner(message);
        let result = future.await;
        spinner.stop().await;
        result
    }

    /// Set a new message for the spinner to display.
    pub async fn set_message(&self, message: impl Into<String>) {
        if let Some(tx) = &self.tx {
            tx.send(Command::Message(Some(message.into()))).await.ok();
        }
    }

    /// Remove the existing spinner message (if any)
    pub async fn unset_message(&self) {
        if let Some(tx) = &self.tx {
            tx.send(Command::Message(None)).await.ok();
        }
    }

    /// Stop the spinner. This requires a bit of cleanup, and so should be
    /// `await`ed before doing any other i/o.
    pub async fn stop(self) {
        if let Some(tx) = &self.tx {
            tx.send(Command::Stop).await.ok();
        }
        if let Some(handle) = self.handle {
            // of no consequence if the spinner thread panics, squash it
            handle.await.ok();
        }
    }

    /// Spin until receiver sees `Command::Stop` or the channel is closed.
    async fn spin(mut rx: Receiver<Command>, msg: Option<String>) {
        let mut msg = msg;
        let mut dots = SPINNER_DOTS.iter().cycle();
        eprint!("{}", ansi::CURSOR_POSITION_SAVE);
        eprint!("{}", ansi::CURSOR_HIDE);
        eprint!("{}", ansi::CLEAR_LINE);
        let mut interval = tokio::time::interval(time::Duration::from_millis(SPINNER_DELAY));
        loop {
            match rx.try_recv() {
                Err(TryRecvError::Empty) => {
                    eprint!("{}", ansi::CURSOR_POSITION_0);
                    eprint!("{}", ansi::CLEAR_LINE);
                    eprint!("{}", dots.next().unwrap());
                    if let Some(msg) = msg.as_ref() {
                        eprint!(" {msg}")
                    }
                    interval.tick().await;
                },
                Ok(Command::Message(new_msg)) => {
                    msg = new_msg;
                },
                _ => {
                    break;
                },
            }
        }
        eprint!("{}", ansi::CLEAR_LINE);
        eprint!("{}", ansi::CURSOR_POSITION_RESTORE);
        eprint!("{}", ansi::CURSOR_SHOW);
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

mod ansi {
    pub const CURSOR_POSITION_SAVE: &str = "\x1B7";
    pub const CURSOR_POSITION_RESTORE: &str = "\x1B8";
    pub const CURSOR_POSITION_0: &str = "\x1B[0G";
    pub const CURSOR_HIDE: &str = "\x1B[?25l";
    pub const CURSOR_SHOW: &str = "\x1B[?25h";
    pub const CLEAR_LINE: &str = "\x1B[2K";
}
