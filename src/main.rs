use geng::prelude::*;
use sciter::windowless as sw;

struct Test {
    geng: Geng,
    timer: Timer,
    overlay: Rc<RefCell<ugli::Texture>>,
    sciter_host: sciter::Host,
    size: Vec2<usize>,
    mouse_buttons: sw::MOUSE_BUTTONS,
}

fn handle_message(wnd: sciter::types::HWINDOW, msg: sw::Message) {
    sw::handle_message(wnd, msg);
}

impl Test {
    fn new(geng: &Geng, sciter_host: sciter::Host) -> Self {
        Self {
            geng: geng.clone(),
            sciter_host,
            timer: Timer::new(),
            overlay: Rc::new(RefCell::new(ugli::Texture::new_uninitialized(
                geng.ugli(),
                vec2(1, 1),
            ))),
            size: vec2(1, 1),
            mouse_buttons: sw::MOUSE_BUTTONS::NONE,
        }
    }
}

impl geng::State for Test {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Rgba::GREEN), None, None);
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::Quad::unit(Rgba::WHITE)
                .scale_uniform(10.0)
                .translate(self.geng.window().mouse_pos().map(|x| x as f32)),
        );

        let needed_size = framebuffer.size();
        self.size = needed_size;
        if self.overlay.borrow().size() != needed_size {
            {
                let mut texture = ugli::Texture::new_uninitialized(self.geng.ugli(), needed_size);
                let mut framebuffer = ugli::Framebuffer::new_color(
                    self.geng.ugli(),
                    ugli::ColorAttachment::Texture(&mut texture),
                );
                ugli::clear(&mut framebuffer, Some(Rgba::TRANSPARENT_BLACK), None, None);
                self.overlay = Rc::new(RefCell::new(texture));
            }
            handle_message(
                WND,
                sw::Message::Size {
                    width: needed_size.x as u32,
                    height: needed_size.y as u32,
                },
            );
        }
        handle_message(
            WND,
            sw::Message::RenderTo(sw::RenderEvent {
                layer: None,
                callback: Box::new({
                    let overlay = self.overlay.clone();
                    move |rect: &sciter::types::RECT, data: &[u8]| {
                        let mut data = data.to_owned();
                        // for (i = 0; i < len; i += 4)
                        for i in (0..data.len()).step_by(4) {
                            data.swap(i, i + 2);
                        }
                        overlay.borrow_mut().sub_image(
                            vec2(rect.left, rect.top).map(|x| x as usize),
                            vec2(rect.width(), rect.height()).map(|x| x as usize),
                            &data,
                        );
                    }
                }),
            }),
        );
        self.geng.draw_2d(
            framebuffer,
            &geng::PixelPerfectCamera,
            &draw_2d::TexturedQuad::new(
                AABB::point(vec2(0.0, framebuffer.size().y as f32)).extend_positive(vec2(
                    framebuffer.size().x as f32,
                    -(framebuffer.size().y as f32),
                )),
                &*self.overlay.borrow(),
            ),
        );
    }
    fn update(&mut self, delta_time: f64) {
        handle_message(
            WND,
            sw::Message::Heartbit {
                milliseconds: (self.timer.elapsed() * 1000.0) as u32,
            },
        );
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseMove { position, .. } => handle_message(
                WND,
                sw::Message::Mouse(sw::MouseEvent {
                    event: sw::MOUSE_EVENTS::MOUSE_MOVE,
                    button: self.mouse_buttons,
                    modifiers: 0.into(),
                    pos: sciter::types::POINT {
                        x: position.x as i32,
                        y: (self.size.y as i32 - 1 - position.y as i32),
                    },
                }),
            ),
            geng::Event::MouseDown { position, button } => {
                self.mouse_buttons = match button {
                    geng::MouseButton::Left => sw::MOUSE_BUTTONS::MAIN,
                    geng::MouseButton::Right => sw::MOUSE_BUTTONS::PROP,
                    geng::MouseButton::Middle => sw::MOUSE_BUTTONS::MIDDLE,
                };
                handle_message(
                    WND,
                    sw::Message::Mouse(sw::MouseEvent {
                        event: sw::MOUSE_EVENTS::MOUSE_DOWN,
                        button: self.mouse_buttons,
                        modifiers: 0.into(),
                        pos: sciter::types::POINT {
                            x: position.x as i32,
                            y: (self.size.y as i32 - 1 - position.y as i32),
                        },
                    }),
                )
            }
            geng::Event::MouseUp { position, button } => {
                handle_message(
                    WND,
                    sw::Message::Mouse(sw::MouseEvent {
                        event: sw::MOUSE_EVENTS::MOUSE_UP,
                        button: self.mouse_buttons,
                        modifiers: 0.into(),
                        pos: sciter::types::POINT {
                            x: position.x as i32,
                            y: (self.size.y as i32 - 1 - position.y as i32),
                        },
                    }),
                );
                self.mouse_buttons = sw::MOUSE_BUTTONS::NONE;
            }
            _ => {}
        }
    }
}

const WND: sciter::types::HWINDOW = 0x123 as _;

fn main() {
    // configure Sciter
    if let Some(arg) = std::env::args().nth(1) {
        println!("loading sciter from {:?}", arg);
        if let Err(_) = sciter::set_options(sciter::RuntimeOptions::LibraryPath(&arg)) {
            panic!("Invalid sciter-lite dll specified.");
        }
    } else {
        panic!("usage: cargo run -p windowless -- sciter-sdk/bin.win/x64lite/sciter.dll")
    }
    println!("create sciter instance");
    sciter::set_options(sciter::RuntimeOptions::UxTheming(true)).unwrap();
    // sciter::set_options(sciter::RuntimeOptions::DebugMode(true)).unwrap();
    sciter::set_options(sciter::RuntimeOptions::ScriptFeatures(0xFF)).unwrap();

    handle_message(
        WND,
        sw::Message::Create {
            backend: sciter::types::GFX_LAYER::SKIA_OPENGL,
            transparent: true,
        },
    );

    // MUST BE AFTER sending CREATE OK?
    let sciter_host = sciter::Host::attach(WND);
    sciter_host.load_html(include_bytes!("test.html"), None);

    logger::init().unwrap();
    let geng = Geng::new("Hello?");

    // release CPU a bit, hackish
    std::thread::sleep(std::time::Duration::from_millis(100));
    geng::run(&geng, Test::new(&geng, sciter_host));

    handle_message(WND, sw::Message::Destroy);
}
