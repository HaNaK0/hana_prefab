use std::time::Duration;

use bevy::{
    asset::ChangeWatcher, input::common_conditions::input_toggle_active, prelude::*,
    render::camera::ScalingMode, ecs::entity,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use pig::PigPlugin;
use prefab::DefaultPrefabsPlugin;
use room::RoomPlugin;
use ui::GameUiPlugin;

use crate::room::Room;

mod pig;
mod prefab;
mod room;
mod ui;

#[derive(Component)]
struct Player {
    speed: f32,
}

#[derive(Resource)]
struct Money(f32);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (640.0, 480.0).into(),
                        resizable: true,
                        title: "rustiant".to_string(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    watch_for_changes: ChangeWatcher::with_delay(Duration::from_secs_f32(1.0)),
                    ..default()
                }),
            PigPlugin,
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F3)),
            GameUiPlugin,
            RoomPlugin,
            DefaultPrefabsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (movement_system, remove_rooms))
        .insert_resource(Money(100.0))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    debug!("set up");
    let mut camera = Camera2dBundle::default();

    camera.projection.scaling_mode = ScalingMode::AutoMin {
        min_width: 1920.,
        min_height: 1080.,
    };

    commands.spawn(camera);

    let room: Handle<Room> = asset_server.load("rooms/test_room.ron");
    commands.spawn((Name::new("main_room"), room));
}

fn movement_system(
    mut players: Query<(&mut Transform, &Player)>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    for (mut transform, player) in &mut players {
        if keyboard_input.pressed(KeyCode::A) {
            transform.translation.x -= player.speed * time.delta_seconds();
        }
        if keyboard_input.pressed(KeyCode::D) {
            transform.translation.x += player.speed * time.delta_seconds();
        }
        if keyboard_input.pressed(KeyCode::W) {
            transform.translation.y += player.speed * time.delta_seconds();
        }
        if keyboard_input.pressed(KeyCode::S) {
            transform.translation.y -= player.speed * time.delta_seconds();
        }
    }
}

fn remove_rooms(
    rooms: Query<(Entity, With<Handle<Room>>)>,
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Q) {
        for (entity, _) in &rooms {
            if let Some(mut commands) = commands.get_entity(entity) {
                info!("Despawned room entity{entity:?}");
                commands.despawn();
            } else {
                warn!("Tried to get entity that did not exist")
            }
        }
    }
}
