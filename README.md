# Media Control - rust

Analogue of my project with similar name, written in rust.

## API

### `GET /data`

Response format (json):

```ts
{
  title: string;
  artist: string;

  album_title: string;
  album_artist: string;

  duration: number; // micros
  position: number; // micros

  cover_data: string; // b64

  state: string; // ['playing', 'paused', 'stopped']
}
```

### `POST /control/<command>`

Command list:
- play
- pause
- toggle_pause
- stop
- prev
- next
