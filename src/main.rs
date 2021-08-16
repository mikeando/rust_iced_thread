use std::hash::{Hash, Hasher};
//use std::sync::mpsc::{Receiver, Sender};
use async_std::channel;
use async_std::channel::{Receiver, Sender};

use futures_core::Stream;
use iced::futures::executor::ThreadPool;
use iced::futures::stream::BoxStream;
use iced::futures::StreamExt;
use iced::{
    button, executor, Align, Application, Button, Clipboard, Column, Command, Element, Settings,
    Text,
};
use iced_native::subscription::Recipe;

pub async fn worker_thread(from_ui: Receiver<i64>, to_ui: Sender<i64>) {
    while let Ok(a) = from_ui.recv().await {
        println!("worker got {} - sending {}", a, a * a);
        to_ui.send(a * a).await.unwrap()
    }
}

pub fn main() -> iced::Result {
    let (to_worker, from_ui) = channel::unbounded();
    let (to_ui, from_worker) = channel::unbounded();

    let worker_pool = ThreadPool::new().unwrap();
    let w = worker_thread(from_ui, to_ui);
    worker_pool.spawn_ok(w);

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
    Dummy,
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
                from_worker: RecvWrapper(flags.1),
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
                let to_worker = self.to_worker.clone();
                let last_sent = self.last_sent;
                let f = async move {
                    to_worker.send(last_sent).await.unwrap();
                    Message::Dummy
                };
                return f.into();
            }
            Message::NewValue(v) => {
                self.last_recv = v;
            }
            Message::Dummy => {}
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
struct RecvWrapper<T>(Receiver<T>);

impl<T> Stream for RecvWrapper<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}

impl<H, E, T> Recipe<H, E> for RecvWrapper<T>
where
    H: Hasher,
    T: 'static + Send + Clone,
{
    type Output = T;

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<E>) -> BoxStream<Self::Output> {
        Box::pin(self)
    }
}
