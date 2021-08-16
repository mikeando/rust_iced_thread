use core::time;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

use iced::futures::stream::BoxStream;
use iced::{
    button, executor, Align, Application, Button, Clipboard, Column, Command, Element, Settings,
    Text,
};
use iced_native::subscription::Recipe;

pub fn main() -> iced::Result {
    let (to_worker, from_ui) = mpsc::channel();
    let (to_ui, from_worker) = mpsc::channel();

    std::thread::spawn(move || {
        while let Ok(a) = from_ui.recv() {
            println!("worker got {} - sending {}", a, a * a);
            // std::thread::sleep(time::Duration::from_secs(2));
            to_ui.send(a * a).unwrap()
        }
    });

    ThreadWatcher::run(Settings::with_flags((to_worker, from_worker)))
}

struct ThreadWatcher {
    last_sent: i64,
    last_recv: i64,
    to_worker: Sender<i64>,
    from_worker: RecvWrapper<i64>,
    dispatch_button: button::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    DispatchPressed,
    NewValue(i64),
}

impl Application for ThreadWatcher {
    type Executor = executor::Default;

    type Flags = (Sender<i64>, Receiver<i64>);
    type Message = Message;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            ThreadWatcher {
                last_sent: 0,
                last_recv: 0,
                to_worker: flags.0,
                from_worker: RecvWrapper(Arc::new(Mutex::new(flags.1))),
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
            Message::NewValue(v) => {
                self.last_recv = v;
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
            .push(Text::new(format!("Last recv: {}", self.last_recv)).size(50))
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::Subscription::from_recipe(self.from_worker.clone()).map(Message::NewValue)
    }
}

#[derive(Clone)]
struct RecvWrapper<T>(Arc<Mutex<Receiver<T>>>);

pub async fn take_from_recv<T>(
    rx: Arc<Mutex<Receiver<T>>>,
) -> Option<(T, Arc<Mutex<Receiver<T>>>)> {
    let v = rx.lock().unwrap().recv();
    match v {
        Ok(v) => Some((v, rx)),
        Err(mpsc::RecvError) => None,
    }
}

impl<H, E, T> Recipe<H, E> for RecvWrapper<T>
where
    H: Hasher,
    T: 'static + Send,
{
    type Output = T;

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<E>) -> BoxStream<Self::Output> {
        let s = iced::futures::stream::unfold(self.0, take_from_recv);
        Box::pin(s)
    }
}
