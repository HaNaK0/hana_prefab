use bevy::{
    asset::{AssetLoader, AsyncReadExt},
    ecs::system::EntityCommands,
    prelude::*,
    reflect::TypePath,
    utils::{HashMap, HashSet},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The plugin that handles the loading and tracking of rooms and prefabs
pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Room>();
        app.init_resource::<PrefabRegistry>();
        app.init_resource::<RoomTracker>();
        app.init_asset_loader::<RoomLoader>();
        app.add_systems(Update, room_system);
    }
}

/// A struct that contains an ammount of prefabs, each room is defined in a ron file
#[derive(Deserialize, TypePath, Asset, Debug)]
pub struct Room {
    prefabs: HashMap<String, PrefabData>,
}

/// A struct containing the data of a single prefab field
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

/// An enum used for determining type of a field.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum PrefabField {
    Number(f32),
    Bool(bool),
    Vec2(f32, f32),
    String(String),
    None,
}

/// All prefabs that should be loaded from a room needs to imlpement the prefab trait.
pub trait Prefab {
    /// The method that is called when a prefab is loaded for the first time and needs to be spawned into the world
    fn spawn_prfab(
        &self,
        fields: &HashMap<String, PrefabField>,
        commands: EntityCommands,
        asset_server: &AssetServer,
    );

    /// The method that is called when a prefab was changed in the ron file
    fn update_prfab(
        &self,
        changed_fields: &HashMap<String, PrefabField>,
        asset_server: &AssetServer,
        commands: EntityCommands,
    );
}

/// The assetloader for the room asset
#[derive(Default)]
pub struct RoomLoader;

#[derive(Error, Debug)]
pub enum RoomLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    /// A [RON](ron) Error
    #[error("Could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

impl AssetLoader for RoomLoader {
    type Asset = Room;
    type Settings = ();
    type Error = RoomLoaderError;

    fn extensions(&self) -> &[&str] {
        &["ron", "room"]
    }

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let room = ron::de::from_bytes::<Room>(&bytes)?;
            Ok(room)
        })
    }
}

/// Tracks which rooms are currently being loaded.
#[derive(Resource, Default)]
struct RoomTracker {
    rooms: HashMap<AssetId<Room>, HashMap<String, (Entity, PrefabData)>>,
}

/// Tracks rooms and whenever changes happens to a room
fn room_system(
    mut asset_events: EventReader<AssetEvent<Room>>,
    mut commands: Commands,
    registry: Res<PrefabRegistry>,
    mut room_tracker: ResMut<RoomTracker>,
    room_assets: Res<Assets<Room>>,
    asset_server: Res<AssetServer>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id: handle } => {
                debug!("Room loaded parsing room. Room:{:?}", handle);
                let room = room_assets.get(*handle).unwrap();

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

                room_tracker.rooms.insert(handle.clone(), entities);
            }
            AssetEvent::Modified { id } => {
                debug!("Room modified, reparsing room. Room:{:?}", id);

                let room = room_assets.get(*id).unwrap();

                let entities: HashMap<String, (Entity, PrefabData)> = room
                    .prefabs
                    .iter()
                    .map(
                        |(prefab_id, new_prefab)| match room_tracker.rooms[id].get(prefab_id) {
                            Some((entity, old_prefab)) => {
                                let changed_fields =
                                    PrefabData::get_changed_fields(old_prefab, new_prefab);

                                registry.update(
                                    &new_prefab.prefab_type,
                                    changed_fields,
                                    commands.entity(entity.clone()),
                                    &asset_server,
                                );

                                (prefab_id.clone(), (entity.clone(), new_prefab.clone()))
                            }
                            None => {
                                let commands = commands.spawn_empty();
                                let entity = commands.id();
                                registry.spawn(new_prefab, commands, &asset_server);
                                (prefab_id.clone(), (entity, new_prefab.clone()))
                            }
                        },
                    )
                    .collect();

                let room_keys: HashSet<&String> = room_tracker.rooms[id].keys().collect();
                let new_room_keys = entities.keys().collect();

                let diff = room_keys.difference(&new_room_keys);

                let remove_count = diff
                    .map(|key| room_tracker.rooms[id][*key].0)
                    .map(|entity| commands.entity(entity).despawn())
                    .count();

                debug!("Removed {} entities", remove_count);

                drop(room_keys);
                drop(new_room_keys);

                room_tracker.rooms.insert(id.clone(), entities);
            }
            AssetEvent::Unused { id } => {
                debug!("Room with handle {id:?} is unused and will be despawned");
                if let Some(entities) = room_tracker.rooms.remove(id) {
                    for (_, (entity, _)) in entities {
                        commands.entity(entity).despawn();
                    }
                }
            }
            AssetEvent::Removed { id } => {
                debug!("Room with id {id:?} removed");
                if let Some(entities) = room_tracker.rooms.remove(id) {
                    for (_, (entity, _)) in entities {
                        commands.entity(entity).despawn();
                    }
                }
            }
            AssetEvent::LoadedWithDependencies { id } => {
                debug!("Room {id:?} loaded with dependencies")
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
    /// Register a prefab to the registry, all prefabs that are going to be loaded needs to be registered before loading.
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
