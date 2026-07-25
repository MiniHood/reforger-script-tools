# [Environment Metrics](https://community.bistudio.com/wiki/Arma_Reforger:Environment_Metrics)

This document provides metrics used to design vanilla gameplay and assets.

⚠

Values in this document are provided **in centimeters** and degrees.

## Character

The Vault/Climb distinction is made at **145 cm**. Anything below will make the character vault, anything over will make the character climb.

| Measurement | Prone | Crouch | Stand |
| --- | --- | --- | --- |
| Pass under | 70 cm | 150 cm | 195 cm |
| Cover | 55 cm | 115 cm | 185 cm |
| Shoot Over | 5 cm | 65 cm | 125 cm |
| Move Over | 20 cm | | 40 cm |
| Ignore | 5 cm | | |

## Vehicle

* **Small Vehicles** represent car-like and civilian-sized vehicles
* **Large Vehicles** represent armored-like and military-sized vehicles

### Underpasses

| Measurement | Small Vehicles | Large Vehicles |
| --- | --- | --- |
| Width | ≥ 340 cm | ≥ 440 cm |
| Height | 270-300 cm | ≥ 400 cm |

## Building

* **Common metrics** represent "normal" buildings (civilian houses, offices, etc)
* **Uncommon metrics** represent special buildings (bunkers, sheds, etc)

| Measurement | Value |
| --- | --- |
| Maximum intended terrain slope | 20° |
| Maximum foundation height | 600 cm |

### Foundation Cheat Sheet

|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Width (cm) | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 | 1000 | 1100 | 1200 | 1300 | 1400 | 1500 | 1600 | 1700 | 1800 | 1900 | 2000 |
| Foundation height (cm) | 37 | 73 | 110 | 146 | 182 | 219 | 255 | 292 | 328 | 364 | 401 | 437 | 474 | 510 | 546 | 583 | 600 | | | |

### Floor and Walls

| Measurement | Common | Uncommon |
| --- | --- | --- |
| Ceiling/floor thickness | 30-50 cm | ≥ 15 cm |
| Ceiling height | 270-330 cm | ≥ 220 cm |
| Load-bearing wall thickness | 40 cm | - |
| Partition wall thickness | 20 cm | ≥ 10 cm |

### Door

| Measurement | Common | Uncommon |
| --- | --- | --- |
| Width | 125 cm | 90 cm |
| Height | 237 cm | 200 cm |
| Handle height | 115 cm | |
| Frameless width | 141 | 106 |
| Frameless height | 245 | 208 |
| Garage width | 400 |
| Garage height | 350 |

### Window

| Measurement | Small Window | Large Window |
| --- | --- | --- |
| Min distance from the floor | 169 cm | 99 cm |

Common dimensions

|  | | Height | | | | |
| --- | --- | --- | --- | --- | --- | --- |
| 47 | 72 | 118 | 142 | 182\* |
| Width | 70 | 70 × 47 | 70 × 72 | - | 70 × 142 | 70 × 182 |
| 90 | - | 90 × 72 | - | 90 × 142 | 90 × 182 |
| 110 | - | - | 110 × 118 | 110 × 142 | - |
| 130 | - | 130 × 72 | - | 130 × 142 | 130 × 182 |
| 170 | - | 170 × 72 | - | 170 × 142 | 170 × 182 |
| 195 | - | 195 × 72 | 195 × 142 | 195 × 182 | - |

*\* recommended ceiling height: 330 cm*

ⓘ

Some other uncommon formats exist, such as e.g 260×230 for the air control tower.

### Stairs

* use only simple shapes (straight, U-shaped, L-shaped)
* narrow stairs should only be straight

| Measurement | Common | Uncommon |
| --- | --- | --- |
| Staircase width | 185 cm | 125 cm |
| Recommended step height | 19 cm | |
| Recommended step depth | 30 cm | |
| Ceiling height | ≥ 270 cm | |
| Maximum slope angle | 45° | |

## Furniture

| Measurement | Dimensions |
| --- | --- |
| Seating (Chair, Sofa, Bed) | 50 cm |
| Bed length | 230 cm |
| Regular table height | 90 cm |
| Work place height | 100 cm |

## Ladder

| Measurement | Value |
| --- | --- |
| First step | 34 cm |
| Step height | 32 cm |
| Width | 44 cm |
| Bar diameter | 4.2 cm |
| Side bars's height above the last step | 70 cm |
| Automatic animation adjustment range | 90-75° (forward slope) |
| Minimum distance from nearby colliders | 80 cm total width, 100 cm behind |
