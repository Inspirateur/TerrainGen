mod draw2d;
mod draw3d;
mod erosion;
use bevy::prelude::*;
use draw2d::Draw2d;
use draw3d::Draw3d;
use erosion::Erosion;
use std::usize;
pub const SIZE: usize = 512;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(Draw3d)
        .add_plugin(Erosion)
        .run();
}
