# [Audio: Vehicle Damage](https://community.bistudio.com/wiki/Arma_Reforger:Audio:_Vehicle_Damage)

* Vehicle damage is connected to the damage state signals that are updated from the cOnStateChanged() event on [SCR\_HitZoneStateSignal](enfusion://ScriptEditor/scripts/Game/HitZone/SCR_HitZoneStateSignal.c;1) (checking [SCR\_HitZone](enfusion://ScriptEditor/scripts/Game/HitZone/SCR_HitZone.c;1)).
* The signal setup is on [SCR\_VehicleSoundComponent](enfusion://ScriptEditor/scripts/Game/Components/VehicleSoundComponent/SCR_VehicleSoundComponent.c;8)'s cGetHitZoneStateSignalData().
* Multiple scripted hit zones can update the same signal. In that case, the signal has the value of the most damaged scripted hit zone.
* TireDamage signals are updated directly from [SCR\_WheelHitZone](enfusion://ScriptEditor/scripts/Game/HitZone/SCR_WheelHitZone.c;9).

## Wheeled

### Engine

* When in idle, engine RPM is modulated EngineDamageRPMRevs signal, creating a sense of unsteady and unreliable engine RPM
* SOUND\_TIMING\_BELT - One-shot sound, periodically triggered. Triggering frequency depends on engine RPM
* SOUND\_ENGINE\_RATTLE\_LP - Looped sound modulated by engine RPM and thrust
* SOUND\_BACK\_FIRE - One-shot sound occasionally triggered

### Gearbox

* SOUND\_VEHICLE\_GEARSHIFT - grinding sound layer is mixed in when the gearbox is damaged

### Wheel Damage

* Driving on a flat tire or rim is mixed into wheel rolling sounds if the wheel is damaged or destroyed

## Helicopters

### Engine

* SOUND\_TURBINE\_LP - the damaged turbine layer is played when the engine is damaged
* SOUND\_ENGINE\_RATTLE - one-shot sound is occasionally triggered.

### Main Rotor

* SOUND\_MAINROTOR\_LP - a loud whooshy sounding layer is mixed in, if the main rotor is damaged

### Tail rotor

* SOUND\_TAILROTOR\_DAMAGED\_LP - metal grinding/scraping sound is triggered, if the tail rotor is damaged.

### Gearbox

* Metal grinding/rattling layer is mixed into SOUND\_MAINROTOR\_LP if the gearbox is damaged.

## Smoke and Fire

Three sound sources are accompanying smoke and fire particles on vehicles.

* **Engine smoke/fire:**
  + On engine position
  + Uses the [EngineFireState](#EngineFireState) signal
  + Representing engine smoking or vehicle fire
  + Defined on [SCR\_FlammableHitZone](enfusion://ScriptEditor/scripts/Game/HitZone/SCR_FlammableHitZone.c;15)
  + Particles size is static, defined on [SCR\_FlammableHitZone](enfusion://ScriptEditor/scripts/Game/HitZone/SCR_FlammableHitZone.c;15)
* **Supplies fire:**
  + On supplies position
  + Size/type of sound is driven by the [SuppliesFireState](#SuppliesFireState) signal
  + Will include ammo cook-off layer
* **Fuel tank fire**
  + On fuel tank position
  + Size/type of sound is driven by the [FuelTankFireState](#FuelTankFireState) signal

## Signals

### EngineFireState

* 0 - no particles
* 10 - 30 - smoke
* 40 - fire

### SuppliesFireState

* 0 - not burning
* 1 - 4 will return based on what particle effect is used. If small, medium, large or massive (Uses [SCR\_ESecondaryExplosionScale](enfusion://ScriptEditor/scripts/GameCode/Components/SCR_SecondaryExplosions.c;15) enum).

### FuelTankFireState

* 0 - not burning
* 1 - 4 will return based on what particle effect is used. If small, medium, large or massive (Uses [SCR\_ESecondaryExplosionScale](enfusion://ScriptEditor/scripts/GameCode/Components/SCR_SecondaryExplosions.c;15) enum).
