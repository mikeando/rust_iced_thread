#![feature(async_stream)]

use std::hash::{Hash, Hasher};
use std::pin::Pin;

use iced::futures::{SinkExt, Stream};
use iced::futures::channel::mpsc;
use iced::futures::channel::mpsc::{Receiver,Sender};

use iced::futures::stream::BoxStream;
use iced::{
    button, executor, Align, Application, Button, Clipboard, Column, Command, Element, Settings,
    Text,
};
use iced_native::subscription::Recipe;

pub fn main() -> iced::Result {
    let (to_worker, mut from_ui) = mpsc::channel(10);
    let (mut to_ui, from_worker) = mpsc::channel(10);

    std::thread::spawn(move || loop {
        match from_ui.try_next() {
            Ok(Some(a)) => {
                println!("worker got {} - sending {}", a, a * a);
                to_ui.try_send(a * a).unwrap()
            },
            Ok(None) => {
                to_ui.close();
                break;
            }
            Err(e) => {
            }
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
        println!("Application::update message={:?}", message);
        match message {
            Message::DispatchPressed => {
                self.last_sent += 1;
                self.to_worker.try_send(self.last_sent).unwrap();
            }
            Message::NewValue(v) => {
                self.last_recv = v;
            },
            
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
        iced::Subscription::from_recipe(self.from_worker).map( |v| Message::NewValue(v))
    }
}

struct RecvWrapper<T>( Receiver<T> );

impl<T> Stream for RecvWrapper<T> {
    type Item = T;
    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        return Pin::new(&mut self.0).poll_next(cx)
    }
}

impl<H, E, T> Recipe<H, E> for RecvWrapper<T>
where
    H: Hasher,
    T: 'static + Send,
{
    type Output=T;

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: BoxStream<E>,
    ) -> BoxStream<Self::Output> {
        Box::pin(self.0)
    }
}
