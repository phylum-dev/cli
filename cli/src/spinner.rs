use futures::Future;
use std::time;
use tokio::{
    sync::mpsc::{self, error::TryRecvError, Receiver, Sender},
    task::JoinHandle,
};

const SPINNER_DELAY: u64 = 40;
const SPINNER_DOTS: [&str; 56] = [
    "⢀⠀", "⡀⠀", "⠄⠀", "⢂⠀", "⡂⠀", "⠅⠀", "⢃⠀", "⡃⠀", "⠍⠀", "⢋⠀", "⡋⠀", "⠍⠁", "⢋⠁", "⡋⠁", "⠍⠉", "⠋⠉",
    "⠋⠉", "⠉⠙", "⠉⠙", "⠉⠩", "⠈⢙", "⠈⡙", "⢈⠩", "⡀⢙", "⠄⡙", "⢂⠩", "⡂⢘", "⠅⡘", "⢃⠨", "⡃⢐", "⠍⡐", "⢋⠠",
    "⡋⢀", "⠍⡁", "⢋⠁", "⡋⠁", "⠍⠉", "⠋⠉", "⠋⠉", "⠉⠙", "⠉⠙", "⠉⠩", "⠈⢙", "⠈⡙", "⠈⠩", "⠀⢙", "⠀⡙", "⠀⠩",
    "⠀⢘", "⠀⡘", "⠀⠨", "⠀⢐", "⠀⡐", "⠀⠠", "⠀⢀", "⠀⡀",
];

pub struct Spinner {
    tx: Sender<Command>,
    handle: JoinHandle<()>,
}

enum Command {
    Stop,
    Message(Option<String>),
}

impl Spinner {
    /// Start a CLI spinner on the current cursor line. To stop it, call `stop`
    /// on the returned `Spinner`.
    pub fn new() -> Self {
        Self::new_inner(None)
    }

    /// Like `new` but also displays a message
    pub fn new_with_message(message: impl Into<String>) -> Self {
        Self::new_inner(Some(message.into()))
    }

    fn new_inner(message: Option<String>) -> Self {
        let (tx, rx) = mpsc::channel(10);
        let handle = tokio::spawn(Self::spin(rx, message));
        Self { tx, handle }
    }

    /// Takes a future and shows a CLI spinner until it's output is ready
    pub async fn wrap<F>(future: F) -> F::Output
    where
        F: Future,
    {
        Self::wrap_inner(future, None).await
    }

    /// Like `wrap` but also displays a message
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

    /// Set a new message for the spinner to display
    pub async fn set_message(&self, message: impl Into<String>) {
        self.tx
            .send(Command::Message(Some(message.into())))
            .await
            .ok();
    }

    /// Remove the existing spinner message (if any)
    pub async fn unset_message(&self) {
        self.tx.send(Command::Message(None)).await.ok();
    }

    /// Stop the spinner. This requires a bit of cleanup, and so should be `await`ed before doing
    /// any other i/o.
    pub async fn stop(self) {
        self.tx.send(Command::Stop).await.ok();
        self.handle.await.ok(); // of no consequence if the spinner thread panics, squash it
    }

    /// Spin until receiver finds unit
    async fn spin(mut rx: Receiver<Command>, msg: Option<String>) {
        let mut msg = msg;
        let mut dots = SPINNER_DOTS.iter().cycle();
        eprint!("\x1B7"); // save position
        eprint!("\x1B[?25l"); // hide cursor
        eprint!("\x1B[2K"); // clear current line
        let mut interval = tokio::time::interval(time::Duration::from_millis(SPINNER_DELAY));
        loop {
            match rx.try_recv() {
                Err(TryRecvError::Empty) => {
                    eprint!("\x1B[0G"); // move to column 0
                    eprint!("\x1B[2K"); // clear current line
                    eprint!("{}", dots.next().unwrap());
                    if let Some(msg) = msg.as_ref() {
                        eprint!(" {}", msg)
                    }
                    interval.tick().await;
                }
                Ok(Command::Message(new_msg)) => {
                    msg = new_msg;
                }
                _ => {
                    break;
                }
            }
        }
        eprint!("\x1B[2K"); // clear current line
        eprint!("\x1B8"); // restore cursor position
        eprint!("\x1B[?25h"); // show cursor
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}
