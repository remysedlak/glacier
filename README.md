# remdaw

<img width="1431" height="854" alt="image" src="https://github.com/user-attachments/assets/8e0dacc7-71fd-49b1-a7dd-3c9485f2ad88" />

audio
- 16 step music sequencer
- loads .wav files into memory
- configure audio output device with CPAL
- mix instruments with volume ramping

graphics
- all logic related to drawing graphics
- setup vertex buffer
- redraw on user input / events

render
- structs for shape abstractions

shader
- setup graphics for start
- respond to user events by redrawing stuff

app
- initialize the graphics and window with `winit`
- setup the user event loop
- track mouse movement
- handle graphics methods
