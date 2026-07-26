# [Textures](https://community.bistudio.com/wiki/Arma_Reforger:Textures)

## Source Image format

* preferred image data format is **TIFF** (.tif, .tiff) - it provides the best output size and compression. We use RGB Color mode and 8 Bits per channel, image compression ideally LZW.
* all other formats (.png, .tga, ...) will work as well, but are not preferred.
* recommended settings for saving in Photoshop (for better compatibility reasons across SW):
  + Compression Type: LZW
  + Layer Compression: Discard Layers and Save a Copy

[![armareforger-modded-weapon-saving-texture.png](/wikidata/images/3/33/armareforger-modded-weapon-saving-texture.png)](/wiki/File:armareforger-modded-weapon-saving-texture.png)

## Basics

⚠

The **texture suffix is important** as the Workbench decides of a default import profile from it, greatly improving performances and storage space - see [Texture Properties](#Texture_Properties) for a list of possible values.

* **All textures** are **8-bit per channel** to save memory, unless "Original Pixel Bit Depth" is checked in texture "import settings", which is really ok only for HDR (ENV) textures used for lighting in Workbench viewport.
* Enfusion uses DirectX normal maps and **only red (+x) and green (-y) channels**.
* Upon registering a texture and importing it for the first time, correct Compression is automatically assigned based on used suffix **except for channel masks 1 and 2** - these must be set **manually**.
* When **RedGreenHQCompression** is used, only Red and Green channels of texture are compressed and used, other channels are not used (appear as black in workbench preview). Similarly, **RedHQCompression** compressed only R channel, others are not used
* Best quality for textures with 3 to 4 channels (RGB/RGBA) is achieved by **ColorHQCompression**.
* When looking for best compression in the case of alpha channel missing, or the alpha is very contrasted like almost just with values 1 and 0 in pixels, **DXTCompression** can be used. This leads to a six times smaller texture but with worse quality than **ColorHQCompression**.

## Texture Types

### Base Color

Albedo of the surface, which basically defines what color each pixel have.

### Roughness

Greyscale texture, which defines which parts of the model are shiny or rough looking.

### Metalness

Greyscale texture which defines which parts of model behave like metal (value 1 - steel, gold, aluminum etc) or dielectrics (value 0 - wood, rubber, soil etc).

### Normal

Define bumpiness of the model surface.

### Opacity

Define which parts of model are transparent (0) or opaque (1).

### Height

Used for parallax effect which emulates high detail per pixel tessellation of decal or terrain surface (currently no usage for model shaders).

### Mask

There are multiple mask types:

#### Global Mask

Determine distribution of "sub-materials" inside PBRMulti and PBR2Layers shaders.
Each channel represents here one "sub-material".
It can be imported as RGB texture or R-channel only depending on how much masks you need to get from the texture.
If a mask for PBR2Layers shader or just for 2 sub-materials in PBRMulti is needed, then the R-channel only mask is enough (black and white).
If a mask for 3+ sub-materials in PBRMulti is needed, then the RGB mask should be used (black, red, green, blue).

#### Detail Mask

Greyscale texture, which determines "breakup" of the "Global Mask". Result masking should look more detailed (hi-res).

#### Normal Mask

Basically the same as one channel Global Mask with a mask for Detail Normal in the second channel. it is used in PBR2Layers shader.

### Camo mask

Special mask, which defines where "Camo" color texture appears in PBRCamo shader.

### Ambient Occlusion

Determine the intensity of ambient lighting in places where the natural light does not reach.

### Cavity

Determine intensity of both Diffuse and Ambient lighting (both in shadows and on sun).

### Global/Macro Textures

These are special textures which combine with underlying sub-materials in a specific way in order to breakup sub-materials' repetitiveness or add some unique "macro" looking details.  
Albedo/roughness uses \_MCR because they are imported differently than \_BCR.
For normal, metal and ambient occlusion the ordinary \_NMO textures is used, because the same texture import is required.
There is an option to use \_O texture suffix for ambient occlusion if only that is required.

### Emissive

Define which parts of model will glow and how much at night or in the dark.

### Environment Cube Map

HDR (High Dynamic Range) texture used only for lighting inside viewport inside Workbench.
It has no real application for game assets but a world can be created with a different lighting setup and atmosphere.

## Texture Import

1. Place the texture into the game or mod project data structure.
2. In Resource Browser, right-click on the texture and select **Register** or **Register and Import**.
   1. **Register** - only meta file containing import settings is created. These setting can now be changed from default in the "Import Settings" tab, which appears when the source texture is opened.
   2. **Register and Import** - the metafile containing import settings is created and the texture is imported right away with default import settings.
   3. **Import** - upon binarised and compressed version import of the source, an .edds texture is created right next to the source texture.
3. After the import, the texture preview can be opened by double-clicking on it or right-click and selecting **Open file in new tab**.

### Import Settings

The import settings are quite complex which is why it is important to use the correct texture suffix,
as it 99% of the tim sets the proper settings automatically (exceptions being \_MCR texture and 1 or 2 channel MASK where manual color space setup is required).

#### Format Compress

Used for quality/speed of internal mipmaps compression, higher values mean bigger compression time.

#### Compress Treshold

Ratio of internal data mipmap compression.
Each mipmap is compressed using internal compression - if the ratio between compressed and original size is less than this ratio, mipmap is stored in compressed format, otherwise in original.

#### Remove Mips

Remove better mipmaps (1, 2, etc) in order to lower texture memory consumption.
The full mipmap chain is generated before the mipmaps are removed unless "ContainMips" is checked when full custom mimmap atlas/texture is imported.

ⓘ

This is useful to e.g create resources for consoles or some demo/low quality version.

#### Max Size

This setting sets the maximum resolution in which the texture is going to be imported.

#### Conversion

Defines conversion of texture format used at runtime (eg. compressed GPU format or channel removal).

ⓘ

| Enfusion | DXT equivalent |
| --- | --- |
| BC1 | DXT1 |
| BC2 | DXT2/DXT3 |
| BC3 | DXT4/DXT5 |

* In order to check size differences between these formats, set **FormatCompress** to **Copy** and check the resulting file size. Make sure to reset **FormatCompress** back to its previous value once done.
* For information on compressed formats, refer to following links (note that BC1 is DXT1, BC2 covers DXT2 and DXT3, BC3 covers DXT4 and DXT5):
  + <https://docs.microsoft.com/en-us/windows/win32/direct3d10/d3d10-graphics-programming-guide-resources-block-compression>
  + <https://docs.microsoft.com/en-us/windows/win32/direct3d11/texture-block-compression-in-direct3d-11>
  + <https://en.wikipedia.org/wiki/S3_Texture_Compression>
  + <https://www.khronos.org/opengl/wiki/S3_Texture_Compression>

##### None

No channel compression is used.

##### DXTCompression

Conversion into lossy DXT1 (RGB) / DXT5 (RGBA) format (decision based on alpha channel's presence).

Final size: usually 1/4 to 1/6 of uncompressed RGBA

##### Alpha8

Only alpha channel is kept - when used as a texture, RGB is automatically 0.

Final size: 1/4 of uncompressed RGBA

##### Red

Only the red channel is kept - when used as a texture, GB is automatically 0 and A is 1.

Final size: 1/4 of uncompressed RGBA

##### RedHQCompression

Only red channel is kept and compressed using lossy BC4 compression - when used as a texture, GB is automatically 0 and A is 1.

Final size: is 1/8 of uncompressed RGBA

##### RedGreen

Only red and green channels are kept - when used like a texture, B is automatically 0 and A is 1

Final size: is 1/2 of uncompressed RGBA

##### RedGreenHQCompression

Only red and green channel are left and compressed by lossy BC5 compression - when used as a texture, B is automatically 0 and A is 1.

Final size: 1/4 of uncompressed RGBA

##### ColorHQCompression

RGBA compression into lossy BC7 format - better quality than DXT compression, a little worse compression speed.

Final size: 1/4 of uncompressed RGBA

##### HDRCompression

16 bits HDR lossy compression into BC6 format

Final size: 1/4 of uncompressed texture

#### ConversionQuality

This slider defines quality of compressor when converting the BCx format.
Higher values may mean much worse compression time with a little quality improvement.

#### OriginalPixelBitDepth

Usually all textures with better precision (16 bits) are converted automatically to 8 bit format. When this option is checked, the original texture format is kept.

ⓘ

**"Conversion**" must be set to "**None"**.

#### Swizzling

Special channel reordering that may differ for various materials and texture usage - it is present as legacy support for some internal test projects.

#### ColorSpace

conversion of color space, albedo maps should be in sRGB, otherwise linear, this value should be usually left as **"Default"**.  
if conversion to another color space is performed, the source would be ideally 16 bits due to possible loss of quality.

##### Default

* ToSRGB or ToLinear is used based on texture suffix
* Tio. - only texture type, where both color spaces are possible is \_MCR texture. The colors in this texture can be prepared for both color space usage.

##### ToSRGB

The given texture is considered to be done in graphics editor such as Photoshop and the source texture values/colors are shifted from sRGB in which they were created so they can be correctly loaded into the shaders,
usually for albedo textures (BCR).

##### ToLinear

The texture values/colors are not altered (are considered already as linear) and are loaded into shaders just as they are.

#### ContainsMips

Must be checked in case the source texture contains mipmaps (see picture).

ⓘ

* **mipmaps or MIPs** = smaller versions of original texture, which get gradually loaded with increasing distance from camera.
* they are next each other, e.g. for 256x256 texture the source width is 256+128+...+1 = 511 and height 256
  + The engine can handle/import of the texture even with 512x256 (and other resolutions with power of 2) and it will remove one horizontal pixel to be 511 automatically!

#### GenerateMips

Generate the full mipmap chain.
This is enabled by default as mipmap helps a lot to:

* speed up rendering
* fight against aliasing
* surprisingly reduce memory overhead in engine even though they are bigger: [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") can load only the necessary mipmaps and not the full mipmap chain

  ⓘ

  This currently only applies to textures used in materials, not to UI textures.

#### MipMapFunction

Define what to do with mipmap after them being generated.

##### Filter

Normal filtering - reduces contrast in lower mipmaps as they are somehow avereage from previous one.

##### Normalize

Useful for normal maps as usual filtering decrease contrast, though it may add noise at lower mipmaps (specular aliasing).

##### ColorNoise

Color is left as it is and to alpha channel is generated noise (for some special purposes)

#### MipMapFilter

Three types for filtering mipmaps, **Box** is default, others can produce better mipmaps but sometimes may show artifacts.

* **Box, Triangle, Kaiser**
* **FoliageAlpha 1 - 5** - this is used for vegetation opacity textures, so they keep better/denser opacity also in smaller MIPs, which are drawn further away from camera

#### TiledTexture

Used to improve wrapping around edges when generating mipmaps

#### VolumeTexture

Will disable other settings and convert flat texture for example 256x16 texture to 16x16x16 3D volume texture.
Used mainly for LUT textures like.

#### GenerateCubeMap

Generate cube map from unwrapped texture with spherical mapping

## Texture Properties

and channel distribution table
below is chart of all valid texture types, in what shaders they are used and some of their "import properties".

| Material | | | | | | | | | Texture | | | | | | | | |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Particle | Grass | Terrain | PBRDecal | PBRTree Crown/Trunk | PBRMulti | PBR2Layers | PBRBasic | PBRCamo | Type | Suffix | Red Channel | Green Channel | Blue Channel | Alpha Channel | Bit depth per pixel | Color space | Compression |
| Checked |  |  |  |  |  |  |  |  | Base Color | \_BC | Albedo | Albedo | Albedo | Unchecked | 32 | sRGB | ColorHQCompression |
| Checked |  |  |  |  |  |  |  |  | Base Color + Opacity mask | \_BCA | Albedo | Albedo | Albedo | Opacity mask | 32 | sRGB | ColorHQCompression |
|  | Checked | Checked | Checked | Checked | Checked | Checked | Checked | Checked | Base Color + Roughness | \_BCR | Albedo | Albedo | Albedo | Roughness | 32 | sRGB | ColorHQCompression |
|  |  |  | Checked | in PBRTree Trunk, shader behaves as N~~M~~O: metalness is ignored | Checked | Checked | Checked | Checked | Normal map + Metalness + Ambient Occlusion | \_NMO | Normal +X | Normal -Y | Metalness | Ambient Occlusion | 32 | linear RGB | ColorHQCompression |
| Checked |  |  |  |  |  | Checked | Checked | Checked | Normal map | \_N | Normal +X | Normal -Y | Unchecked | Unchecked | 16 | linear RGB | RedGreenHQCompression |
|  | Checked |  | Checked | Checked |  |  | Checked |  | Transparency/Opacity | \_A | Opacity mask | Unchecked | Unchecked | Unchecked | 8 | linear RGB | RedHQCompression |
|  |  |  | Checked |  |  |  |  |  | Height | \_H | Height | Unchecked | Unchecked | Unchecked | 8 | linear RGB | RedHQCompression |
|  |  |  |  |  | Checked 1 channel |  |  |  | 1 channel Detail Mask | \_DETAIL\_MASK | Mask | Unchecked | Unchecked | Unchecked | 8 | linear RGB | RedHQCompression (must set manually) |
|  |  |  |  |  | Checked 1 channel | Checked 1 channel |  |  | 1 channel Mask | \_GLOBAL\_MASK | Mask | Unchecked | Unchecked | Unchecked | 8 | linear RGB | RedHQCompression (must set manually) |
|  |  |  |  |  |  | Checked 2 channels |  |  | 2 channels Mask | \_GLOBAL\_MASK | Mask | Mask (DetailNormal) | Unchecked | Unchecked | 16 | linear RGB | RedGreenHQCompression (must set manually) |
|  |  |  |  |  | Checked 3 channels |  |  |  | 3 channels Mask | \_GLOBAL\_MASK | Mask | Mask | Mask | Unchecked | 24 | linear RGB | ColorHQCompression/ DXTCompression |
| Used in "clutter collection config" and "procedural grass config" to drive the distribution | | | | | | | | | Clutter Mask | \_CMASK | Clutter Distribution Mask | Clutter Height Mask | Unchecked | Unchecked | 16 | linear RGB | RedGreenHQCompression |
|  |  |  |  |  | Checked |  |  |  | VFX | \_VFX | Mask (dirt) | Mask (mud) | Unchecked | Unchecked | 16 | linear RGB | RedGreenHQCompression |
|  |  |  |  | Checked | Checked | Checked |  |  | Global/Macro Base color and Roughness | \_MCR | Color | Color | Color | Roughness | 32 | linear RGB / sRGB | ColorHQCompression |
|  | Checked |  |  | Checked in PBRTree Trunk, shader behaves as N~~M~~O: metalness is ignored |  |  |  |  | Normal map + Transmittance + Cavity | \_NTC | Normal +X | Normal -Y | Transmittance | Cavity | 32 | linear RGB | ColorHQCompression |
|  |  | Checked |  |  |  |  |  |  | Normal map + Height + Ambient Occlusion | \_NHO | Normal +X | Normal -Y | Height | AO | 32 | linear RGB | ColorHQCompression |
|  |  |  |  |  |  |  |  | Checked | Camo mask + roughness + metalness | \_CRM | Mask | Roughness | Metalness | Unchecked | 24 | linear RGB | ColorHQCompression/ DXTCompression |
| Checked |  |  |  |  |  |  | Checked |  | Emissive | \_EM | Color | Color | Color | Unchecked | 24 | linear RGB | ColorHQCompression/ DXTCompression |
| For lighting purposes inside Workbench viewport and in World editor (replaces atmosphere lighting). It has no in-game usage in shaders. | | | | | | | | | Environment cube map | \_ENV | Color | Color | Color | Unchecked | 24+ | linear RGB | HDRCompression |

## Texture Preview

After opening a texture in Workbench, you will see its preview in the Workbench viewport, above it are some buttons and right to it "Details" and "Import Settings" tabs.

See [Resource Manager: Texture Editor](/wiki/Arma_Reforger:Resource_Manager:_Texture_Editor "Arma Reforger:Resource Manager: Texture Editor").
