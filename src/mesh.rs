use alexandria_common::Input;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

pub struct Mesh<V> {
    vertex_buffer: win32::ID3D11Buffer,
    index_buffer: win32::ID3D11Buffer,
    index_count: u32,
    _phantom: PhantomData<V>,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
}

pub struct LineMesh<V> {
    vertex_buffer: win32::ID3D11Buffer,
    vertex_count: u32,
    strip: bool,
    _phantom: PhantomData<V>,
    device_context: Rc<RefCell<win32::ID3D11DeviceContext>>,
}

impl<V> Mesh<V> {
    pub fn vertex_buffer(&mut self) -> &mut win32::ID3D11Buffer {
        &mut self.vertex_buffer
    }
}

impl<V> alexandria_common::Mesh<V> for Mesh<V> {
    type Window<I: Input> = Box<crate::Window<I>>;

    fn new<I: Input>(
        vertices: &[V],
        indices: &[u32],
        window: &mut Self::Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let vertex_buffer_desc = win32::D3D11BufferDesc::new(
            (std::mem::size_of::<V>() * vertices.len()) as u32,
            win32::D3D11Usage::Default,
            &[win32::D3D11BindFlag::VertexBuffer],
            &[],
            &[],
            0,
        );

        let device = window.device();

        let vertex_data = win32::D3D11SubresourceData::new(vertices, 0, 0);

        let vertex_buffer = device.create_buffer(&vertex_buffer_desc, Some(&vertex_data))?;

        let index_buffer_desc = win32::D3D11BufferDesc::new(
            (std::mem::size_of::<u32>() * indices.len()) as u32,
            win32::D3D11Usage::Default,
            &[win32::D3D11BindFlag::IndexBuffer],
            &[],
            &[],
            0,
        );

        let index_data = win32::D3D11SubresourceData::new(indices, 0, 0);

        let index_buffer = device.create_buffer(&index_buffer_desc, Some(&index_data))?;

        Ok(Mesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            _phantom: PhantomData,
            device_context: window.device_context().clone(),
        })
    }

    fn update_vertices<I: Input>(
        &mut self,
        vertices: &[V],
        window: &mut Self::Window<I>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vertex_buffer_desc = win32::D3D11BufferDesc::new(
            (std::mem::size_of::<V>() * vertices.len()) as u32,
            win32::D3D11Usage::Default,
            &[win32::D3D11BindFlag::VertexBuffer],
            &[],
            &[],
            0,
        );
        let vertex_data = win32::D3D11SubresourceData::new(vertices, 0, 0);
        self.vertex_buffer = window
            .device()
            .create_buffer(&vertex_buffer_desc, Some(&vertex_data))?;
        Ok(())
    }

    fn update_indices<I: Input>(
        &mut self,
        indices: &[u32],
        window: &mut Self::Window<I>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.index_count = indices.len() as u32;
        let index_buffer_desc = win32::D3D11BufferDesc::new(
            (std::mem::size_of::<u32>() * indices.len()) as u32,
            win32::D3D11Usage::Default,
            &[win32::D3D11BindFlag::IndexBuffer],
            &[],
            &[],
            0,
        );
        let index_data = win32::D3D11SubresourceData::new(indices, 0, 0);
        self.index_buffer = window
            .device()
            .create_buffer(&index_buffer_desc, Some(&index_data))?;
        Ok(())
    }

    fn render(&mut self) {
        let mut device_context = self.device_context.borrow_mut();
        device_context.ia_set_vertex_buffers(
            0,
            &mut [&mut self.vertex_buffer],
            &[std::mem::size_of::<V>() as u32],
            &[0],
        );
        device_context.ia_set_index_buffer(&mut self.index_buffer, win32::DXGIFormat::R32Uint, 0);

        device_context.draw_indexed(self.index_count, 0, 0);
    }
}

impl<V> LineMesh<V> {
    pub fn buffer(&mut self) -> &mut win32::ID3D11Buffer {
        &mut self.vertex_buffer
    }
}

impl<V> alexandria_common::LineMesh<V> for LineMesh<V> {
    type Window<I: Input> = Box<crate::Window<I>>;

    fn new<I: Input>(
        vertices: &[V],
        strip: bool,
        window: &mut Self::Window<I>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let device = window.device();

        let vertex_buffer_desc = win32::D3D11BufferDesc::new(
            (std::mem::size_of::<V>() * vertices.len()) as u32,
            win32::D3D11Usage::Default,
            &[win32::D3D11BindFlag::VertexBuffer],
            &[],
            &[],
            0,
        );

        let vertex_data = win32::D3D11SubresourceData::new(vertices, 0, 0);

        let vertex_buffer = device.create_buffer(&vertex_buffer_desc, Some(&vertex_data))?;

        Ok(LineMesh {
            vertex_buffer,
            vertex_count: vertices.len() as u32,
            strip,
            _phantom: PhantomData,
            device_context: window.device_context().clone(),
        })
    }

    fn render(&mut self) {
        let mut device_context = self.device_context.borrow_mut();
        device_context.ia_set_primitive_topology(if self.strip {
            win32::D3D11PrimitiveTopology::LineStrip
        } else {
            win32::D3D11PrimitiveTopology::LineList
        });
        device_context.ia_set_vertex_buffers(
            0,
            &mut [&mut self.vertex_buffer],
            &[std::mem::size_of::<V>() as u32],
            &[0],
        );
        device_context.draw(self.vertex_count, 0);
        device_context.ia_set_primitive_topology(win32::D3D11PrimitiveTopology::TriangleList);
    }
}
