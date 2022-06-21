use std::{cell::RefCell, ptr::null, rc::Rc};

#[derive(Debug)]
pub enum GraphicsCreationErrorClass {
    DXGIFactoryCreation,
    PrimaryAdapter,
    PrimaryOutput,
    DisplayModes,
    DeviceAndSwapChain,
    BackBuffer,
    RenderTargetView,
    DepthStencilBuffer,
    DepthStencilState,
    DepthStencilView,
    Rasterizer,
    BlendState,
    InfoQueue,
}

#[allow(unused)]
pub struct Graphics {
    swap_chain: win32::IDXGISwapChain,
    device: Rc<win32::ID3D11Device>,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
    render_target_view: win32::ID3D11RenderTargetView,
    depth_stencil_buffer: win32::ID3D11Texture2D,
    depth_stencil_state: win32::ID3D11DepthStencilState,
    depth_stencil_view: win32::ID3D11DepthStencilView,
    rasterizer_state: win32::ID3D11RasterizerState,
    blend_state: win32::ID3D11BlendState,
    rendering: bool,
    viewport: win32::D3D11Viewport,

    #[cfg(debug_assertions)]
    info_queue: win32::ID3D11InfoQueue,
}

#[derive(Debug)]
pub struct GraphicsCreationError {
    class: GraphicsCreationErrorClass,
    error: win32::DirectXError,
}

#[derive(Debug)]
pub struct RenderError(win32::DirectXError);

fn get_refresh_rate(width: u32, height: u32) -> Result<(u32, u32), GraphicsCreationError> {
    // Create a factory
    let mut factory = match win32::IDXGIFactory::new() {
        Ok(factory) => factory,
        Err(error) => {
            return Err(GraphicsCreationError::new(
                GraphicsCreationErrorClass::DXGIFactoryCreation,
                error,
            ))
        }
    };

    // Get primary adapter
    let mut adapter = match factory.enum_adapters(0) {
        Ok(adapter) => adapter,
        Err(error) => {
            return Err(GraphicsCreationError::new(
                GraphicsCreationErrorClass::PrimaryAdapter,
                error,
            ))
        }
    };

    // Get primary output
    let mut output = match adapter.enum_outputs(0) {
        Ok(output) => output,
        Err(error) => {
            return Err(GraphicsCreationError::new(
                GraphicsCreationErrorClass::PrimaryOutput,
                error,
            ))
        }
    };

    // Get number of modes
    let display_mode_list = match output.get_display_mode_list(
        win32::DXGIFormat::R8G8B8A8Unorm,
        &[win32::DXGIEnumModes::Interlaced],
    ) {
        Ok(list) => list,
        Err(error) => {
            return Err(GraphicsCreationError::new(
                GraphicsCreationErrorClass::DisplayModes,
                error,
            ))
        }
    };

    // Get the refresh for the monitor
    let mut numerator = 0;
    let mut denominator = 0;
    for mode in display_mode_list {
        if mode.width() == width && mode.height() == height {
            (numerator, denominator) = mode.refresh_rate();
        }
    }

    Ok((numerator, denominator))
}

impl Graphics {
    pub fn new(
        handle: win32::HWnd,
        width: u32,
        height: u32,
    ) -> Result<Self, GraphicsCreationError> {
        // Get the refresh rate
        let (numerator, denominator) = get_refresh_rate(width, height)?;

        // Create device and swap chain
        let swap_chain_desc = win32::DXGISwapChainDesc::new(
            3,
            width,
            height,
            win32::DXGIFormat::R8G8B8A8Unorm,
            numerator,
            denominator,
            &[win32::DXGIUsage::RenderTargetOutput],
            handle,
            1,
            0,
            true,
            win32::DXGIModeScanlineOrder::Unspecified,
            win32::DXGIModeScaling::Unspecified,
            win32::DXGISwapEffect::FlipDiscard,
            &[],
        );

        #[cfg(debug_assertions)]
        let flags = &[win32::D3D11CreateDeviceFlag::Debug];
        #[cfg(not(debug_assertions))]
        let flags = &[];

        let (mut swap_chain, device, mut device_context) =
            match win32::d3d11_create_device_and_swap_chain(
                None,
                win32::D3DDriverType::Hardware,
                null(),
                flags,
                &[
                    win32::D3DFeatureLevel::Level11_0,
                    win32::D3DFeatureLevel::Level11_1,
                ],
                &swap_chain_desc,
            ) {
                Ok(ret) => ret,
                Err(error) => {
                    return Err(GraphicsCreationError::new(
                        GraphicsCreationErrorClass::DeviceAndSwapChain,
                        error,
                    ))
                }
            };

        // Create render target view
        let mut back_buffer = match swap_chain.get_buffer(0) {
            Ok(buffer) => buffer,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::BackBuffer,
                    error,
                ))
            }
        };
        let render_target_view = match device.create_render_target_view(&mut back_buffer, None) {
            Ok(render_target_view) => render_target_view,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::RenderTargetView,
                    error,
                ))
            }
        };

        // Create a depth buffer
        let depth_buffer_desc = win32::D3D11Texture2DDesc::new(
            width,
            height,
            1,
            1,
            win32::DXGIFormat::D24UnormS8Uint,
            1,
            0,
            win32::D3D11Usage::Default,
            &[win32::D3D11BindFlag::DepthStencil],
            &[],
            &[],
        );
        let mut depth_stencil_buffer = match device.create_texture_2d(&depth_buffer_desc, None) {
            Ok(texture) => texture,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::DepthStencilBuffer,
                    error,
                ))
            }
        };

        // Create a depth stencil
        let depth_stencil_desc = win32::D3D11DepthStencilDesc::new(
            true,
            win32::D3D11DepthWriteMask::All,
            win32::D3D11ComparisonFunc::Less,
            true,
            0xFF,
            0xFF,
            win32::D3D11StencilOp::Keep,
            win32::D3D11StencilOp::Incr,
            win32::D3D11StencilOp::Keep,
            win32::D3D11ComparisonFunc::Always,
            win32::D3D11StencilOp::Keep,
            win32::D3D11StencilOp::Decr,
            win32::D3D11StencilOp::Keep,
            win32::D3D11ComparisonFunc::Always,
        );
        let mut depth_stencil_state = match device.create_depth_stencil_state(&depth_stencil_desc) {
            Ok(depth_stencil_state) => depth_stencil_state,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::DepthStencilState,
                    error,
                ))
            }
        };

        // Set depth stencil state
        device_context.om_set_depth_stencil_state(&mut depth_stencil_state, 1);

        // Create depth stencil view
        let depth_stencil_view_desc = win32::D3D11DepthStencilViewDesc::new(
            win32::DXGIFormat::D24UnormS8Uint,
            win32::D3D11DSVDimension::Texture2D,
            &[],
        );
        let depth_stencil_view = match device
            .create_depth_stencil_view(&mut depth_stencil_buffer, &depth_stencil_view_desc)
        {
            Ok(depth_stencil_view) => depth_stencil_view,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::DepthStencilView,
                    error,
                ))
            }
        };

        // Create rasterizer
        let raster_desc = win32::D3D11RasterizerDesc::new(
            win32::D3D11FillMode::Solid,
            win32::D3D11CullMode::Back,
            false,
            0,
            0.0,
            0.0,
            true,
            false,
            false,
            false,
        );
        let rasterizer_state = match device.create_rasterizer_state(&raster_desc) {
            Ok(rasterizer_state) => rasterizer_state,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::Rasterizer,
                    error,
                ))
            }
        };

        // Set the viewport
        let viewport = win32::D3D11Viewport::new(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
        device_context.rs_set_viewports(&[&viewport]);

        // Create the blend state
        let blend_desc = win32::D3D11BlendDesc::new(
            false,
            false,
            &[win32::D3D11RenderTargetBlendDesc::new(
                true,
                win32::D3D11Blend::SrcAlpha,
                win32::D3D11Blend::InvSrcAlpha,
                win32::D3D11BlendOp::Add,
                win32::D3D11Blend::One,
                win32::D3D11Blend::Zero,
                win32::D3D11BlendOp::Add,
                win32::D3D11ColorWriteEnable::All,
            )],
        );

        #[cfg(debug_assertions)]
        let info_queue = match device.query_interface() {
            Ok(info_queue) => info_queue,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::InfoQueue,
                    error,
                ))
            }
        };

        let blend_state = match device.create_blend_state(&blend_desc) {
            Ok(blend_state) => blend_state,
            Err(error) => {
                return Err(GraphicsCreationError::new(
                    GraphicsCreationErrorClass::BlendState,
                    error,
                ))
            }
        };

        Ok(Graphics {
            swap_chain,
            device: Rc::new(device),
            device_context: Rc::new(RefCell::new(device_context)),
            render_target_view,
            depth_stencil_buffer,
            depth_stencil_state,
            depth_stencil_view,
            rasterizer_state,
            blend_state,
            rendering: false,
            viewport,
            #[cfg(debug_assertions)]
            info_queue,
        })
    }

    pub fn begin_render(&mut self, clear_color: [f32; 4]) {
        self.rendering = true;

        let mut device_context = self.device_context.borrow_mut();

        device_context.clear_render_target_view(&mut self.render_target_view, clear_color);
        device_context.clear_depth_stencil_view(
            &mut self.depth_stencil_view,
            &[win32::D3D11ClearFlag::Depth],
            1.0,
            0,
        );
        device_context.om_set_render_targets(
            &mut [&mut self.render_target_view],
            Some(&mut self.depth_stencil_view),
        );
        device_context.ia_set_primitive_topology(win32::D3D11PrimitiveTopology::TriangleList);
        device_context.om_set_blend_state(&mut self.blend_state, [1.0, 1.0, 1.0, 1.0], u32::MAX);
        device_context.rs_set_viewports(&[&self.viewport]);
    }

    pub fn end_render(&mut self) -> Result<(), RenderError> {
        if self.rendering {
            self.swap_chain.present(1, 0)?;
            self.rendering = false;
        }

        #[cfg(debug_assertions)]
        {
            let num_messages = self.info_queue.get_num_stored_messages();
            for i in 0..num_messages {
                let message = self.info_queue.get_message(i)?;
                println!("DirectX11: {}", message.description());
            }
            self.info_queue.clear_stored_messages();
        }

        Ok(())
    }

    pub fn device(&self) -> &Rc<win32::ID3D11Device> {
        &self.device
    }

    pub fn device_context(&self) -> &Rc<RefCell<win32::ID3D11DeviceContext>> {
        &self.device_context
    }
}

impl GraphicsCreationError {
    pub fn new(class: GraphicsCreationErrorClass, error: win32::DirectXError) -> Self {
        GraphicsCreationError { class, error }
    }
}

impl std::error::Error for GraphicsCreationError {}

impl std::fmt::Display for GraphicsCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.class, self.error)
    }
}

impl std::fmt::Display for GraphicsCreationErrorClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GraphicsCreationErrorClass::DXGIFactoryCreation => "Unable to create DXGI factory",
                GraphicsCreationErrorClass::PrimaryAdapter => "Unable to get primary adapter",
                GraphicsCreationErrorClass::PrimaryOutput => "Unable to get primary output",
                GraphicsCreationErrorClass::DisplayModes => "Unable to get list of display modes",
                GraphicsCreationErrorClass::DeviceAndSwapChain =>
                    "Unable to create device and swap chain",
                GraphicsCreationErrorClass::BackBuffer => "Unable to get back buffer",
                GraphicsCreationErrorClass::RenderTargetView =>
                    "Unable to create render target view",
                GraphicsCreationErrorClass::DepthStencilBuffer =>
                    "Unable to create depth stencil buffer",
                GraphicsCreationErrorClass::DepthStencilState =>
                    "Unable to create depth stencil state",
                GraphicsCreationErrorClass::DepthStencilView =>
                    "Unable to create depth stencil view",
                GraphicsCreationErrorClass::Rasterizer => "Unable to create rasterizer",
                GraphicsCreationErrorClass::BlendState => "Unable to create blend state",
                GraphicsCreationErrorClass::InfoQueue => "Unable to create info queue",
            }
        )
    }
}

impl std::error::Error for RenderError {}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to render  ({})", self.0)
    }
}

impl From<win32::DirectXError> for RenderError {
    fn from(error: win32::DirectXError) -> Self {
        RenderError(error)
    }
}
