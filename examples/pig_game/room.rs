use bevy::{
    asset::{AssetLoader, LoadedAsset},
    ecs::system::EntityCommands,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::{HashMap, HashSet},
};
use serde::{Deserialize, Serialize};

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Room>();
        app.init_resource::<PrefabRegistry>();
        app.init_resource::<RoomTracker>();
        app.add_asset_loader(RoomLoader);
        app.add_systems(Update, parse_room_system);
    }
}

#[derive(Deserialize, TypePath, TypeUuid, Debug)]
#[uuid = "23c6bd8f-a194-43f3-93be-9a3f95354c7f"]
pub struct Room {
    prefabs: HashMap<String, PrefabData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrefabData {
    #[serde(rename = "type")]
    pub prefab_type: String,
    pub fields: HashMap<String, PrefabField>,
}

impl PrefabData {
    fn get_changed_fields(
        old_prefab: &PrefabData,
        new_prefab: &PrefabData,
    ) -> HashMap<String, PrefabField> {
        if old_prefab.prefab_type != new_prefab.prefab_type {
            warn!("trying to find changed fields of prefabs of different types (old_prefab: {}, new_prefab: {})", old_prefab.prefab_type, new_prefab.prefab_type);
            return HashMap::new();
        }

        new_prefab
            .fields
            .iter()
            .filter_map(|(key, field)| match old_prefab.fields.get(key) {
                Some(other_field) => {
                    if other_field != field {
                        Some((key.clone(), field.clone()))
                    } else {
                        None
                    }
                }
                None => Some((key.clone(), field.clone())),
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum PrefabField {
    String(String),
    Number(f64),
    Bool(bool),
}

pub trait Prefab {
    fn spawn_prfab(
        &self,
        fields: &HashMap<String, PrefabField>,
        commands: EntityCommands,
        asset_server: &AssetServer,
    );

    fn update_prfab(
        &self,
        changed_fields: &HashMap<String, PrefabField>,
        asset_server: &AssetServer,
        commands: EntityCommands,
    );
}

/// The assetloader
#[derive(Default)]
pub struct RoomLoader;

impl AssetLoader for RoomLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            debug!("Loading room at {:?}", load_context.path());
            let room = ron::de::from_bytes::<Room>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(room));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron", "room"]
    }
}

/// Tracks which rooms are currently being loaded.
#[derive(Resource, Default)]
struct RoomTracker {
    rooms: HashMap<Handle<Room>, HashMap<String, (Entity, PrefabData)>>,
}

/// Checks wether a room asset has been loaded and the parses that room asset and spawns the prefab it has
fn parse_room_system(
    mut asset_events: EventReader<AssetEvent<Room>>,
    mut commands: Commands,
    registry: Res<PrefabRegistry>,
    mut room_tracker: ResMut<RoomTracker>,
    room_assets: Res<Assets<Room>>,
    asset_server: Res<AssetServer>,
) {
    for event in asset_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                debug!("Room loaded parsing room. Room:{:?}", handle);
                let room = room_assets.get(handle).unwrap();

                let entities = room
                    .prefabs
                    .iter()
                    .map(|(id, prefab_data)| {
                        let commands = commands.spawn_empty();
                        let entity = commands.id();
                        registry.spawn(prefab_data, commands, &asset_server);
                        (id.clone(), (entity, prefab_data.clone()))
                    })
                    .collect();

                room_tracker.rooms.insert(handle.clone_weak(), entities);
            }
            AssetEvent::Modified { handle } => {
                debug!("Room modified, reparsing room. Room:{:?}", handle);

                let room = room_assets.get(handle).unwrap();

                let entities: HashMap<String, (Entity, PrefabData)> = room.prefabs.iter().map(|(id, new_prefab)| {
                    match room_tracker.rooms[handle].get(id) {
                        Some((entity, old_prefab)) => {
                            let changed_fields =
                                PrefabData::get_changed_fields(old_prefab, new_prefab);

                            registry.update(
                                &new_prefab.prefab_type,
                                changed_fields,
                                commands.entity(entity.clone()),
                                &asset_server,
                            );

                            (id.clone(), (entity.clone(), new_prefab.clone()))
                        }
                        None => {
                            let commands = commands.spawn_empty();
                            let entity = commands.id();
                            registry.spawn(new_prefab, commands, &asset_server);
                            (id.clone(), (entity, new_prefab.clone()))
                        },
                    }
                }).collect();

                let room_keys : HashSet<&String> = room_tracker.rooms[handle].keys().collect();
                let new_room_keys = entities.keys().collect();

                let diff = room_keys.difference(&new_room_keys);

                let remove_count = diff
                    .map(|key| room_tracker.rooms[handle][*key].0)
                    .map(|entity| commands.entity(entity).despawn())
                    .count();

                debug!("Removed {} entities", remove_count);

                drop(room_keys);
                drop(new_room_keys);

                room_tracker.rooms.insert(handle.clone_weak(), entities);
            }
            AssetEvent::Removed { handle } => {
                debug!("Room with handle {handle:?} removed");
                if let Some(entities) = room_tracker.rooms.remove(handle) {
                    for (_, (entity, _)) in entities {
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

/// A struct that tracks the spawn functions for all available prefabs
#[derive(Default, Resource)]
pub struct PrefabRegistry {
    prefabs: HashMap<String, Box<dyn Prefab + Sync + Send>>,
}

impl PrefabRegistry {
    /// Register a prefab to the registry
    pub fn register_prefab(&mut self, name: &str, prefab: impl Prefab + Sync + Send + 'static) {
        self.prefabs.insert(name.to_string(), Box::new(prefab));
    }

    /// Calls the correct spawn function for a prefab of given type
    pub fn spawn(
        &self,
        prefab_data: &PrefabData,
        commands: EntityCommands,
        asset_server: &AssetServer,
    ) {
        self.prefabs[&prefab_data.prefab_type].spawn_prfab(
            &prefab_data.fields,
            commands,
            asset_server,
        )
    }

    /// Calls the correct update function prefab
    pub fn update(
        &self,
        prefab_type: &String,
        changed_fields: HashMap<String, PrefabField>,
        commands: EntityCommands,
        asset_server: &AssetServer,
    ) {
        self.prefabs[prefab_type].update_prfab(&changed_fields, asset_server, commands);
    }
}
