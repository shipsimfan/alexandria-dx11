use crate::{graphics::Graphics, Viewport};
use alexandria_common::{Input, Vector2, Viewport as CommonViewport};
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
            win32::WM_KEYDOWN => self.input.key_down(w_param as u8),
            win32::WM_KEYUP => self.input.key_up(w_param as u8),
            win32::WM_LBUTTONDOWN => self.input.mouse_down(0),
            win32::WM_LBUTTONUP => self.input.mouse_up(0),
            win32::WM_RBUTTONDOWN => self.input.mouse_down(1),
            win32::WM_RBUTTONUP => self.input.mouse_up(1),
            win32::WM_MBUTTONDOWN => self.input.mouse_down(2),
            win32::WM_MBUTTONUP => self.input.mouse_up(2),
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
