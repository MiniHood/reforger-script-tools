# [Enfusion Blender Tools: Materials Library](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Materials_Library)

Library provides some of the Enfusion shaders converted to Blender via its Shader Editor.
Materials in Blender visually assist users while creating a model from scratch and help by creating additional data like baking textures and more.

Keep in mind that the main purpose of converted materials is to get a "close enough" preview that should speed up assets creation in Blender;
the [Technicalities](#Technicalities) section below sums up reasons why materials in Blender vs. Workbench aren't visually the same.

ⓘ

Materials library setup is not required in case user don't want to take advantage of drag & dropping materials onto an Asset Browser mesh.

## Setup

1. In Blender, go to **Edit > Preferences > File Paths > Asset Libraries** and add a new library with the **plus (+)** button.
2. Set EnfusionBlenderTools\data folder as **Path**, typically C:\Users\<username>\AppData\Roaming\Blender Foundation\Blender\<blenderVersion>\scripts\addons\EnfusionBlenderTools\data  
   The path to the addon folder can be found in **Preferences > Add-ons** under Enfusion Tools)
3. Set the library name - note that under this name a library can be found in Asset Browser
4. Set **Import Method** to **Append**
5. Save the preferences

## Usage

1. In Blender, go to **Asset Browser** under **Data**
2. Select the previously created library from the list

   ⚠

   Before dragging a material from **Asset Browser**, make sure that the import method is set to **Follow Preferences** (meaning the import method set in Library setup (Append) will be used).  
   This is important as it prevents breaking Material workflow in Enfusion Blender Tools!
3. Drag a material from **Asset Browser** onto a mesh

   ⓘ

   Beware, as:
   * Drag & drop from **Asset Browser** is possible only in Object mode (this is a Blender limitation)
   * It is highly recommended to rename assigned material from *ShaderClassName* to something meaningful, e.g MatPBRBasic → Bricks

## Technicalities

Enfusion shaders converted to Blender serve just as a "close enough" preview of what can be achieved in Workbench.
There are several factors that contribute to the impossibility of delivering an identical visual experience in Blender's EEVEE renderer.
Here are a few key facts that highlight these limitations:

* PBR implementation in Blender vs Enfusion is different
* There is currently no Shader API in Blender ([Eevee](https://docs.blender.org/manual/en/latest/render/eevee/introduction.html)) that could be used to write an Enfusion shader "twin", instead Shader Editor has to be used to manually create a material
* Nodes provided by Shader Editor do not allow to access things like GBuffer or scene lighting information, therefore some specific material properties cannot be supported
* PP effects implementation in Blender is different, some PP effects are not implemented at all
* The way that Blender treats Alpha information is different from Enfusion (e.g MatPBRBasic)
  + Alpha Test (Clip) is not allowed in Alpha Blend mode (only in Alpha *Clip* mode)
* Shaders in Blender use source texture formats (tif, png...) whereas shaders in Enfusion use compressed .edds textures
  + There may be visual differences spotted due to compression for example when playing with Alpha Test/Alpha Mul parameters
