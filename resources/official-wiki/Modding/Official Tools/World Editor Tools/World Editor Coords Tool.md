# [World Editor: Coords Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Coords_Tool)

The **Coords Tool** is a [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor") tool that allows terrain creators to quickly navigate to specific coordinates and save important locations.

When creating terrains and scenes in Arma Reforger, it is often needed to:

* Return to specific locations to check or modify elements
* Share precise positions with other team members
* Document points of interest or issues to resolve
* Test different camera views

The Coords Tool facilitates these tasks by allowing you to:

* Save positions with a simple click
* Generate shareable links (enfusion:// protocol)
* Navigate instantly to coordinates
* Keep a history of visited positions

## User Interface

[![](/wikidata/images/thumb/3/34/Coords_Tool_Interface.png/400px-Coords_Tool_Interface.png)](/wiki/File:Coords_Tool_Interface.png)

Coords Tool Interface

The tool can be accessed through **Menu**: Tools → Coords Tool.  
It consists of three main sections: [Coordinates](#Coordinates), [Options](#Options) and [Data](#Data).

### Coordinates

* **Position**: X, Y, Z coordinates in world space
* **Rotation**: Camera angles (Pitch, Yaw, Roll) in degrees

### Options

* **Use Web Prefix**: Adds the https://enfusionengine.com/api/redirect?to= prefix to generated links, allowing them to be used in web browsers

### Data

* **Entries**: List of saved positions (most recent entry at the top)

## Usage

### Navigate to Manual Coordinates

1. Enter the **Position** coordinates (X, Y, Z)
2. Enter the **Rotation** (Pitch, Yaw, Roll) if needed
3. Click **Go to coords**
4. The camera instantly moves to this position

### Copy Current Position

1. Position your camera at the desired location
2. Click **Copy link to clipboard**
3. A link in the format enfusion://WorldEditor/... is copied to the clipboard
4. You can share this link with other developers

**Link format:** enfusion://WorldEditor/worlds/world\_name.ent;X,Y,Z;Yaw,Pitch,Roll - see [Workbench Links](/wiki/Arma_Reforger:Workbench_Links "Arma Reforger:Workbench Links").

### Navigate from Clipboard

1. Copy an enfusion:// link (received from a colleague or generated previously) to the clipboard (e.g `Ctrl` + `C`)
2. Click **Go to clipboard link**
3. The camera moves to this position

**Accepted formats:**

* enfusion://WorldEditor/worlds/...;X,Y,Z;Pitch,Yaw,Roll
* https://enfusionengine.com/api/redirect?to=enfusion://...
* X,Y,Z;Pitch,Yaw,Roll (simplified format, works across all terrains)

### Save an Entry to History

1. Position your camera
2. Click **Add entry**
3. A new entry is created with:
   * An automatic name: Link #X (world\_name)
   * The complete position link
4. The entry appears at the top of the **Entries** list

**Using a saved entry:**

1. Open the entry in the **Entries** list
2. Copy the content of the **Link** field
3. Click **Go to clipboard link**

## Practical Use Cases

### Bug Documentation

Problematic position found → Add entry → Share link in bug report

### Collaborative Work

* Creator A: Finds a good viewpoint → Copy link to clipboard → Sends to Creator B
* Creator B: Pastes the link → Go to clipboard link → Sees exactly the same view

### Recurring Test Points

Save multiple entries for:

* Player spawn point
* Main city center
* Map boundaries
* Problem areas

## Console Messages

The tool displays messages in the Workbench console:

| Message | Meaning |
| --- | --- |
| Camera set to Position = X Y Z, Rotation = P Y R | Successful navigation |
| Link copied to clipboard: enfusion://... | Link successfully copied |
| Current position is last log entry | This position is already the last entry |
| String is in invalid format... | Pasted link format is incorrect |
| Error obtaining the link | Error generating the link |

## Limitations

* History is not persistent (lost when Workbench closes)
* Coordinates are specific to the currently open terrain (unless simple coordinates are used)
* Links only work if the terrain is accessible to the recipient

## Tips

* Enable "Use Web Prefix" to create clickable links in web browsers (useful for wikis and documentation)
* Entries can be manually renamed for better organisation
* Use the simple format X,Y,Z;Pitch,Yaw,Roll for quick annotations in your notes
