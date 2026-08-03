# [Directory Structure](https://community.bistudio.com/wiki/Arma_Reforger:Directory_Structure)

This page represents the internal organisation of assets/game files.
The directories are usually well named and self-explanatory; see below for more detailed explanations.

| Directory Tree | | | | | Description |
| --- | --- | --- | --- | --- | --- |
| AI |  |  |  |  | This directory contains AI-related information, such as behaviour trees. AI is used in computer-managed human entities, but also in environmental wildlife |
| Anims |  |  |  |  | Main category holding configuration and actual character animations Contains also procedural animations shared between assets |
|  | anm |  |  |  | Folder with actual animations |
|  | export\_profiles |  |  |  |  |
|  | proc |  |  |  | Place for shared procedural animations (wheel or turret movement, small props animations) |
|  | workspaces |  |  |  | Contains main character configuration |
| Assets |  |  |  |  |  |
|  | \_SharedData |  |  |  | Data that can be shared between all the assets |
|  | \_SharedMaterials |  |  |  | Materials that can be shared between all the assets |
|  | Characters |  |  |  | This folder contains human characters and various equipment models and materials |
|  |  | Basebody |  |  | Contains models and materials for base character - without any equipment (only tactical underwear) |
|  |  | Footwear |  |  | Contains footwear models and materials |
|  |  | HeadGear |  |  | Contains head gear (caps, helmets, etc) models and materials |
|  |  | Heads |  |  | Contains character heads models and materials |
|  |  | Uniforms |  |  | Contains character uniforms (jackets, pants, etc) models and materials |
|  |  | Vests |  |  | Contains character vests (suspenders, bullet proof vests, chestrigs, etc) models and materials |
|  | Clutter |  |  |  | This folder contains various terrain clutter data - both masks and 3D models and materials |
|  |  | Masks |  |  | Contains distribution mask (per surface clutter) - a 2D texture which store information of distribution and max height for clutter **Sets** which define where (per pixel in texture) will appear each of clutter Sets |
|  |  | Procedural |  |  | Contains procedural 2D clutter mask |
|  |  | Rocks |  |  | Contains 3D model and materials for clutter rocks |
|  |  | Vegetation |  |  | Contains 3D model and materials for clutter vegetation |
|  |  |  | Debris |  |  |
|  |  |  | Grass |  |  |
|  |  |  | Mushrooms |  |  |
|  |  |  | Plants |  |  |
|  | Conflict |  |  |  |  |
|  | Decals |  |  |  | Contains 2D models and materials for various in game decals (road dirt, debris, grass, runway markings, etc) |
|  | Editor |  |  |  | Contains models and materials used by in-game editor |
|  | Items |  |  |  | Contains models and materials for various interactive items like food, part of equipment (flash lights, backpacks, radios, etc) and similar |
|  | Props |  |  |  | Contains models and materials for props |
|  | Rocks |  |  |  | Contains models and materials for large rocks |
|  | Structures |  |  |  | Contains models and materials for various structures like houses, harbours, sheds, etc |
|  | Vegetation |  |  |  | Contains models and materials for vegetation like trees or bushes |
|  | Vehicles |  |  |  | Contains data related to vehicles like models, materials, textures and animations |
|  |  | Airplanes |  |  | Category for fixed wing aircrafts |
|  |  | Boats |  |  | Category for boats (ships, kayaks, pontoons, etc) |
|  |  | Helicopters |  |  | Category for helicopters |
|  |  | Tracked |  |  | Category for tracked vehicles (tanks, APCs, etc) |
|  |  | Wheeled |  |  | Category for wheeled vehicles (cars, trucks, wheeled APCs) |
|  |  |  | Ural4320 |  | In this folder there are models, animations, materials and textures related to Ural-4320 |
|  |  |  |  | anm | Animation files specific to given asset - they are injected to main graph through prefab |
|  |  |  |  | Data | Materials and textures tied to specific vehicle |
|  |  |  |  | Dst | Dst folders contains destructible parts of the vehicle |
|  |  |  |  | Lights | This folder contains various light parts of the model like emissive surfaces when given light bulb is turned on |
|  |  |  |  | workspaces | Animation workspace specific to given asset - those workspaces are |
|  | Weapons |  |  |  | Contains models and materials for weapons, ammunitions, tracers and accessories (optics, flashlights etc) |
|  |  | Ammo |  |  |  |
|  |  |  | Bullets |  |  |
|  |  |  | Grenades |  |  |
|  |  |  | Missiles |  |  |
|  |  |  | Shells |  |  |
|  |  |  | Tracers |  |  |
|  |  | Accessories |  |  |  |
|  |  |  | Scopes |  |  |
|  |  |  | Lasers |  |  |
|  |  | Attachments |  |  |  |
|  |  |  | Bayonets |  |  |
|  |  |  | Flashlights |  |  |
|  |  |  | Muzzle |  |  |
|  |  |  | Optics |  |  |
|  |  |  | Underbarrel |  |  |
|  |  | Flares |  |  |  |
|  |  | Grenades |  |  |  |
|  |  | Handguns |  |  |  |
|  |  | HeavyWeapons |  |  |  |
|  |  | Launchers |  |  |  |
|  |  | MachineGuns |  |  |  |
|  |  | Magazines |  |  |  |
|  |  | Melee |  |  | Melee weapons - empty as of now |
|  |  | Mortars |  |  |  |
|  |  | Pods |  |  |  |
|  |  | Rifles |  |  |  |
|  |  |  | M16A2 |  | Contains the 3D model |
|  |  |  |  | Data | Contains textures and materials |
|  |  | Tripods |  |  |  |
| Backend |  |  |  |  | - |
| Brushes |  |  |  |  | Contains custom brushes for sculpting and surfaces painting |
| Common |  |  |  |  | Contains the most common data, like shared textures/particles/config/default keybinds/physics materials |
|  | Materials |  |  |  | This folder contains common materials used across the game |
|  |  | ColorPalette |  |  | Contains very simple materials used for assets last LODs |
|  |  | Game |  |  | Contains game materials |
|  |  | Physics |  |  | Contains physics materials |
|  |  | Vehicles |  |  | Contains Contact Surface materials - vehicle contact material that defines the behavior of vehicles on such surface (friction, bumpiness, wheel resistance) |
|  | Models |  |  |  |  |
|  | Postprocess |  |  |  |  |
|  | Textures |  |  |  |  |
| Configs |  |  |  |  |  |
|  | Campaign |  |  |  | This folder contains configs related to Conflict game mode: vehicle list, mission settings, etc |
|  | Core |  |  |  |  |
|  | Editor |  |  |  | Contains configs related to Game Master (in-game editor): placeable unit list, compositions, editor settings, etc |
|  | Weapons |  |  |  | Contains configs related to weapon configuration (ammo mix in magazine) |
|  | Workbench |  |  |  | Contains config files for Workbench-related utilities (plugins/extensions) |
|  | WorldHeaders |  |  |  | Contains configs for world headers - files which contain information about the terrain like user-friendly name |
| Docs |  |  |  |  | Contains configuration for generation documentation via Doxygen |
| Extern |  |  |  |  | Contains External libraries such as e.g BattlEye |
| Language |  |  |  |  | Localisation files. Every language has its file, unlike older Stringtable CSV/XML |
| Missions |  |  |  |  | This folder contains scenario configurations |
| Particles |  |  |  |  | Shared particles |
| PrefabLibrary |  |  |  |  | Used for terrain making; includes templates of all maps assets |
| Prefabs |  |  |  |  | Configuration. Its structure almost mirrors the Assets directory's (**not** a 1:1) |
| PrefabsEditable |  |  |  |  | Contains prefabs which are available in in-game editor |
|  | Auto |  |  |  | Contains prefabs which are **automatically generated** by Workbench plugin. Don't touch it manually unless you know what you are doing! |
| Publishing |  |  |  |  | Contains installation information |
| Scripts |  |  |  |  | Contains scripting and logic for Arma Reforger and Workbench |
|  | Game |  |  |  | Contain's game's regular script files |
|  | GameCode |  |  |  | Contains auto-generated script files |
|  | Workbench |  |  |  | Contains common editor plugins/tools |
|  | WorkbenchGame |  |  |  | Editor plugins/tools Specific for current project |
| Sounds |  |  |  |  | Contains samples (wav) and configuration (audio configuration projects and signals) for all sounds in game |
|  | \_SharedData |  |  |  |  |
|  | Character |  |  |  |  |
|  | Compositions |  |  |  |  |
|  | Destruction |  |  |  |  |
|  | Editor |  |  |  |  |
|  | Environment |  |  |  |  |
|  | Explosions |  |  |  |  |
|  | Impacts |  |  |  |  |
|  | Items |  |  |  |  |
|  | Music |  |  |  |  |
|  | Props |  |  |  |  |
|  | RadioBroadcast |  |  |  |  |
|  | RadioProtocol |  |  |  |  |
|  | Structures |  |  |  |  |
|  | UI |  |  |  |  |
|  | Vegetation |  |  |  |  |
|  | Vehicles |  |  |  |  |
|  | VON |  |  |  |  |
|  | Weapons |  |  |  |  |
| Terrains |  |  |  |  |  |
|  | Common |  |  |  | Contains common data |
|  |  | Sky |  |  |  |
|  |  | Surfaces |  |  |  |
|  |  | Water |  |  |  |
|  |  | WeatherPresets |  |  |  |
|  | FlatLand |  |  |  |  |
| UI |  |  |  |  | Contains UI layouts, fonts, textures and sounds |
|  | Data |  |  |  |  |
|  | Fonts |  |  |  |  |
|  | Imagesets |  |  |  |  |
|  | Layouts |  |  |  |  |
|  | Materials |  |  |  |  |
|  | Styles |  |  |  |  |
|  | Textures |  |  |  |  |
| WBData |  |  |  |  | Contains Workbench-related data like custom components icons |
| Worlds |  |  |  |  | Contains worlds |
|  | Eden |  |  |  |  |
