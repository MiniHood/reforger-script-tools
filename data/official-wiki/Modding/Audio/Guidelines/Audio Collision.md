# [Audio: Collision](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Collision)

## Vegetation

Signals:

* BushContact: connected to SoundType parameter on Tree entity
* BushHeight: height of the Bush object. Relies on boundingbox and can deliver unexpected results.

| SoundTypes | Signal Value |
| --- | --- |
| None | 0 |
| Bush | 1 |
| Leafy | 2 |
| Conifer | 3 |
| Stump | 4 |
| Withered | 5 |
| LeafyDomestic | 6 |
| Bush\_Leafy | 7 |
| Bush\_Reed | 8 |
| Bush\_Small | 9 |

## Special

Signals:

* SpecialContact: type of entity the character collides with
* SpecialContactEntityHeight: height of the colliding object; relies on boundingbox and can deliver unexpected results, but can be overridden on the Prefab.

| SoundTypes | Signal Value |
| --- | --- |
| None | 0 |
| Barbed Wire | 1 |
