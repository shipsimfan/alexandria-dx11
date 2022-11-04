use alexandria_common::{Vector2, ViewportUpdater};
use std::{cell::RefCell, rc::Rc};

pub struct Viewport {
    viewport: win32::D3D11Viewport,
    updater: Option<Box<dyn ViewportUpdater>>,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
    key: usize,
}

impl Viewport {
    pub(crate) fn new(
        top_left: Vector2,
        size: Vector2,
        updater: Option<Box<dyn ViewportUpdater>>,
        device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
        key: usize,
    ) -> Self {
        Viewport {
            viewport: win32::D3D11Viewport::new(
                top_left.x(),
                top_left.y(),
                size.x(),
                size.y(),
                0.0,
                1.0,
            ),
            updater,
            device_context,
            key,
        }
    }

    pub(crate) fn updater(&mut self) -> Option<&mut Box<dyn ViewportUpdater>> {
        self.updater.as_mut()
    }

    pub(crate) fn key(&self) -> usize {
        self.key
    }
}

impl alexandria_common::Viewport for Viewport {
    fn set_active(&mut self) {
        self.device_context
            .borrow_mut()
            .rs_set_viewports(&[&self.viewport])
    }

    fn update(&mut self, top_left: alexandria_common::Vector2, size: alexandria_common::Vector2) {
        self.viewport =
            win32::D3D11Viewport::new(top_left.x(), top_left.y(), size.x(), size.y(), 0.0, 1.0);
    }
}
