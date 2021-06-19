use crate::SIZE;
use bevy::prelude::*;
use itertools::iproduct;
use noise::{Fbm, NoiseFn, Seedable};
use std::usize;

// Hydrolic erosion constants
const EVAPORATION: f32 = 0.05;
const INERTIA: f32 = 0.1;
const MINSLOPE: f32 = 0.;
const CAPACITY: f32 = 800.0;
const DEPOSITION: f32 = 0.1;
const EROSION: f32 = 0.01;

fn rand_pos() -> Vec2 {
    Vec2::new(
        rand::random::<f32>() * SIZE as f32,
        rand::random::<f32>() * SIZE as f32,
    )
}

pub struct Source {
    pub pos: Vec2,
    flux: f32,
    stock: f32,
}

impl Source {
    fn new(pos: Vec2, flux: f32) -> Self {
        Source {
            pos: pos,
            flux: flux,
            stock: 0.,
        }
    }

    fn flow(&mut self) -> u32 {
        self.stock = self.stock + self.flux;
        let drops = self.stock.floor();
        self.stock = self.stock - drops;
        drops as u32
    }
}

pub struct Droplet {
    pub pos: Vec2,
    dir: Vec2,
    vel: f32,
    pub water: f32,
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

pub fn unroll(pos: Vec2, size: usize) -> usize {
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
pub struct Elevation {
    pub data: Vec<f32>,
    size: usize,
}

impl Elevation {
    fn new(size: usize, noise: Fbm) -> Self {
        let sizef = size as f32;
        Elevation {
            data: iproduct!(0..size, 0..size)
                .map(|(x, y)| (2. * (x as f32) / sizef - 1., 2. * (y as f32) / sizef - 1.))
                .map(|(x, y)| {
                    noise.get([x as f64, y as f64]) as f32 - ((x * x + y * y) as f32).sqrt() + 0.5
                })
                //.map(|(x, y)| (x as f32 / sizef) * (y as f32 / sizef).max(0.5))
                .collect(),
            size: size,
        }
    }

    pub fn grad(&self, mut i: usize) -> Vec2 {
        if i % self.size == self.size - 1 {
            i -= 1;
        }
        if i + self.size >= self.data.len() {
            i -= self.size;
        }
        Vec2::new(
            (self.data[i + 1] - self.data[i]) * 0.5
                + (self.data[i + 1 + self.size] - self.data[i + self.size]) * 0.5,
            (self.data[i + self.size] - self.data[i]) * 0.5
                + (self.data[i + 1 + self.size] - self.data[i + 1]) * 0.5,
        )
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

fn setup_elevation(mut commands: Commands) {
    let elevation = Elevation::new(SIZE, Fbm::new().set_seed(rand::random::<u32>()));
    // initialize the sources
    let mut count = 0;
    for _ in 0..400 {
        let pos = rand_pos();
        if elevation.data[unroll(pos, SIZE)] > 0.3 {
            count += 1;
            commands.spawn().insert(Source::new(pos, 0.01));
        }
    }
    println!("{} rivers", count);
    // initialize the texture
    commands.spawn().insert(elevation);
}

fn rain(mut commands: Commands) {
    for _ in 0..5 {
        commands.spawn().insert(Droplet::new(rand_pos()));
    }
}

fn flows(mut commands: Commands, mut query: Query<&mut Source>) {
    for mut source in query.iter_mut() {
        let drops = source.flow();
        for _ in 0..drops {
            commands.spawn().insert(Droplet::new(source.pos));
        }
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
            let g = elevation.grad(unroll(droplet.pos, SIZE));
            droplet.dir = (droplet.dir * INERTIA * droplet.vel - g * (1. - INERTIA * droplet.vel))
                .normalize();
            let old_pos: Vec2 = droplet.pos;
            droplet.pos = droplet.pos + droplet.dir;
            let h = elevation.data[unroll(droplet.pos, SIZE)];
            // if we're below water level we immediatly deposit all sediment and water
            let hdif = elevation.data[unroll(old_pos, SIZE)] - h;
            let cdif = f32::max(hdif, MINSLOPE) * droplet.vel * droplet.water * CAPACITY
                - droplet.sediment;
            if cdif < 0. {
                // we deposit sediment
                let deposit = -cdif * DEPOSITION;
                droplet.sediment = droplet.sediment - deposit;
                elevation.add(old_pos, deposit);
            } else if h >= 0. {
                // we draw sediment if we're above water
                let erosion = f32::min(cdif * EROSION, hdif);
                droplet.sediment = droplet.sediment + erosion;
                elevation.add(old_pos, -erosion);
            }
            droplet.vel = (droplet.vel.powi(2) + hdif).max(0.).sqrt();
            droplet.water = droplet.water * (1. - EVAPORATION * (1. - droplet.vel));
        }
    }
}

pub struct Erosion;

impl Plugin for Erosion {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_elevation.system())
            .add_system(rain.system())
            .add_system(flows.system())
            .add_system(evaporation.system())
            .add_system(hydrolic_erosion.system());
    }
}
