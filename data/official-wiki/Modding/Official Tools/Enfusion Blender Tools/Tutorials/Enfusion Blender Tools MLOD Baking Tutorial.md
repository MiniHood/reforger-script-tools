# [Enfusion Blender Tools: MLOD Baking Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_MLOD_Baking_Tutorial)

A small tutorial for how to bake MLODs with Blender for those who aren't familiar with Blender.

1. Use the EBT importer
2. Select LOD0 and the MLOD
3. Select the last LOD and navigate to the material section. If you're just rebaking the MLOD and there's an imported MLOD material, remove it and add a new one (if you'll export the model, put the old one back in after you're done with this).
4. Add an image texture to the material
5. Create an empty texture
6. Go to render properties and change the render engine to Cycles
7. Scroll down and open Bake
8. Switch bake type to Diffuse and set Contributions to color only
9. Check Selected to Active and set the extrusion (or also max ray distance) to a distance that will cover the differences between your MLOD and LOD0. If there's transparency where there shouldn't be any, adjust the extrusion distance
10. Right click on the corner of the area with the model, select vertical split and click somewhere in the area with the model to create a new area
11. Switch the area/editor type to image editor
12. Select all by pressing "A" (or select the objects that belong to LOD0 and the last LOD) and make sure the last LOD is highlighted in yellow. If it's orange, "Ctrl+left click" it until it is.
13. Press Bake and wait
14. The resulting texture will appear in the image editor where you can save it
15. Set the file format to TIFF and very importantly, set the compression to LZW.

Done! :-)
