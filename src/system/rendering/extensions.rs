use log::info;

use crate::{
    asset_managment::AssetStore,
    assets::{BindgroupState, Material, Mesh},
    components,
    ecs::World,
    FORMAT,
};

pub struct AttachmentData {
    pub color: wgpu::Texture,
    pub depth_stencil: wgpu::Texture,
}

///Trait that all rendering extensions must implement
///
///Allows for extending the renderer
pub trait RenderingExtension {
    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world: &World,
        assets: &AssetStore,
        attachments: &AttachmentData,
    );

    fn get_order(&self) -> u32;
}

impl std::cmp::PartialEq for dyn RenderingExtension {
    fn eq(&self, other: &Self) -> bool {
        self.get_order().eq(&other.get_order())
    }
}

impl std::cmp::Eq for dyn RenderingExtension {}

impl std::cmp::PartialOrd for dyn RenderingExtension {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get_order().partial_cmp(&other.get_order())
    }
}

impl std::cmp::Ord for dyn RenderingExtension {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get_order().cmp(&other.get_order())
    }
}

#[derive(Clone)]
pub struct Base {
    order: u32,
}
impl Base {
    pub fn new(order: u32) -> Self {
        Self { order }
    }
}

impl RenderingExtension for Base {
    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world: &World,
        assets: &AssetStore,
        attachments: &AttachmentData,
    ) {
        let binding = world
            .get_all_components::<components::camera::MainCamera>()
            .expect("Could not find the main camera");
        let camera = binding.first().unwrap();

        //This is cached, so should be reasonably fast
        let meshes = world.get_all_components::<crate::components::mesh::Mesh>();
        // let main_camera = world.get_all_components::crate
        //
        //No rendering needs to be done
        //NO there IS work to be done here, like the skybox and shit
        if meshes.is_none() {
            info!("Rendered 0 meshes");
            return;
        }
        let meshes = meshes.unwrap();

        let mut camera = camera.borrow_mut();

        camera.update_gpu(encoder);
        let mut materials = Vec::new();

        //Update the gpu data for every Mesh
        for m in &meshes {
            let m = m.borrow();
            m.update_gpu(encoder);
            materials.push(m.get_material_id().unwrap());
        }

        //Initialize bindgroups for all needed materials
        for m in materials {
            let m = assets.get_by_id::<Material>(m).unwrap();
            let mut m = m.borrow_mut();

            if let BindgroupState::Initialized = m.get_bindgroup_state() {
                continue;
            }

            m.initialize_bindgroups(assets);
        }

        let color_view = attachments.color.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Color attachment view"),
            format: Some(*FORMAT.get().unwrap()),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let depth_setencil_veiw =
            attachments
                .depth_stencil
                .create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Depth stencil attachment"),
                    format: Some(wgpu::TextureFormat::Depth32Float),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("First pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_setencil_veiw,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        //Set the camera
        camera.set_bindgroup(&mut render_pass);

        //Iterate through the meshes and render them
        for m in &meshes {
            let m = m.borrow();

            m.set_bindgroup(&mut render_pass);

            let mat = assets
                .get_by_id::<Material>(m.get_material_id().unwrap())
                .unwrap();
            let mat = mat.borrow();

            mat.render(&mut render_pass);

            let mesh = assets.get_by_id::<Mesh>(m.get_mesh_id().unwrap()).unwrap();
            let mesh = mesh.borrow();

            mesh.render(&mut render_pass);
        }
        drop(render_pass);
    }

    fn get_order(&self) -> u32 {
        self.order
    }
}

// pub struct Screenshot {
//     order: u32,
// }

// impl Screenshot {
//     pub fn new(order: u32) -> Self {
//         Self { order }
//     }
// }

// impl RenderingExtension for Screenshot {
//     fn render(
//         &self,
//         encoder: &mut wgpu::CommandEncoder,
//         world: &World,
//         assets: &AssetStore,
//         attachments: &AttachmentData,
//     ) {
//         let image_size = attachments.color.size();

//         let bpr = helpers::calculate_bpr(image_size.width, *FORMAT.get().unwrap()) as u32;

//         encoder.copy_texture_to_buffer(
//             frame.texture.as_image_copy(),
//             wgpu::ImageCopyBufferBase {
//                 buffer: &self.screenshot_buffer,
//                 layout: wgpu::ImageDataLayout {
//                     offset: 0,
//                     bytes_per_row: Some(bpr), //(image_size.width * 4 * 4),
//                     rows_per_image: Some(image_size.height), //(image_size.height),
//                 },
//             },
//             frame.texture.size(),
//         );

//         queue.submit(Some(encoder.finish()));
//         self.staging_belt.recall();

//         let slice = self.screenshot_buffer.slice(..);
//         slice.map_async(wgpu::MapMode::Read, |_| {});
//         device.poll(wgpu::Maintain::Wait);
//         let buffer = slice
//             .get_mapped_range()
//             .iter()
//             .copied()
//             .collect::<Vec<u8>>();
//         self.screenshot_buffer.unmap();

//         let p = Path::new(grimoire::SCREENSHOT_DIRECTORY);
//         if !p.exists() {
//             if let Err(e) = std::fs::create_dir(p) {
//                 log::error!("Failed to create screenshots directory {e}");
//             }
//         }
//         let filename = format!(
//             "{}/screenshot_{}.png",
//             grimoire::SCREENSHOT_DIRECTORY,
//             chrono::Local::now().format(grimoire::FILE_TIME_FORMAT)
//         );
//         log::info!("Screenshot filename = {filename}");

//         thread::spawn(move || {
//             let image = lunar_lib::helpers::arr_to_image(
//                 &buffer,
//                 bpr / 4,
//                 image_size.width,
//                 image_size.height,
//                 image::ImageOutputFormat::Png,
//             )
//             .unwrap();

//             if let Err(e) = std::fs::write(filename, image) {
//                 log::error!("Failed to write image {e}");
//             }
//         });
//         self.screenshot = false;
//     }
//     fn get_order(&self) -> u32 {
//         self.order
//     }
// }