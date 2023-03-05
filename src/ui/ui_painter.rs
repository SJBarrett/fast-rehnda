use ahash::AHashMap;
use ash::vk;
use bevy_ecs::prelude::Resource;
use egui::{ClippedPrimitive, Color32, ImageData, Rect, TextureId, TextureOptions, TexturesDelta};
use egui::epaint::{Primitive, Vertex};
use log::info;

use crate::etna::{CommandPool, Device, GraphicsSettings, HostMappedBuffer, HostMappedBufferCreateInfo, PhysicalDevice, Swapchain, Texture, TextureCreateInfo};
use crate::etna::material_pipeline::DescriptorManager;
use crate::rehnda_core::ConstPtr;
use crate::ui::ui_pipeline::{egui_pipeline, UiPipeline};

#[derive(Resource)]
pub struct UiPainter {
    device: ConstPtr<Device>,
    descriptor_manager: DescriptorManager,
    pipeline: UiPipeline,
    textures: AHashMap<TextureId, Texture>,
    texture_free_queue: Vec<Texture>,
    ui_meshes: Vec<UiMesh>,
    mesh_destroy_queue: Vec<HostMappedBuffer>,
    ui_mesh_destroy_queue: Vec<UiMesh>,
}

#[derive(Resource, Default)]
pub struct EguiOutput {
    pub clipped_primitives: Vec<ClippedPrimitive>,
    pub texture_delta: TexturesDelta,
    pub screen_state: ScreenState,
}

struct UiMesh {
    vertex_buffer: HostMappedBuffer,
    index_buffer: HostMappedBuffer,
    index_count: u32,
    texture_id: TextureId,
    clip_rect: vk::Rect2D,
}

impl UiPainter {
    pub fn create(device: ConstPtr<Device>, graphics_settings: &GraphicsSettings, swapchain: &Swapchain) -> Self {
        let mut descriptor_manager = DescriptorManager::create(device);
        UiPainter {
            device,
            ui_meshes: Vec::new(),
            pipeline: egui_pipeline(device, &mut descriptor_manager, graphics_settings, swapchain),
            descriptor_manager,
            mesh_destroy_queue: Vec::new(),
            textures: AHashMap::new(),
            texture_free_queue: Vec::new(),
            ui_mesh_destroy_queue: Vec::new(),
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

    pub fn update_resources(&mut self, physical_device: &PhysicalDevice, command_pool: &CommandPool, egui_output: &EguiOutput) {
        self.mesh_destroy_queue.clear();
        self.ui_mesh_destroy_queue.clear();
        self.texture_free_queue.clear();
        for (texture_id, image_delta) in egui_output.texture_delta.set.iter() {
            if let Some(po) = image_delta.pos {
                // TODO copy new data
                info!("Changed image");
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

        for (i, clipped_primitive) in egui_output.clipped_primitives.iter().enumerate() {
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
                            clip_rect: egui_output.screen_state.get_clip_rect(&clipped_primitive.clip_rect),
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
                    mesh_ref.clip_rect = egui_output.screen_state.get_clip_rect(&clipped_primitive.clip_rect);
                }
                Primitive::Callback(_) => panic!("Expected no egui callbacks"),
            }
        }

        if egui_output.clipped_primitives.len() < self.ui_meshes.len() {
            for _ in 0..(self.ui_meshes.len() - egui_output.clipped_primitives.len()) {
                self.ui_mesh_destroy_queue.push(self.ui_meshes.pop().unwrap());
            }
        }

        for texture_id in egui_output.texture_delta.free.iter() {
            self.textures.remove(texture_id).unwrap();
        }
    }

    pub fn draw(&self, device: &Device, swapchain: &Swapchain, command_buffer: vk::CommandBuffer, egui_output: &EguiOutput) {
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
                let screen_size = egui_output.screen_state.size_in_points();
                let screen_size_data: &[u8] = bytemuck::cast_slice(&screen_size);
                device.cmd_push_constants(command_buffer, self.pipeline.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, screen_size_data);

                device.cmd_draw_indexed(command_buffer, ui_mesh.index_count, 1, 0, 0, 0);
            }
        }
    }
}

#[derive(Default)]
pub struct ScreenState {
    pub size_in_pixels: [u32; 2],
    pub pixels_per_point: f32,
}

impl ScreenState {
    fn size_in_points(&self) -> [f32; 2] {
        [self.size_in_pixels[0] as f32 / self.pixels_per_point, self.size_in_pixels[1] as f32 / self.pixels_per_point]
    }

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

