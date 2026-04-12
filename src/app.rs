use crate::audio::AudioCommand;
use crate::graphics::{create_graphics, ClickResult, Graphics, Rc};
use ringbuf::traits::{Consumer, Producer};
use ringbuf::{HeapCons, HeapProd};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

// commands that the audio engine sends to the window
pub enum UiCommand {
    StepAdvanced(usize),
    LoadTrack(usize, String, [bool; 16], bool),
    LoadBpm(f32),
    // InstrumentAdded(...) when we get there
}

// the app is in initializing state or its ready to draw
enum State {
    Ready(Graphics),
    Init(Option<EventLoopProxy<Graphics>>),
}

pub struct App {
    producer: HeapProd<AudioCommand>,
    consumer: HeapCons<UiCommand>,
    state: State,
    mouse_x: f64,
    mouse_y: f64,
}

impl App {
    // initalize the event loop on creation
    pub fn new(
        producer: HeapProd<AudioCommand>,
        consumer: HeapCons<UiCommand>,
        event_loop: &EventLoop<Graphics>,
    ) -> Self {
        Self {
            producer,
            consumer,
            state: State::Init(Some(event_loop.create_proxy())),
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    // if the state is ready, draw the frame
    fn draw(&mut self) {
        if let State::Ready(gfx) = &mut self.state {
            while let Some(cmd) = self.consumer.try_pop() {
                match cmd {
                    UiCommand::StepAdvanced(size) => {
                        gfx.active_step = size;
                    }
                    UiCommand::LoadTrack(i, name, steps, mute) => {
                        gfx.load_track(i, name, steps, mute);
                    }
                    UiCommand::LoadBpm(bpm) => {
                        gfx.bpm = bpm;
                    }
                }
            }
            gfx.draw(self.mouse_x, self.mouse_y);
            gfx.request_redraw(); // add this
        }
    }

    // handles window resizing and min/maximizing
    fn resized(&mut self, size: PhysicalSize<u32>) {
        if let State::Ready(gfx) = &mut self.state {
            gfx.resize(size);
        }
    }
}

// app startup
impl ApplicationHandler<Graphics> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let State::Init(proxy) = &mut self.state {
            if let Some(proxy) = proxy.take() {
                let mut win_attr = Window::default_attributes();

                #[cfg(not(target_arch = "wasm32"))]
                {
                    win_attr = win_attr.with_title("WebGPU example");
                }

                #[cfg(target_arch = "wasm32")]
                {
                    use winit::platform::web::WindowAttributesExtWebSys;
                    win_attr = win_attr.with_append(true);
                }

                let window = Rc::new(
                    event_loop
                        .create_window(win_attr)
                        .expect("create window err."),
                );

                #[cfg(target_arch = "wasm32")]
                wasm_bindgen_futures::spawn_local(create_graphics(window, proxy));

                #[cfg(not(target_arch = "wasm32"))]
                pollster::block_on(create_graphics(window, proxy));
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, graphics: Graphics) {
        // Request a redraw now that graphics are ready
        graphics.request_redraw();
        self.state = State::Ready(graphics);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() && button == MouseButton::Left {
                    if let State::Ready(gfx) = &mut self.state {
                        match gfx.handle_button_click(self.mouse_x, self.mouse_y) {
                            ClickResult::Step(track, step) => {
                                self.producer
                                    .try_push(AudioCommand::ToggleStep(track, step))
                                    .ok();
                            }
                            ClickResult::Mute(track) => {
                                self.producer.try_push(AudioCommand::ToggleMute(track)).ok();
                            }
                            ClickResult::ChangeBpm(bpm) => {
                                self.producer.try_push(AudioCommand::ChangeBpm(bpm)).ok();
                            }
                            ClickResult::None => {}
                        }
                    } // closes if let
                    self.draw();
                } // closes if state.is_pressed()
            } // closes MouseInput arm

            WindowEvent::CursorMoved { position, .. } => {
                // position.x and position.y are available here
                self.mouse_x = position.x;
                self.mouse_y = position.y;
                self.draw();
            }
            _ => {}
        }
    }
}
