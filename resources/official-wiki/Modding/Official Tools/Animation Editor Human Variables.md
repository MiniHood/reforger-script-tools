# [Animation Editor: Human Variables](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Human_Variables)

| Variable Name | Type | Min | Max | Usage | Used In |
| --- | --- | --- | --- | --- | --- |
| MovementSpeed | float | 0 | 3 | 0 is not moving, 1 is walking, 2 is running, 3 is sprinting |  |
| MovementSpeedReal | float | 0 | 10 | Animation variable which keeps the current speed of the character in meters per second |  |
| MovementDirection | float | -180 | 180 | Movement direction in degrees, 0 is forward, -90 is left, etc |  |
| HasLocomotionInput | bool |  |  | Has locomotion input for animators to be able to read if there is any input or if the character should stop - used for inertia |  |
| Stance | Int | 0 | 2 | 0 is standing, 1 is crouching, 2 is prone |  |
| Look | bool |  |  | True = looking dir is used, false = looking dir is not used |  |
| LookX | float | -180 | 180 | Used for look at |  |
| LookY | float | -90 | 90 | Used for look at |  |
| WaterLevel | float | 0.0 | 1.0 | This causes the character to act as if he is walking in water (only with rifles so far) |  |
| TriggerPulled | bool |  |  | If true, the finger on the character's weapon hand will be pulled | Weapon |
| WeaponObstruction | bool |  |  | If true, the weapon will be pointing upwards to avoid the obstacle in front of them |  |
| ADSAdjust | float | 0.0 | 1.0 | If 0.0, the character will be almost crouched down and if 1.0, the character will be standing up normally. Used to adjust for obstacles. |  |
| ADS | bool |  |  | This makes the character aim down sights |  |
| AimX | float | -180 | 180 | Used for aiming | Weapon |
| AimY | float | -90 | 90 | Used for aiming | Weapon |
| LeanForward | float | 0.0 | 90 | This leans the character forward while in melee combat |  |
| TurnAmount | float | -2.0 | 2.0 | Use with command **CMD\_Turn**. -2 is turning left 180, -1 is turning left 90, 1 is turning right 90, 2 is turning right 180 |  |
| Lean | float | -1.0 | 1.0 | This leans the character to the left and right |  |
| Low | bool |  |  |  |  |
| ArmIK | int | 0 | 7 | Used by character controller |  |
| AimIKX | float | -180 | 180 | Used by character controller |  |
| DamageDirection | float | -180 | 180 | This is used with **CMD\_Combat\_DamageLight** and **CMD\_Combat\_DamageHeavy** to determine the direction of the hit |  |
| AimIKZ | float | -180 | 180 | Used by character controller |  |
| AimTransX | float | -100 | 100 | Used by character controller |  |
| AimTransY | float | -100 | 100 | Used by character controller |  |
| AimTransZ | float | -100 | 100 | Used by character controller |  |
| VehicleType | int | -1 | 10 | This sets the current vehicle that you're going to enter |  |
| VehicleSteering | float | -1.0 | 1.0 | -1 is steering 100% left, and 1 is turning 100% right | Vehicle |
| VehicleThrottle | float | 0.0 | 1.0 | This controls how much the throttle foot is pushing down | Vehicle |
| VehicleClutch | float | 0.0 | 1.0 | This controls how much the clutch foot is pushing down | Vehicle |
| VehicleBrake | bool |  |  | This makes the character brake, which overrides the VehicleClutch variable | Vehicle |
| VehicleAccelerationFB | float | -1 | 1 | Vehicle acceleration force (front/back) |  |
| VehicleAccelerationLR | float | -1 | 1 | Vehicle acceleration force (left/right) |  |
| Recoil | float | 0 | 1 |  |  |
| LadderMovement | int | -2 | 2 |  |  |
| Look | bool |  |  |  |  |
| Firing | bool |  |  | True when weapon is being fired | Weapon |
| SeatPositionType | int | 0 | 15 | See [Vehicle Actions Commands](/wiki/Arma_Reforger:Animation_Editor:_Vehicle_Action_Commands#SeatPositionType "Arma Reforger:Animation Editor: Vehicle Action Commands") |  |
| DebugNewAimTurns | bool |  |  |  |  |
| AimTurnX | float | -90 | 90 |  |  |
| Horn | bool |  |  | True when vehicle horn is active | Vehicle |
| LeftHandGadget | int | 0 | 9 |  |  |
| SightElevation | float | 0 | 1 | State of sight adjustment (like ironsights) on the weapon. |  |
| Bipod | bool |  |  | True when bipod is deployed on weapon | Weapon |
| LastBullet | bool |  |  | Was the last bullet being fired? Can be used for weapons which holds bolt back after last round | Weapon |
| Empty | bool |  |  | True when weapon is empty | Weapon |
| Cocked | bool |  |  | True when weapon (pistol) is cocked | Weapon |
| Focused | bool |  |  | True when gadget (like compass or binoculars) is focused | Gadget |
| State | int | -1 | 2 | Used for fire mode switch. -1 safe, 0 semi, 1 burst, 2 full auto | Weapon |
| WeaponType | int | 0 | 20 |  |  |
| BandageSelf | bool |  |  | True when character is supposed to bandage himself (instead of i.e. other character) |  |
| FreeLook | bool |  |  | True when freelook is activated |  |
| ObstacleHeight | float | 0 | 210 |  |  |
| ObstructionTEST | float | 0 | 2 |  |  |
| BodyPart | int | 1 | 8 | Controls which body part should be bandaged |  |
| BankAmount | float | -1 | 1 |  |  |
| NewTurns | bool |  |  |  |  |
| IsDriver | bool |  |  | True when character is driver of the vehicle | Vehicle |
| WeaponObstructionFloat | float | 0 | 1 |  |  |
| FloorAngle | float | -90 | 90 |  |  |
| FloorAngleLateral |  |  |  |  |  |
| UGL | bool |  |  |  |  |
| ADSTime | float | 0 | 2 |  |  |
| IdlesOff | bool |  |  | When true, idle animations on given character are disabled |  |
| TargetYOffset | float | -180 | 180 |  |  |
| SpineAccelerationFB | float | -1 | 1 |  |  |
| SpineAccelerationLR | float | -1 | 1 |  | Vehicle |
| CrankX | float | 0 | 1 |  |  |
| CrankY | float | 0 | 1 |  |  |
| WeaponHeight | float | 50 | 120 |  |  |
| StepSize | float | 0 | 1 |  |  |
| WeaponDeployed | bool |  |  | True when weapon is deployed | Weapon |
| IsSimulatedIK | bool |  |  |  |  |
| WeaponInspectionState | int | -2 | 2 | State of weapon inspection | Weapon |
| ItemInspectionState | int | 0 | 1 | State of item inspection | Gadget |
| Vehicle\_Wobble | float | 0 | 1 | Character wobble factor in vehicles. Depends on the roughness of terrain vehicle is ridding on | Vehicle |
| GetInNoBlending | bool |  |  |  | Helicopter |
| CyclicAside | float | -1 | 1 | Rotor mast machinery (or rotor) animations based on cyclic controls | Helicopter |
| Collective | float | -1 | 1 | Helicopter collective source | Helicopter |
| CyclicForward | float | -1 | 1 | Rotor mast machinery (or rotor) animations based on cyclic controls | Helicopter |
| AntiTorque | float | -1 | 1 | Helicopter anti torque variable | Helicopter |
| GearFB | float | 0 | 0.09 | Used in conjunction with **Depth Front/Back** parameter located in **BaseLoadoutClothComponent**. Controls Z forward offset of held weapon |  |
| GearLR | float | 0 | 0.08 | Used in conjunction with **Depth Left/Right** parameter located in **BaseLoadoutClothComponent**. Controls left X offset of elbows when cloth is equipped |  |
| Facial | float | 0 | 1 | Controls unconscious facial animation |  |
| UnconsciousPose | int | 1 | 5 | Controls which unconscious pose will character end up |  |
| VehicleSpeed | float | 0 | 100 | Vehicle speed in m/s | Vehicle |
| ADSOffsetHorizontal | float | -10 | 10 |  |  |
| ADSOffsetVertical | float | -10 | 10 |  |  |
| Stamina | float | 0 | 1 |  |  |
| IsControlledByPlayer | bool |  |  | True if character is controlled by player. Currently it is used to disable idle animations on player controlled characters |  |
