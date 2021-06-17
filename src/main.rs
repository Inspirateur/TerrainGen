use bevy::prelude::*;
use bevy::render::texture::{Extent3d, TextureDimension, TextureFormat};
use itertools::iproduct;
use noise::{Fbm, NoiseFn, Seedable};
use std::usize;
const SIZE: usize = 600;
// Hydrolic erosion constants
const EVAPORATION: f32 = 0.01;
const INERTIA: f32 = 0.01;
const MINSLOPE: f32 = 0.0001;
const CAPACITY: f32 = 50.0;
const DEPOSITION: f32 = 0.2;
const EROSION: f32 = 0.1;

struct Droplet {
    pos: Vec2,
    dir: Vec2,
    vel: f32,
    water: f32,
    sediment: f32,
}

impl Droplet {
    fn new(pos: Vec2) -> Self {
        Droplet {
            pos: pos,
            dir: Vec2::default(),
            vel: 0.,
            water: 1.,
            sediment: 0.,
        }
    }
}

fn unroll(pos: Vec2, size: usize) -> usize {
    let x = if pos.x < 0. {
        0
    } else if pos.x >= size as f32 {
        size - 1
    } else {
        pos.x as usize
    };
    let y = if pos.y < 0. {
        0
    } else if pos.y >= size as f32 {
        size - 1
    } else {
        pos.y as usize
    };
    x % size + y * size
}
struct Elevation {
    data: Vec<f32>,
    size: usize,
}

impl Elevation {
    fn new(size: usize, noise: Fbm) -> Self {
        let sizef = size as f64;
        Elevation {
            data: iproduct!(0..size, 0..size)
                .map(|(x, y)| (2. * (x as f64) / sizef - 1., 2. * (y as f64) / sizef - 1.))
                .map(|(x, y)| noise.get([x, y]) as f32 - ((x * x + y * y) as f32).sqrt() + 0.5)
                .collect(),
            size: size,
        }
    }

    fn grad(&self, pos: Vec2) -> Vec2 {
        let i = unroll(pos, self.size);
        if i + 1 + self.size >= self.data.len() {
            return Vec2::default();
        }
        Vec2::new(
            (self.data[i + 1] - self.data[i]) * 0.5
                + (self.data[i + 1 + self.size] - self.data[i + self.size]) * 0.5,
            (self.data[i + self.size] - self.data[i]) * 0.5
                + (self.data[i + 1 + self.size] - self.data[i + 1]) * 0.5,
        )
    }

    fn get(&self, pos: Vec2) -> f32 {
        self.data[unroll(pos, self.size)]
    }

    fn add(&mut self, pos: Vec2, v: f32) {
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                let delta = Vec2::new(dx as f32, dy as f32);
                let dist = dx.abs() + dy.abs();
                self.data[unroll(pos + delta, self.size)] += v * if dist == 2 {
                    0.05
                } else if dist == 1 {
                    0.1
                } else {
                    0.4
                };
            }
        }
    }
}

fn new_tex(width: usize, height: usize) -> Texture {
    Texture::new(
        Extent3d::new(width as u32, height as u32, 1),
        TextureDimension::D2,
        vec![0; (width * height * 4) as usize],
        TextureFormat::Rgba8Unorm,
    )
}

fn setup(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let elevation = Elevation::new(SIZE, Fbm::new().set_seed(rand::random::<u32>()));
    let tex = new_tex(SIZE, SIZE);
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(ColorMaterial::texture(textures.add(tex).into())),
            ..Default::default()
        })
        .insert(elevation);
}

fn rain(mut commands: Commands) {
    for _ in 0..20 {
        commands.spawn().insert(Droplet::new(Vec2::new(
            rand::random::<f32>() * SIZE as f32,
            rand::random::<f32>() * SIZE as f32,
        )));
    }
}

fn evaporation(mut commands: Commands, query: Query<(Entity, &Droplet)>) {
    for (entity, droplet) in query.iter() {
        if droplet.water < f32::EPSILON {
            commands.entity(entity).despawn();
        }
    }
}

fn hydrolic_erosion(
    mut query_elevation: Query<&mut Elevation>,
    mut query_droplet: Query<&mut Droplet>,
) {
    if let Ok(mut elevation) = query_elevation.single_mut() {
        for mut droplet in query_droplet.iter_mut() {
            let g = elevation.grad(droplet.pos);
            droplet.dir = (droplet.dir * INERTIA - g * (1. - INERTIA)).normalize();
            let old_pos: Vec2 = droplet.pos;
            droplet.pos = droplet.pos + droplet.dir;
            let h = elevation.get(droplet.pos);
            // if we're below water level we immediatly deposit all sediment and water
            let hdif = elevation.get(old_pos) - h;
            let cdif = f32::max(hdif, MINSLOPE) * droplet.vel * droplet.water * CAPACITY
                - droplet.sediment;
            if cdif < 0. {
                // we deposit sediment
                let deposit = -cdif * DEPOSITION;
                droplet.sediment = droplet.sediment - deposit;
                elevation.add(old_pos, deposit);
            } else {
                // we draw sediment
                let erosion = f32::min(cdif * EROSION, hdif);
                droplet.sediment = droplet.sediment + erosion;
                elevation.add(old_pos, -erosion);
            }
            droplet.vel = (droplet.vel.powi(2) + hdif).max(0.).sqrt();
            droplet.water = droplet.water * (1. - EVAPORATION);
        }
    }
}

fn draw(
    query: Query<(&Elevation, &Handle<ColorMaterial>)>,
    materials: Res<Assets<ColorMaterial>>,
    mut textures: ResMut<Assets<Texture>>,
) {
    if let Ok((elevation, mat_handle)) = query.single() {
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
                data[i * 4] = vu;
                data[i * 4 + 1] = vu;
                data[i * 4 + 2] = vu;
                data[i * 4 + 3] = 255;
            }
        }
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(rain.system())
        .add_system(evaporation.system())
        .add_system(hydrolic_erosion.system())
        .add_system(draw.system())
        .run();
}
