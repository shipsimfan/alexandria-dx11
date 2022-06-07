use alexandria_common::Input;
use std::{cell::RefCell, marker::PhantomData, mem::size_of, rc::Rc};

pub struct ConstantBuffer<T: Sized> {
    constant_buffer: win32::ID3D11Buffer,
    slot: usize,
    phantom: PhantomData<T>,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
}

impl<T: Sized> alexandria_common::ConstantBuffer<T> for ConstantBuffer<T> {
    type Window<I: Input> = Box<crate::Window<I>>;

    fn new<I: Input>(
        initial_data: T,
        slot: usize,
        window: &mut Self::Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let buffer_desc = win32::D3D11BufferDesc::new(
            size_of::<T>() as u32,
            win32::D3D11Usage::Dynamic,
            &[win32::D3D11BindFlag::ConstantBuffer],
            &[win32::D3D11CPUAccessFlag::Write],
            &[],
            0,
        );

        let initial_data = {
            let arr = [initial_data];
            Some(win32::D3D11SubresourceData::new(&arr, 0, 0))
        };

        let buffer = window
            .device()
            .create_buffer(&buffer_desc, initial_data.as_ref())?;

        Ok(ConstantBuffer {
            constant_buffer: buffer,
            slot,
            phantom: PhantomData,
            device_context: window.device_context().clone(),
        })
    }

    fn set_data(&mut self, new_data: T) -> Result<(), Box<dyn std::error::Error>> {
        let mut device_context = self.device_context.borrow_mut();

        let mut mapped_resource = device_context.map(
            &mut self.constant_buffer,
            0,
            win32::D3D11Map::WriteDiscard,
            &[],
        )?;

        let data = mapped_resource.as_ref::<T>();
        *data = new_data;

        Ok(())
    }

    fn set_slot(&mut self, slot: usize) {
        self.slot = slot;
    }

    fn set_active(&mut self) {
        let mut device_context = self.device_context.borrow_mut();
        device_context.vs_set_constant_buffers(self.slot as u32, &mut [&mut self.constant_buffer]);
        device_context.ps_set_constant_buffers(self.slot as u32, &mut [&mut self.constant_buffer]);
    }

    fn set_active_compute(&mut self) {
        self.device_context
            .borrow_mut()
            .cs_set_constant_buffers(self.slot as u32, &mut [&mut self.constant_buffer]);
    }
}
