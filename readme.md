# hana_prefab 
**A plugin for storing data**

Hana prefab is a plugin for bevy that allows you to store level and other data in rooms using prefabs created in code. This plugin was created because I did not get the built in scenes in bey to work and they were in an incomplete state.
## Overview

### Prefab
A prefab is predefined game object or resource that can be loaded from a ron file. The way this is done by an interface from the varibles declared in the ron file and compnents and entities in game. 

### Room
A room is a collection of prefabs and resources declared in a [ron](https://github.com/ron-rs/ron) file. 

```rust
(
    prefabs: {
        "player" : (
            type: "Player",
            fields: {
                "sprite" : String("sprites/bevy-icon.png"),
                "position" : String("(0, 0)"),
                "speed" : Number(300.0),
                "alive" : Bool(true),
            }
        ),
        "pig_parent" : (
            type: "PigParent",
            fields: { },
       )
    }
)
```
The room file is made of a root object with a filed called `prefabs` which is a map type. The prefabs map contains prefab objects which has two fields, `type` and `fields`. Type declares which prefab type the object is and fileds contains the fields of the prefab.
The fields are of a enum type with the following variants
```rust
pub enum PrefabField {
    Vec2(f32)
    Number(f32),
    Bool(bool),
    String(String),
}
```
