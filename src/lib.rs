use macroquad::miniquad::conf::Platform;
/* C functions to implement
char* GetName();
void SetButtons(uint32_t bitfield);
void SetLights(uint8_t left, uint32_t pos, uint8_t r, uint8_t g, uint8_t b);
void Tick(float deltaTime);

//Return 0 on success
int Init(void(*)(char*));
int Close();
*/
use macroquad::prelude::*;
static LOG: OnceCell<extern "C" fn(c: *const u8)> = OnceCell::new();
static SENDER: OnceCell<Mutex<Sender<UpdateLights>>> = OnceCell::new();

#[no_mangle]
pub extern "C" fn GetName() -> *const u8 {
    "Light Test Window".as_ptr()
}

#[no_mangle]
pub extern "C" fn SetButtons(bitfield: u32) {
    if let Some(Ok(sender)) = SENDER.get().map(|s| s.try_lock()) {
        if let Err(update) = sender.send(UpdateLights::Buttons(bitfield)) {
            unsafe {
                LOG.get_unchecked()(format!("{:?}", update).as_ptr());
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn SetLights(left: u8, pos: u32, r: u8, g: u8, b: u8) {
    if let Some(Ok(sender)) = SENDER.get().map(|s| s.try_lock()) {
        let update_msg = if left == 1 {
            UpdateLights::Left(Color::from_rgba(r, g, b, 255), pos as usize)
        } else {
            UpdateLights::Right(Color::from_rgba(r, g, b, 255), pos as usize)
        };

        if let Err(update) = sender.send(update_msg) {
            unsafe {
                LOG.get_unchecked()(format!("{:?}", update).as_ptr());
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn Tick(_delta_time: f32) {
    if let Some(Ok(sender)) = SENDER.get().map(|s| s.try_lock()) {
        if let Err(update) = sender.send(UpdateLights::NextFrame) {
            unsafe {
                LOG.get_unchecked()(format!("{:?}", update).as_ptr());
            }
        }
    }
}
#[no_mangle]
pub extern "C" fn Close() {
    if let Some(Ok(sender)) = SENDER.get().map(|s| s.try_lock()) {
        if let Err(update) = sender.send(UpdateLights::Quit) {
            unsafe {
                LOG.get_unchecked()(format!("{:?}", update).as_ptr());
            }
        }
    }
}

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Mutex};

use once_cell::sync::OnceCell;

#[derive(Debug, Default)]
struct LightStatus {
    buttons: u32,
    left: [Color; 3],
    right: [Color; 3],
}

enum UpdateLights {
    Buttons(u32),
    Left(Color, usize),
    Right(Color, usize),
    NextFrame,
    Quit,
}

struct LightTester {
    light_status: LightStatus,
    rx: Receiver<UpdateLights>,
}

async fn render(mut ctx: LightTester) {
    let bt_width = 75.0;
    let st_width = 25.0;
    let fx_width = bt_width * 1.75;
    for msg in &ctx.rx {
        match msg {
            UpdateLights::Buttons(b) => ctx.light_status.buttons = b,
            UpdateLights::Left(c, i) => ctx.light_status.left[i] = c,
            UpdateLights::Right(c, i) => ctx.light_status.right[i] = c,
            UpdateLights::NextFrame => {
                next_frame().await;

                for i in 0..ctx.light_status.left.len() {
                    let height = screen_height() / ctx.light_status.left.len() as f32;
                    draw_rectangle(
                        0.0,
                        i as f32 * height,
                        100.0,
                        height,
                        ctx.light_status.left[i],
                    );

                    draw_rectangle(
                        screen_width() - 100.0,
                        i as f32 * height,
                        100.0,
                        height,
                        ctx.light_status.right[i],
                    );
                }

                for i in 0..4 {
                    let bt_rect = Rect {
                        x: (screen_width() / 2.0) - bt_width * 2.0 + bt_width * i as f32,
                        y: bt_width + 50.0,
                        w: bt_width,
                        h: bt_width,
                    };

                    if ctx.light_status.buttons & 1 << i > 0 {
                        draw_rectangle(bt_rect.x, bt_rect.y, bt_rect.w, bt_rect.h, WHITE);
                    }
                    draw_rectangle_lines(bt_rect.x, bt_rect.y, bt_rect.w, bt_rect.h, 2.0, GRAY);
                }

                for i in 0..2 {
                    let fx_rect = Rect {
                        x: (screen_width() / 2.0) - fx_width + fx_width * i as f32,
                        y: 2.0 * bt_width + 50.0 + 25.0,
                        w: fx_width,
                        h: bt_width / 2.0,
                    };

                    if ctx.light_status.buttons & 1 << (i + 4) > 0 {
                        draw_rectangle(fx_rect.x, fx_rect.y, fx_rect.w, fx_rect.h, ORANGE);
                    }
                    draw_rectangle_lines(fx_rect.x, fx_rect.y, fx_rect.w, fx_rect.h, 2.0, RED);
                }

                let st_rect = Rect {
                    x: (screen_width() / 2.0) - st_width / 2.0,
                    y: 25.0,
                    w: st_width,
                    h: st_width,
                };
                if ctx.light_status.buttons & 1 << 6 > 0 {
                    draw_rectangle(st_rect.x, st_rect.y, st_rect.w, st_rect.h, BLUE);
                }
                draw_rectangle_lines(st_rect.x, st_rect.y, st_rect.w, st_rect.h, 2.0, BLUE);
            }
            UpdateLights::Quit => return,
        }
    }
}

#[no_mangle]
pub extern "C" fn Init(log: extern "C" fn(c: *const u8)) -> i32 {
    std::panic::set_hook(Box::new(|e| {
        if let Some(log) = LOG.get() {
            log(e.to_string().as_ptr());
        }
    }));

    if LOG.set(log).is_err() {
        return 1;
    }

    let (tx, rx) = mpsc::channel::<UpdateLights>();
    if SENDER.set(Mutex::new(tx)).is_err() {
        return 1;
    }
    std::thread::spawn(|| {
        let light_tester = LightTester {
            light_status: LightStatus::default(),
            rx,
        };

        macroquad::Window::from_config(
            Conf {
                window_title: "USC Lights".to_string(),
                high_dpi: true,
                platform: Platform {
                    swap_interval: Some(0),
                    ..Default::default()
                },
                ..Default::default()
            },
            render(light_tester),
        );
    });

    0
}
