# [Enfusion Blender Tools: Materials Preview](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Materials_Preview)

**Materials Preview** (referenced as MP in this document) feature allows to mimic Enfusion shaders in Blender to visually assist users while creating a model from scratch, to help creating additional data like baking textures for last LODs and more.
MP feature currently supports loading of **MatPBRBasic, MatPBRMulti, MatPBRDecal, MatPBRBasicGlass** shader classes.
MP feature focuses mainly to support (mimic) core functionality of particular shaders wherever possible, good example is PBRMulti shader, its core functionality is based on layering several materials and textures on top of each other by using various masks.

[![armareforger-ebt materialspreview comparison1.jpg](/wikidata/images/thumb/1/1f/armareforger-ebt_materialspreview_comparison1.jpg/400px-armareforger-ebt_materialspreview_comparison1.jpg)](/wiki/File:armareforger-ebt_materialspreview_comparison1.jpg) [![armareforger-ebt materialspreview comparison2.jpg](/wikidata/images/thumb/7/7b/armareforger-ebt_materialspreview_comparison2.jpg/500px-armareforger-ebt_materialspreview_comparison2.jpg)](/wiki/File:armareforger-ebt_materialspreview_comparison2.jpg)

[![](/wikidata/images/thumb/a/a6/armareforger-ebt_materialpreview_compression_difference.png/300px-armareforger-ebt_materialpreview_compression_difference.png)](/wiki/File:armareforger-ebt_materialpreview_compression_difference.png)

because of compression there may be visual differences spotted for example when playing with Alpha Test/Alpha Mul parameters

Please be aware that even though both Blender and Enfusion uses PBR pipeline, it's necessary to point out few facts that makes delivery of exactly the same visual experience in Blender (EEVEE renderer) impossible:

* PBR implementation in Blender and Enfusion is different
* there's currently no Shader API in Blender (EEVEE) that could be used to write Enfusion shader "twin", instead Shader Editor has to be used to manually build up a material by connecting various nodes together  
  [![armareforger-ebt materialspreview shadereditor 1.png](/wikidata/images/thumb/b/b5/armareforger-ebt_materialspreview_shadereditor_1.png/500px-armareforger-ebt_materialspreview_shadereditor_1.png)](/wiki/File:armareforger-ebt_materialspreview_shadereditor_1.png)[![armareforger-ebt materialspreview shadereditor 2.png](/wikidata/images/thumb/b/b2/armareforger-ebt_materialspreview_shadereditor_2.png/469px-armareforger-ebt_materialspreview_shadereditor_2.png)](/wiki/File:armareforger-ebt_materialspreview_shadereditor_2.png)
* nodes provided by Shader Editor doesn't allow to access things like GBuffer or scene lighting information therefore some specific material's properties can't be supported via MP feature
* PP effects implementation in Blender is different, some PP effects aren't implemented at all
* shaders in Blender use source texture formats (tif, png...) whereas shaders in Enfusion use compressed edds textures

## Features

### Materials Import

EBT provides custom FBX Import via **EnfusionTools** menu, that imports meshes with Enfusion materials.  
[![armareforger-ebt materialspreview fbximport menu.png](/wikidata/images/a/a2/armareforger-ebt_materialspreview_fbximport_menu.png)](/wiki/File:armareforger-ebt_materialspreview_fbximport_menu.png)

In order to successfully import meshes with Enfusion materials, it's expected that:

* FBX is registered in Workbench (xob and xob.meta files already exists)
* meshes have materials assigned

⚠

Materials Preview feature can only import materials/textures from within your own addon (expects source textures to be in PNG, TIFF... format), in other words you cannot import materials inherited from Vanilla Reforger.

Loading time depends on model's complexity as well as total number of unique materials/textures that need to be loaded.

Once loading is done, a log of issues (in case there are any) is provided. You can access the log by changing one of the editors to **INFO** editor.
[![armareforger-ebt materialspreview fbximport infowindow menu.png](/wikidata/images/7/72/armareforger-ebt_materialspreview_fbximport_infowindow_menu.png)](/wiki/File:armareforger-ebt_materialspreview_fbximport_infowindow_menu.png)

[![armareforger-ebt materialspreview fbximport infowindow example.png](/wikidata/images/thumb/9/90/armareforger-ebt_materialspreview_fbximport_infowindow_example.png/800px-armareforger-ebt_materialspreview_fbximport_infowindow_example.png)](/wiki/File:armareforger-ebt_materialspreview_fbximport_infowindow_example.png)  
Example of an issue

Logs provides valuable information in case some materials or textures could not be loaded.

### Materials Export

EBT allow users to export Enfusion materials (.emat) from Blender. There are two ways to export Enfusion materials, each used in a specific scenario.

ⓘ

Both ways of exporting materials require a running Workbench instance.

#### FBX Export

If a user creates a new material by drag and dropping it from the Material library, the material is not yet registered and does not contain a GUID in its name (e.g., bricks\_15884ADEF846848).
The registration process is carried out with EBT's Export FBX feature (**Enfusion Tools > Export > FBX (.fbx)**).

When using this Export FBX feature, not only does it export the FBX file, but it also goes through all Enfusion materials (non-Enfusion materials are skipped) and registers them.
Materials that are already registered (having a GUID in their name) are skipped during this process.

Once the export process is complete, the exported .emat files (along with their .emat.meta files) can be found in the Data subdirectory, located next to the exported FBX file.
If the Data subdirectory does not exist, it will be created automatically.

ⓘ

* User can edit Enfusion materials even before they are registered. All the edits made to the materials will be properly exported to the .emat file during the registration process.
* If a user needs to rename already registered material, he can do so by simply renaming it without the GUID, for example, changing "bricks\_15884ADEF846848" to "tiles". Once renamed, the material needs to be registered again using the Export FBX feature.  
  It is indeed a good practice to delete the obsolete material in Workbench after renaming and re-registering the material with the updated name.

⚠

Every time a new material is created and needs to be registered, the FBX must be exported. This means that any mesh changes made will also be exported to the FBX file.  
Therefore, it is important to ensure that any unwanted changes to the mesh are avoided or properly adjusted before registering the material and exporting the FBX file.

[![armareforger-enfusion blender tools material export.png](/wikidata/images/thumb/4/4b/armareforger-enfusion_blender_tools_material_export.png/300px-armareforger-enfusion_blender_tools_material_export.png)](/wiki/File:armareforger-enfusion_blender_tools_material_export.png)

#### Material Export

This functionality can be only used on already registered materials (which have a GUID in name).

ⓘ

Materials that are imported together with the FBX using the **Import FBX** feature can be edited and exported immediately and do not need to export FBX first, given they are already registered.

⚠

The **Export Material** feature only exports the changes made to the selected material. If there are any mesh-related changes that need to be exported, use the **Export FBX** feature again.

[![armareforger-ebt materialspreview material sync.png](/wikidata/images/1/1e/armareforger-ebt_materialspreview_material_sync.png)](/wiki/File:armareforger-ebt_materialspreview_material_sync.png)

### Materials Synchronisation

The feature let user to keep materials synced with Enfusion after FBX Import, in case user makes a change to material in Workbench (changing texture or value), pressing **Update Materials** button will reflect these changes in Blender.

Valid (absolute) path to xob.meta file has to be provided, note that during EBT custom FBX Import this path gets filled automatically if such file exists.

In case user creates an asset from scratch, before using **Update Materials** for the first time, FBX must be exported and registered in Workbench, this creates xob.meta file that holds materials mapping, material's name (in Blender) → Enfusion material (emat).

Make sure that changes made to a material in Workbench get saved before **Update Materials** button is pressed.

[![armareforger-ebt materialspreview materialsediting ui.png](/wikidata/images/thumb/8/88/armareforger-ebt_materialspreview_materialsediting_ui.png/300px-armareforger-ebt_materialspreview_materialsediting_ui.png)](/wiki/File:armareforger-ebt_materialspreview_materialsediting_ui.png)

### Materials Editing

Along with Materials import, EBT also allows users to edit supported Enfusion materials. For this, custom panels were added to **Material Properties**.
Note that editing material's properties is local right now, it means that changes can't be exported to Workbench.

User can edit material's properties by selecting one of the material slots and panels change according to selected Enfusion shader type (PBRBasic, PBRMulti, etc.).

ⓘ

User can leave any UV source widget blank in case first UV set supposed to be used.  

[![armareforger-ebt materialspreview uv 1.png](/wikidata/images/8/89/armareforger-ebt_materialspreview_uv_1.png)](/wiki/File:armareforger-ebt_materialspreview_uv_1.png)

[![armareforger-ebt materialspreview uv 2.png](/wikidata/images/e/e2/armareforger-ebt_materialspreview_uv_2.png)](/wiki/File:armareforger-ebt_materialspreview_uv_2.png)

⚠

Do not manually edit text field in texture slots widgets!

[![armareforger-ebt materialspreview textureslot field.png](/wikidata/images/0/05/armareforger-ebt_materialspreview_textureslot_field.png)](/wiki/File:armareforger-ebt_materialspreview_textureslot_field.png)

User can only set a texture that has been already registered (has meta file).

### Debug Channels

|  |  |
| --- | --- |
| Users can take advantage of debug channels like in Workbench.  [armareforger-ebt materialspreview debugchannels.png](/wiki/File:armareforger-ebt_materialspreview_debugchannels.png)  Supported channels:   * None * Albedo * Roughness * Metallic * World Normals * Mask | * [Normals](/wiki/File:armareforger-ebt_materialspreview_debugchannel_normals.jpg)  Normals * [Normals](/wiki/File:armareforger-ebt_materialspreview_debugchannel_normals.jpg "Normals")  Normals * [Roughness](/wiki/File:armareforger-ebt_materialspreview_debugchannel_roughness.jpg "Roughness")  Roughness * [Mask](/wiki/File:armareforger-ebt_materialspreview_debugchannel_mask.jpg "Mask")  Mask * [Albedo](/wiki/File:armareforger-ebt_materialspreview_debugchannel_albedo.jpg "Albedo")  Albedo |
