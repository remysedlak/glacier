# Audio Mental model

## Project
A project is a file for a song, saved to a `.toml` file.
- bpm
- project name
- tracks
  - name
  - path
- patterns
  - sequences
  - name
- events
  - pattern or track
  - id

## Playlist
The playlist is where patterns and samples (events) are arranged to be played together.
- there is only one playlist component per song. 
Playlists are divided into **steps** and **bars**. 
- There are four steps in a bar, like a standard 4/4 music measure.
- There are many many tracks (horizontal rows to arrange one or some things)


## Pattern
A pattern is a sequence of steps for a set of any tracks
- can be placed on the playlist
- unique ID's

Patterns can be different sizes, start at different times, and overlap at any points.

## Track
Tracks are files that can be added to a project once.
- any pattern can use any track
- unique ID's

All tracks can be loaded as samples to the playlist as well.


---

### UI
A playlist has
- bars to track sequences
  - place patterns or tracks down on a step or bar
- tracks to layer sequences
  - place patterns or tracks on a step or bar on a different row

### AUDIO
A playlist has
- a vector of audio events to play in order
  - audio event
    - time: what step does it start
    - length: how many steps is it.
      - can halve a pattern or slice an track
    - type: what kind of audio is it? pattern or sample
