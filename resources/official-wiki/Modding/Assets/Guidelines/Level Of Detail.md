# [Level Of Detail](https://community.bistudio.com/wiki/Arma_Reforger:Level_Of_Detail)

## Basics

**L**evel **O**f **D**etail (**LOD**) in words one LOD or more LODs (**LOD0, LOD1 - LODn**) are versions of a **model with specific triangle and shader complexity**.
The lower the LOD number is the better the quality/higher complexity is. LOD0 stands for the nicest and most performance heavy one. Then the higher the number, the less complex and often also worse looking the model is.

These performance-cheaper model variants (all but LOD0) are to be rendered generally at higher distances in order to save performance on the GPU.
The closer the LODs are rendered, the more GPU performance it saves. A user who will set lower quality in **Settings/Video/Model geometric detail** in the game's options will get closer switching to worse LODs to save more performance.

How to set which model part will be part of which LOD is described elsewhere (FBX import), but generally put the \_LODn suffix into object name inside FBX.

## Preview

How each LOD is looking, what materials they have and how much triangles or vertices they have can be directly checked in Workbench. Just find the desired XOB model in the Resource Browser and double-click on it.
Then check the Details tab on the right part of the viewport and open its LODs sub-tab. Then, if "Force selected LOD to render" is checked, it is possible to scroll through the LODs and actually see them.

[![armareforger levelofdetail-preview.png](/wikidata/images/thumb/5/5a/armareforger_levelofdetail-preview.png/400px-armareforger_levelofdetail-preview.png)](/wiki/File:armareforger_levelofdetail-preview.png)

## Debug

There is a diagnostic tool which tells which LOD is currently visible on any visible entity instance in a map in [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor").
Turn it on with the [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") by pressing `Alt` + `⊞ Win` → **Scene/Colorize objects LODs**.
Now each LOD except for LOD0 will have color overlay. If no other than LOD0 is present for the object, then it will constantly switch all colors.

|  |  |  |  |  |  |  |  |
| --- | --- | --- | --- | --- | --- | --- | --- |
| no color overlay |  |  |  |  |  |  | ... |
| LOD0 | LOD1 | LOD2 | LOD3 | LOD4 | LOD5 | LOD6 | ... |

## Automatic System

Why an automatic system, when other engines offer manual control of LOD behaviour?
The automatic system is much less prone to human error and always ensures LOD functionality, even though it might not be always ideal.
For these "not ideal" cases there are manual overrides which will be presented later.

The LOD system tries to keep a constant triangle/pixel ratio across the screen.
It is possible to estimate LOD distances considering LOD factors as a multiplier of triangle count that will change the system's behaviour into earlier/further LODing distance.

### LOD0 - LOD1

When the model switches from LOD0 to LOD1 (generally begins to switch LODs) depends on several parameters:

* Spherical size of the object - the larger the object is, the further away it starts to switch to worse LODs

  [![armareforger levelofdetail-boundingsphere.jpg](/wikidata/images/thumb/b/b6/armareforger_levelofdetail-boundingsphere.jpg/200px-armareforger_levelofdetail-boundingsphere.jpg)](/wiki/File:armareforger_levelofdetail-boundingsphere.jpg)
* Triangle count of LOD0 - the more triangles the closer it starts to switch to LOD1

  [![armareforger levelofdetail-trianglecount.png](/wikidata/images/5/55/armareforger_levelofdetail-trianglecount.png)](/wiki/File:armareforger_levelofdetail-trianglecount.png)
* Field of View (FOV) - the larger FOV the closer it starts to switch to worse LODs

  [![armareforger levelofdetail-fov.png](/wikidata/images/thumb/1/1e/armareforger_levelofdetail-fov.png/500px-armareforger_levelofdetail-fov.png)](/wiki/File:armareforger_levelofdetail-fov.png)
* Graphic settings - "Model geometric detail" on Ultra results in switching to worse LODs further away, Lowest vice-versa

  [![armareforger levelofdetail-model-geometric-detail.png](/wikidata/images/thumb/9/97/armareforger_levelofdetail-model-geometric-detail.png/500px-armareforger_levelofdetail-model-geometric-detail.png)](/wiki/File:armareforger_levelofdetail-model-geometric-detail.png)

[![](/wikidata/images/thumb/1/16/armareforger_levelofdetail-lod-1-2-3.png/300px-armareforger_levelofdetail-lod-1-2-3.png)](/wiki/File:armareforger_levelofdetail-lod-1-2-3.png)

LOD1-2-3 size and triangle count

### LOD1 - LOD2 - LOD3+

When the model starts to switch from LOD1 to LOD2, LOD2 to LOD3 and so on depends on two criteria:

* The LOD switch happens roughly when the spherical surface size of the currently loaded LOD on the screen reaches **half the size compared to the moment of its first appearance**, and the same goes for the next LOD. "Roughly" because it also depends on the other criterion:
* Relative **triangle count between LODs** - when each LODs have exactly **half the triangle count of the previous one**, then the first rule based on surface size applies exactly. When the relative triangle count is not done more or less by half, then it might switch a lot sooner or later.  
    
  Sometimes the LODs are not switching ideally. To save the most performance and get the most from LODs, there are manual overrides.

## Manual Override

The "triangle count" parameter, which heavily influence how fast or slowly prefab is LODing, can be overridden with **"LOD Factors"**.

Yet it multiplies it only for the LOD switching behaviour and it is not really increasing/decreasing number of triangles in the model.
By setting it, it just forces the specific LOD to remain in the scene for shorter (more triangles) or longer (less triangles) distance from the camera until it switches to the next LOD.

The parameter can be found in **Entity/Mesh object component/LOD Factors**.  
[![armareforger levelofdetail-meshobject.png](/wikidata/images/5/56/armareforger_levelofdetail-meshobject.png)](/wiki/File:armareforger_levelofdetail-meshobject.png)

There needs to have as many "LOD Factors" as there are LODs in XOB model, which are inserted in Mesh object component, in order to influence all the LODs.
The good news is that the number of factor fills in automatically based on the number of LODs in the inserted XOB model. Each factor corresponds with respective LOD, so factor 3 influence LOD3 and so on.
When there are less factors than there are LODs in a model, then these LODs, which do not have their factors assigned, are simply not affected at all...
or they simply behave like if the factors for them were set to 1 - as multiplying by 1 means no change.

[![armareforger levelofdetail-lodfactor.png](/wikidata/images/8/85/armareforger_levelofdetail-lodfactor.png)](/wiki/File:armareforger_levelofdetail-lodfactor.png)

An important note is that **LOD Factors are inherited from parent prefabs to children prefabs** as all the other mesh object component parameters.
So it is possible to have some factors setup for the "base" or "core" prefab which fits most of the prefabs inheriting from it;
yet there is always the possibility to not inherit and to change the factors for any specific prefab/entity in the prefab hierarchy.

### Save Process

Saving of the LOD Factors is done the same way as any other parameters saved on entity components. Here is a guide for the saving process just in case:

The LOD factors for specific prefab can be saved by:

1. Changing the factor values on "Entity instance" placed in map, applying them to specific inherited prefab by "Apply to prefab" button and then save the world... which saves also all changes in prefabs. Just be aware that this applies all the parameters that you changed on the mesh object component so be sure have not touched anything else on the Entity instance
2. Selecting specific inherited prefab first (on the Entity instance placed in the world), then changing the factors values and saving the world.
