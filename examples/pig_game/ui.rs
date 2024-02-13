use bevy::prelude::*;

use crate::Money;

pub struct GameUiPlugin;

#[derive(Component)]
struct MoneyText;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui_system);
        app.add_systems(Update, update_money_ui_system);
    }
}

fn spawn_ui_system(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(10.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                background_color: Color::BLUE.into(),
                ..default()
            },
            Name::new("Ui Root"),
        ))
        .with_children(|commands| {
            commands.spawn((
                TextBundle {
                    text: Text::from_section(
                        "Gold!",
                        TextStyle {
                            font_size: 32.0,
                            ..default()
                        },
                    ),
                    ..default()
                },
                MoneyText,
                Name::new("Money Text"),
            ));
        });
}

fn update_money_ui_system(mut money_text: Query<&mut Text, With<MoneyText>>, money: Res<Money>) {
    let mut text = money_text.single_mut();

    text.sections[0].value = format!("Gold {}", money.0)
}
