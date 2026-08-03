# [Enfusion Blender Tools: Portal Tools](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Portal_Tools)

## Description

This feature allows users to automatically generate Portals and Portal Volumes on a structure. It will generate all portals on a right position, with naming convention and a material which will automatically be assigned when exporting to Workbench.

It also can be used to generate Portal Volumes(with dummyVolume material) which will automatically resize depending on a room size in real-time. After applying them, the Portal Volumes can be manually adjusted to your needs.

## Portals

Before you want to generate Portals, please ensure you have everything set upped:

1. Collider with **FireView/FireGeo** or **BuildingFireView** layer preset
   1. [Here](/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Objects_Tools "Arma Reforger:Enfusion Blender Tools: Objects Tools") you can learn how to simply assign layer presets to colliders
2. **Sockets** for doors/windows on the **right location** and with the right **naming convention**
3. Opened Workbench with **NET API enabled**

There is no need for anything to be selected or even visible, just make sure everything is in the scene

[![](/wikidata/images/thumb/8/82/armareforger-blender-portal-interface.png/989px-armareforger-blender-portal-interface.png)](/wiki/File:armareforger-blender-portal-interface.png)

Interface of Portal Tools

Once all the necessary preparations are in place, you can press "**Generate Portals**", this will generates planes the **right transformations**, **normals**, **naming convention** and **materials** from **\_SharedData**. Materials depends on the portal size, so if the door/window frame is slightly bigger than the norm no portal material will be assigned! In such cases, a basic Blender Material will be generated, and the user will need to manually assign the appropriate material in Workbench.

ⓘ

**Keep in mind!**  

When generating PRTs, there's a slight margin for error, so it's essential to check all portals if they were generated right and make any necessary adjustments if not.

* Portal will have no scale (If the socket is placed inconveniently)
* Portal material will be default blender material (If the material with these sizes wasn't found in the \_SharedData)
* Portal will have huge scale (If the socket was placed in a location where there is a hole in the collider mesh)

Everything can be fixed manually by adjusting the scale, etc..

## Portal Volumes

[![](/wikidata/images/thumb/9/9e/armareforger-blender-portal-tools-volumes.png/998px-armareforger-blender-portal-tools-volumes.png)](/wiki/File:armareforger-blender-portal-tools-volumes.png)

Portal Volumes generation

To generate a PRTVOL, you have to **select at least two portals** to determine in **which room** the PRTVOL should be generated in. The origin of the PRTVOL will be generated in the **middle of the selected portals**. Then you can simply press "**Generate PRTVOL"** and it will create a PRVOL box with Geometry Nodes on it. This allows you to move the PRTVOL in real-time, as it **dynamically adjust itself according to the room size**. It resizes itself depending where the origin of the PRTVOL is.

✩

**Tip!**  

The resizing will work even when nothing is visible in the scene, but my advice is to set visible either a collider or the BSP and then turn Xray in the Viewport Shading settings to properly see the PRTVOL with the room but in most case you wouldn't need to move the generated PRVOL.

[![armareforger-blender-portal-tools-xray.png](/wikidata/images/4/4a/armareforger-blender-portal-tools-xray.png)](/wiki/File:armareforger-blender-portal-tools-xray.png)

After you have all PRTVOLs at its place you can **apply them** by pressing "**Apply PRTVOL**" and it will apply the geometry nodes and **assign it a dummy volume material** which will be automatically linked in Workbench while exporting. After pressing the apply button you can then **adjust any PRTVOL to your needs** so everything is set up correctly.

The Portal Volume will automatically be placed in a "*Light Portals*" collection.

ⓘ

**Keep in mind!**  

This feature may not be fully automatic for more complex structures that requires cutting planes, so some manual adjustments may still be required!

This button applies the geometry on all PRTVOLs that were generated!
