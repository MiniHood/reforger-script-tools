# [Editor Entity Naming Conventions](https://community.bistudio.com/wiki/Arma_Reforger:Editor_Entity_Naming_Conventions)

## Localisation String Naming

The string naming conventions in Arma Reforger regarding Editor Entities goes as follows:

```
AR-EditableEntity_Beehive_01_Small_Blue_Name
```

| Part | Followed by | Mandatory | Description |
| --- | --- | --- | --- |
| AR- | dash - | Checked | Code for the name of the project (AR = Arma Reforger, A4 = Arma 4, etc), separated from the next variable by a hyphen -. |
| EditableEntity\_ | underscore \_ | Unchecked | Always use EditableEntity for an in-game asset that is not a character, vehicle, weapon, or item of equipment. This variable should be followed by an underscore. |
| Beehive\_ | underscore \_ | Checked | Noun, pertaining to the entity. If the noun is two words, such as "Beer Bottle", it must be arranged in PascalCase (e.g "BeerBottle") and *not* be connected with underscores (e.g "Beer\_Bottle"). |
| 01\_ | underscore \_ | Checked | Number with a leading zero, allowing for the possibility of future variants. |
| Small\_ | underscore \_ | Unchecked | Defines the asset's size. |
| Blue\_ | underscore \_ | Unchecked | Defines the colour. |
| Name | N/A | Checked | Finally, all names for Editor Entities end with the label 'Name', indicating their function in-game. |

## Localisation Content Naming

The string naming conventions in Arma Reforger regarding Editor Entities goes as follows:

```
Small Cyan Table - Broken
```

All names should be in [Title case](https://en.wikipedia.org/wiki/Title_case).
Also, ensure links to images are included in the String Editor so nomenclature can be checked by a native speaker.
This is essential, as many assets are misnamed on our initial pass.

| Part | Mandatory | Description |
| --- | --- | --- |
| Small | Unchecked | Size. The first component in the name relates to the asset's size, which may be either:  * Miniature * Small * Medium * Large * Massive   ⓘ  "Miniature" should only be used when the asset is far smaller than it should be; as in a replica or model (e.g "Miniature Belfry"). Similarly, "Massive" should only be used for assets that are extraordinarily larger than normal. |
| Cyan | Unchecked | Colour This component relates to the asset's colour. It can be one of:  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | | Colour | Red | Orange | Yellow | Green | Cyan | Blue | Magenta | Purple | White | Black | Gray | Silver | Pink | Maroon | Brown | Beige | Tan | Peach | Lime | Olive | Turquoise | Teal | Navy blue | Indigo | Violet | | Sample |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |   If the asset contains a myriad of colours, the prefix 'multi-colored' can be used. Try to avoid colours that are also materials, as this can be confusing (e.g., 'Ivory Crate' or 'Gold Container').  When a colour combination is involved, it is acceptable to use the conjunction 'and' (e.g., Blue and Red). Precedence in such pairings should be alphabetical (e.g., Beige and Black, Tan and Teal, etc.). |
| Table | Checked | Base Descriptor. This part is the most essential and complex. It must consist of at least a noun, but will often be preceded by an adjective. It may also be split by a preposition or conjunction, although prepositions should be avoided if grammatically possible. Note; any adjective relating to the state of the asset (e.g., 'Old', 'New', 'Rusted', 'Broken', 'Open', 'Closed') should be separated as a hyphenated adjective (see below).  | Correct | Incorrect | | --- | --- | | * Standing Lamp * Brick Stack * Mop and Bucket * Flowers in Pot * Wicker Basket with Firewood | * Stack of Bricks (> Brick Stack) * Pile of Scrap (> Scrap Pile) * Mop in Bucket (> Mop and Bucket) * Old Water Cistern (> Water Cistern - Old) * Old Opened Barrel (> Barrel - Old, Open) | |
| Broken | Unchecked | Adjectives. Finally, certain adjectives may be tacked to the end of the name *via* a hyphen. This can be particularly useful for specification, where the noun of the base descriptor is not sufficient (e.g., 'Statue' > 'Statue - Soldier', 'Sign' > 'Sign - Slow Speed'). Common hyphenated adjectives can be taken from the list below. Assets where a pair of adjectives are needed should be divided by a comma and a space (e.g., - Old, Open), and should be ordered alphabetically *except* with non-permanent states, which should always be last (e.g., 'Open', 'Closed', 'Locked', 'Unlocked', 'On', 'Off', etc.).   | Correct (permanent states) | Incorrect (temporary states) | | --- | --- | | * Old * New * Rusted * Broken * Dismantled * Camouflaged | * On * Off * Empty * Full * Open * Closed | |

## See Also

* Localisation plugin, accessible in [Resource Manager](/wiki/Arma_Reforger:Resource_Manager "Arma Reforger:Resource Manager") with `Ctrl` + `Alt` + `L`
