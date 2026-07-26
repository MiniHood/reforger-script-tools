# [Persistence System](https://community.bistudio.com/wiki/Arma_Reforger:Persistence_System)

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.6.0 "Category:Arma Reforger/Version 1.6.0") [1.6.0](/wiki?title=Category:Arma_Reforger/Version_1.6.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.6.0 (page does not exist)")

This page provides some basic information on how the Persistence System operates, its interactions with the rest of the game, and how to add support for custom entities, components, and scripted instances for saving and loading data.

ⓘ

See also persistence's [server configuration](/wiki/Arma_Reforger:Server_Config#persistence "Arma Reforger:Server Config").

## System Setup

Persistence is a [WorldSystem](enfusion://ScriptEditor/scripts/GameLib/generated/WorldSystems/WorldSystem.c;16), defined through the world systems config. For example [GameMasterSystems.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Systems/GameMasterSystems.conf) contains a [SCR\_PersistenceSystem](enfusion://ScriptEditor/scripts/Game/Plugins/Persistence/System/SCR_PersistenceSystem.c;10) entry which itself then links to persistence config - in this case [GameMaster.conf](enfusion://ResourceManager/~ArmaReforger:Configs/Systems/Persistence/GameMode/GameMaster.conf).

All persistence-related config files can be found inside Configs/Systems/Persistence.
The configuration relies heavily on inheritance and splitting data into reusable groups. All properties have tooltips that describe what they do.

## Process Overview

### Instance Registration

All persistent instances must be tracked by the system. This usually happens through a Persistence component on a prefab or map entity.
The component is not instantiated at runtime. It serves mostly as a configuration. Instances can also be manually registered on the system using StartTracking.
Registrations are processed lazily. All configurations defined in the config are checked until a rule matches.
The order of evaluation follows the descending priority defined in the config, and then by whatever rule is most specific (more precise prefab or inherited type).
During registration, each instance gets assigned a 128-bit UUID, which is non-guessable, sortable by creation time and separates data by hive, which allows complex setups sharing a centralised database.
Loaded entities get them deterministically assigned to identify them on load, as long as the map has not changed significantly.
If one wants to ensure a special map entity is always found, even when moved, then it should be named in the [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor").

### Save Request

Once a save was requested (either from [SaveGameManager](enfusion://ScriptEditor/scripts/Game/generated/Plugins/Persistence/SaveGame/SaveGameManager.c;12) or direct script API calls), all pending registrations are handled, and all tracked instances are checked for saving.
Depending on the parent handling configured, only the roots of any entity hierarchy will get serialized and are then responsible for handling any children.
Default serialisation results indicate that the instance had no data that is needed to load it back.
Any info known due to prefabs or loaded map entities must not be stored. If there is some data, it is added to the configuration's collection.
For example, all item type entities go into the Item collection.
Collections serve as first level of organisation, grouping similar data types for visual separation in folders/database tables.

### Bundling

The system config may optionally have one or more bundles configured. These group collections together, who are likely to be fetched at the same time.
For example: Items, Vehicles, AI Characters, etc, will all be loaded back on the restart of a server.
If the data must not be individually loaded on demand later, or processed externally by third-party tools, then the bundle eliminates save and load time overheads by merging potentially thousands of tiny data files into one bigger bundle file.
Bundling is enabled by default for all collections.

### Database commit

After all data is read into collections (and bundled) it is then committed as one large transaction to the storage the collections belong to.
The system comes with two storage types: The SessionStorage represents individual save points for a current playthrough.
A session begins at the first launch of a mission and may be interrupted by restarts. It typically ends when the gamemode concludes with some result.
A gamemode may have a single, forever-running session.
This storage will usually contain all the world's information, so that after a restart, everything appears to be the same when players rejoin.
The GamemodeStorage can be used to store information across multiple sessions.
It is most suitable for things like player experience, currencies, unlocks, achievements, etc. This data can also be shared across multiple mission variants.
For example, CTI\_Arland and CTI\_Eden may have their own individual sessions running, but use the same shared gamemode storage, which transfers player data between them.

### Load

When a saved session is continued, the system will automatically spawn back any entities and persistent scripted states setup in the config.
Additional spawn/load requests can be issued via script API on the system. Save and load API calls can be made even if some transactions are currently ongoing.
So sending a query after an instance was saved, but before the data was committed, will still return the transient data.
Because the operations are generally all asynchronous, there are a few utilities on the persistence system class to await complex relationships of instances to be made (see cAddDeferredDeserializeTask() and cWhenAvailable() methods).

## Custom Serialisers

To support save and load for custom data, there are 3 serialiser types available.
They are the interface that know how to read the data from runtime instances, save it in a serialisation context, load it back and apply it to a target instance.

* [ScriptedEntitySerializer](enfusion://ScriptEditor/scripts/Game/generated/Plugins/Persistence/System/Serializers/ScriptedEntitySerializer.c;12) for IEntity handling
* [ScriptedComponentSerializer](enfusion://ScriptEditor/scripts/Game/generated/Plugins/Persistence/System/Serializers/ScriptedComponentSerializer.c;12) for ScriptComponent handling
* [ScriptedStateSerializer](enfusion://ScriptEditor/scripts/Game/generated/Plugins/Persistence/System/Serializers/ScriptedStateSerializer.c;12) for any other managed class.

For some usage examples, please see:

* [TimeAndWeatherManagerEntitySerializer](enfusion://ScriptEditor/scripts/Game/Plugins/Persistence/System/Serializers/Entities/TimeAndWeatherManagerEntitySerializer.c;1)
* [SCR\_NightModeGameModeComponentSerializer](enfusion://ScriptEditor/scripts/Game/Plugins/Persistence/System/Serializers/Components/GameMode/SCR_NightModeGameModeComponentSerializer.c;1)
* [SCR\_EditableEntityCoreSerializer](enfusion://ScriptEditor/scripts/Game/Plugins/Persistence/System/Serializers/States/SCR_EditableEntityCoreSerializer.c;12)

## SaveGameManager

The persistence system is loosely coupled with the game's [SaveGameManager](enfusion://ScriptEditor/scripts/Game/generated/Plugins/Persistence/SaveGame/SaveGameManager.c;12).
It is responsible for communication with user interfaces and accepts user inputs.
The persistence system gets notified about any save game requests, collects the necessary runtime data, and submits it via the SaveGameDatabase as a local save.
This works - with modded data - on all platforms, including dedicated servers.
It is the default use-case, but any custom gamemode may choose to store its data outside of any save-game logic using a different database type.
In conclusion, persistence can be used without any interaction with the save game manager.
