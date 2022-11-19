use crate::{graphics::Graphics, Viewport};
use alexandria_common::{Input, Key, MouseButton, Vector2, Viewport as CommonViewport};
use std::{cell::RefCell, ffi::CString, ptr::null, rc::Rc};

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
            win32::WM_KEYDOWN => {
                parse_key(w_param as u8).map(|key| self.input.key_down(key));
            }
            win32::WM_KEYUP => {
                parse_key(w_param as u8).map(|key| self.input.key_up(key));
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

fn parse_key(key: u8) -> Option<Key> {
    match key {
        x if x == Key::Backspace as u8 => Some(Key::Backspace),
        x if x == Key::Tab as u8 => Some(Key::Tab),
        x if x == Key::Enter as u8 => Some(Key::Enter),
        x if x == Key::Shift as u8 => Some(Key::Shift),
        x if x == Key::Control as u8 => Some(Key::Control),
        x if x == Key::Alt as u8 => Some(Key::Alt),
        x if x == Key::Pause as u8 => Some(Key::Pause),
        x if x == Key::CapsLock as u8 => Some(Key::CapsLock),
        x if x == Key::Escape as u8 => Some(Key::Escape),
        x if x == Key::Space as u8 => Some(Key::Space),
        x if x == Key::PageUp as u8 => Some(Key::PageUp),
        x if x == Key::PageDown as u8 => Some(Key::PageDown),
        x if x == Key::End as u8 => Some(Key::End),
        x if x == Key::Home as u8 => Some(Key::Home),
        x if x == Key::LeftArrow as u8 => Some(Key::LeftArrow),
        x if x == Key::UpArrow as u8 => Some(Key::UpArrow),
        x if x == Key::RightArrow as u8 => Some(Key::RightArrow),
        x if x == Key::DownArrow as u8 => Some(Key::DownArrow),
        x if x == Key::PrintScreen as u8 => Some(Key::PrintScreen),
        x if x == Key::Insert as u8 => Some(Key::Insert),
        x if x == Key::Delete as u8 => Some(Key::Delete),
        x if x == Key::_0 as u8 => Some(Key::_0),
        x if x == Key::_1 as u8 => Some(Key::_1),
        x if x == Key::_2 as u8 => Some(Key::_2),
        x if x == Key::_3 as u8 => Some(Key::_3),
        x if x == Key::_4 as u8 => Some(Key::_4),
        x if x == Key::_5 as u8 => Some(Key::_5),
        x if x == Key::_6 as u8 => Some(Key::_6),
        x if x == Key::_7 as u8 => Some(Key::_7),
        x if x == Key::_8 as u8 => Some(Key::_8),
        x if x == Key::_9 as u8 => Some(Key::_9),
        x if x == Key::A as u8 => Some(Key::A),
        x if x == Key::B as u8 => Some(Key::B),
        x if x == Key::C as u8 => Some(Key::C),
        x if x == Key::D as u8 => Some(Key::D),
        x if x == Key::E as u8 => Some(Key::E),
        x if x == Key::F as u8 => Some(Key::F),
        x if x == Key::G as u8 => Some(Key::G),
        x if x == Key::H as u8 => Some(Key::H),
        x if x == Key::I as u8 => Some(Key::I),
        x if x == Key::J as u8 => Some(Key::J),
        x if x == Key::K as u8 => Some(Key::K),
        x if x == Key::L as u8 => Some(Key::L),
        x if x == Key::M as u8 => Some(Key::M),
        x if x == Key::N as u8 => Some(Key::N),
        x if x == Key::O as u8 => Some(Key::O),
        x if x == Key::P as u8 => Some(Key::P),
        x if x == Key::Q as u8 => Some(Key::Q),
        x if x == Key::R as u8 => Some(Key::R),
        x if x == Key::S as u8 => Some(Key::S),
        x if x == Key::T as u8 => Some(Key::T),
        x if x == Key::U as u8 => Some(Key::U),
        x if x == Key::V as u8 => Some(Key::V),
        x if x == Key::W as u8 => Some(Key::W),
        x if x == Key::X as u8 => Some(Key::X),
        x if x == Key::Y as u8 => Some(Key::Y),
        x if x == Key::Z as u8 => Some(Key::Z),
        x if x == Key::Windows as u8 => Some(Key::Windows),
        x if x == Key::Numpad0 as u8 => Some(Key::Numpad0),
        x if x == Key::Numpad1 as u8 => Some(Key::Numpad1),
        x if x == Key::Numpad2 as u8 => Some(Key::Numpad2),
        x if x == Key::Numpad3 as u8 => Some(Key::Numpad3),
        x if x == Key::Numpad4 as u8 => Some(Key::Numpad4),
        x if x == Key::Numpad5 as u8 => Some(Key::Numpad5),
        x if x == Key::Numpad6 as u8 => Some(Key::Numpad6),
        x if x == Key::Numpad7 as u8 => Some(Key::Numpad7),
        x if x == Key::Numpad8 as u8 => Some(Key::Numpad8),
        x if x == Key::Numpad9 as u8 => Some(Key::Numpad9),
        x if x == Key::Multiply as u8 => Some(Key::Multiply),
        x if x == Key::Add as u8 => Some(Key::Add),
        x if x == Key::Seperator as u8 => Some(Key::Seperator),
        x if x == Key::Subtract as u8 => Some(Key::Subtract),
        x if x == Key::Decimal as u8 => Some(Key::Decimal),
        x if x == Key::Divide as u8 => Some(Key::Divide),
        x if x == Key::F1 as u8 => Some(Key::F1),
        x if x == Key::F2 as u8 => Some(Key::F2),
        x if x == Key::F3 as u8 => Some(Key::F3),
        x if x == Key::F4 as u8 => Some(Key::F4),
        x if x == Key::F5 as u8 => Some(Key::F5),
        x if x == Key::F6 as u8 => Some(Key::F6),
        x if x == Key::F7 as u8 => Some(Key::F7),
        x if x == Key::F8 as u8 => Some(Key::F8),
        x if x == Key::F9 as u8 => Some(Key::F9),
        x if x == Key::F10 as u8 => Some(Key::F10),
        x if x == Key::F11 as u8 => Some(Key::F11),
        x if x == Key::F12 as u8 => Some(Key::F12),
        x if x == Key::F13 as u8 => Some(Key::F13),
        x if x == Key::F14 as u8 => Some(Key::F14),
        x if x == Key::F15 as u8 => Some(Key::F15),
        x if x == Key::F16 as u8 => Some(Key::F16),
        x if x == Key::F17 as u8 => Some(Key::F17),
        x if x == Key::F18 as u8 => Some(Key::F18),
        x if x == Key::F19 as u8 => Some(Key::F19),
        x if x == Key::F20 as u8 => Some(Key::F20),
        x if x == Key::F21 as u8 => Some(Key::F21),
        x if x == Key::F22 as u8 => Some(Key::F22),
        x if x == Key::F23 as u8 => Some(Key::F23),
        x if x == Key::F24 as u8 => Some(Key::F24),
        x if x == Key::NumLock as u8 => Some(Key::NumLock),
        x if x == Key::ScrollLock as u8 => Some(Key::ScrollLock),
        _ => None,
    }
}
