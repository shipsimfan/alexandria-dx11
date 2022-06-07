use alexandria_common::{Format, Input};
use std::{cell::RefCell, ffi::CString, rc::Rc};
use win32::{DirectXError, ID3DBlob};

pub struct Shader {
    vertex_shader: win32::ID3D11VertexShader,
    pixel_shader: win32::ID3D11PixelShader,
    input_layout: win32::ID3D11InputLayout,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
}

pub struct ShaderCreationError {
    error: DirectXError,
    blob: Option<ID3DBlob>,
}

impl alexandria_common::Shader for Shader {
    type Window<I: Input> = Box<crate::Window<I>>;

    fn new<S: AsRef<str>, I: Input>(
        code: S,
        vertex_layout: &[(&str, Format)],
        window: &mut Self::Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let device = window.device();

        let shader_code = CString::new(code.as_ref()).unwrap();
        let (vertex_shader_blob, errors) = win32::d3d_compile(
            &shader_code,
            None,
            &[],
            Some(&CString::new("vertex_main").unwrap()),
            &CString::new("vs_5_0").unwrap(),
            &[win32::D3DCompileFlag::EnableStrictness],
            &[],
        );

        let (vertex_shader, vertex_shader_blob) = match vertex_shader_blob {
            Ok(blob) => (device.create_vertex_shader(&blob)?, blob),
            Err(error) => {
                return Err(Box::new(ShaderCreationError {
                    error,
                    blob: errors,
                }))
            }
        };

        let (pixel_shader_blob, errors) = win32::d3d_compile(
            &shader_code,
            None,
            &[],
            Some(&CString::new("pixel_main").unwrap()),
            &CString::new("ps_5_0").unwrap(),
            &[win32::D3DCompileFlag::EnableStrictness],
            &[],
        );

        let pixel_shader = match pixel_shader_blob {
            Ok(blob) => device.create_pixel_shader(&blob)?,
            Err(error) => {
                return Err(Box::new(ShaderCreationError {
                    error,
                    blob: errors,
                }))
            }
        };

        let mut input_layout_desc = Vec::with_capacity(vertex_layout.len());
        let mut names = Vec::with_capacity(vertex_layout.len());
        for (name, format) in vertex_layout {
            let i = names.len();
            names.push(CString::new(*name).unwrap());

            input_layout_desc.push(win32::D3D11InputElementDesc::new(
                &names[i],
                0,
                crate::alexandria_to_dxgi(format),
                0,
                None,
                win32::D3D11InputClassification::PerVertexData,
                0,
            ))
        }

        let input_layout =
            device.create_input_layout(input_layout_desc.as_slice(), &vertex_shader_blob)?;

        Ok(Shader {
            vertex_shader,
            pixel_shader,
            input_layout,
            device_context: window.device_context().clone(),
        })
    }

    fn set_active(&mut self) {
        let mut device_context = self.device_context.borrow_mut();
        device_context.ia_set_input_layout(&mut self.input_layout);
        device_context.vs_set_shader(&mut self.vertex_shader);
        device_context.ps_set_shader(&mut self.pixel_shader);
    }
}

impl ShaderCreationError {
    pub fn new(error: DirectXError, blob: Option<ID3DBlob>) -> Self {
        ShaderCreationError { error, blob }
    }
}

impl std::error::Error for ShaderCreationError {}

impl std::fmt::Display for ShaderCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self.blob {
                Some(errors) => format!("{}", errors),
                None => format!("{}", self.error),
            }
        )
    }
}

impl std::fmt::Debug for ShaderCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<win32::DirectXError> for ShaderCreationError {
    fn from(error: win32::DirectXError) -> Self {
        ShaderCreationError { error, blob: None }
    }
}
