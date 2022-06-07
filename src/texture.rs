use std::{cell::RefCell, rc::Rc};

use crate::Window;
use alexandria_common::Input;
use ginger::{Image, Pixel};
use win32::{D3D11SubresourceData, DXGIFormat};

pub struct Texture {
    texture: win32::ID3D11Texture2D,
    srv: win32::ID3D11ShaderResourceView,
    uav: win32::ID3D11UnorderedAccessView,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
    slot: usize,
}

impl Texture {
    pub fn new_1f<I: Input>(
        image: &[f32],
        width: usize,
        slot: usize,
        window: &mut Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let initial_data =
            win32::D3D11SubresourceData::new(image, (std::mem::size_of::<f32>() * width) as u32, 0);

        Self::create(
            initial_data,
            width,
            image.len() / width,
            slot,
            DXGIFormat::R32Float,
            window,
        )
    }

    pub fn inner_mut(&mut self) -> &mut win32::ID3D11Texture2D {
        &mut self.texture
    }

    fn create<I: Input>(
        initial_data: D3D11SubresourceData,
        width: usize,
        height: usize,
        slot: usize,
        format: DXGIFormat,
        window: &mut Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let desc = win32::D3D11Texture2DDesc::new(
            width as u32,
            height as u32,
            1,
            1,
            format,
            1,
            0,
            win32::D3D11Usage::Default,
            &[
                win32::D3D11BindFlag::ShaderResource,
                win32::D3D11BindFlag::UnorderedAccess,
            ],
            &[],
            &[],
        );

        let mut texture = window
            .device()
            .create_texture_2d(&desc, Some(&initial_data))?;

        let srv_desc = win32::D3D11ShaderResourceViewDesc::new(format, &mut texture);

        let srv = window
            .device()
            .create_shader_resource_view(&mut texture, &srv_desc)?;

        let uav_desc = win32::D3D11UnorderedAccessViewDesc::new(format, &mut texture);

        let uav = window
            .device()
            .create_unordered_access_view(&mut texture, &uav_desc)?;

        Ok(Texture {
            texture,
            srv,
            uav,
            slot,
            device_context: window.device_context().clone(),
        })
    }
}

impl alexandria_common::Texture for Texture {
    type Window<I: Input> = Box<crate::Window<I>>;

    fn new<I: Input>(
        image: &Image<f32>,
        slot: usize,
        window: &mut Self::Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let initial_data = win32::D3D11SubresourceData::new(
            image.pixels(),
            (std::mem::size_of::<Pixel<f32>>() * image.width()) as u32,
            0,
        );

        Self::create(
            initial_data,
            image.width(),
            image.height(),
            slot,
            DXGIFormat::R32G32B32A32Float,
            window,
        )
    }

    fn set_slot(&mut self, slot: usize) {
        self.slot = slot
    }

    fn set_active(&mut self) {
        let mut device_context = self.device_context.borrow_mut();
        device_context.vs_set_shader_resources(self.slot as u32, &mut [&mut self.srv]);
        device_context.ps_set_shader_resources(self.slot as u32, &mut [&mut self.srv]);
    }

    fn set_active_compute(&mut self) {
        self.device_context
            .borrow_mut()
            .cs_set_shader_resources(self.slot as u32, &mut [Some(&mut self.srv)])
    }

    fn set_active_compute_rw(&mut self) {
        self.device_context
            .borrow_mut()
            .cs_set_unordered_access_views(self.slot as u32, &mut [Some(&mut self.uav)]);
    }
}
