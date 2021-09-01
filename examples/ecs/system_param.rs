use bevy::{
    ecs::system::{ParamSet, SystemParam},
    prelude::*,
};

/// This example creates a SystemParam struct that counts the number of players
fn main() {
    App::new()
        .insert_resource::<usize>(0)
        .add_startup_system(spawn)
        .run();
}

/// Spawn some players to count
fn spawn(mut test: ParamSet<(ResMut<usize>, ResMut<usize>)>) {
    let mut res = test.p0();
    *res += 1;
    println!("res now {}", *res);

    let mut res2 = test.p1();
    *res2 += 1;
    println!("res then {}", *res2);
}
