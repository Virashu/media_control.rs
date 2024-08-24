use futures::executor::block_on;
use json::JsonValue;
use media_session::{MediaInfo, MediaSession};
use std::{
    sync::{Arc, Mutex},
    thread::spawn,
};

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

fn main() {
    colog::default_builder()
        .filter(None, log::LevelFilter::Debug)
        .filter(Some("saaba"), log::LevelFilter::Warn)
        .init();

    let data: Arc<Mutex<MediaInfo>> = Arc::new(Mutex::new(MediaInfo::new()));
    
    let data_session = Arc::clone(&data);
    let thread_session = spawn(|| {
        let handler = move |mi: MediaInfo| {
            *data_session.lock().unwrap() = mi;
        };

        block_on(async {
            let mut player = MediaSession::new().await;
            player.set_callback(handler);

            loop {
                player.update().await;
            }
        });
    });

    let thread_http = spawn(|| {
        let mut app = saaba::App::new();

        app.get("/data", move |_| {
            let info = data.lock().unwrap().clone();
            let content: String = json::stringify(jsonify(info));

            let mut res = saaba::Response::from_content_string(content);
            res.set_header("Access-Control-Allow-Origin", "*");
            res
        });

        match app.run("0.0.0.0", 8888) {
            Err(_) => log::error!("Address is occupied"),
            Ok(_) => {}
        }
    });

    thread_session.join().unwrap();
    thread_http.join().unwrap();
}
