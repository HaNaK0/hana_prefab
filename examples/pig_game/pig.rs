use bevy::prelude::*;

use crate::{Money, Player};

pub struct PigPlugin;

impl Plugin for PigPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_pig_system, update_pig_system));
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct Pig {
    lifetime: Timer,
}

#[derive(Component)]
pub struct PigParent;

fn spawn_pig_system(
    mut commands: Commands,
    mut money: ResMut<Money>,
    player: Query<&Transform, With<Player>>,
    pig_parent: Query<Entity, With<PigParent>>,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if !keyboard_input.just_pressed(KeyCode::Space) || money.0 < 10.0 {
        return;
    }

    money.0 -= 10.0;

    let texture = asset_server.load("sprites/Animals/pig.png");
    let player_transform = player.single();
    let pig_parent = pig_parent.single();

    commands.entity(pig_parent).with_children(|commands| {
        commands.spawn((
            SpriteBundle {
                texture,
                transform: *player_transform,
                ..default()
            },
            Pig {
                lifetime: Timer::from_seconds(2.0, TimerMode::Once),
            },
            Name::new("Pig"),
        ));
    });

    info!("you bought a pig and now have {} gold", money.0);
}

fn update_pig_system(
    mut commands: Commands,
    mut pigs: Query<(Entity, &mut Pig)>,
    mut money: ResMut<Money>,
    time: Res<Time>,
) {
    for (entity, mut pig) in &mut pigs {
        pig.lifetime.tick(time.delta());

        if pig.lifetime.finished() {
            commands.entity(entity).despawn();
            money.0 += 15.0;

            info!(
                "A pig died and you sold its meat and now have {} gold",
                money.0
            )
        }
    }
}
