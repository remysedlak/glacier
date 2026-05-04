
# How to build a UI library 

a plan for integrating user i/o into UI interactivity.

### Design Philosophies
- Updating a field with a one dimensional range of values (-1.0, 1.0) allows both typed precision and UI interactivity
- UI components should only have one implementation allowing shape changes.
- Small repeated components like steps and buttons should look good vertically stacked and in a row.
- Drag and drop boundaries are strict.
- Colors should come from a hardcoded set of rgb floats
- The app owned graphics state includes all coordinates of movable components

### Infrastructure Phil
- The graphics module only ever takes commands from the app and audio
  - All logic related to user input stays in app module
  - All logic related to audio stays in the audio module

---

# Detecting user events

### Mouse movement

All mouse movement is updated by WindowEvent::CursorMoved. (cursor position = (x: float, y: float)))
- Mouse coordinates are saved in state every frame allowing evaluations of UI borders and hovering
- this helps with knowing when buttons are clicked or when the mouse is on top of any component.

### Clicking

All clicks can be tracked by watching MouseButton's press state.
- buttons and primary features use the left click like usual
- same with right click for options
- The graphics scans every components coordinates to see if any action should be taken (repaint or IPC to audio thread)

# Dragging

Dragging is defined by the movement of the mouse only while a click sustains
- So the first step is checking if the mouse is pressed
  - within this condition, get the states mouse position and do your logic (drag and drop, dial/knobs)

---

# UI

## Buttons (anything you click to interact with)

A button requires watching if MouseButton::Left is pressed. within it's coordinates.
- boundaries for a rectangle appear as
- `if (mouse_posx > r.x && mouse_posx < r.x + rectangle.width && mouse_posy > r.y && mouse_posy < r.y + r.height)`
- button actions
  - perform button logic on release (click is just holding the button down)
  - perform button logic on click (instantly perform action, a toggle, release is redundnat)

## Sliders (Drag a value on a track from min to max)

A slider is like a button, you must press it with Mouse::Left
- Buttons are glued to a window or popup, and sliders are glued to a **track**
- This time the button must be held down and not released until a value is changed
  - values are changed by moving your mouse up and down or left and right
  - to normalize the input, divide pixels moved by the max width or height of the track
- Sliders usually change one dimension of data (horizontal sliders track x vs vertical sliders and y)
- The slider UI coordinates should reflect where the user has dragged the paint.
  - but this mouse movement is ONLY considered when the mouse has already started to be pressed.

## Drag and Drop

Like sliders, Drag and Drop requires pressing onto something with Mouse::Left and sustaining the press.
- Sliders were limited to one dimension, being x or y with a minimum or maximum value possible.
- Drag and drop has no limit to its minimum or maximum and it has both dimensions free to drag.
- The developer must specify where to watch for the initial click, and where to drop it off
  - if the drop is not in a targetted interaction, then the action is ignored/killed.
  - barriers for drag and drop should match sizes of components
  - items should show the state that they are being dragged and dropped.
