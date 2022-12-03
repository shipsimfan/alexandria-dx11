use crate::{graphics::Graphics, Viewport};
use alexandria_common::{Input, Key, MouseButton, Vector2, Viewport as CommonViewport};
use std::{cell::RefCell, ffi::CString, ptr::null, rc::Rc};
use win32::RawInput;

pub struct Window<I: Input> {
    input: I,
    width: usize,
    height: usize,

    h_wnd: win32::HWnd,
    msg: win32::Msg,
    graphics: Option<Graphics>,

    mouse_center: (i32, i32),
    update_mouse_center: bool,

    debug_logging: bool,

    minimized: bool,
    in_size_move: bool,
    window_size_changed: bool,
}

const MIN_SIZE_X: win32::Long = 800;
const MIN_SIZE_Y: win32::Long = 600;

extern "C" fn message_router<I: Input>(
    h_wnd: win32::HWnd,
    msg: win32::UInt,
    w_param: win32::WParam,
    l_param: win32::LParam,
) -> win32::LResult {
    let app: &mut Window<I> = if msg == win32::WM_CREATE {
        let ptr = win32::CreateStructA::from_l_param(l_param).create_params() as *mut _;
        win32::set_window_long_ptr(h_wnd, win32::GWLP_USERDATA, ptr as *const _);
        unsafe { &mut *ptr }
    } else {
        unsafe { &mut *(win32::get_window_long_ptr(h_wnd, win32::GWLP_USERDATA) as *mut _) }
    };

    app.wnd_proc(h_wnd, msg, w_param, l_param)
}

impl<I: Input> Window<I> {
    pub fn device(&self) -> &Rc<win32::ID3D11Device> {
        &self.graphics.as_ref().unwrap().device()
    }

    pub fn device_context(&self) -> &Rc<RefCell<win32::ID3D11DeviceContext>> {
        &self.graphics.as_ref().unwrap().device_context()
    }

    fn wnd_proc(
        &mut self,
        h_wnd: win32::HWnd,
        msg: win32::UInt,
        w_param: win32::WParam,
        l_param: win32::LParam,
    ) -> win32::LResult {
        match msg {
            win32::WM_SIZE => match w_param == win32::SIZE_MINIMIZED {
                true => self.minimized = true,
                false => match self.minimized {
                    true => self.minimized = false,
                    false => self.update_size(),
                },
            },
            win32::WM_ENTERSIZEMOVE => self.in_size_move = true,
            win32::WM_EXITSIZEMOVE => {
                self.in_size_move = false;
                self.update_size();
            }
            win32::WM_GETMINMAXINFO => {
                win32::MinMaxInfo::from_l_param(l_param).set_min_size(MIN_SIZE_X, MIN_SIZE_Y)
            }
            win32::WM_DESTROY => win32::post_quit_message(0),
            win32::WM_CLOSE => win32::destroy_window(h_wnd).unwrap_or(()),
            win32::WM_WINDOWPOSCHANGED => {
                self.update_mouse_center = true;
            }
            win32::WM_LBUTTONDOWN => self.input.mouse_down(MouseButton::Primary),
            win32::WM_LBUTTONUP => self.input.mouse_up(MouseButton::Primary),
            win32::WM_RBUTTONDOWN => self.input.mouse_down(MouseButton::Secondary),
            win32::WM_RBUTTONUP => self.input.mouse_up(MouseButton::Secondary),
            win32::WM_MBUTTONDOWN => self.input.mouse_down(MouseButton::Middle),
            win32::WM_MBUTTONUP => self.input.mouse_up(MouseButton::Middle),
            win32::WM_MOUSEMOVE => {
                let x = (l_param & 0xFFFF) as i16;
                let y = (l_param.wrapping_shr(16) & 0xFFFF) as i16;

                let width2 = self.width as isize / 2;
                let height2 = self.height as isize / 2;

                self.input
                    .update_mouse_position((x as isize - width2, y as isize - height2));

                if self.input.is_mouse_locked() {
                    self.reset_mouse_position();
                }
            }
            win32::WM_SETFOCUS => {
                if self.input.is_mouse_locked() {
                    self.reset_mouse_position();
                }
            }
            win32::WM_INPUT => {
                let mut size = 0;
                win32::get_raw_input_data(
                    l_param,
                    win32::RawInputDataCommand::Input,
                    None,
                    &mut size,
                );

                let mut data = vec![0; size as usize];
                win32::get_raw_input_data(
                    l_param,
                    win32::RawInputDataCommand::Input,
                    Some(&mut data),
                    &mut size,
                );

                let raw = RawInput::from(&data);

                let key = match raw.keyboard() {
                    Some(key) => key,
                    None => return 0,
                };

                let pressed = key.pressed();
                match parse_vkey(&key) {
                    Some(key) => match pressed {
                        true => self.input.key_down(key),
                        false => self.input.key_up(key),
                    },
                    None => {}
                }
            }
            _ => return win32::def_window_proc(h_wnd, msg, w_param, l_param),
        }

        0
    }

    fn reset_mouse_position(&mut self) {
        if self.update_mouse_center {
            self.update_mouse_center()
        }

        win32::set_cursor_pos(self.mouse_center.0, self.mouse_center.1)
            .expect("Failed to set mouse position!");
    }

    fn update_mouse_center(&mut self) {
        self.mouse_center = win32::client_to_screen(
            self.h_wnd,
            (self.width / 2) as i32,
            (self.height / 2) as i32,
        )
        .expect("Failed to convert coordinates!");
        self.update_mouse_center = false;
    }

    fn update_size(&mut self) {
        if self.h_wnd == null() {
            return;
        }

        let rect = win32::get_window_rect(self.h_wnd).unwrap();
        let new_width = (rect.right - rect.left) as usize;
        let new_height = (rect.bottom - rect.top) as usize;

        if new_width == self.width && new_height == self.height {
            return;
        }

        self.width = new_width;
        self.height = new_height;
        self.window_size_changed = true;

        let graphics = self.graphics.as_mut().unwrap();
        graphics.resize_swap_chain(self.width as u32, self.height as u32);

        graphics.update_viewports(Vector2::new(self.width as f32, self.height as f32));
    }
}

impl<I: Input> alexandria_common::Window<I> for Box<Window<I>> {
    type Viewport = Viewport;

    fn new(
        title: &str,
        width: usize,
        height: usize,
        debug_logging: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        const STYLE: &[win32::Ws] = &[win32::Ws::OverlappedWindow, win32::Ws::Visible];

        // Create window box
        let mut window = Box::new(Window {
            h_wnd: null(),
            msg: win32::Msg::default(),
            input: I::new(),
            graphics: None,
            width,
            height,
            mouse_center: (0, 0),
            update_mouse_center: true,
            debug_logging,
            minimized: false,
            in_size_move: false,
            window_size_changed: false,
        });

        // Register window class
        let window_name = CString::new(title).unwrap();
        let wnd_class = win32::WndClassEx::new(
            &[win32::Cs::OwnDC, win32::Cs::HRedraw, win32::Cs::VRedraw],
            message_router::<I>,
            0,
            0,
            None,
            None,
            None,
            None,
            None,
            &window_name,
            None,
        );
        win32::register_class_ex(&wnd_class)?;

        // Create window
        let mut rect = win32::Rect::default();
        rect.right = width as i32;
        rect.bottom = height as i32;
        win32::adjust_window_rect(&mut rect, &STYLE, false)?;

        window.h_wnd = win32::create_window_ex(
            &[],
            &window_name,
            &window_name,
            STYLE,
            None,
            None,
            rect.right - rect.left,
            rect.bottom - rect.top,
            None,
            None,
            None,
            Some(window.as_ref() as *const _ as *const _),
        )?;

        // Register Raw Input
        win32::register_raw_input_devices(&[win32::RawInputDevice::new(
            win32::RawInputUsage::GenericKeyboard,
            &[win32::RawInputFlag::NoLegacy],
            None,
        )])?;

        // Create Graphics
        window.graphics = Some(Graphics::new(window.h_wnd, width as u32, height as u32)?);

        window.update_mouse_center();

        Ok(window)
    }

    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn input(&self) -> &I {
        &self.input
    }

    fn size_changed(&self) -> bool {
        self.window_size_changed
    }

    fn input_mut(&mut self) -> &mut I {
        &mut self.input
    }

    fn begin_render(&mut self, clear_color: [f32; 4]) {
        let graphics = self.graphics.as_mut().unwrap();
        graphics.begin_render(clear_color);
        let default_viewport = graphics.default_viewport();
        self.set_active_viewport(default_viewport);
    }

    fn end_render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.graphics
            .as_mut()
            .unwrap()
            .end_render(self.debug_logging)?;
        Ok(())
    }

    fn poll_events(&mut self) -> bool {
        self.input.frame_reset();
        self.window_size_changed = false;

        while win32::peek_message(&mut self.msg, None, 0, 0, &[win32::Pm::Remove]) {
            if self.msg.message == win32::WM_QUIT {
                return false;
            }

            win32::translate_message(&self.msg);
            win32::dispatch_message(&self.msg);
        }

        true
    }

    fn set_debug_logging(&mut self, enable: bool) {
        self.debug_logging = enable;
    }

    fn create_viewport(
        &mut self,
        top_left: alexandria_common::Vector2,
        size: alexandria_common::Vector2,
        updater: Option<Box<dyn alexandria_common::ViewportUpdater>>,
    ) -> usize {
        self.graphics
            .as_mut()
            .unwrap()
            .create_viewport(top_left, size, updater)
    }

    fn set_default_viewport(&mut self, viewport: usize) {
        self.graphics
            .as_mut()
            .unwrap()
            .set_default_viewport(viewport);
    }

    fn set_active_viewport(&mut self, viewport: usize) {
        self.graphics
            .as_mut()
            .unwrap()
            .get_viewport(viewport)
            .map(|viewport| viewport.set_active());
    }

    fn update_viewport(
        &mut self,
        viewport: usize,
        top_left: alexandria_common::Vector2,
        size: alexandria_common::Vector2,
    ) {
        self.graphics
            .as_mut()
            .unwrap()
            .get_viewport(viewport)
            .unwrap()
            .update(top_left, size);
    }

    fn remove_viewport(&mut self, viewport: usize) {
        self.graphics.as_mut().unwrap().remove_viewport(viewport);
    }
}

fn parse_vkey(key: &win32::RawKeyboard) -> Option<Key> {
    let key_code = key.make_code() as usize & 0x7F;

    if key_code >= key::CODES.len() {
        None
    } else {
        match key_code {
            0x1C => Some(match key.e0() {
                false => Key::Enter,
                true => Key::NumpadEnter,
            }),
            0x1D => Some(match key.e0() {
                false => match key.e1() {
                    true => Key::Pause,
                    false => Key::LeftControl,
                },
                true => Key::RightControl,
            }),
            0x2A => match key.e0() {
                false => Some(Key::LeftShift),
                true => None,
            },
            0x35 => Some(match key.e0() {
                false => Key::ForwardSlash,
                true => Key::NumpadDivide,
            }),
            0x37 => Some(match key.e0() {
                false => Key::NumpadMultiply,
                true => Key::PrintScreen,
            }),
            0x38 => Some(match key.e0() {
                false => Key::LeftAlt,
                true => Key::RightAlt,
            }),
            0x47 => Some(match key.e0() {
                false => Key::Numpad7,
                true => Key::Home,
            }),
            0x48 => Some(match key.e0() {
                false => Key::Numpad8,
                true => Key::UpArrow,
            }),
            0x49 => Some(match key.e0() {
                false => Key::Numpad9,
                true => Key::PageUp,
            }),
            0x4B => Some(match key.e0() {
                false => Key::Numpad4,
                true => Key::LeftArrow,
            }),
            0x4D => Some(match key.e0() {
                false => Key::Numpad6,
                true => Key::RightArrow,
            }),
            0x4F => Some(match key.e0() {
                false => Key::Numpad1,
                true => Key::End,
            }),
            0x50 => Some(match key.e0() {
                false => Key::Numpad2,
                true => Key::DownArrow,
            }),
            0x51 => Some(match key.e0() {
                false => Key::Numpad3,
                true => Key::PageDown,
            }),
            0x52 => Some(match key.e0() {
                false => Key::Numpad0,
                true => Key::Insert,
            }),
            0x53 => Some(match key.e0() {
                false => Key::NumpadDecimal,
                true => Key::Delete,
            }),
            _ => key::CODES[key_code],
        }
    }
}

mod key {
    use alexandria_common::Key::{self, *};

    pub const CODES: &[Option<Key>] = &[
        None,
        Some(Escape),
        Some(_1),
        Some(_2),
        Some(_3),
        Some(_4),
        Some(_5),
        Some(_6),
        Some(_7),
        Some(_8),
        Some(_9),
        Some(_0),
        Some(Dash),
        Some(Equal),
        Some(Backspace),
        Some(Tab),
        Some(Q),
        Some(W),
        Some(E),
        Some(R),
        Some(T),
        Some(Y),
        Some(U),
        Some(I),
        Some(O),
        Some(P),
        Some(LeftSquareBracket),
        Some(RightSquareBracket),
        Some(Enter),
        Some(LeftControl),
        Some(A),
        Some(S),
        Some(D),
        Some(F),
        Some(G),
        Some(H),
        Some(J),
        Some(K),
        Some(L),
        Some(SemiColon),
        Some(Quote),
        Some(Tilde),
        Some(LeftShift),
        Some(BackSlash),
        Some(Z),
        Some(X),
        Some(C),
        Some(V),
        Some(B),
        Some(N),
        Some(M),
        Some(Comma),
        Some(Period),
        Some(ForwardSlash),
        Some(RightShift),
        Some(NumpadMultiply),
        Some(LeftAlt),
        Some(Space),
        Some(CapsLock),
        Some(F1),
        Some(F2),
        Some(F3),
        Some(F4),
        Some(F5),
        Some(F6),
        Some(F7),
        Some(F8),
        Some(F9),
        Some(F10),
        None, // Num lock removed due to conflict with pause key, Num lock shouldn't really be used by any games anyways
        Some(ScrollLock),
        Some(Numpad7),
        Some(Numpad8),
        Some(Numpad9),
        Some(NumpadSubtract),
        Some(Numpad4),
        Some(Numpad5),
        Some(Numpad6),
        Some(NumpadAdd),
        Some(Numpad1),
        Some(Numpad2),
        Some(Numpad3),
        Some(Numpad0),
        Some(NumpadDecimal),
        None, // 0x54
        None,
        None,
        Some(F11),
        Some(F12),
        None, // 0x59
        None,
        Some(Windows),
        None, // 0x5C
        Some(Menu),
    ];
}
