# [Collision Layer](https://community.bistudio.com/wiki/Arma_Reforger:Collision_Layer)

## Layers Setup

* Buoyancy
* Camera - Special layer for camera collision
* Character - For collider capsule of characters
* CharacterAI - For collider capsule of AI characters
* CharCollide - Used for Character-only collisions (e.g. wheels, which are otherwise raytraced)
* CharNoCollide - Only for objects, which are NOT supposed to collide with character
* Cover *- deprecated?*
* Debris - Special layer for debris, we want it to collide with e.g. static, but not dynamic objects or character
* Default - Default layer, will collide with everything
* Dynamic - For dynamic objects like props are moveable in world
* FireGeometry - Used only for damage, projectile tracing, etc.
* Foliage - Layer used foliage on vegetation (mainly on trees)
* Interaction - Special layer for interaction (usually not really used, only for special cases when any other collision layer used for interaction wouldn't be enough)
* Ladder - For detection of ladders vs character (used for get in on ladder)
* Navmesh - Navmesh layer for soldiers
* NavmeshVehicle - Navmesh layer for vehicles
* Perception - For trace to collide with ViewGeometry layer
* Projectile - for bigger projectiles (bigger than bullet - e.g. RPG rocket)
* Ragdoll - Special layer for ragdoll, e.g. character shouldn't collide with dead soldiers, when they are in ragdoll
* Static - For static objects like buildings
* Terrain - Special layers just for terrain
* Vegetation - use for "soft" parts of vegetation - e.g. leaves
* Vehicle - Original layer for vehicles
* VehicleCast - used by wheels
* VehicleSimple - used to collision between vehicles and static assets, should be more "rough" and made from as little as possible colliders
* VehicleComplex - used for collision between vehicles and characters

* ViewGeometry - Used only for perception (AI)
* Water -

## LayerPresets Setup

(this is what must be set in the value of "usage" custom property in FBX)

* **Main**
  + Default
* **Building**
  + Static
  + Navmesh
* **BuildingFire**
  + Static
  + FireGeometry
  + Navmesh
* **BuildingFireView**
  + Static
  + FireGeometry
  + ViewGeometry
  + Navmesh
* **Bush**
  + Foliage
* **Cover**
  + Cover
* **Character**
  + Character
* **CharacterAI**
  + CharacterAI
* **CharNoCollide**
  + CharNoCollide
* **Debris**
  + Debris
* **Door**
  + Static
* **DoorFireView**
  + Static
  + FireGeometry
  + ViewGeometry

* **FireGeo**
  + FireGeometry
* **FireView**
  + FireGeometry
  + ViewGeometry
* **Foliage**
  + Foliage
* **Interaction**
  + Interaction
* **ItemFireView**
  + CharNoCollide
  + FireGeometry
  + ViewGeometry
* **Ladder**
  + Static
  + Ladder
* **Projectile**
  + Projectile
* **Prop**
  + Dynamic
* **PropView**
  + Dynamic
  + ViewGeometry
* **PropFireView**
  + Dynamic
  + FireGeometry
  + ViewGeometry
* **RockFireView**
  + Static
  + FireGeometry
  + ViewGeometry
  + Navmesh
* **Terrain**
  + Terrain
  + Navmesh

* **Tree**
  + Static
  + Foliage
* **TreeFireView**
  + Static
  + FireGeometry
  + ViewGeometry
* **TreePart**
  + Dynamic
* **Vehicle**
  + Vehicle
* **VehicleFire**
  + Vehicle
  + FireGeometry
* **VehicleFireView**
  + Vehicle
  + FireGeometry
  + ViewGeometry
* **Weapon**
  + CharNoCollide
* **Wheel**
  + Static
  + FireGeometry

| LayerPresets | List of active layers | Description | Examples of usage |
| --- | --- | --- | --- |
| Main | Default | Default LayerPreset | Not really used anywhere |
| Cover | Cover | To be removed? | |
| Character | Character | Used on character colliders | |
| CharacterAI | CharacterAI | Special layerpreset with layer just for AI (they don't collide with vehicles for now, etc.) | |
| Projectile | Projectile | Used by projectiles (bullets, rockets), interacts with FireGeometry | |
| Vehicle | Vehicle | Collides with static and dynamic objects like buildings, props, other vehicles, etc. | On vehicles |
| VehicleFire | Vehicle FireGeometry | Same as above + Fire Geometry layer | On vehicles parts which can used both for collision and fire geometry |
| VehicleFireView | Vehicle FireGeometry  ViewGeometry | Same as above + View Geometry layer | On vehicles parts which can used for collision, Fire and View |
| ItemFireView | CharNoCollide FireGeometry  ViewGeometry | Used on props, items where collision with player is not desired - e.g. really small objects like chocolate bar | |
| Door | Static Navmesh | Used on doors | |
| DoorFireView | Static FireGeometry  ViewGeometry  Navmesh | Used on doors, | Same as above |
| Weapon | CharNoCollide | Used on weapons | |
| Terrain | Terrain Navmesh | Special preset for terrain mesh, is set in GenericTerrainEntity | On terrain meshes |
| Prop | Dynamic | Used on dynamic assets - props | Used on assets like e.g. bottle, ammo box, chair, etc. |
| PropView | Dynamic ViewGeometry | + View Geometry layer | |
| PropFireView | Dynamic FireGeometry  ViewGeometry  NavmeshVehicle | Used on props which has only one geometry (no separated Fire geometry) | |
| Tree | Static NavmeshVehicle | Used on tree/bush trunks (hard parts) | |
| TreeFireView | Static FireGeometry  ViewGeometry  NavmeshVehicle | Used on tree/bush trunks (hard parts) where one collider can be used for both collision with soldier and then as Fire Geometry | |
| TreePart | Dynamic | Used on tree parts - branches, twigs, etc. | |
| CharNoCollide | CharNoCollide | Not used? | |
| FireGeo | FireGeometry | Collides only with projectiles | Used on Fire Geometry type of colliders |
| Building | Static Navmesh | Used on buildings as they use static colliders (+ special layer for detecting for Navmesh) | |
| BuildingFire | Static FireGeometry  Navmesh | + Fire and View Geometry layer | Used on buildings |
| BuildingFireView | Static FireGeometry  ViewGeometry  Navmesh | + View Geometry layer | Used on buildings |
| RockFireView | Static FireGeometry  ViewGeometry  Navmesh | Used on rocks | |
| Debris | Debris | Used on small debris pieces which should not collide with dynamic assets |  |
| Interaction | Interaction | Not used? |  |
| Ladder | Static Ladder | Used on ladders, mainly for detecting ladder itself, so player can "snap" on it, but also used as collision |  |
| Bush | Foliage | Used on "soft" parts of bushes |  |
| Foliage | Foliage | Used on "soft" parts of trees - leaves, needles |  |
| Wheel | FireGeometry CharCollide | Used on vehicle wheels |  |
| Glass | Static | Used on glass, in case Fire Geometry would need to be separated from normal collisions | |
| GlassFire | Static FireGeometry | Same as above + Fire Geometry layer | |

## InteractionMatrixRow Setup

[![armareforger interactionmatrixrow-setup.png](/wikidata/images/1/1f/armareforger_interactionmatrixrow-setup.png)](/wiki/File:armareforger_interactionmatrixrow-setup.png)
