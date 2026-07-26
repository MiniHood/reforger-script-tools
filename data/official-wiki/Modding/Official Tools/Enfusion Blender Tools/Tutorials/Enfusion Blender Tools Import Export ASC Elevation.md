# [Enfusion Blender Tools: Import/Export ASC Elevation](https://community.bistudio.com/wiki/Arma_Reforger:Enfusion_Blender_Tools:_Import/Export_ASC_Elevation)

[![](/wikidata/images/thumb/3/39/armareforger-ebt_asc_file_import_example.png/800px-armareforger-ebt_asc_file_import_example.png)](/wiki/File:armareforger-ebt_asc_file_import_example.png)

Example of procedural terrain editing by sticking terrain vertices to a complex shape and then result imported in World Editor

This feature allows to import an [ESRI asc grid](https://en.wikipedia.org/wiki/Esri_grid) format into Blender as regular grid mesh as well as to export regular grid mesh back to ASC file (and import it in World Editor).

ⓘ

This tool is a very simplistic implementation that imports the whole file at once; therefore, depending on grid resolution, operations may take some time.

## Import ASC File

[![armareforger-ebt asc file import menu.png](/wikidata/images/3/38/armareforger-ebt_asc_file_import_menu.png)](/wiki/File:armareforger-ebt_asc_file_import_menu.png)

In the Enfusion Blender Tools menu, use **Import > ASC elevation**.

Once the import is done, Min/Max elevation and Elevation range information is printed out in the Info log.

[![armareforger-ebt asc file info log.png](/wikidata/images/thumb/0/04/armareforger-ebt_asc_file_info_log.png/600px-armareforger-ebt_asc_file_info_log.png)](/wiki/File:armareforger-ebt_asc_file_info_log.png)

## Export ASC File

[![armareforger-ebt asc file export menu.png](/wikidata/images/4/4e/armareforger-ebt_asc_file_export_menu.png)](/wiki/File:armareforger-ebt_asc_file_export_menu.png)

Select the terrain object first, then in the BT menu use **Export > Terrain to ASC**.

Once the export is done, *Min/Max elevation* and *Elevation range* info is printed out in Info editor.

## Elevation Modification

[![armareforger-ebt asc file good bad modification.png](/wikidata/images/thumb/6/60/armareforger-ebt_asc_file_good_bad_modification.png/400px-armareforger-ebt_asc_file_good_bad_modification.png)](/wiki/File:armareforger-ebt_asc_file_good_bad_modification.png)

The elevation of an imported terrain mesh can be modified in various ways in Blender, e.g via Edit/Sculpt mode or by using procedural approaches.

Terrain mesh is represented as a regular grid and must remain as such after any modifications are done to mesh, otherwise elevation changes can appear in a slightly shifted position.

### Edit Mode

In **Edit** mode, the safest way to prevent such issue is to only shift vertices in the vertical axis by dragging the gizmo's blue arrow which locks movement to Z (vertical) axis.

[![armareforger-ebt asc file edit mode.png](/wikidata/images/thumb/c/c6/armareforger-ebt_asc_file_edit_mode.png/400px-armareforger-ebt_asc_file_edit_mode.png)](/wiki/File:armareforger-ebt_asc_file_edit_mode.png)

### Sculpt Mode

In **Sculpt** mode, user can prevent vertices shifting in horizontal plane by locking the X and Y axis in the following menu:

[![armareforger-ebt asc file sculpt mode.png](/wikidata/images/thumb/6/62/armareforger-ebt_asc_file_sculpt_mode.png/800px-armareforger-ebt_asc_file_sculpt_mode.png)](/wiki/File:armareforger-ebt_asc_file_sculpt_mode.png)
