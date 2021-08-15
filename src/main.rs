use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};

use iced::{
    button, executor, Align, Application, Button, Clipboard, Column, Command, Element, Settings,
    Text,
};

pub fn main() -> iced::Result {
    let (to_worker, from_ui) = mpsc::channel();
    let (to_ui, from_worker) = mpsc::channel();

    std::thread::spawn(move || loop {
        let a = from_ui.recv().unwrap();
        println!("worker got {} - sending {}", a, a * a);
        to_ui.send(a * a).unwrap()
    });

    let last_recv = Arc::new(AtomicI64::new(0));

    {
        let last_recv = last_recv.clone();
        std::thread::spawn(move || loop {
            let rv = from_worker.recv().unwrap();
            last_recv.store(rv, Ordering::SeqCst);
        });
    }

    ThreadWatcher::run(Settings::with_flags((to_worker, last_recv)))
}

struct ThreadWatcher {
    last_sent: i64,
    last_recv: Arc<AtomicI64>,
    to_worker: Sender<i64>,
    dispatch_button: button::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    DispatchPressed,
}

impl Application for ThreadWatcher {
    type Executor = executor::Default;

    type Flags = (Sender<i64>, Arc<AtomicI64>);
    type Message = Message;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            ThreadWatcher {
                last_sent: 0,
                last_recv: flags.1,
                to_worker: flags.0,
                dispatch_button: button::State::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Thread watcher")
    }

    fn update(&mut self, message: Message, _clipboard: &mut Clipboard) -> Command<Message> {
        match message {
            Message::DispatchPressed => {
                self.last_sent += 1;
                self.to_worker.send(self.last_sent).unwrap();
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(20)
            .align_items(Align::Center)
            .push(
                Button::new(&mut self.dispatch_button, Text::new("Dispatch"))
                    .on_press(Message::DispatchPressed),
            )
            .push(Text::new(format!("Last sent: {}", self.last_sent)).size(50))
            .push(
                Text::new(format!(
                    "Last recv: {}",
                    self.last_recv.load(Ordering::SeqCst)
                ))
                .size(50),
            )
            .into()
    }
}
