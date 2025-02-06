// proc_windows contains support functions from winit 0.30.8 and wgpu 23.0.1 (both valid at Jan 1/25)
// for creating a window and asynchronously handling its I/O events
// The basic code comes largely from this GitHub issues page: https://github.com/sotrh/learn-wgpu/issues/549
// and the wgpu tutorial (https://sotrh.github.io/learn-wgpu/#what-is-wgpu) and on YouTube (https://www.youtube.com/watch?v=I8Uf7akOYo0&t=4s)
// also contains the pipelinebuilder and related functions
// 
use std::sync::Arc;

use rand::seq::index;
use wgpu::{ naga::common::wgsl, util::DeviceExt, Adapter, Device, Instance, PresentMode, Queue, Surface, SurfaceCapabilities};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{EventLoop,ActiveEventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use pollster::{block_on, FutureExt};

use arrayfire as af;
use af::*;


pub async fn run() {
    let event_loop = EventLoop::new().unwrap();
    let mut window_state = WindowStateApplication::new();
    let _ = event_loop.run_app(&mut window_state);
}

struct WindowState {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: Arc<Window>,
    render_pipeline: wgpu::RenderPipeline,
    
    // Below will change depending on what objects are being displayed
    // May want to remove this to a separate struct
    //triangle_mesh: wgpu::Buffer,
    quad_mesh: Mesh,
    transform_data: TransformData, // For TransformData, buffer, and bind group (see below)
}

struct TransformData {
    transform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Transform {
    position: [f32; 4],
    scale: f32,
    color: [f32; 4], 
}

/*#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Transform {
    matrix: [[f32; 4]; 4],
    color: [f32; 4],
}*/

struct WindowStateApplication {
    window_state: Option<WindowState>,
}

impl WindowStateApplication {
    pub fn new() -> Self {
        Self { window_state: None }
    }
}

impl ApplicationHandler for WindowStateApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes()
            .with_inner_size(PhysicalSize::new(1920, 1080))
            .with_title("Hello!"))
            .unwrap();
        self.window_state = Some(WindowState::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = self.window_state.as_ref().unwrap().window();

        if window.id() == window_id {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    self.window_state.as_mut().unwrap().resize(physical_size);
                }
                WindowEvent::RedrawRequested => {
                    self.window_state.as_mut().unwrap().render().unwrap();
                }
                WindowEvent::KeyboardInput {event,..} => {
                    if event.state.is_pressed() { 
                        match event.logical_key  {
                            Key::Named(NamedKey::ArrowUp) => {
                                println!("ArrowUp");
                            }
                            Key::Named(NamedKey::ArrowDown) => {
                                println!("ArrowDown");
                            }
                            Key::Named(NamedKey::ArrowLeft) => {
                                println!("ArrowLeft");
                            }
                            Key::Named(NamedKey::ArrowRight) => {
                                println!("ArrowRight");
                            }
                            _ => {}
                        }
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let window = self.window_state.as_ref().unwrap().window();
        window.request_redraw();
    }

}


impl WindowState {
    pub fn new(window: Window) -> WindowState {
        let instance = Self::create_gpu_instance();
        let window_arc = Arc::new(window);
        let size = window_arc.inner_size();
        let surface = instance.create_surface(window_arc.clone()).unwrap();
        let adapter = Self::create_adapter(instance, &surface);
        let surface_caps = surface.get_capabilities(&adapter);
        let config = Self::create_surface_config(size, surface_caps);
        let (device, queue) = Self::create_device(&adapter);

        surface.configure(&device, &config);
        //let triangle_mesh = make_triangle(&device);

        // TODO make multiple quads, each with its own position, enough to fill the window based on size of quad
        let quad_mesh = make_quad(&device, DFLT_QUAD_SIZE);

        let render_pipeline = PipelineBuilder::new_with_parms(
                                                        "shaders.wgsl",
                                                        "vs_main", 
                                                        "fs_main", 
                                                        wgpu::TextureFormat::Bgra8UnormSrgb,
                                                        vec![Vertex::layout()],
                                                )
                                            //.finalize()
                                            .build_render_pipeline(&device);    
        // Compute size of transform_buffer
        // figure out number of transforms required
        let quad_size = DFLT_QUAD_SIZE;
        println!("Window width: {}, height: {}", size.width, size.height);
        println!("Quad size: ({}, {}) with between spacing of ({}, {})", quad_size.0, quad_size.1, DFLT_QUAD_SPACING.0, DFLT_QUAD_SPACING.1);

        // Calculate the number of quads horizontally and vertically with inter-quad spacing
        let wgsl_normd_x = 2.0; // / size.width as f32;
        let wgsl_normd_y = 2.0; // / size.height as f32;
        let num_quads_x = ((wgsl_normd_x - quad_size.0) / (quad_size.0 + DFLT_QUAD_SPACING.0)) as u64;
        let num_quads_y = ((wgsl_normd_y - quad_size.1) / (quad_size.1 + DFLT_QUAD_SPACING.1)) as u64;
        println!("In Window::new(): Num Quads X: {}, Num Quads Y: {} with normd_x: {}, normd_y: {}", num_quads_x, num_quads_y, wgsl_normd_x, wgsl_normd_y);
        wait_for_enter();

        // create array for transforms of positions and colors
        let mut transforms: Vec<Transform>  = Vec::with_capacity((num_quads_x * num_quads_y) as usize);
        transforms.resize((num_quads_x * num_quads_y) as usize, Transform { position: [0.0, 0.0, 0.0, 0.0], scale: 1.0, color: [0.0, 0.0, 0.0, 0.0] });

        // Create a storage buffer for the transformation matrix
        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transform Buffer"),
            size: (transforms.capacity() * std::mem::size_of::<Transform>()) as wgpu::BufferAddress,
            // size: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress, //TODO  Not the right size here
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX, // The buffer is used in the vertex shader
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true }, // Match the shader's `storage` buffer
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create a bind group
        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Transform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_buffer.as_entire_binding(),
            }],
        });

        println!("Create data for WindowState in window::new()!");
        WindowState {
            surface,
            device,
            queue,
            config,
            size,
            window: window_arc,
            render_pipeline,
            //triangle_mesh,
            quad_mesh,
            transform_data: TransformData {
                transform_buffer,
                bind_group: transform_bind_group,
            }
        }
    }

    fn create_surface_config(
        size: PhysicalSize<u32>,
        capabilities: SurfaceCapabilities,
    ) -> wgpu::SurfaceConfiguration {
        let surface_format = capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(capabilities.formats[0]);

        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }

    fn create_device(adapter: &Adapter) -> (Device, Queue) {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::default()
                },
                None,
            )
            .block_on().unwrap()
            //.await.unwrap()
    }

    fn create_adapter(instance: Instance, surface: &Surface) -> Adapter {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .unwrap()
    }

    fn create_gpu_instance() -> Instance {
        Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;

        self.config.width = new_size.width;
        self.config.height = new_size.height;

        self.surface.configure(&self.device, &self.config);

        //println!("Resized to {:?} from WindowState!", new_size);
    }
    


    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        println!("Rendering from WindowState");
        let output = self.surface.get_current_texture().unwrap();
        let image_view_descriptor = wgpu::TextureViewDescriptor::default();
        let view = output
            .texture
            .create_view(&image_view_descriptor);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
            //transform_buffer,
            //transform_bind_group,

        let white = wgpu::Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        /*let sepia = wgpu::Color {
            r: 0.75,
            g: 0.5,
            b: 0.25,
            a: 1.0,
        };*/

        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(white),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };

    // figure out number of transforms required
    let quad_size = DFLT_QUAD_SIZE;

    println!("Window width: {}, height: {}", self.size.width, self.size.height);
    println!("Quad size: ({}, {}) with between spacing of ({}, {})", quad_size.0, quad_size.1, DFLT_QUAD_SPACING.0, DFLT_QUAD_SPACING.1);
    // Calculate the number of quads horizontally and vertically with inter-quad spacing
    //let num_quads_x = (self.size.width as f32 / (quad_size.0 + DFLT_QUAD_SPACING.0)).ceil() as u64;
    //let num_quads_y = (self.size.height as f32 / (quad_size.1 + DFLT_QUAD_SPACING.1)).ceil() as u64;
    //println!("In render Num Quads X: {}, Num Quads Y: {}", num_quads_x, num_quads_y);
    //wait_for_enter();
   
    let wgsl_normd_x = 2.0; // / self.size.width as f32;
    let wgsl_center_x = wgsl_normd_x / 2.0;
    let wgsl_normd_y = 2.0; // / self.size.height as f32;
    let wgsl_center_y = wgsl_normd_y / 2.0;
    let num_quads_x = ((wgsl_normd_x - quad_size.0) / (quad_size.0 + DFLT_QUAD_SPACING.0)) as u64;
    let num_quads_y = ((wgsl_normd_y - quad_size.1) / (quad_size.1 + DFLT_QUAD_SPACING.1)) as u64;
    println!("In render(): Num Quads X: {}, Num Quads Y: {}", num_quads_x, num_quads_y);

    // create arrays for transforms of positions and colors
    let mut new_transforms: Vec<Transform>  = Vec::with_capacity((num_quads_x * num_quads_y) as usize);
    println!("Allocated {} new transforms", new_transforms.capacity());

    // Create random arrays for  color (100rows of 4 elems each)
    let dims = af::Dim4::new(&[num_quads_x * num_quads_y, 3, 1, 1]); 
    println!("Allocated {} random colors", dims.elements()/3);
    let colors = af::randu::<f32>(dims);  
    println!("Generated random colors");   

    // Transfer random color data to host memory arrays
    let mut color_host: Vec<f32> = Vec::with_capacity(colors.elements() as usize);
    color_host.resize(colors.elements() as usize, 0.0);
    println!("Allocated {} host colors vs. colors dims{:?} and elements {} ", color_host.capacity(), colors.dims(), colors.elements());
    colors.host(&mut color_host);
    println!("Transferred colors to host");

    // Get ready to build the Vec<Transform>
    let mut idx: usize = 0; // for indexing through rows of the colors array

    {// limit scope of move of encoder
    let mut render_pass = encoder.begin_render_pass(&render_pass_descriptor);
    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.set_vertex_buffer(0, self.quad_mesh.vertex_buffer.slice(..));

    println!("About to calc new xforms");
    // Calculate the position of each quad and set position (in wgsl coords) and color into transforms
        for y in 0..num_quads_y {
            let pos_y = y as f32 * (quad_size.1 + DFLT_QUAD_SPACING.1) - wgsl_center_y;
            for x in 0..num_quads_x {
                let pos_x = x as f32 * (quad_size.0 + DFLT_QUAD_SPACING.0) - wgsl_center_x ;
                println!("Xform for quad ({},{}) is ({}, {}) ", x, y, pos_x, pos_y);                
                
                new_transforms.push(Transform {
                    position: [pos_x, pos_y, 0.0, 1.0],
                    scale: 1.0,
                    color: [color_host[idx], color_host[idx + 1], color_host[idx + 2], 1.0],
                });
                idx += 3; // Advance the index for the next row (i.e. ahead by 3 idxs)
            }
        }

        println!("{} Transforms setup", new_transforms.len());
        self.queue.write_buffer(&self.transform_data.transform_buffer, 0, bytemuck::cast_slice(&new_transforms));
        println!("and queued");
        render_pass.set_bind_group(0, &self.transform_data.bind_group, &[]); // set to Transform data

        //setup for indexed drawing of quads 
        /*let instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&transforms),
            usage: wgpu::BufferUsages::VERTEX,
        });*/

        //render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(self.quad_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16); 
        println!("Ready to draw");
        render_pass.draw(0..6, 0..new_transforms.len() as u32);
    } // end of scope of move of encoder

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}


// mod Pipelinebuilder==============================================================================================================================
// for resolving paths and filenames
use std::{
    env::current_dir,
    fs,
};
use std::path::{Path,PathBuf};


#[derive(Debug,Clone)]
struct PipelineBuilder {
    shader_filename: String,
    vertex_entry_pt: String,
    fragment_entry_pt: String,
    pixel_format: wgpu::TextureFormat,
    vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
}

impl PipelineBuilder {
    fn new_with_parms(
                        shader_filename: &str, 
                        vertex_entry_pt: &str, 
                        fragment_entry_pt: &str, 
                        pixel_format: wgpu::TextureFormat,
                        vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>> ) -> Self {
    // create a new PipelineBuilder using the given parameters. TODO 
        Self {
            shader_filename: shader_filename.to_string(),
            vertex_entry_pt: vertex_entry_pt.to_string(),
            fragment_entry_pt: fragment_entry_pt.to_string(),
            pixel_format,
            vertex_buffer_layouts, // may want to initialize
        }
    }

    pub fn new() -> Self {
        PipelineBuilder {
            shader_filename: "dummy".to_string(),
            vertex_entry_pt: "vs_main".to_string(),
            fragment_entry_pt: "fs_main".to_string(),
            pixel_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            vertex_buffer_layouts: Vec::new(),
        }
    }
    
    pub fn with_shader_parms(&mut self, shader_filename: &str, vertex_entry_pt: &str, fragment_entry_pt: &str) ->&mut Self{
        // set the parameters of the PipelineBuilder to given values
        self.shader_filename = shader_filename.to_string();
        self.vertex_entry_pt = vertex_entry_pt.to_string();
        self.fragment_entry_pt = fragment_entry_pt.to_string();
        self
    }

    pub fn with_pixel_format(&mut self, pixel_format: wgpu::TextureFormat) -> &mut Self {
        self.pixel_format = pixel_format;
        self
    }

    pub fn with_vertex_buffer_layout(&mut self, layout: wgpu::VertexBufferLayout<'static>) -> &mut Self {
        self.vertex_buffer_layouts.push(layout);
        self
    }
   

    pub fn finalize(&mut self) -> Self {
        // finalize the building of the PipelineBuilder
        self.clone()
    }

    pub fn build_render_pipeline(&self, device: &Device) -> wgpu::RenderPipeline {
        // build a RenderPipeline from the PipelineBuilder and return the RenderPipeline.
        
        // first make the filepath to the shader file per AI

        let current_dir = current_dir().unwrap();
        let crate_dir = current_dir.parent().unwrap().parent().unwrap();
        let shader_file_path = crate_dir.join("src/shaders/shader2d.wgsl");
        println!("Shader file path: {}", shader_file_path.display());
        let shader_code = fs::read_to_string(shader_file_path).expect("Can't read shader file!");

        // Now to setup the fields for the RenderPipeline
        let shader_mod_desc = wgpu::ShaderModuleDescriptor {
            label: Some("Shader Module"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        };
        let shader_module = device.create_shader_module(shader_mod_desc);

        // Create the pipeline layout with the bind group layout for transform posn
        /*let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Transform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            }
        );*/
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX, // The buffer is used in the vertex shader
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true }, // Match the shader's `storage` buffer
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout_desc = &wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        };

        let pipeline_layout = device.create_pipeline_layout(pipeline_layout_desc);

        let render_targets = [Some(wgpu::ColorTargetState{
            format: self.pixel_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        
        // incorporate those fields into the RenderPipelineDescriptor
        let render_pipeline_desc = wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some(&self.vertex_entry_pt),
                compilation_options: Default::default(),
                buffers: &self.vertex_buffer_layouts,
            },

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None, 
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some(&self.fragment_entry_pt),
                compilation_options: Default::default(),
                targets: &render_targets,
            }),

            cache: Default::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0, // !0 also used (sets to all 1s)
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        };

        // and finally create the RenderPipeline
        device.create_render_pipeline(&render_pipeline_desc)

    }

}

// mod Meshbuilder==============================================================================================================================
// for making meshes outside of .wgsl shader files

//use glam::*;  // May want to try using arrayfire arrays instead
use bytemuck:: {Pod, Zeroable, cast_slice};

use crate::circuit::wait_for_enter;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: [f32;3], // or Vec3 w/o Pod and Zeroable and bytemuck
    color: [f32;3],
}

impl Vertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {

        
        // alt. for below const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1=>Float32x3];
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
            wgpu::VertexAttribute {
                // for posn
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                // for color
                offset: std::mem::size_of::<[f32;3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

unsafe fn as_u8_slice<T:Sized>(slice: &[T]) -> &[u8] {
    // recast any slice to a slice of u8 bytes. Using this instead of the bytemuck crate b/c bytemuck needs Pod Trait
    // It may be possible just to use a standard array like [0.0f32; 3] in Vertex
    std::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * std::mem::size_of::<T>())
}

pub fn make_triangle(device: &wgpu::Device) -> wgpu::Buffer {
    let vertices: [Vertex; 3] = [
        Vertex {position: [-0.75, -0.75, 0.0], color: [1.0, 0.0, 0.0]},
        Vertex {position: [0.75, -0.75, 0.0], color: [0.0, 1.0, 0.0]},
        Vertex {position: [0.0, 0.75, 0.0], color: [0.0, 0.0, 1.0]},
    ];

    //let byte_slice = unsafe {as_u8_slice(&vertices)}; // This is redundant with bytemuck
    //let byte_slice = bytemuck::bytes_of(&vertices);

    let buffer_desc = wgpu::util::BufferInitDescriptor {
        label: Some("2D Triangle Vertex Buffer"),
        contents: bytemuck::bytes_of(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    };

    let buffer = device.create_buffer_init(&buffer_desc); //wgpu::util::DeviceExt::create_buffer_init(device, &buffer_desc);
    buffer
}

const DFLT_QUAD_SIZE: (f32, f32) = (0.01, 0.01);
const DFLT_QUAD_SPACING: (f32, f32) = (0.005, 0.005);

pub struct Mesh {
    // for use in indexed meshes, see make_quad
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}
pub fn make_quad(device: &wgpu::Device, quad_size: (f32, f32)) -> Mesh {

    let vertices: [Vertex; 4] = [
        Vertex {position: [-quad_size.0, -quad_size.1, 0.0], color: [1.0, 0.0, 0.0]}, // Bottom-left
        Vertex {position: [quad_size.0, -quad_size.1, 0.0], color: [0.0, 1.0, 0.0]},  // Bottom-right
        Vertex {position: [quad_size.0, quad_size.1, 0.0], color: [0.0, 1.0, 0.0]},   // Top-right
        Vertex {position: [-quad_size.0, quad_size.1, 0.0], color: [0.0, 0.0, 1.0]},  // Top-left
    ];
    //let byte_slice: &[u8] = bytemuck::bytes_of(&vertices);

    let vertex_buffer_desc = wgpu::util::BufferInitDescriptor {
        label: Some("2D Quad Vertex Buffer"),
        contents: bytemuck::bytes_of(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    };
    let vertex_buffer = device.create_buffer_init(&vertex_buffer_desc); 

    let indices: [u16;6] = [0,1,2,2,3,0];
    let index_slice: &[u8] = bytemuck::bytes_of(&indices);

    let index_buffer_desc = wgpu::util::BufferInitDescriptor {
        label: Some("2D Quad Index Buffer"),
        contents: index_slice,
        usage: wgpu::BufferUsages::INDEX,
    };
    let index_buffer = device.create_buffer_init(&index_buffer_desc);

    Mesh {
        vertex_buffer,
        index_buffer,
    }
}

/*pub fn make_quad(device: &wgpu::Device, quad_size: (f32, f32)) -> Mesh {
    // for using TriangleStrip
    let vertices: [Vertex; 4] = [
        Vertex {position: [-quad_size.0, -quad_size.1, 0.0], color: [1.0, 0.0, 0.0]}, // Bottom-left
        Vertex {position: [quad_size.0, -quad_size.1, 0.0], color: [0.0, 1.0, 0.0]},  // Bottom-right
        Vertex {position: [-quad_size.0, quad_size.1, 0.0], color: [0.0, 0.0, 1.0]},  // Top-left
        Vertex {position: [quad_size.0, quad_size.1, 0.0], color: [0.0, 1.0, 0.0]},   // Top-right
    ];

    let byte_slice: &[u8] = bytemuck::bytes_of(&vertices);

    let vertex_buffer_desc = wgpu::util::BufferInitDescriptor {
        label: Some("2D Quad Vertex Buffer with TriangleStrip"),
        contents: byte_slice,
        usage: wgpu::BufferUsages::VERTEX,
    };
    let vertex_buffer = device.create_buffer_init(&vertex_buffer_desc); 

    Mesh {
        vertex_buffer,
        index_buffer: device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dummy Index Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        }), 
    }*/
