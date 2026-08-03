# [Animation Editor: Vehicle Action Commands](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Vehicle_Action_Commands)

## Commands

### CMD\_Vehicle\_GetIn

*integers*

| A | A | B | B | C | C |
| --- | --- | --- | --- | --- | --- |
| DoorFL | 0 | TeleportIn | 0 |  |  |
| DoorFR | 1 | GetIn | 1 |  |  |
| DoorBL | 2 | ... | 2 |  |  |
| DoorBR | 3 |  |  |  |  |

### CMD\_Vehicle\_GetOut

*integers*

| A | A | B | B | C | C |
| --- | --- | --- | --- | --- | --- |
| DoorFL | 0 | TeleportOut | 0 |  |  |
| DoorFR | 1 | GetOut | 1 |  |  |
| DoorBL | 2 | CrawlOut | 2 |  |  |
| DoorBR | 3 | JumpOut | 3 |  |  |
|  |  | ClimbOut | 4 |  |  |

#### CMD\_Vehicle\_GearSwitch

to trigger gear switch, no particular integer

#### CMD\_Vehicle\_Engine\_StartStop

| Integer | Action |
| --- | --- |
| 0 | Interrupt animation |
| 1 | Initial Start animation |
| 2 | Final Start animation (based on Vehicle\_StartSop event) |
| 3 | Engine Stop animation |

#### CMD\_Lights

| Integer | Action |
| --- | --- |
| 0 | Interrupt |
| 1 | Lights On |
| 2 | Lights Off |

## Variables

#### SeatPositionType

Variable (int)

| Integer | UAZ | BTR | Ural | Jeep | HMMWV | M923 |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | Driver | Driver | Driver | Driver | Driver | Driver |
| 1 | Codriver | Commander | Codriver | Codriver | Codriver | Codriver |
| 2 | Passenger Left | Generic | Cargo Left Bench | Passenger Left | Passenger Left | Generic |
| 3 | Passenger Right | Generic | Cargo Right Bench | Passenger Right | Passenger Right | Generic |
| 4 | Passenger Middle | Generic | CodriverC | CodriverC | Generic | CodriverC |
| 5 | / | Gunner | / | Gunner | Gunner | / |
| 6 |  |  |  |  |  |  |

#### Pedals

| Name | Type | Usage |
| --- | --- | --- |
| VehicleThrottle | float (0.0 - 1.0) | Amount that throttle pedal is pushed back (0.0 (not pushed) - 1.0 (pushed back completely)) |
| VehicleClutch | float (0.0 - 1.0) | Amount that clutch pedal is pushed back (0.0 (not pushed) - 1.0 (pushed back completely)) |
| VehicleBrake | bool | Toggle braking |
