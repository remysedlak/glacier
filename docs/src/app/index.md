# glacier-app

The Glacier app module contains all of the code for setting up and running the audio and graphics thread.

The architecture involve two decoupled threads communicating events to ring buffers. The app logic is combined on a native desktop application via `winit`.

```mermaid
sequenceDiagram
    participant U as UI Thread
    participant RB as ring buffer
    participant A as Audio Thread
    U->>RB: try_push(AudioCommand)
    RB->>A: try_pop(AudioCommand)
    Note over U,A: Audio Thread pops on its own callback tick, no direct call
    A->>A: is_playing = !is_playing
```
