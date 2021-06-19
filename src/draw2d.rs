use crate::erosion::{unroll, Droplet, Elevation, Source};
use crate::SIZE;
use bevy::prelude::*;
use bevy::render::texture::{Extent3d, TextureDimension, TextureFormat};

fn new_tex(width: usize, height: usize) -> Texture {
    Texture::new(
        Extent3d::new(width as u32, height as u32, 1),
        TextureDimension::D2,
        vec![0; (width * height * 4) as usize],
        TextureFormat::Rgba8Unorm,
    )
}

fn setup_draw2d(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let tex = new_tex(SIZE, SIZE);
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(ColorMaterial::texture(textures.add(tex).into())),
        ..Default::default()
    });
}

fn draw2d(
    query_elevation: Query<&Elevation>,
    query_sources: Query<&Source>,
    query_droplets: Query<&Droplet>,
    query_mat: Query<&Handle<ColorMaterial>>,
    materials: Res<Assets<ColorMaterial>>,
    mut textures: ResMut<Assets<Texture>>,
) {
    if let Ok(elevation) = query_elevation.single() {
        if let Ok(mat_handle) = query_mat.single() {
            let data = &mut *textures
                .get_mut(
                    materials
                        .get(mat_handle.id)
                        .unwrap()
                        .texture
                        .as_ref()
                        .unwrap()
                        .id,
                )
                .unwrap()
                .data;

            for (i, v) in elevation.data.iter().enumerate() {
                if *v < 0. {
                    data[i * 4] = 0;
                    data[i * 4 + 1] = 0;
                    data[i * 4 + 2] = 0;
                    data[i * 4 + 3] = 255;
                } else {
                    let vu = (*v * 255.) as u8;

                    if elevation.grad(i).length() > 0.008 {
                        // draw rocks
                        data[i * 4] = vu;
                        data[i * 4 + 1] = vu / 2;
                        data[i * 4 + 2] = vu / 3;
                        data[i * 4 + 3] = 255;
                    } else {
                        // draw herb
                        data[i * 4] = vu / 4;
                        data[i * 4 + 1] = vu;
                        data[i * 4 + 2] = vu / 3;
                        data[i * 4 + 3] = 255;
                    }
                }
            }
            for source in query_sources.iter() {
                let i = unroll(source.pos, SIZE);
                data[i * 4] = 255;
                data[i * 4 + 1] = 0;
                data[i * 4 + 2] = 0;
            }
            for droplet in query_droplets.iter() {
                let i = unroll(droplet.pos, SIZE);
                if data[i * 4] > 0 {
                    let w = (255. * droplet.water) as u8;
                    let v = (data[i * 4] as f32 * (1. - droplet.water)) as u8;
                    data[i * 4] = v;
                    data[i * 4 + 1] = v;
                    data[i * 4 + 2] = w + v;
                }
            }
        }
    }
}

pub struct Draw2d;

impl Plugin for Draw2d {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_draw2d.system())
            .add_system(draw2d.system());
    }
}
