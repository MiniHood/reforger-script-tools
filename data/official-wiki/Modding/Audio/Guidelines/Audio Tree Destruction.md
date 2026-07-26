# [Audio: Tree Destruction](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Tree_Destruction)

This page covers the sound aspects of tree destruction.

* Tree prefabs themselves make use of the [Multiphase Destruction system](/wiki/Arma_Reforger:Audio:_Multiphase_Destruction "Arma Reforger:Audio: Multiphase Destruction"), but everything sound-related is handled on the "spawned debris".
* Unlike regular entities using Multiphase Destruction, trees spawn debris **Prefabs** of type SCR\_TreeDebrisSmallEntity rather than just models. The tree destruction sounds are defined in these Prefabs.
* The sounds themselves are designed and set up so that each tree "type" (leafy, coniferous, rotten) has its own .acp. The sounds automatically "scale" their sound characteristics based on the debris' bounding box size.
* A [DestructionManager](enfusion://ResourceManager/~ArmaReforger:Prefabs/Systems/Destruction/DestructionManager.et) entity must be present within the gameworld.

## SCR\_TreeDebrisSmallEntity Setup

In the *Unsorted* section, there are **Audio Source Configuration Impact** and **Audio Source Configuration Break**.

* **Audio Source Configuration Impact** defines the sound played when the tree debris impacts onto/with something else.
* **Audio Source Configuration Break** defines the sound played when the tree debris spawns (= when the tree breaks).

The AudioSourceConfiguration classes are configured as follows:

| **Audio Source Configuration Impact** defines the sound played when the tree debris impacts onto/with something else | |
| --- | --- |
| Sound Project | One of the tree-type specific .acp files |
| Sound Event Name | SOUND\_TREE\_IMPACT |
| **Audio Source Configuration Break** defines the sound played when the tree debris spawns (= when the tree breaks) | |
| Sound Project | One of the tree-type specific .acp files |
| Sound Event Name | SOUND\_TREE\_BREAK |

## Available Signals

Not all signals are available for each type of sound.

| Signal Name | Values | Description | Available for Break Sounds | Available for Impact Sounds |
| --- | --- | --- | --- | --- |
| EntitySize | = mass of object | The entity's mass, usually used in order to "scale" sounds. | Checked | Checked |
| CollisionDV | = change in speed of entity | A value reflecting the change in speed of an entity upon contact (how fast was it before, how much was it slowed down?) | Unchecked | Checked |

## Debugging

Tree debris Prefabs will make use of the [Multiphase Destruction Debug values](/wiki/Arma_Reforger:Audio:_Multiphase_Destruction#Debugging "Arma Reforger:Audio: Multiphase Destruction").
