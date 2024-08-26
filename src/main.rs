use futures::executor::block_on;
use json::JsonValue;
use media_session::{MediaInfo, MediaSession, MediaSessionControls};

use std::panic;
use std::process;
use std::sync::{mpsc::channel, Arc, Mutex};
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

fn main() {
    colog::default_builder()
        .filter(None, log::LevelFilter::Debug)
        .filter(Some("saaba"), log::LevelFilter::Warn)
        .init();

    let data: Arc<Mutex<MediaInfo>> = Arc::new(Mutex::new(MediaInfo::new()));
    let (tx, rx) = channel::<Controls>();

    let data_session = Arc::clone(&data);
    let thread_session = thread::spawn(move || {
        let handler = move |mi: MediaInfo| {
            *data_session.lock().unwrap() = mi;
        };

        block_on(async {
            let mut player = MediaSession::new().await;
            player.set_callback(handler);

            loop {
                while let Ok(control) = rx.try_recv() {
                    match control {
                        Controls::Next => player.next().await,
                        Controls::Prev => player.prev().await,
                        Controls::Play => MediaSession::play(&player).await,
                        Controls::Pause => MediaSession::pause(&player).await,
                        Controls::Stop => MediaSession::stop(&player).await,
                        Controls::TogglePause => MediaSession::toggle_pause(&player).await,
                    }
                    .unwrap();
                }
                player.update().await;
            }
        });
    });

    let thread_http = thread::spawn(move || {
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

            let tx_clone = tx.clone();

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
            Ok(_) => {}
        }
    });

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    thread_session.join().unwrap();
    thread_http.join().unwrap();
}
