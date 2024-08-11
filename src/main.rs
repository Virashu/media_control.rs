use futures::executor::{self, block_on};
use json::JsonValue;
use media_session::{MediaInfo, MediaSession};

fn jsonify(info: MediaInfo) -> JsonValue {
    json::object! {
        title: info.title,
        artist: info.artist,

        album_title: info.album_title,
        album_artist: info.album_artist,

        duration: info.duration,
        position: info.position,

        state: info.state,
    }
}

fn main() {
    async fn get_player() -> MediaSession {
        MediaSession::new().await
    }

    let player = executor::block_on(get_player());

    let mut app = saaba::App::new();

    app.get("/", move |_| {
        let info = block_on(player.clone().get_info());

        let content: String = json::stringify(jsonify(info));

        saaba::Response::from_content_string(content)
    });

    app.run("0.0.0.0", 8888);
}
