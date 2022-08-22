#![feature(generic_associated_types)]

mod constant_buffer;
mod graphics;
mod matrix;
mod mesh;
mod shader;
mod texture2d;
mod window;

pub use constant_buffer::*;
pub use matrix::*;
pub use mesh::*;
pub use shader::*;
pub use texture2d::*;
pub use window::*;

fn alexandria_to_dxgi(format: &alexandria_common::Format) -> win32::DXGIFormat {
    match format {
        alexandria_common::Format::R32G32B32A32Float => win32::DXGIFormat::R32G32B32A32Float,
        alexandria_common::Format::R32G32B32Float => win32::DXGIFormat::R32G32A32Float,
        alexandria_common::Format::R32A32Float => win32::DXGIFormat::R32A32Float,
    }
}
