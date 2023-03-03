use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;
use ash::vk;
use egui::{ClippedPrimitive, Rect, TextureId};
use egui::epaint::{Primitive, Vertex};
use memoffset::offset_of;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use crate::etna::shader::ShaderModule;
use crate::etna::{Device, GraphicsSettings, HostMappedBuffer, HostMappedBufferCreateInfo, Swapchain};
use crate::etna::material_pipeline::{DescriptorManager, MaterialPipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription, RasterizationOptions};
use crate::rehnda_core::{ConstPtr, Vec2};


pub struct RehndaUi {
    egui_ctx: egui::Context,
    winit_integration: egui_winit::State,
    ui_state: UiState,
    pub egui_renderer: EguiRenderer,
}

pub struct ScreenState {
    pub size_in_pixels: [u32; 2],
    pub pixels_per_point: f32,
}

struct UiState {
    name: String,
    age: u32,
}

impl RehndaUi {
    pub fn create(device: ConstPtr<Device>, event_loop: &EventLoopWindowTarget<()>, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> Self {
        RehndaUi {
            winit_integration: egui_winit::State::new(event_loop),
            egui_ctx: egui::Context::default(),
            ui_state: UiState {
                name: "empty".to_string(),
                age: 1,
            },
            egui_renderer: EguiRenderer::create(device, graphics_settings, swapchain),
        }
    }

    pub fn handle_window_event(&mut self, window_event: &WindowEvent) {
        // TODO handle egui wanting exclusive use of an input event (i.e click on gui not in game)
        let _ = self.winit_integration.on_event(&self.egui_ctx, window_event);
    }

    pub fn update_ui_state(&mut self, window: &winit::window::Window) {
        let new_input = self.winit_integration.take_egui_input(window);
        let full_output = self.egui_ctx.run(new_input, |egui_ctx| {
            egui::Window::new("My window").show(egui_ctx, |ui| {
                // ui.heading("My egui Application");
                // ui.horizontal(|ui| {
                //     ui.label("Your name: ");
                //     ui.text_edit_singleline(&mut self.ui_state.name);
                // });
                ui.add(egui::Slider::new(&mut self.ui_state.age, 0..=120).text("age"));
                // if ui.button("Click each year").clicked() {
                //     self.ui_state.age += 1;
                // }
                ui.label(format!("Hello '{}', age {}", self.ui_state.name, self.ui_state.age));
            });
        });
        self.winit_integration.handle_platform_output(window, &self.egui_ctx, full_output.platform_output);
        self.egui_renderer.clipped_primitives = self.egui_ctx.tessellate(full_output.shapes);
        self.egui_renderer.screen_state = ScreenState {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point: self.egui_ctx.pixels_per_point(),
        };
    }
}

pub struct EguiRenderer {
    device: ConstPtr<Device>,
    descriptor_manager: DescriptorManager,
    clipped_primitives: Vec<ClippedPrimitive>,
    ui_meshes: Vec<UiMesh>,
    pipeline: MaterialPipeline,
    screen_state: ScreenState,
}

struct UiMesh {
    vertex_buffer: HostMappedBuffer,
    index_buffer: HostMappedBuffer,
    index_count: u32,
    texture_id: TextureId,
    clip_rect: vk::Rect2D,
}

impl EguiRenderer {
    pub fn create(device: ConstPtr<Device>, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> Self {
        let mut descriptor_manager = DescriptorManager::create(device);
        EguiRenderer {
            device,
            ui_meshes: Vec::new(),
            pipeline: egui_pipeline(device, &mut descriptor_manager, graphics_settings, swapchain),
            descriptor_manager,
            clipped_primitives: Vec::new(),
            screen_state: ScreenState {
                size_in_pixels: [1, 1],
                pixels_per_point: 1.0,
            },
        }
    }

    pub fn update_resources(&mut self) {
        for (i, clipped_primitive) in self.clipped_primitives.iter().enumerate() {
            match &clipped_primitive.primitive {
                Primitive::Mesh(mesh) => {
                    let required_vertex_buffer_size = (mesh.vertices.len() * std::mem::size_of::<Vertex>()) as u64;
                    let required_index_buffer_size = (mesh.indices.len() * std::mem::size_of::<u32>()) as u64;
                    // create buffer if one doesn't exist for the mesh, or create a new one if too small
                    if self.ui_meshes.len() <= i {
                        self.ui_meshes.push(UiMesh {
                            vertex_buffer: HostMappedBuffer::create(self.device, HostMappedBufferCreateInfo {
                                size: required_vertex_buffer_size,
                                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                            }),
                            index_buffer: HostMappedBuffer::create(self.device, HostMappedBufferCreateInfo {
                                size: required_index_buffer_size,
                                usage: vk::BufferUsageFlags::INDEX_BUFFER,
                            }),
                            index_count: mesh.indices.len() as _,
                            texture_id: mesh.texture_id,
                            clip_rect: self.screen_state.get_clip_rect(&clipped_primitive.clip_rect),
                        });
                    } else {
                        if self.ui_meshes.get(i).unwrap().vertex_buffer.size() < required_vertex_buffer_size {
                            self.ui_meshes.get_mut(i).unwrap().vertex_buffer = HostMappedBuffer::create(self.device, HostMappedBufferCreateInfo {
                                size: required_vertex_buffer_size,
                                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                            });
                        }
                        if self.ui_meshes.get(i).unwrap().index_buffer.size() < required_vertex_buffer_size {
                            self.ui_meshes.get_mut(i).unwrap().index_buffer = HostMappedBuffer::create(self.device, HostMappedBufferCreateInfo {
                                size: required_index_buffer_size,
                                usage: vk::BufferUsageFlags::INDEX_BUFFER,
                            });
                        }
                    }

                    let vertex_data: &[u8] = bytemuck::cast_slice(mesh.vertices.as_slice());
                    self.ui_meshes.get(i).unwrap().vertex_buffer.write_data(vertex_data);
                    let index_data: &[u8] = bytemuck::cast_slice(mesh.indices.as_slice());
                    self.ui_meshes.get(i).unwrap().index_buffer.write_data(index_data);
                    self.ui_meshes.get_mut(i).unwrap().index_count = mesh.indices.len() as _;
                }
                Primitive::Callback(_) => panic!("Expected no egui callbacks"),
            }
        }
    }

    pub fn draw(&self, device: &Device, swapchain: &Swapchain, command_buffer: vk::CommandBuffer) {
        // bind the pipeline
        unsafe { device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline.graphics_pipeline()); }
        let viewport = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swapchain.extent().width as f32)
            .height(swapchain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];
        unsafe { device.cmd_set_viewport(command_buffer, 0, &viewport); }


        for ui_mesh in self.ui_meshes.iter().rev() {
            let scissor = [ui_mesh.clip_rect];
            unsafe { device.cmd_set_scissor(command_buffer, 0, &scissor); }
            // bind mesh data
            let vert_buffers = &[ui_mesh.vertex_buffer.vk_buffer()];
            let offsets = &[0u64];
            unsafe {
                device.cmd_bind_vertex_buffers(command_buffer, 0, vert_buffers, offsets);
                device.cmd_bind_index_buffer(command_buffer, ui_mesh.index_buffer.vk_buffer(), 0, vk::IndexType::UINT32);
                // TODO actually calculate screen size
                let screen_size = &[Vec2::new(swapchain.extent.width as f32, swapchain.extent.height as f32)];
                let screen_size_data: &[u8] = bytemuck::cast_slice(screen_size);
                device.cmd_push_constants(command_buffer, self.pipeline.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, screen_size_data);

                device.cmd_draw_indexed(command_buffer, ui_mesh.index_count, 1, 0, 0, 0);
            }
        }
    }
}

impl ScreenState {
    fn get_clip_rect(&self, egui_clip: & Rect) -> vk::Rect2D {
        // Transform clip rect to physical pixels:
        let clip_min_x = self.pixels_per_point * egui_clip.min.x;
        let clip_min_y = self.pixels_per_point * egui_clip.min.y;
        let clip_max_x = self.pixels_per_point * egui_clip.max.x;
        let clip_max_y = self.pixels_per_point * egui_clip.max.y;

        // Round to integer:
        let clip_min_x = clip_min_x.round() as u32;
        let clip_min_y = clip_min_y.round() as u32;
        let clip_max_x = clip_max_x.round() as u32;
        let clip_max_y = clip_max_y.round() as u32;

        // Clamp:
        let clip_min_x = clip_min_x.clamp(0, self.size_in_pixels[0]);
        let clip_min_y = clip_min_y.clamp(0, self.size_in_pixels[1]);
        let clip_max_x = clip_max_x.clamp(clip_min_x, self.size_in_pixels[0]);
        let clip_max_y = clip_max_y.clamp(clip_min_y, self.size_in_pixels[1]);

        vk::Rect2D {
            offset: vk::Offset2D {
                x: clip_min_x as _,
                y: clip_min_y as _,
            },
            extent: vk::Extent2D {
                width: clip_max_x - clip_min_x,
                height: clip_max_y - clip_min_y,
            },
        }
    }
}

fn egui_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> MaterialPipeline {
    let vert_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/egui.vert_spv"));
    let frag_shader_module = ShaderModule::load_from_file(device, Path::new("shaders/spirv/egui.frag_spv"));
    let main_function_name = CString::new("main").unwrap();
    let vertex_shader_stage_ci = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vert_shader_module.handle())
        .name(main_function_name.as_c_str())
        .build();
    let frag_shader_stage_ci = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader_module.handle())
        .name(main_function_name.as_c_str())
        .build();

    let vertex_attributes = egui_vertex_descriptions();
    let vertex_input = PipelineVertexInputDescription {
        bindings: &[egui_binding_description()],
        attributes: vertex_attributes.as_slice(),
    };
    // push constant for pushing screen size
    let push_constant = vk::PushConstantRange::builder()
        .offset(0)
        .size((size_of::<u32>() * 2) as u32)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();

    let multisampling = PipelineMultisamplingInfo {
        msaa_samples: graphics_settings.msaa_samples,
        enable_sample_rate_shading: graphics_settings.sample_rate_shading_enabled,
    };

    let create_info = PipelineCreateInfo {
        global_set_layouts: &[],
        additional_descriptor_set_layouts: &[],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
        rasterization_options: &RasterizationOptions {
            cull_mode: vk::CullModeFlags::NONE,
        }
    };

    MaterialPipeline::create(device, &create_info)
}

fn egui_vertex_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
    vec![
        // position attribute
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(offset_of!(Vertex, pos) as u32)
            .build(),
        // uv attribute
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(offset_of!(Vertex, uv) as u32)
            .build(),
        // color attribute
        vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32_UINT)
            .offset(offset_of!(Vertex, color) as u32)
            .build(),
    ]
}

fn egui_binding_description() -> vk::VertexInputBindingDescription {
    vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(size_of::<Vertex>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build()
}