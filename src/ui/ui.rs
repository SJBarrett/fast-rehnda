use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;
use ahash::AHashMap;

use ash::vk;
use egui::{ClippedPrimitive, Color32, ImageData, Rect, TextureFilter, TextureId, TextureOptions, TexturesDelta, Visuals};
use egui::epaint::{Primitive, Vertex};
use memoffset::offset_of;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::etna::{CommandPool, Device, GraphicsSettings, HostMappedBuffer, HostMappedBufferCreateInfo, PhysicalDevice, Swapchain, Texture, TextureCreateInfo};
use crate::etna::material_pipeline::{DescriptorManager, layout_binding, MaterialPipeline, PipelineCreateInfo, PipelineMultisamplingInfo, PipelineVertexInputDescription, RasterizationOptions};
use crate::etna::shader::ShaderModule;
use crate::rehnda_core::{ConstPtr};

pub struct RehndaUi {
    egui_ctx: egui::Context,
    winit_integration: egui_winit::State,
    ui_state: UiState,
    pub egui_renderer: EguiRenderer,
}

struct ScreenState {
    size_in_pixels: [u32; 2],
    pixels_per_point: f32,
}

impl ScreenState {
    fn size_in_points(&self) -> [f32; 2] {
        [self.size_in_pixels[0] as f32 / self.pixels_per_point, self.size_in_pixels[1] as f32 / self.pixels_per_point]
    }
}

struct UiState {
    name: String,
    age: u32,
}

impl RehndaUi {
    pub fn create(device: ConstPtr<Device>, event_loop: &EventLoopWindowTarget<()>, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> Self {
        let egui_ctx = egui::Context::default();

        RehndaUi {
            winit_integration: egui_winit::State::new(event_loop),
            egui_ctx,
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
                ui.heading("My egui Application");
                ui.horizontal(|ui| {
                    ui.label("Your name: ");
                    ui.text_edit_singleline(&mut self.ui_state.name);
                });
                ui.add(egui::Slider::new(&mut self.ui_state.age, 0..=120).text("age"));
                if ui.button("Click each year").clicked() {
                    self.ui_state.age += 1;
                }
                ui.label(format!("Hello '{}', age {}", self.ui_state.name, self.ui_state.age));
            });
        });
        self.winit_integration.handle_platform_output(window, &self.egui_ctx, full_output.platform_output);
        self.egui_renderer.egui_output = EguiOutput {
            clipped_primitives: self.egui_ctx.tessellate(full_output.shapes),
            texture_delta: full_output.textures_delta,
            screen_state: ScreenState {
                size_in_pixels: [window.inner_size().width, window.inner_size().height],
                pixels_per_point: self.egui_ctx.pixels_per_point(),
            },
        };
    }
}

pub struct EguiRenderer {
    device: ConstPtr<Device>,
    descriptor_manager: DescriptorManager,
    pipeline: UiPipeline,
    egui_output: EguiOutput,
    textures: AHashMap<TextureId, Texture>,
    texture_free_queue: Vec<Texture>,
    ui_meshes: Vec<UiMesh>,
    mesh_destroy_queue: Vec<HostMappedBuffer>,
}

struct EguiOutput {
    clipped_primitives: Vec<ClippedPrimitive>,
    texture_delta: TexturesDelta,
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
            egui_output: EguiOutput {
                clipped_primitives: Vec::new(),
                screen_state: ScreenState {
                    size_in_pixels: [1, 1],
                    pixels_per_point: 1.0,
                },
                texture_delta: TexturesDelta::default(),
            },
            mesh_destroy_queue: Vec::new(),
            textures: AHashMap::new(),
            texture_free_queue: Vec::new(),
        }
    }

    fn create_ui_texture(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, physical_device: &PhysicalDevice, command_pool: &CommandPool, size: &[usize; 2], texture_options: &TextureOptions, data: &[u8]) -> Texture {
        Texture::create(device, physical_device, command_pool, descriptor_manager, &TextureCreateInfo {
            width: size[0] as _,
            height: size[1] as _,
            mip_levels: None,
            data,
            sampler_info: Some(
                vk::SamplerCreateInfo::builder()
                    .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                    .anisotropy_enable(false)
                    .min_filter(vk::Filter::LINEAR)
                    .mag_filter(vk::Filter::LINEAR)
                    .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
                    .min_lod(0.0)
                    .max_lod(vk::LOD_CLAMP_NONE)
                    .build()
            ),
        })
    }

    pub fn update_resources(&mut self, physical_device: &PhysicalDevice, command_pool: &CommandPool) {
        self.mesh_destroy_queue.clear();
        self.texture_free_queue.clear();
        for (texture_id, image_delta) in self.egui_output.texture_delta.set.iter() {
            if let Some(po) = image_delta.pos {
                // TODO copy new data
            } else {
                match &image_delta.image {
                    ImageData::Color(color_image) => {
                        self.textures.insert(*texture_id, Self::create_ui_texture(self.device, &mut self.descriptor_manager, physical_device, command_pool, &color_image.size, &image_delta.options, bytemuck::cast_slice(color_image.pixels.as_slice())));
                    }
                    ImageData::Font(font_image) => {
                        let data: Vec<Color32> = font_image.srgba_pixels(None).collect();
                        self.textures.insert(*texture_id, Self::create_ui_texture(self.device, &mut self.descriptor_manager, physical_device, command_pool, &font_image.size, &image_delta.options, bytemuck::cast_slice(data.as_slice())));
                    }
                }
            }
        }

        for (i, clipped_primitive) in self.egui_output.clipped_primitives.iter().enumerate() {
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
                            clip_rect: self.egui_output.screen_state.get_clip_rect(&clipped_primitive.clip_rect),
                        });
                    } else {
                        if self.ui_meshes.get(i).unwrap().vertex_buffer.size() < required_vertex_buffer_size {
                            let mut new_buffer = HostMappedBuffer::create(self.device, HostMappedBufferCreateInfo {
                                size: required_vertex_buffer_size,
                                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                            });

                            std::mem::swap(&mut self.ui_meshes.get_mut(i).unwrap().vertex_buffer, &mut new_buffer);
                            self.mesh_destroy_queue.push(new_buffer);
                        }
                        if self.ui_meshes.get(i).unwrap().index_buffer.size() < required_vertex_buffer_size {
                            let mut new_buffer = HostMappedBuffer::create(self.device, HostMappedBufferCreateInfo {
                                size: required_index_buffer_size,
                                usage: vk::BufferUsageFlags::INDEX_BUFFER,
                            });
                            std::mem::swap(&mut self.ui_meshes.get_mut(i).unwrap().index_buffer, &mut new_buffer);
                            self.mesh_destroy_queue.push(new_buffer);
                        }
                    }

                    let mut mesh_ref = self.ui_meshes.get_mut(i).unwrap();

                    let vertex_data: &[u8] = bytemuck::cast_slice(mesh.vertices.as_slice());
                    mesh_ref.vertex_buffer.write_data(vertex_data);
                    let index_data: &[u8] = bytemuck::cast_slice(mesh.indices.as_slice());
                    mesh_ref.index_buffer.write_data(index_data);
                    mesh_ref.index_count = mesh.indices.len() as _;
                    mesh_ref.clip_rect = self.egui_output.screen_state.get_clip_rect(&clipped_primitive.clip_rect);
                }
                Primitive::Callback(_) => panic!("Expected no egui callbacks"),
            }
        }

        for texture_id in self.egui_output.texture_delta.free.iter() {
            self.textures.remove(texture_id).unwrap();
        }
    }

    pub fn draw(&self, device: &Device, swapchain: &Swapchain, command_buffer: vk::CommandBuffer) {
        // bind the pipeline
        unsafe { device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline); }
        let viewport = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swapchain.extent().width as f32)
            .height(swapchain.extent().height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];
        unsafe { device.cmd_set_viewport(command_buffer, 0, &viewport); }


        for ui_mesh in self.ui_meshes.iter() {
            let scissor = [ui_mesh.clip_rect];
            unsafe { device.cmd_set_scissor(command_buffer, 0, &scissor); }
            // bind mesh data
            let vert_buffers = &[ui_mesh.vertex_buffer.vk_buffer()];
            let offsets = &[0u64];
            unsafe {
                device.cmd_bind_vertex_buffers(command_buffer, 0, vert_buffers, offsets);
                device.cmd_bind_index_buffer(command_buffer, ui_mesh.index_buffer.vk_buffer(), 0, vk::IndexType::UINT32);
                let descriptor_sets = &[self.textures.get(&ui_mesh.texture_id).unwrap().descriptor_set];
                device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline_layout, 0, descriptor_sets, &[]);
                let screen_size = self.egui_output.screen_state.size_in_points();
                let screen_size_data: &[u8] = bytemuck::cast_slice(&screen_size);
                device.cmd_push_constants(command_buffer, self.pipeline.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, screen_size_data);

                device.cmd_draw_indexed(command_buffer, ui_mesh.index_count, 1, 0, 0, 0);
            }
        }
    }
}

impl ScreenState {
    fn get_clip_rect(&self, egui_clip: &Rect) -> vk::Rect2D {
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

fn egui_pipeline(device: ConstPtr<Device>, descriptor_manager: &mut DescriptorManager, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> UiPipeline {
    let texture_binding_description = descriptor_manager.layout_cache.create_descriptor_layout_for_binding(&layout_binding(0, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, vk::ShaderStageFlags::FRAGMENT));
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
        additional_descriptor_set_layouts: &[texture_binding_description],
        shader_stages: &[vertex_shader_stage_ci, frag_shader_stage_ci],
        push_constants: &[push_constant],
        extent: swapchain.extent,
        image_format: swapchain.image_format,
        vertex_input,
        multisampling,
        rasterization_options: &RasterizationOptions {
            cull_mode: vk::CullModeFlags::NONE,
        },
    };

    create_ui_pipeline(device, &create_info)
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
            .format(vk::Format::R8G8B8A8_UNORM)
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

pub struct UiPipeline {
    device: ConstPtr<Device>,
    pub pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
}

impl Drop for UiPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            // layouts are destroyed by the layout cache
        }
    }
}

pub fn create_ui_pipeline(device: ConstPtr<Device>, create_info: &PipelineCreateInfo) -> UiPipeline {
    let vertex_input_ci = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(create_info.vertex_input.bindings)
        .vertex_attribute_descriptions(create_info.vertex_input.attributes);

    // let us change viewport and scissor state without rebuilding the pipeline
    let input_assembly_ci = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(create_info.extent.width as f32)
        .height(create_info.extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);
    let viewports = &[viewport.build()];

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(create_info.extent);
    let scissors = &[scissor.build()];

    let viewport_state_ci = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(viewports)
        .scissors(scissors);

    let dynamic_state_ci = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

    let rasterization_ci = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
        .line_width(1.0);

    let multisample_state_ci = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(create_info.multisampling.msaa_samples.to_sample_count_flags())
        .sample_shading_enable(create_info.multisampling.enable_sample_rate_shading)
        .min_sample_shading(if create_info.multisampling.enable_sample_rate_shading { 0.2 } else { 1.0 }) // closer to 1 is smoother
        .alpha_to_coverage_enable(false)
        .alpha_to_one_enable(false);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::ONE)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA);
    let color_blend_attachments = &[color_blend_attachment.build()];

    let color_blend_state_ci = vk::PipelineColorBlendStateCreateInfo::builder()
        .attachments(color_blend_attachments);
    // stencil op
    let stencil_op = vk::StencilOpState::builder()
        .fail_op(vk::StencilOp::KEEP)
        .pass_op(vk::StencilOp::KEEP)
        .compare_op(vk::CompareOp::ALWAYS)
        .build();
    let depth_stencil_ci = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false)
        .front(stencil_op)
        .back(stencil_op);

    let color_attachment_formats = &[create_info.image_format];
    let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
        .color_attachment_formats(color_attachment_formats)
        .depth_attachment_format(vk::Format::D32_SFLOAT); // TODO don't assume this format

    let set_layouts: Vec<vk::DescriptorSetLayout> = [create_info.global_set_layouts, create_info.additional_descriptor_set_layouts].concat();
    let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(set_layouts.as_slice())
        .push_constant_ranges(create_info.push_constants);

    let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_ci, None) }
        .expect("Failed to create pipline layout");

    let pipeline_ci = vk::GraphicsPipelineCreateInfo::builder()
        .stages(create_info.shader_stages)
        .vertex_input_state(&vertex_input_ci)
        .input_assembly_state(&input_assembly_ci)
        .viewport_state(&viewport_state_ci)
        .rasterization_state(&rasterization_ci)
        .multisample_state(&multisample_state_ci)
        .color_blend_state(&color_blend_state_ci)
        .dynamic_state(&dynamic_state_ci)
        .layout(pipeline_layout)
        .render_pass(vk::RenderPass::null())
        .push_next(&mut pipeline_rendering_create_info)
        .depth_stencil_state(&depth_stencil_ci)
        .subpass(0);
    let pipeline_create_infos = &[pipeline_ci.build()];
    let pipeline = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), pipeline_create_infos, None) }
        .expect("Failed to create graphics pipeline")[0];

    UiPipeline {
        device,
        pipeline_layout,
        pipeline,
    }
}
