use bevy::{ecs::system::{ParamSet, SystemParam}, prelude::*};

/// This example creates a SystemParam struct that counts the number of players
fn main() {
    App::new()
        .insert_resource::<usize>(0)
        .add_startup_system(spawn)
        .run();
}



/// Spawn some players to count
fn spawn(mut test: ParamSet<(ResMut<usize>, ResMut<usize>)>) {
    test.p0();
}

