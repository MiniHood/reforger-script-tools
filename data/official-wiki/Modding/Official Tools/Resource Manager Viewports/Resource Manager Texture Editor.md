# [Resource Manager: Texture Editor](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Manager:_Texture_Editor)

## Horizontal Bar

### Preview background

* checker board pattern
* white color
* grey color
* black color

### Show alpha mask

Highlights alpha mask's area in transparent red.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Pick color to clipboard

Copies the clicked pixel's colour to the clipboard in HTML format (e.g #025D00).

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

### Lock texture settings

Locks all the Horizontal Bar's settings, preventing any further modification.

### Color scheme

* Default
* sRGB
* Linear

### Brightness

Adjust brightness through the slider.

### Toggle/Isolate channel

Option for RGB channels together or separate RGBA channels.

### Filtering mode

#### Point filter

#### Bilinear

#### Trilinear

Mipmaps are automatically chosen and blended.

##### mip level

In range 1..12.

### Auto scale to fit

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** this must be updated.

### Depth

🚧

**[TODO](/wiki/Category:To-do "Category:To-do"):** this must be updated.

## Details Tab

### Texture properties

* File size: the file's size (0 for game data)
* Surface size: the file's uncompressed size
* Image format: as the name says
* Image data format
* Mipmaps: number of mipmaps levels
* Width: image's width in pixels
* Height: image's height in pixels
* Depth: levels of depth
* Contains alpha channel: as the name says
* Is black and white: as the name says
* Last pixel: X and Y coordinates of the currently-hovered pixel
* Last pixel <0, 1>: same, but in image's 0..1 range (0.5 = middle)
* Last pixel color (float):

### Histogram

* Pixel count: as the name says (width × height)
* Red
* Green
* Blue
* Alpha

### Color graph

This color statistics graph can be manipulated with left and right click, to respectively zoom on a selected range and slide the graph values. Mouse wheel can also be used.

### Reset

This button resets the color graph to 0..256 range.

## Import Settings Tab

* Edit also selected files: also apply changes to the files selected in Resource Browser
* Apply changes to all platforms: as the name says
* Name
* Tags
* License

### General

#### Present

Whether the resource will be imported and available in data.

#### Source File

Can be used to override source file to any compatible file (model) in folder.

### Unsorted

#### Target Format

Can be one of:

* EnfusionDDS
* DirectXDDS

#### Format Compress

Can be one of:

* Copy
* Fastest
* Medium
* Best

#### Compress Treshold

[LZ4 Compression](https://en.wikipedia.org/wiki/LZ4_(compression_algorithm)) threshold in percent in range 0..100. If the compression ratio is bigger then the uncompressed version is used.

#### Remove Mips

Remove the specified number of large mipmaps.

This functionality may be affected by the Max Size parameter (below) which can also remove some mipmaps to achieve a specified size.

#### Max Size

Set the texture size limit (dimensions, not file weight) in powers of 2 (4, 8, 16 etc up to 16384). The texture will be downsised when its width or height is larger than this value.

This functionality may be affected by the [Remove Mips](#Remove_Mips) parameter above which can affect texture size too.

#### Conversion

Set conversion from RGBA into specific format of channels. Can be one of:

* None
* DXTCompression
* Alpha8
* Red
* RedHQCompression
* RedGreen
* RedGreenHQCompression
* ColorHQCompression
* HDRCompression

#### Conversion Quality

Set conversion quality for compressed formats in range 0..1. Higher value means better quality ratio but slower compression.

#### Original Pixel Bit Depth

Set whether or not the original pixel bit depth should *try* to be kept between conversion; otherwise the texture will be converted to 8 bit format.

#### Color Space

Can be one of:

* ToLinear
* ToSRGB

#### Contains Mips

#### Generate Mips

#### Mip Map Function

Set mipmap downsize function. Can be one of:

* Filter
* Normalize
* ColorNoise
* FoliageAlpha1
* FoliageAlpha2
* FoliageAlpha3
* FoliageAlpha4
* FoliageAlpha5

#### Mip Map Filter

Set mipmap generation filter. Can be one of:

* Box
* Triangle
* Kaiser

#### Mip Alpha Fade Start

Set the alpha-to-black fading's mipmap start threshold in range 0..1.

For example with an image with 8 mimaps, 0.5 means the 4th mipmap.

#### Mip Alpha Fade End

Set the alpha-to-black fading's mipmap end threshold in range 0..1.

For example with an image with 8 mimaps, 1 means the 8th mipmap.

#### Mip Alpha Fade Value

Set the alpha fading's mipmap value in range 0..1.

For example, 0 means that the alpha will be faded to black.

#### Tiled Texture

Set if the texture is tiled - affects mimaps generation.

#### Generate Cubemap

Set the conversion from a panoramic/spherical texture to a cubemap texture.

Source texture must be an equirectangular panorama.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.1.0 "Category:Arma Reforger/Version 1.1.0") [1.1.0](/wiki?title=Category:Arma_Reforger/Version_1.1.0&action=edit&redlink=1 "Category:Arma Reforger/Version 1.1.0 (page does not exist)")

#### Volume Texture

Converts 2D-layered texture to 3D texture.
