# [FPS Diagnostic Plugin](https://community.bistudio.com/wiki/Arma_Reforger:FPS_Diagnostic_Plugin)

| FPS Diagnostic |
| --- |
| [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") plugin |
| `Ctrl` + `Alt` + `⇧ Shift` + `F` |
| Generate an FPS heatmap to detect terrain areas with performance issues |
| **File:** [SCR\_BIKIWeaponHelper.c](enfusion://ScriptEditor/scripts/WorkbenchGame/ResourceManager/BIKI/Helpers/SCR_BIKIWeaponHelper.c) |

The **FPS Diagnostic** plugin is an advanced [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") plugin designed to analyse the performance of a terrain.
It automatically collects FPS values on a grid of points distributed across the map and generates a visual heatmap from them.
This heatmap allows terrain makers to quickly identify areas that suffer from performance drops.

## Main Features

* **Automated Capture:** The plugin moves the camera over a grid covering the entire terrain, stopping at each point to measure FPS.
* **Multi-Angle Analysis:** At each grid point, it can take measurements from several camera orientations (by default, the four cardinal directions) for a more comprehensive analysis.
* **Heatmap Generation:** The result is an image (.dds file) where colours represent performance levels.
* **Accurate Measurement Mode:** It can run measurements in "Game Mode" (rather than in Workbench) to avoid the performance overhead of the Workbench and obtain results closer to the player experience.
* **High Configurability:** The plugin offers numerous options to fine-tune the accuracy, resolution, appearance of the heatmap, and capture behaviour.

## Parameters

### Camera

* **Position offset**: defines the camera's position offset from the terrain or ocean surface. By default, it is placed 7.5 meters above.
* **Analyse On Ocean**: points located over the ocean will be included in the analysis, otherwise they are ignored.
* **Randomise Positions**: the order of capture points is random. Otherwise they are processed sequentially, which can benefit from pre-loading areas.
* **Orientations**: allows to define a list of orientations (Pitch, Yaw, Roll) for the camera at each point. If left empty, the plugin uses 4 default angles (North, East, South, West with a pitch of -10°).
* **Start Delay**: waiting time (in seconds) before the capture begins, to allow the game to load correctly and switch to fullscreen if necessary.
* **Scene Pause**: pause time (in seconds) after each camera movement to let the scene stabilise before measuring FPS.

### Heatmap

* **Heatmap Colour Mode**: chooses the colour style of the heatmap:
  + Greyscale: Black (low FPS) to White (high FPS).
  + Thermal: Blue (low) → Green → Red (high).
  + Alpha: Transparent (low) to White (high).
* **Heatmap Value Inversion**: inverts the colour scale (e.g. in Greyscale black will represent the highest FPS, etc).
* **Heatmap Max Value Mode**: determines the FPS value considered as the maximum for the colour scale. This helps to normalise the heatmap and eventually highlight extreme values (see below).
* **Heatmap Highlight Values Above Max**: if an FPS value exceeds the maximum defined above, it will be highlighted with a specific colour (e.g. red in Greyscale).
* **Heatmap Definition**: defines the resolution of the analysis grid (e.g. 64 for a 64x64 grid); a higher definition gives a more precise heatmap but significantly increases the capture time.
* **Heatmap Resolution Factor**: multiplies the size of the final image. For example with a factor of 2, each data point will occupy 4 pixels (2×2) in the image.

### Mode

* **Use Game Mode**: strongly recommended - launches the capture in game mode for more reliable FPS measurements (less Workbench overhead).
* **Use Fullscreen**: if Use Game Mode above is enabled, the capture is done in fullscreen.

### Misc

* **Open Heatmap**: allows to automatically open the heatmap image and/or its containing folder once the generation is complete.
* **Time Estimate Frequency**: frequency (in seconds) at which the remaining time estimate is refreshed in the console.

### Debug

* **Use Fake Data**: uses random FPS data instead of real measurements in order to quickly test the heatmap configuration without waiting for a long capture to finish.

## Typical Usage

1. Load the world to analyse.
2. Launch the **FPS Diagnostic** plugin via the Plugins menu or the shortcut.
3. Configure the settings in the dialog box. For a first pass, the default values are a good starting point except for the four camera angles; start with only one (north) at first. Consider using Game Mode.
4. A second dialog box appears with an estimate of the total capture time. Confirm to launch.
5. **Do not touch the computer**. The plugin will take control. Workbench window losing window will cancel the capture.
6. Once finished, the plugin generates the .dds heatmap file in your project's main folder and opens it if the option was checked.
7. Analyse the image: "cold" (or dark, depending on the mode) colour areas indicate performance issues to investigate.
