# [RichText Format](https://community.bistudio.com/wiki/Arma_Reforger:RichText_Format)

The [RichTextWidget](enfusion://ScriptEditor/scripts/Core/generated/UI/RichTextWidget.c;12) is a multiline text widget. It is useful for displaying complex text layouts that are composed by text and/or images.

This widget works by parsing an HTML-like markup language, with tags explained below.

## Supported Styling

### Bold

Supported by SDF fonts only - for bitmap fonts, it can be simulated with the <font> tag

Example

```
<b>bold text</b>
```

### Italic

Supported by SDF fonts only - for bitmap fonts, it can be simulated with the <font> tag

Example

```
<i>italic text</i>
```

### Outline

Supported by SDF fonts only.  
Supports the following attributes:

* color: a comma-separated RGBA value (range 0..255)
* size: the outline's size

Example

```
<outline color="255,128,0,255" size="5">outlined text</outline>
```

### Shadow

Tags producing images support a simplified version of shadows where the image is rendered offset and possibly scaled with a solid color. To use images, use mode="image".

ⓘ

Shadow without an offset with increased size can serve as a simple outline.

Example

|  |  |
| --- | --- |
| Text: | ``` <shadow color="255,0,0" size="5" offset="5,5" opacity="0.5">shadowed text</shadow> ``` |
| Image: | ``` <shadow color="255,0,0" offset="5,5" opacity="0.5" mode="image"><image set="{134A30222457B142}UI/Imagesets/game_template.imageset" name="ImageIncubatorLogo" /></shadow> ``` |
| Image outline: | ``` <shadow color="255,0,0" size="5" mode="image"><image set="{134A30222457B142}UI/Imagesets/game_template.imageset" name="ImageIncubatorLogo" /></shadow> ``` |

### Image

Inserts an image from an ImageSet. Both the image name and the image set are case-sensitive. Default scale is 1.0 which means a line height depends on font.

Example

```
<image set="{134A30222457B142}UI/Imagesets/game_template.imageset" name="ImageIncubatorLogo" scale="2" />
```

### Color

Changes foreground color.  
Supported attributes:

* rgba: a comma-separated RGB**A** value (range 0..255), e.g "255,0,0,255" for bright red
* hex: the colour in '*A*RGB hexadecimal format (e.g "0xFF000000" for black, "0x880000FF" for half-transparent blue)
* name: supported color names:

  + red
  + green
  + blue
  + cyan
  + magenta
  + yellow
  + black
  + white
  + orange
  + purple

Example

```
<color rgba="0,0,255,255">blue text</color>
<color hex="0xFFFF6600">orange text</color>
<color name="green">green text</color>
```

### Font

Renders text with a different font. In case of not finding the supplied font. The text will be rendered with the system font. '*force*' property will disable font override by language settings (e.g. in Asian languages is special font used which support Asian characters)

Example

```
<font name="{54A29B2BD9DE5841}UI/Fonts/NotoSans/NotoSansCJK-Light-jp.fnt" force=1>text with NotoSans font</font>
```

### Paragraph

A text blob that can be aligned.  
Supported attributes:

* align

Example

```
<p align="left">normal paragraph</p>
<p align="center">centered paragraph!</p>
<p align="right">right paragraph!</p>
```

### Header

Header paragraphs.  
Supported attributes:

* align
* scale

Example

```
<h align="right" scale="4.0">Paragraph with x4 text scale</h>
<h1 align="center"> paragraph with x2 text scale </h1>
<h2 align="left"> paragraph with x1.5 text scale </h2>
```

### Line break

inserts a line break manually.

Example

```
one line <br /> another line
```

### Unicode Character

Inserts a Unicode string defined by UCS codepoints.
Fully supported by SDF fonts, partially supported by bitmap fonts.
If a codepoint is not found in the glyphset of the current rendering font, a space is likely to be inserted instead.

ⓘ

Custom vectorial shapes can be inserted into unused codepoints (see [FontForge](https://fontforge.github.io/en-US/)).

Example

```
<ucs codepoints="\u1f600\u1f601\u1f602" />
```

### Action

Inserts text or icon of key binded to given action.   
Supported attributes:

* name
* preset
* mode - "image", "text" (default: "image")
* device - "keyboard", "gamepad", "joystick", "track\_ir"
* scale - size multiplicator factor (default: 1.0)
* index - if there are more keybinds, shows only specified one (default: 1)
* hideEmpty - if true, nothing is showed when there is no keybind (by default "No keybind" text is showed in this case)

Example

```
<action name="CharacterFire" scale="1.5" />
<action name="CharacterAction" mode="text" />
<action name="CharacterAction" mode="text" device="keyboard" />
```

### Key

Inserts game control key's text or icon.

Example

```
<key name="gamepad0:x" scale="1.5" />
<key name="gamepad0:pad_left" mode="text" />
<key name="gamepad0:pad_left" mode="text" device="gamepad" />
```

## Limitations

### Markup

The markup hierarchy must not have overlapping tags in order for the parser to work properly.
The output will provide a different result than usually expected.

| Incorrect | Correct |
| --- | --- |
| ``` <b>bold <i>"bold italic"</b> italic</i> ``` | ``` <b>bold <i>"bold italic"</i></b> <i>italic</i> ``` |
| **bold *"bold italic"* italic** | **bold *"bold italic"*** *italic* |

### Performance

Fitting the text to its parent rectangle is usually a slow operation; this happens when the options "Exact text" is disabled and "Wrap" is enabled, which will cause the text to be compressed until everything fits in the rectangle.
Avoid manipulating the widget in any manner that may trigger a re-wrap (e.g. changing the widget size on runtime, changing the text, etc...) if you have a really long text or a text with a lot of segments (e.g images or a lot of different styles) the performance penalty will be bigger.
