use alexandria_common::{Input, SampleType};
use ginger::{Image, Pixel};
use std::{cell::RefCell, rc::Rc};
use win32::DXGIFormat;

pub struct Texture2D {
    _texture: win32::ID3D11Texture2D,
    sampler: win32::ID3D11SamplerState,
    srv: win32::ID3D11ShaderResourceView,
    _uav: win32::ID3D11UnorderedAccessView,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
    slot: usize,
}

impl alexandria_common::Texture2D for Texture2D {
    type Window<I: Input> = Box<crate::Window<I>>;

    fn new<I: Input>(
        image: &Image<f32>,
        slot: usize,
        sample_type: SampleType,
        window: &mut Self::Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let initial_data = win32::D3D11SubresourceData::new(
            image.pixels(),
            (std::mem::size_of::<Pixel<f32>>() * image.width()) as u32,
            0,
        );

        let desc = win32::D3D11Texture2DDesc::new(
            image.width() as u32,
            image.height() as u32,
            1,
            1,
            DXGIFormat::R32G32B32A32Float,
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

        let srv_desc =
            win32::D3D11ShaderResourceViewDesc::new(DXGIFormat::R32G32B32A32Float, &mut texture);

        let srv = window
            .device()
            .create_shader_resource_view(&mut texture, &srv_desc)?;

        let uav_desc =
            win32::D3D11UnorderedAccessViewDesc::new(DXGIFormat::R32G32B32A32Float, &mut texture);

        let _uav = window
            .device()
            .create_unordered_access_view(&mut texture, &uav_desc)?;

        let mut sampler_desc = win32::D3D11SamplerDesc::default();
        sampler_desc.set_filter(match sample_type {
            SampleType::Point => win32::D3D11Filter::MinMagMipPoint,
            SampleType::Linear => win32::D3D11Filter::Anisotropic,
        });
        let sampler = window.device().create_sampler_state(&sampler_desc)?;

        Ok(Texture2D {
            _texture: texture,
            sampler,
            srv,
            _uav,
            slot,
            device_context: window.device_context().clone(),
        })
    }

    fn set_slot(&mut self, slot: usize) {
        self.slot = slot
    }

    fn set_active(&mut self) {
        let mut device_context = self.device_context.borrow_mut();
        device_context.vs_set_shader_resources(self.slot as u32, &mut [Some(&mut self.srv)]);
        device_context.ps_set_shader_resources(self.slot as u32, &mut [Some(&mut self.srv)]);
        device_context.vs_set_samplers(self.slot as u32, &mut [Some(&mut self.sampler)]);
        device_context.ps_set_samplers(self.slot as u32, &mut [Some(&mut self.sampler)]);
    }

    fn clear_active(&mut self) {
        let mut device_context = self.device_context.borrow_mut();
        device_context.vs_set_shader_resources(self.slot as u32, &mut [None]);
        device_context.ps_set_shader_resources(self.slot as u32, &mut [None]);
        device_context.vs_set_samplers(self.slot as u32, &mut [None]);
        device_context.ps_set_samplers(self.slot as u32, &mut [None]);
    }

    /*
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
    */
}
