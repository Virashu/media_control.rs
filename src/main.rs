use futures::executor::block_on;
use json::JsonValue;
use media_session::{MediaInfo, MediaSession, MediaSessionControls};

use std::panic;
use std::process;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;

fn jsonify(info: MediaInfo) -> JsonValue {
    json::object! {
        title: info.title,
        artist: info.artist,

        album_title: info.album_title,
        album_artist: info.album_artist,

        duration: info.duration,
        position: info.position,

        cover_data: info.cover_b64,

        state: info.state,
    }
}

#[derive(Copy, Clone)]
enum Controls {
    Play,
    Pause,
    Stop,
    TogglePause,
    Next,
    Prev,
}

const ACTIONS: &[(&str, Controls)] = &[
    ("play", Controls::Play),
    ("pause", Controls::Pause),
    ("stop", Controls::Stop),
    ("toggle_pause", Controls::TogglePause),
    ("next", Controls::Next),
    ("prev", Controls::Prev),
];

type MediaInfoMutex = Arc<Mutex<MediaInfo>>;

fn run_media_session(data: MediaInfoMutex, channel_rx: Receiver<Controls>) {
    let handler = move |mi: MediaInfo| {
        *data.lock().unwrap() = mi;
    };

    block_on(async {
        let mut player = MediaSession::new().await;
        player.set_callback(handler);

        loop {
            while let Ok(control) = channel_rx.try_recv() {
                match control {
                    Controls::Next => player.next().await,
                    Controls::Prev => player.prev().await,
                    Controls::Play => player.play().await,
                    Controls::Pause => player.pause().await,
                    Controls::Stop => player.stop().await,
                    Controls::TogglePause => player.toggle_pause().await,
                }
                .unwrap();
            }
            player.update().await;
        }
    });
}

fn run_http_server(data: MediaInfoMutex, channel_tx: Sender<Controls>) {
    let mut app = saaba::App::new();

    app.get("/data", move |_| {
        let info = data.lock().unwrap().clone();
        let content: String = json::stringify(jsonify(info));

        saaba::Response::from_content_string(content)
            .with_header("Access-Control-Allow-Origin", "*")
    });

    for (codename, control) in ACTIONS {
        let path = format!("/control/{}", codename);
        let control = control.clone();

        let tx_clone = channel_tx.clone();

        app.post(path.as_str(), move |_| {
            log::info!("Command: {}", codename);
            tx_clone.send(control).unwrap();
            saaba::Response::new()
        });
    }

    match app.run("0.0.0.0", 8888) {
        Err(_) => {
            log::error!("Address is occupied");
            panic!("Address is occupied");
        }
        _ => {}
    }
}

fn main() {
    colog::default_builder()
        .filter(None, log::LevelFilter::Debug)
        .filter(Some("saaba"), log::LevelFilter::Warn)
        .filter(Some("media_session"), log::LevelFilter::Warn)
        .init();

    let data = Arc::new(Mutex::new(MediaInfo::new()));
    let (tx, rx) = channel::<Controls>();

    let data_move = data.clone();
    let thread_session = thread::spawn(|| run_media_session(data_move, rx));

    let data_move = data.clone();
    let thread_http = thread::spawn(|| run_http_server(data_move, tx));

    let orig_hook = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    thread_session.join().unwrap();
    thread_http.join().unwrap();
}
