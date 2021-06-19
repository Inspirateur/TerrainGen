use crate::erosion::Elevation;
use crate::SIZE;
use bevy::math::f32;
use bevy::{
    prelude::*,
    render::{
        camera::Camera,
        mesh::Indices,
        pipeline::{PipelineDescriptor, PrimitiveTopology, RenderPipeline},
        shader::{ShaderStage, ShaderStages},
    },
};

use std::ops::Rem;
const HEIGHTMULT: f32 = 60.;
use itertools::iproduct;
const VERTEX_SHADER: &str = r"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Color;
layout(location = 1) out vec3 v_Color;
layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};
void main() {
    v_Color = Vertex_Color;
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
}
";

const FRAGMENT_SHADER: &str = r"
#version 450
layout(location = 1) in vec3 v_Color;
layout(location = 0) out vec4 o_Target;
void main() {
    o_Target = vec4(v_Color, 1.0);
}
";

fn setup_draw3d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        // Vertex shaders are run once for every vertex in the mesh.
        // Each vertex can have attributes associated to it (e.g. position,
        // color, texture mapping). The output of a shader is per-vertex.
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        // Fragment shaders are run for each pixel belonging to a triangle on
        // the screen. Their output is per-pixel.
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));
    // Create the mesh
    let size = SIZE as u32;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    let v_pos = iproduct!(0..size, 0..size)
        .map(|(x, y)| [x as f32, 0., y as f32])
        .collect::<Vec<[f32; 3]>>();
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; v_pos.len()]);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0, 0.0]; v_pos.len()]);
    mesh.set_attribute("Vertex_Color", vec![[0., 0., 0.]; v_pos.len()]);
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

    mesh.set_indices(Some(Indices::U32(
        iproduct!(0..size - 1, 0..size - 1)
            .map(|(x, y)| x % size + y * size)
            .flat_map(|i| {
                IntoIterator::into_iter([i, i + 1, i + size, i + 1, i + 1 + size, i + size])
            })
            .collect(),
    )));
    commands.spawn_bundle(MeshBundle {
        mesh: meshes.add(mesh),
        render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
            pipeline_handle,
        )]),
        ..Default::default()
    });
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz((SIZE / 2) as f32, 50., -100.)
            .looking_at(Vec3::new((SIZE / 2) as f32, 0., (SIZE / 2) as f32), Vec3::Y),
        ..Default::default()
    });
}

fn draw3d(
    query_elevation: Query<&Elevation>,
    query_mesh: Query<&Handle<Mesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if let Ok(elevation) = query_elevation.single() {
        if let Ok(mesh_handle) = query_mesh.single() {
            let mesh = &mut *meshes.get_mut(mesh_handle.id).unwrap();
            let v_pos = iproduct!(0..SIZE, 0..SIZE)
                .map(|(x, y)| {
                    [
                        x as f32,
                        elevation.data[y % SIZE + x * SIZE].max(0.) * HEIGHTMULT,
                        y as f32,
                    ]
                })
                .collect::<Vec<[f32; 3]>>();
            mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, v_pos.clone());
            mesh.set_attribute(
                "Vertex_Color",
                elevation
                    .data
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, h)| (h, elevation.grad(i).length()))
                    .map(|(h, g)| {
                        if h < f32::EPSILON {
                            [0.01, 0.05, 0.2]
                        } else if g > 0.01 {
                            [h * 0.8, h * 0.6, h * 0.5]
                        } else if h > 0.03 {
                            [h / 4., h, h / 3.]
                        } else {
                            [0.8, 0.9, 0.2]
                        }
                    })
                    .collect::<Vec<[f32; 3]>>(),
            );
        }
    }
}

fn rotate_cam(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    let hsize = (SIZE / 2) as f32;
    for mut transform in query.iter_mut() {
        let alpha = (time.seconds_since_startup() as f32 / 10.).rem(2. * std::f32::consts::PI);
        *transform = Transform::from_xyz(
            hsize + alpha.cos() * hsize,
            50.,
            hsize + alpha.sin() * hsize,
        )
        .looking_at(Vec3::new(hsize, 0., hsize), Vec3::Y);
    }
}
pub struct Draw3d;

impl Plugin for Draw3d {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_draw3d.system())
            .add_system(rotate_cam.system())
            .add_system(draw3d.system());
    }
}
