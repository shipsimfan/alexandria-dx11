use alexandria_common::{Input, SampleType, TextureFormat, TextureFormatClass};
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
use win32::DXGIFormat;

pub struct Texture2D<F: TextureFormat> {
    texture: win32::ID3D11Texture2D,
    sampler: win32::ID3D11SamplerState,
    srv: win32::ID3D11ShaderResourceView,
    _uav: win32::ID3D11UnorderedAccessView,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
    slot: usize,

    phantom: PhantomData<F>,
}

const fn class_to_format(texture_format_class: TextureFormatClass) -> DXGIFormat {
    match texture_format_class {
        TextureFormatClass::Unsigned8_1 => DXGIFormat::R8Uint,
        TextureFormatClass::Unsigned8_4 => DXGIFormat::R8G8B8A8Uint,
        TextureFormatClass::Unsigned16_1 => DXGIFormat::R16Uint,
        TextureFormatClass::Unsigned32_1 => DXGIFormat::R32Uint,
        TextureFormatClass::Signed8_1 => DXGIFormat::R8Sint,
        TextureFormatClass::Signed16_1 => DXGIFormat::R16Sint,
        TextureFormatClass::Signed32_1 => DXGIFormat::R32Sint,
        TextureFormatClass::Float32_1 => DXGIFormat::R32Float,
        TextureFormatClass::Float32_4 => DXGIFormat::R32G32B32A32Float,
    }
}

impl<F: TextureFormat> alexandria_common::Texture2D<F> for Texture2D<F> {
    type Window<I: Input> = Box<crate::Window<I>>;

    fn new<I: Input>(
        image: &[F],
        width: usize,
        height: usize,
        slot: usize,
        sample_type: SampleType,
        window: &mut Self::Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let initial_data =
            win32::D3D11SubresourceData::new(image, (std::mem::size_of::<F>() * width) as u32, 0);

        let format = class_to_format(F::CLASS);
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
            texture,
            sampler,
            srv,
            _uav,
            slot,
            device_context: window.device_context().clone(),
            phantom: PhantomData,
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

    fn update_region(&mut self, region: alexandria_common::UpdateRegion, data: &[F]) {
        self.device_context.borrow_mut().update_subresource(
            &mut self.texture,
            0,
            Some(&win32::D3D11Box {
                left: region.left() as u32,
                right: (region.left() + region.width()) as u32,
                top: region.top() as u32,
                bottom: (region.top() + region.height()) as u32,
                front: 0,
                back: 1,
            }),
            data,
            (std::mem::size_of::<F>() * region.width()) as u32,
            0,
        )
    }
}
