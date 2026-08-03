# [Car Modding](https://community.bistudio.com/wiki/Arma_Reforger:Car_Modding)

## Goals of this tutorial

💬

**Overview**

In this tutorial you will learn about:

* How to modify basic damage characteristics of the vehicle
* How to change fuel consumption
* How to change basic driving characteristics
* Changing exhaust particle effects
* Adding additional elements to the vehicle with **BaseSlotComponent**
* How actions are working and how to add custom one

ⓘ

If you **don't have any experience with Workbench** yet, it is recommended to **go through [modded weapon tutorial](/wiki/Arma_Reforger:Weapon_Modding "Arma Reforger:Weapon Modding")** to familiarise with some of the concepts present in the **Workbench**

📥

Sources files for this tutorial can be found on
[**Arma Reforger Samples Github repository**](https://github.com/BohemiaInteractive/Arma-Reforger-Samples/tree/main/SampleMod_ModdedCar)

## Modifying existing car

**Overview**

Before going deeper into vehicle configuration, first we will prepare basic prefab to work with. In this chapter we will take a look at:

* Basic file structure
* Preparing basic prefab inheriting from UAZ-469
* Briefly check already assigned components to UAZ-469

#### File structure

While keeping to official structure is not mandatory and there are no engine restrictions asset wise about it, it's recommended to follow guidelines listed here - [Data (file) structure](/wiki/Arma_Reforger:Directory_Structure "Arma Reforger:Directory Structure") - to ensure that all automation plugins are parsing your assets correctly and make it later easy to navigate.

Therefore, your first task will be preparing following file structure

[![armareforger-modded-car-file-structure2.png](/wikidata/images/thumb/0/0a/armareforger-modded-car-file-structure2.png/1200px-armareforger-modded-car-file-structure2.png)](/wiki/File:armareforger-modded-car-file-structure2.png)

#### Preparing prefab

Let's start with creating new prefab. Look for [**UAZ469.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et) in **Resource Browser** and then by using of the methods mentioned in Sample Modded Weapon documentation. To make it obvious what we are doing, call that new prefab **UAZ469\_modded**.et.

ⓘ

In this article, [**Enfusion links**](/wiki/Arma_Reforger:Resource_Manager:_Options#Register_.22enfusion:.2F.2F.22_protocol "Arma Reforger:Resource Manager: Options") are used. With those links it is possible to open specific resource just by simply clicking on that link. Enfusion links **has to be manually activated in Workbench options** (Workbench -> Options -> Workbench -> Register "enfusion://" protocol) before it can be used]

#### Changing Multi Material base color

Next thing that we will quickly do is replacement of currently assigned materials. In principle, everything works exactly same as on weapons so try to locate [**UAZ469\_Body.emat**](enfusion://ResourceManager/~ArmaReforger:Assets/Vehicles/Wheeled/UAZ469/Data/UAZ469_Body.emat) and create duplicated material (as described here for instance) called "**UAZ469\_Body\_Modded.emat**" and afterwards move it Sample Mod structure. **MatPBRMulti** is powerful kind of material which is going to be described more in detail in Sample New Car page. For now, try changing **Material 1 (Base)** to get some interesting variation and just to play with sliders. You can try to play with adjusting other mask this way too but as mentioned before, this material will be described in detail at later stage.

[File:armareforger-modded-car-changing-material.mp4](/wiki/File:armareforger-modded-car-changing-material.mp4 "File:armareforger-modded-car-changing-material.mp4")

#### Components overview

As you can notice, list of components attached to vehicle is bit larger comparing to weapon we were editing before. From that list, we will use following components:

[![](/wikidata/images/8/8a/armareforger-modded-car-component-overview.png)](/wiki/File:armareforger-modded-car-component-overview.png)

Overview

* **ActionsManagerComponent** - To manage contextual actions (including get in/out)
* **SCR\_VehicleDamageMangerComponent**- To modify damage our vehicle is taking
* **MeshObject -** To change used material
* **SCR\_FuelConsumptionComponent & FuelManagerComponent -** To change fuel consumption and capacity of fuel tanks
* **SCR\_MotorExhaustEffectGeneralComponent** - To change exhaust particle effects
* **VehicleWheeledSimulation -** To change general driving behavior

Full list of components with some brief info on what they are doing can be obtained from Vehicle Components page.
As you probably noticed, components listed in **Object Properties** list have different colors & shapes. Those are some visual cues whether they are for instance scripted or handled by game.

|  |  |
| --- | --- |
| **Icon** | **Description** |
| [armareforger-component-engine.png](/wiki/File:armareforger-component-engine.png) | Engine component - Implemented deeply in Enfusion code |
| [armareforger-component-game.png](/wiki/File:armareforger-component-game.png) | Game component - Implemented in game code - *may vary from game to game* |
| [armareforger-component-hybrid.png](/wiki/File:armareforger-component-hybrid.png) | Hybrid component - component is partially handled by engine and partially by script |
| [armareforger-component-scripted.png](/wiki/File:armareforger-component-scripted.png) | Scripted component - Component is fully scripted |
| [armareforger-component-editor.png](/wiki/File:armareforger-component-editor.png) | Editor scripted component - Behaves in same way as any other scripted component. It is using custom icon to distinguish it from other components. |

## Modifying damage behaviour

**Overview**

In this chapter following topics will be explained

* How to increase armor on vehicle
* ...yet make it super fragile to collisions!

[![](/wikidata/images/9/95/armareforger-modded-car-damage-overview.png)](/wiki/File:armareforger-modded-car-damage-overview.png)

Overview

#### Damage manager overview

Whole vehicle damage simulation is contained in **SCR\_VehicleDamageMangerComponent.** There you can modify various parameters controlling how object will react to projectiles, explosions and collisions. The main hit. This *Vehicle Hit Zone* element serve as global damage collector - once Total HP is destroyed, whole vehicle is considered destroyed and damage particles will spawn (like ***Destruction Particle*** , ***Burning Particle Resource*** ) together with wreck model (defined in **Wreck Model**). More information about how damage is handled can be found here - Damage - Vehicles.

#### Increasing armor

In order to increase armor of our vehicle, we will have to change **HP Max** parameter. To ensure that it survives even the strongest projectiles in game, I would recommend to add 2 zeros to **HP Max** value. Similarly **HP Damaged**, which controls when damaged particles are started to be played (dark exhaust smoke in this case) can be increased too so smoke doesn't start to appear to early.

#### Increasing collision impulse

Next thing on our *to do list* is increasing amount of damage received from collision with environment. To do so, simply increase **Collision multiplier** to i.e. **100000,** which will result in huge damage being applied upon collision & decrease **Collision Velocity Threshold** to 1.

As a result, you should end up with thing which can survive direct RPG hit yet it will be destroyed by slightly bumping into obstacle

[![](/wikidata/images/6/6e/armareforger-modded-car-damage-tweak.png)](/wiki/File:armareforger-modded-car-damage-tweak.png)

Overview

## Changing fuel consumption

**Overview**

In this chapter we will learn how fuel is simulated in game and we will change following things:

* Type of fuel vehicle is using
* Amount of fuel in fuel tanks
* Fuel consumption

#### Fuel handling overview

After filtering down components containing "*Fuel*" in their name, we should see 2 components in **Object Properties** list.

|  |  |
| --- | --- |
| **FuelManagerComponent** | **SCR\_FuelConsumptionComponent** |
| [armareforger-fuel-manager.png](/wiki/File:armareforger-fuel-manager.png) | [armareforger-fuel-consumption.png](/wiki/File:armareforger-fuel-consumption.png) |

How both **FuelManagerComponent** & **SCR\_FuelConsumptionComponents** works is described quite in detail on Fuel: Fuel System & Fuel: Fuel Consumption pages, so I recommend to read those pages first before proceeding further in this tutorial.

When looking at the UAZ-469 **Fuel Nodes** array, you might notice that it has two **SCR\_FuelNode** elements. This is due to fact, that UAZ-469, like quite plenty of military vehicles, has 2 separate fuel tanks and this thing is simulated in game.

#### Changing fuel type

To change fuel type, select one of the **SCR\_FuelNodes** in **FuelManagerComponent** components, and then simply click on **"Fuel Type"** list box and change fuel type from **Petrol** to **Diesel.** Repeat same procedure with 2nd fuel tank.

#### Changing fuel tanks capacity

Changing fuel capacity is as simple as previous change to fuel type. To change maximum capacity, increase "**Max Fuel**" parameter from **39 to 69 liters** on both fuel tanks. Beside that, it might be also wise to change initial fuel tank state by increasing **Initial Fuel Tank State** on one engine from 39 to 69 too. Just for sake of experimentation, we will leave 2nd fuel tank with same amount of **initial fuel** as before.

#### Increasing fuel consumption

Next, let's move to changing fuel consumption. Select **SCR\_FuelConsumptionComponents** and locate **Fuel Consumption** property. This property controls amount of **liters** fuel consumed **at max power RPM per hour**. Change it to **145** and voila - your vehicle will consume fuel quite fast even though it has larger fuel tanks.

## Changing driving behaviour

**Overview**

Following topics will be covered in this paragraph

* Basics of wheeled simulation
* How to change engine parameters
* Changing of driving characteristics

#### Wheeled simulation overview

Wheeled vehicles simulation is contained in **VehicleWheeledSimulation** component. In that component, after expanding **Simulation** list, you can find five major categories:

* **Engine** - responsible for engine simulation
* **Clutch** - contains all parameters controlling clutch simulation
* **Gearbox** - here you can change gearbox settings like amount of gears, gear ratios or gearbox latency
* **Differentials** - differentials controls
* **Axles -** contains definition of each axle together with wheels and tyres

Whole topic is rather complex so lets start with some simple changes first.

#### Changing engine parameters

[![armareforger-modded-car-max-power.gif](/wikidata/images/3/37/armareforger-modded-car-max-power.gif)](/wiki/File:armareforger-modded-car-max-power.gif)

ⓘ

One such simple change might be for sure changing engine power. To do so, search for **Max Power** parameter, which describes maximal engine power output in **kW**. To get a nice power boost, let's change it from 55 to 255! Next, we can change engine torque by **100Nm**  by modifying **Max Torque** parameters from 172Nm to 272Nm. After that change, our modded vehicle should gain speed much faster! Be careful though - don't forget it's still super fragile!

ⓘ

To see how the parameters affect the engine maximal torque you can check [Vehicle Creation - Engine Parameters](/wiki/Arma_Reforger:Vehicle_Creation#Engine_parameters "Arma Reforger:Vehicle Creation") page

#### Changing driving characteristics

Another thing that we might try is changing turning radius of front axle. To do so, navigate & expand **Axels** list and then select top one **Axle.** Over there you should see **Max Steering Angle** parameter, which controls maximum rotation of front axle. Parameter is in degrees. To gain some more maneuverability on tight corners, we will increase that value to 48.

Above changes are just quickly demonstration on how simulation works so feel free to play with those values mores. A bit more information can be also found in Sample New Car - Documentation page where things like wheel position and setting up gearbox are described.

## Adding turret to vehicle

**Overview**

Time to add some extra firepower to our vehicle! In this chapter we will learn how to add working**\*** turret our vehicle

* Adding turret via **SlotManagerComponent**

While it might look quite advanced at first glance, adding turrets to vehicle is not that complicated if you want to use one of the already existing turrets. Whole procedure can be done in two steps:

⚠

If you are adding component to **entity instance** , don't forget to click "**Apply to prefab**" button once you are done with making changes!

|  |  |
| --- | --- |
| 1. Adding base slot component | 2. Filling in proper values to base slot component |
| [armareforger-modded-car-new-component.gif](/wiki/File:armareforger-modded-car-new-component.gif) | [armareforger-modded-car-register-slot.gif](/wiki/File:armareforger-modded-car-register-slot.gif) |
| Click on "**+ Add Component**" and from that list pick up " **SlotManagerComponent**". | Once you have that component ready, assign [**M151A2\_gun\_mount\_M2HB.et**](enfusion://ResourceManager/~ArmaReforger:Prefabs/Vehicles/Wheeled/M151A2/M151A2_gun_mount_M2HB.et) prefab to **EntityPrefab** slot & then change **AttachType** to **RegisteringComponentSlotInfo.** After that, you should few more parameters should be available to you - toggle all of them on and adjust **Offset** value to your liking. |

As a result, you should end up with something like this

[![armareforger-modded-car-turret-result.png](/wikidata/images/thumb/8/81/armareforger-modded-car-turret-result.png/400px-armareforger-modded-car-turret-result.png)](/wiki/File:armareforger-modded-car-turret-result.png)

## Changing exhaust effects

**Overview**

As last thing, we will try to do some modifications to particle effects to get some sparks going out of exhaust tubes. To do so, we will learn about

* Adding additional emitor to particle effect
* Changing exhaust particle effects

[![](/wikidata/images/5/5b/armareforger-modded-car-exhaust-sfx.png)](/wiki/File:armareforger-modded-car-exhaust-sfx.png)

SCR\_MotorExhaustEffectGeneralComponent

Vehicle exhaust effects configuration is located in **SCR\_MotorExhaustEffectGeneralComponent.**  Assuming you already know how to handle particle from [Weapon Modding page](/wiki/Arma_Reforger:Weapon_Modding#Particle_Effects_Change "Arma Reforger:Weapon Modding"), let's move to creating derivative of [**Vehicle\_smoke\_car\_exhaust\_normal.ptc**](enfusion://ResourceManager/~ArmaReforger:Particles/Vehicle/Vehicle_smoke_car_exhaust_normal.ptc%7C) and call that new effect **Vehicle\_smoke\_car\_exhaust\_black\_02\_modded.ptc.** If you want, you can straight away go to changing **Particle Effect** parameter in **SCR\_MotorExhaustEffectGeneralComponent.**

[![armareforger-modded-car-exhaust-sfx2.gif](/wikidata/images/3/34/armareforger-modded-car-exhaust-sfx2.gif)](/wiki/File:armareforger-modded-car-exhaust-sfx2.gif)

Once you are **Particle Editor,** try to adding new editor by clicking on "**+ Add**" button. Name that new emitor "**s1\_sparks**" and proceed to creating new particle effect. This it's time for some challenge - **try to replicate effect on the right by doing it on your own.** If you have troubles with recreating it, try looking at previously made explosion effect or take a look at the spoiler below.

Small tip: make sure that "Repeat" parameter is checked!

[![](/wikidata/images/8/8d/armareforger-modded-car-exhaust-add.png)](/wiki/File:armareforger-modded-car-exhaust-add.png)

Viewport

ⓘ

Click on "Show text" button to see full example configuration → Show text

[![](/wikidata/images/d/df/armareforger-modded-car-exhaust-full.png)](/wiki/File:armareforger-modded-car-exhaust-full.png)

Viewport

## Testing results in-game

[![](/wikidata/images/3/3d/armareforger-modded-testing-play.png)](/wiki/File:armareforger-modded-testing-play.png)

Switching to play mode

Once everything is complete, it is finally possible to place this prefab on some terrain and spawn in-game.

Easiest way to do it would be selecting of "**Play from camera position"** option in Play menu (accessible by clicking on little arrow to right of green one)

Additionally, it is worth to consider adding this new asset to [**Game Master Entity Browser**](/wiki/Arma_Reforger:Game_Master#Entity_Browser "Arma Reforger:Game Master"). All instructions for that are listed on [**Asset Browser Mod Integration page**](/wiki/Arma_Reforger:Asset_Browser_Mod_Integration "Arma Reforger:Asset Browser Mod Integration")
