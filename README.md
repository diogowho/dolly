# dolly

my personal matrix bot. does two things well, nothing else _(for now)_.

## features

### `/2fa` command
fetches steam guard codes from [archi steam farm](https://github.com/JustArchiNET/ArchiSteamFarm) and sends them to you in matrix. only responds to a predefined allowed user.

### bgp alerts
optionally starts an http server that listens for webhooks from [bgp.tools](https://bgp.tools) and forwards them to a matrix room.

## setup

### prerequisites
- rust
- a matrix homeserver account
- an ASF instance (for `/2fa`, with 2FA module enabled)
- optional: an ASN registered on bgp.tools 

### environment variables

| variable | required | default | description |
|----------|----------|---------|-------------|
| `DOLLY_DATA_DIR` | no | `.` | where dolly data will live |
| `PORT` | no | `3000` | port for bgp alerts server |
| `ENABLE_BGPALERTS` | no | `false` | set to `true` to enable bgp alerts |
| `MATRIX_HOMESERVER` | yes | - | matrix homeserver url |
| `MATRIX_USERNAME` | yes | - | matrix username |
| `MATRIX_PASSWORD` | yes | - | matrix password |
| `MATRIX_ROOM_ID` | yes | - | room id to send stuff |
| `MATRIX_ALLOWED_USER` | yes | - | only this user can use `/2fa` |
| `ASF_BOT_NAME` | no | `default` | asf bot name to fetch 2fa from |
| `ASF_IPC_PASSWORD` | no | - | asf ipc password |
| `ASF_BASE_URL` | no | `http://127.0.0.1:1242` | asf instance url |

### running

```sh
cargo run
```

## implementation details

- uses `matrix-sdk` for matrix interactions
- uses `axum` for the http server
- persists matrix session to `matrix_session.json` for restarts
- tracks seen matrix events in `seen_events.json` to avoid duplicates
- uses sqlite for matrix state storage (`matrix_store.sqlite`)

## license

[zlib](./LICENSE): do whatever you want with this, but don't blame me if it breaks.
