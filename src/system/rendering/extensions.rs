#![allow(clippy::too_many_lines)]

use std::{num::NonZeroU64, sync::Arc};

use log::{debug, trace};
use vec_key_value_pair::set::VecSet;
use wgpu::util::DeviceExt;

use crate::{
    asset_managment::AssetStore,
    assets::{BindgroupState, Material, Mesh},
    components,
    ecs::{ComponentReference, World},
    DEVICE, STAGING_BELT,
};

pub struct AttachmentData {
    pub color: wgpu::TextureView,
    pub depth_stencil: wgpu::TextureView,
}

///Trait that all rendering extensions must implement
///
///Allows for extending the renderer
pub trait RenderingExtension {
    fn render(
        &mut self,
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
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for dyn RenderingExtension {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get_order().cmp(&other.get_order())
    }
}

#[derive(Default)]
pub struct Base {
    order: u32,
    ///Stores vector of (mesh_id, material_id) for caching
    identifier: Vec<(u128, u128)>,
    v_buffers: Vec<wgpu::Buffer>,
    mesh_materials: Vec<MeshMaterial>,
    num_instances: Vec<usize>,
    mesh_refs: Vec<Vec<ComponentReference<components::mesh::Mesh>>>,
}

impl Base {
    #[must_use]
    pub const fn new(order: u32) -> Self {
        Self {
            order,
            identifier: Vec::new(),
            v_buffers: Vec::new(),
            mesh_materials: Vec::new(),
            num_instances: Vec::new(),
            mesh_refs: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct MeshMaterial {
    mesh_id: u128,
    material_id: u128,
}

impl PartialEq<(u128, u128)> for MeshMaterial {
    fn eq(&self, other: &(u128, u128)) -> bool {
        self.mesh_id == other.0 && self.material_id == other.1
    }
}

impl MeshMaterial {
    fn new(mesh_id: u128, material_id: u128) -> Self {
        Self {
            mesh_id,
            material_id,
        }
    }
}

impl RenderingExtension for Base {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        world: &World,
        assets: &AssetStore,
        attachments: &AttachmentData,
    ) {
        trace!("Started frame");

        //Update camera first
        let binding = world
            .get_all_components::<components::camera::MainCamera>()
            .expect("Could not find the main camera");

        let mut camera = binding.first().unwrap().borrow_mut();
        camera.update_gpu(encoder);
        trace!("Accquired camera");

        //This is cached, so should be reasonably fast
        let meshes = world
            .get_all_components::<crate::components::mesh::Mesh>()
            .unwrap_or_default();
        trace!("Got all the meshes");

        //List of materials used for rendering
        let mut materials = VecSet::new();
        //List of (mesh_ID, (transformation matrix, material_id));
        let mut matrices = Vec::new();

        //Collect all the matrices
        for m in &meshes {
            let m = m.borrow();
            materials.insert(m.get_material_id().unwrap());
            matrices.push((
                m.get_mesh_id().unwrap(),
                (m.get_matrix(), m.get_material_id().unwrap()),
            ));
        }

        let mut matrices = matrices
            .iter()
            .zip(meshes.into_iter())
            .map(|i| (i.0 .0, (i.0 .1 .0, i.0 .1 .1, i.1)))
            .collect::<Vec<_>>();

        //determine if can re use cache
        let mut identical = true;

        if matrices.len() == self.identifier.len() {
            for (index, data) in self.identifier.iter().enumerate() {
                if data.0 == matrices[index].0 && data.1 == matrices[index].1 .1 {
                    continue;
                } else {
                    identical = false;
                    break;
                }
            }
        } else {
            identical = false;
        }

        if !identical {
            debug!("Generating new cache data");
            self.identifier = matrices.iter().map(|i| (i.0, i.1 .1)).collect::<Vec<_>>();

            //Sort meshes by mesh id for easier buffer creation
            matrices.sort_unstable_by(|a, b| a.0.cmp(&b.0));

            //This is so jank omg
            //Yea... i agree

            //Find points where mesh changes
            let mut split_points = Vec::new();
            let mut old = 0;
            for (index, m) in matrices.iter().enumerate() {
                if m.0 != old {
                    split_points.push(index);
                    old = m.0;
                }
            }

            //Guarantee that there's at least 1 window
            split_points.push(matrices.len());

            //assemble vertex buffers
            let mut v_buffers = Vec::new();

            let device = DEVICE.get().unwrap();

            let mut mesh_materials = Vec::new();
            let mut num_instances = Vec::new();

            let mut mesh_refs = Vec::new();

            for m in split_points.windows(2) {
                //beginning and end of the window
                let points = (*m.first().unwrap(), *m.last().unwrap());

                //Label for easier debugging
                let label = format!("Instances: {}..{}", m.first().unwrap(), m.last().unwrap());

                //(Mesh, (Matrix, Material))
                let mut current_window = matrices[points.0..points.1].iter().collect::<Vec<_>>();

                //Split into vectors and sorted by material
                //Sort the window by materials
                current_window.sort_unstable_by(|s, o| s.1 .1.cmp(&o.1 .1));

                //find where materials change, similar to how meshes were sorted
                let mut material_split_points = Vec::new();
                let mut old = 0;
                for (i, m) in current_window.iter().enumerate() {
                    if m.1 .1 != old {
                        material_split_points.push(i);
                        old = m.1 .1;
                    }
                }
                //Again insure at least one window
                material_split_points.push(current_window.len());

                let mut last = MeshMaterial {
                    mesh_id: 0,
                    material_id: 0,
                };

                //Need to iterate over it twice...
                //Get indicators for every block of what mesh and material they are1
                for i in &material_split_points[..material_split_points.len() - 1] {
                    let curent = current_window[*i];
                    if last != (curent.0, curent.1 .1) {
                        last = MeshMaterial::new(curent.0, curent.1 .1);
                        mesh_materials.push(last);
                    }
                }

                mesh_refs.push(
                    current_window
                        .iter()
                        .map(|i| i.1 .2.clone())
                        .collect::<Vec<_>>(),
                );

                //AGAIN!?!?
                //Create vertex buffers for matrices
                for m in material_split_points.windows(2) {
                    //Now this is stored per mesh per material
                    let points = (*m.first().unwrap(), *m.last().unwrap());

                    num_instances.push(points.1 - points.0);

                    let matrices = current_window
                        .iter()
                        .flat_map(|i| bytemuck::bytes_of(&i.1 .0))
                        .copied()
                        .collect::<Vec<u8>>();
                    v_buffers.push(
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&label),
                            contents: &matrices,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        }),
                    );
                }
            }
            //Check if they're the same length
            assert_eq!(
                v_buffers.len(),
                mesh_materials.len(),
                "You are a moron, they're not the same"
            );
            assert_eq!(
                v_buffers.len(),
                mesh_refs.len(),
                "You are stupid, they're not the same"
            );
            assert_eq!(
                num_instances.len(),
                mesh_materials.len(),
                "You are an idiot, they're not the same"
            );

            self.v_buffers = v_buffers;
            self.mesh_materials = mesh_materials;
            self.num_instances = num_instances;
            self.mesh_refs = mesh_refs;
        } else {
            //Reusing data
            trace!("Cache exists, updating v buffers");
            let mut belt = STAGING_BELT.get().unwrap().write().unwrap();
            let device = DEVICE.get().unwrap();

            for (buffer, meshes) in self.v_buffers.iter().zip(self.mesh_refs.iter()) {
                //I do have to collect here
                let matrices = meshes
                    .iter()
                    .map(|m| m.borrow().get_matrix())
                    .collect::<Vec<_>>();

                let matrix_data = matrices
                    .iter()
                    .flat_map(|m| bytemuck::bytes_of(m))
                    .copied()
                    .collect::<Vec<u8>>();

                belt.write_buffer(
                    encoder,
                    buffer,
                    0,
                    NonZeroU64::new(buffer.size()).unwrap(),
                    device,
                )
                .copy_from_slice(matrix_data.as_slice());
            }
        }

        //Initialize bindgroups for all needed materials
        for m in materials {
            let m = assets.get_by_id::<Material>(m).unwrap();
            let mut m = m.borrow_mut();

            if matches!(m.get_bindgroup_state(), BindgroupState::Initialized) {
                continue;
            }
            m.initialize_bindgroups(assets);
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("First pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &attachments.color,
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
                view: &attachments.depth_stencil,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        //Set the camera
        camera.set_bindgroup(&mut render_pass);

        let mut previous_mat = 0;

        //Iterate through the meshes and render them
        for (i, m) in self.mesh_materials.iter().enumerate() {
            let mat = m.material_id;

            if mat != previous_mat {
                let mat = assets.get_by_id::<Material>(mat).unwrap();
                let mat = mat.borrow();

                mat.render(&mut render_pass);
            }
            previous_mat = mat;

            let mesh = assets.get_by_id::<Mesh>(m.mesh_id).unwrap();
            let mesh = mesh.borrow();

            let vert = unsafe { Arc::as_ptr(&mesh.get_vertex_buffer()).as_ref().unwrap() };
            let ind = unsafe { Arc::as_ptr(&mesh.get_index_buffer()).as_ref().unwrap() };

            render_pass.set_vertex_buffer(0, vert.slice(..));
            render_pass.set_vertex_buffer(1, self.v_buffers[i].slice(..));

            render_pass.set_index_buffer(ind.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(
                0..mesh.get_index_count(),
                0,
                0..(self.num_instances[i] as u32),
            );
        }
        drop(render_pass);
    }

    fn get_order(&self) -> u32 {
        self.order
    }
}
