# [World Editor: Prefab Management Tool](https://community.bistudio.com/wiki/Arma_Reforger:World_Editor:_Prefab_Management_Tool)

**Prefab Management Tool** is a toolbox allowing for multiple Prefab operations, such as:

* import 3D models (XOBs) into a new Prefab
* create existing Prefabs' children
* clone existing Prefabs

## Usage

* Select resources in one of the possible ways:
  + Open a Prefab in Prefab Edit Mode
  + (Multi-)select models/Prefabs in the **Resource Browser**
  + Highlight one directory in the **Resource Browser** (XOB import only, to mimic its structure - see [**Keep Single Directory Structure**](#XOB_Import) below)
* Fill the wanted options
* Press the wanted button
* ????
* Profit - Prefabs are created

## Parameters

### General

* **Addon**: file system where new prefabs will be created
* **Overwrite Files**: if a destination file already exists, it is overwritten

### Suffixes

* **Suffixes 1..3**: suffixes to name clones, e.g \_red, \_blue, \_US, etc; the underscore is added automatically if needed - accepted chars: [a-zA-Z0-9\_-]

### Clone

* **Clone Suffixes**: which Suffixes array will be used for Clones - 0 = no suffixes
* **Clone Save Path**: path to save the clones

### Children

* **Children Suffixes**: which Suffixes array will be used for creating Children - 0 = no suffixes
* **Children Save Path**: path to save the Prefabs' children

### XOB Import

* **XOB Save Path**: path to save the XOB Prefabs
* **Keep Single Directory Structure**: if ONE directory is selected and **XOB Save Path** above is defined, relative paths will be used to mimic that directory's structure  
  e.g Assets/Rocks/Granite/Granite\_01**.xob** saves to *XOBSavePath*/Rocks/Granite/Granite\_01**.et**
* **Parent Prefab**: parent Prefab to created Prefabs
* **Create Base Prefab**: create a "\_base" prefab for each imported XOB
