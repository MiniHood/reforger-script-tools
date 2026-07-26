# [Vehicle: Wheeled Simulation](https://community.bistudio.com/wiki/Arma_Reforger:Vehicle:_Wheeled_Simulation)

## Input (Controller)

### SCR\_CarControllerComponent

#### General

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Type |  |  | Type of gearbox  * **Automatic** - Automatic shifting and transition to reversing, W/S - throttle/brake or brake/throttle when reversing. * **Manual** - Manual shifting, W - always throttle, S - always brake, Q/E to shift gears |
| Transmission RND | *bool* |  | Transmission have three settings: reverse, neutral and drive |

#### Steering speed coefficients

The following are properties for smoothing the digital or small range/insensitive analog input (gamepad thumbstick). The setup should be quick and responsive enough while allowing the player to keep a smooth ride (e.g. by tapping the keys), without having to constantly counter compensate.

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Steering Forward Speed | *pairs of floats* | `[vehicle speed in km/h, steering speed in °/s]` | Pairs of velocity and steering speed at the given velocity |
| Steering Backward Speed | *pairs of floats* | `[vehicle speed in km/h, steering speed in °/s]` | Pairs of velocity and counter-steering speed (recentering via input) at the given velocity |
| Steering Center Speed | *pairs of floats* | `[vehicle speed in km/h, centering speed in °/s]` | Pairs of velocity and recentering speed (when no steering input is given / caster effect) at the given velocity |

#### Throttle

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Throttle Curve | *pairs of floats* | `[engine rpm, amount of throttle]` | Throttle application with respect to engine's RPM |
| Reverse Curve | *pairs of floats* | `[engine rpm, amount of throttle]` | Throttle application with respect to engine's RPM while in reverse |
| Throttle Reaction Time | *float* | `s` | Time (in seconds) it takes to get wanted value of throttle - e.g. to interpolate from 0.0 to 1.0 throttle input |
| Throttle Turbo Time | *float* | `s` | Time (in seconds) to reach wanted value of throttle in turbo mode |
| Throttle Turbo | *float* |  | Amount of throttle that is reserved for turbo mode. For instance 0.2 means that without turbo, vehicle will be moving with maximum 0.8 throttle |

#### Brake

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Braking Curve | *pairs of floats* | `[time in seconds, amount of braking force]` | Brake application over time |
| Brake Turbo Time | *float* | `s` | Time to reach wanted value of brake in turbo mode |

#### Engine

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Max Startup Time | *float* | `seconds` | Failsafe time for the engine to start (it can also bypass animations if it is shorter) |
| Max Startup Attempts | *float* |  | How many times you can be "stuck" in the startup loop animation |
| Engine Startup Chance | *float* | `%` | Probability that each startup attempt has to turn on the engine (0 - 100) (should be tied to engine below "damaged" threshold) |
| Air Intakes | *float* | `array of PointInfo classes` | Air intake positions in local vehicle space |
| Drowning Time | *float* | `s` | Amount of time needed to completely drown the engine when all air intakes are underwater |
| Shutdown Time | *float* | `s` | Amount of time (some) vehicle systems automatically toggle off after the shutdown |
| Max Lights Time | *float* | `s` | Maximum amount of time the light toggle should take (or if there are no animations) |

#### Clutch

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Clutch Uncouple Time | *float* | `seconds` | Time to disengage clutch before switching gears |
| Clutch Couple Time | *float* | `seconds` | Time to engage clutch after switching gears |
| Clutch Uncouple Rpm | *float* | `RPM` | Engine RPM at which clutch is fully uncoupled while moving off |
| Clutch Couple Rpm | *float* | `RPM` | Engine RPM at which clutch is fully coupled while moving off |
| Clutch Uncouple Factor | *float* |  | Clutch uncouple RPM factor while moving off uphill or downhill |
| Clutch Couple Factor | *float* |  | Clutch couple RPM factor while moving off uphill or downhill |
| Clutch Minimum Position | *float* |  | Minimum clutch position while moving off |
| Clutch Minimum Factor | *float* |  | Minimum clutch position factor while moving off uphill or downhill |

#### Shifting

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Slope Smoothing | *float* |  | Factor of filter that smooths out upshift and downshift RPMs |
| Latency | *float* | `seconds` | Minimum time between gear switches |
| Up Shift Factor | *float* |  | Upshift RPM factor while going uphill or downhill |
| Up Shift Rpm | *float* | `RPM` | Engine RPM required for upshifting |
| Down Shift Factor | *float* |  | Downshift RPM factor while going uphill or downhill |
| Down Shift Rpm | *float* | `RPM` | Engine RPM required for downshifting |
| Turbo Shift Factor | *float* |  | Upshifting and downshifting RPM ratio in Turbo mode |

## Simulation

### VehicleWheeledSimulation

#### General

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Solver Type |  |  | Solver selector - only V1 solver is available right now |
| Solver Update Rate |  | `Hz` | Solver update rate in Hz (number of ticks per second) |

#### Engine

Controls engine power and its other properties. All values must be greater than 0 to be accepted as valid.

The engine is simulated as rotating cylinder around its central axis (simplification of crankshaft).

| Param | Type | Unit | Description | References |
| --- | --- | --- | --- | --- |
| Inertia | *float* | `kg.m2` | Moment of inertia | [armareforger-new-engine-inertia.png](/wiki/File:armareforger-new-engine-inertia.png) https://www.researchgate.net/publication/258176892\_Evaluation\_of\_variable\_mass\_moment\_of\_inertia\_of\_the\_piston-crank\_mechanism\_of\_an\_internal\_combustion\_engine |
| Max Power | *float* | `kW` | Maximum power that the engine can provide | * [armareforger-vehicle-wheeled-engine-graph.png](/wiki/File:armareforger-vehicle-wheeled-engine-graph.png)   You can use [**this calculator**](https://www.desmos.com/calculator/j63rwoyvnh) to visualize RPM curve   * `maxTorqueRPM <= maxPowerRPM < maxRPM` * Use https://www.automobile-catalog.com/, https://www.dieselhub.com/ and similar sources to check for real torque/power curves of the engines |
| Max Torque | *float* | `Nm` | Maximum torque that engine can provide (peak torque) |
| Rpm Max Power | *float* | `RPM` | RPM where engine outputs maximum power |
| Rpm MaxT orque | *float* | `RPM` | RPM where maximum torque is produced |
| Rpm Idle | *float* | `RPM` | RPM when engine is idling, e.g. in neutral |
| Rpm Redline | *float* | `RPM` | Redline RPM **This parameter is currently ignored** |
| Rpm Max | *float* | `RPM` | Maximum RPM |
| Steepness | *float* |  | Controls how fast engine can reach max torque. It can be used to flatten the torque curve before max torque rpm are reached |
| Friction | *float* |  | Engine's braking torque |
| Output |  |  | Powertrain part driven by the engine (clutch) |  |

#### Clutch

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Max Clutch Torque | *float* | `Nm` | Maximum torque that clutch can provide. (1.6\*MaxTorque can be a good starting point) **This parameter is currently ignored** |
| Output |  |  | Powertrain part driven by the clutch (gearbox) |

#### Gearbox

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Reverse | *float* |  | Reverse gear ratio |
| Forward | *array of floats* |  | Array of forward gear ratios, order of the values are mapped to gears respectively |
| Efficiency | *float* |  | Transmission efficiency - scales the engine output passed down |
| Output |  |  | Powertrain part driven by the gearbox (differential) |

#### Differentials

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Type | **Open** |  | Same torque on both outputs, different rotational speeds |
| **LSD** |  | Limited slip differential - limiting rotational difference between outputs. Opens Anti slip and Anti slip torque parameters. |
| Ratio | *float* |  | Differential ratio (sometime "final drive") |
| Strength | *float* |  | Determines the magnitude of the extra force that is applied to the gripping wheel |
| Output0 |  |  | Powertrain parts driven by the differential (other differential or axle differential) |
| Output1 |  |  |

#### Axles

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Torque Share | *float* |  | Defines how much torque is delivered to this axle. Sum of Torque Share for all axles should be equal to 1 |
| Has Handbrake | *bool* |  | Determines whether this axle is used for handbrakes. Handbrake force is same as Brake Torque |

##### (Axle) Differential

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| <same as differentials> | | | |
| Output0 |  |  | Driven wheels |
| Output1 |  |  |

##### Suspension

Accelerating/braking/turning should noticeably shift the weight of the vehicle. Weight shifting affects the grip of the tires - allowing more grip on the side with more weight. Center of mass should be set realistically high and the tendency to roll should be limited by a sway (anti-roll) bar if necesary, not by setting the CoM below the vehicle or just the wheel center.

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Max Steering Angle | *float* | `degrees` | Specifies the maximum steering angle of this axle, if negative value is given, the axis will steer in opposite direction of the steering wheel. |
| Spring Rate | *float* | ``` N/mm ``` | Spring force per mm. |
| Compression Damper | *float* | ``` Ns/m ``` | Compression damper force per 1m/s. |
| Relaxation Damper | *float* | ``` Ns/m ``` | Relaxation damper force per 1m/s. |
| Max Travel Up | *float* | `m` | Maximum distance that the suspension can be compressed from modeled position. Standard cars 0.06 - 0.1 m. Off-road cars >0.1 m. |
| Max Travel Down | *float* | `m` | Maximum distance that the suspension can be expanded from modeled position. Standard cars 0.07 - 0.12 m. Off-road cars >0.1 m. |

##### Wheel

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Radius | *float* | `m` | Radius of the wheel |
| Ratio | *float* |  | Wheel reduction ratio |
| Mass | *float* | `kg` | Mass of the wheel on this axle |
| Brake Torque | *float* | `Nm` | Amount of brake torque applied to each wheel on this axle |

##### Tyre

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Rolling Resistance | *float* |  | **Currently not used in game**  Linearly proportional to speed. Acts against the wheel torque. For limiting acceleration. (in addition to surface property) |
| Rolling Drag | *float* |  | **Currently not used in game**  Proportional to speed squared. For limiting high speeds. (in addition to surface property) |
| Roughness | *float* | `m` | Bumpiness height - how bumpy is the wheel itself (in addition to surface property) |
| Longitudinal Friction | *float* |  | Directly affects tyre grip in longitudinal direction |
| Lateral Friction | *float* |  | Directly affects tyre grip in lateral direction |
| Tread | *float* |  | Ratio of the "Thread" - related to how well wheel performs on specific surface. |

##### Swaybar

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Stiffness | *float* | `N` | Sway bar stiffness ( anti-roll force ) |

##### WheelPositions

#### Inertia

[![armareforger-vehicle-wheeled-inertia.png](/wikidata/images/c/cc/armareforger-vehicle-wheeled-inertia.png)](/wiki/File:armareforger-vehicle-wheeled-inertia.png)

* **InertiaOverrideEnable**
  + Enables manual override of vehicle inertia - the way how to "simulate" mass distribution on the vehicle.
* **InertiaOverride**
  + Inertia values around each axis. Copy initial values from diag or via context menu opened on `VehicleWheeledSimulation` on the Entity instance (you have to *Apply to prefab* later)

##### Aerodynamics

| Param | Type | Unit | Description |
| --- | --- | --- | --- |
| Reference Area | *float* | `m2` | Drag reference area - see following page for more details https://en.wikipedia.org/wiki/Automobile\_drag\_coefficient#Drag\_area |
| Drag coefficient | *float* |  | Drag coefficient - see following page for more details https://en.wikipedia.org/wiki/Automobile\_drag\_coefficient#Typical\_drag\_coefficients |

#### Pacejka

[![armareforger-vehicle-wheeled-pacejka.png](/wikidata/images/5/50/armareforger-vehicle-wheeled-pacejka.png)](/wiki/File:armareforger-vehicle-wheeled-pacejka.png)

* https://www.edy.es/dev/docs/pacejka-94-parameters-explained-a-comprehensive-guide/
* http://www.racer.nl/pacejka/pacplay.htm

Fill in initial values via context menu opened on `VehicleWheeledSimulation` on the Entity instance (you have to *Apply to prefab* later)

##### Longitudinal

###### b0

| Param | Role | Units | Typical range | Sample | Description | Dependency |
| --- | --- | --- | --- | --- | --- | --- |
| Shape factor |  | `1.4 .. 1.8` | `1.5` | General shape of the curve. Defines the amount of falloff after the peak. *The Pacejka model defines b0 = 1.65 for the longitudinal force.* | *Load-independent* |
| b1 | Load influence on longitudinal friction coefficient (\*1000) | `1/kN` | `-80 .. +80` | `0` | Change of the friction coefficient at the peak. Positive = more friction with more load. Negative = less friction with more load. | Load-dependent |
| b2 | Longitudinal friction coefficient (\*1000) |  | `900 .. 1700` | `1100` | Friction coefficient at the peak (vertical coordinate) \*1000. | *Load-independent* |
| b3 | Curvature factor of stiffness/load | `N/%/kN^2` | `-20 .. +20` | `0` | Change of the peak’s horizontal position. Positive = increases ascent rate with load (moves to the left). Negative = decreases ascent rate with load (moves to the right). | Load-dependent |
| b4 | Change of stiffness with slip | `N/%` | `100 .. 500` | `300` | Peak’s horizontal position specified as “ascent rate”. | *Load-independent* |
| b5 | Change of progressivity of stiffness/load | `1/kN` | `-1 .. +1` | `0` | Lineal change of the peak’s horizontal position. Similar to b3 but more lineal and with reverse effect positive-negative. Positive = decreases ascent rate with load. Negative = increases ascent rate with load. | Load-dependent |
| b6 | Curvature change with load^2 |  | `-0.1 .. +0.1` | `0` | Quadratic change of the curvature at the peak. Positive = more flat with load. Negative = sharper with load. | Load-dependent |
| b7 | Curvature change with load |  | `-1 .. +1` | `0` | Change of the curvature at the peak. Same as b6 but more lineal. Positive = more flat with load. Negative = sharper with load. | Load-dependent |
| b8 | Curvature factor |  | `-20 .. +1` | `-2` | Curvature at the peak. The more negative = more “sharp”. Has influence on the falloff afterwards. | *Load-independent* |
| b9 | Load influence on horizontal shift | `%/kN` | `-1 .. +1` | `0` | Change of the horizontal shift. Positive = shifts to the left with more load. Negative = shifts to the right with more load. | Load-dependent |
| b10 | Horizontal shift | `%` | `-5 .. +5` | `0` | Curve’s horizontal shift | *Load-independent* |

##### Lateral

###### a0

| Param | Role | Units | Typical range | Sample | Description | Dependency |
| --- | --- | --- | --- | --- | --- | --- |
| Shape factor |  | `1.2 .. 18` | `1.4` | General shape of the curve. Defines the amount of falloff after the peak. *The Pacejka model defines a0 = 1.3 for the lateral force.* |  |
| a1 | Load influence on lateral friction coefficient (\*1000) | `1/kN` | `-80 .. +80` | `0` | Change of the friction coefficient at the peak. Positive = more friction with more load. Negative = less friction with more load. | Load-dependent |
| a2 | Lateral friction coefficient (\*1000) |  | `900 .. 1700` | `1100` | Friction coefficient at the peak (vertical coordinate) \*1000. |  |
| a3\* | Change of stiffness with slip | `N/deg` | `500 .. 2000` | `1100` | Peak’s horizontal position at the reference load, specified as “ascent rate”. |  |
| a4\* | Change of progressivity of stiffness / load | `1/kN` | `0 .. 50` | `10` | Change of the peak’s horizontal position with load. Smaller value = bigger change with load. |  |
| a5 | Camber influence on stiffness | `%/deg/100` | `-0.1 .. +0.1` | `0` | Change of the peak’s horizontal position. Positive = decreases ascent rate with camber (moves to the right).  Negative = increases ascent rate with load (moves to the left). | Camber-dependent |
| a6 | Curvature change with load |  | `-2 .. +2` | `0` | Change of the curvature at the peak. Positive = more flat with load. Negative = sharper with load. | Load-dependent |
| a7 | Curvature factor |  | `-20 .. +1` | `-2` | Curvature at the peak. The more negative = more “sharp”. Has influence on the falloff afterwards |  |
| a8 | Load influence on horizontal shift | `deg/kN` | `-1 .. +1` | `0` | Change of the horizontal shift. Positive = shifts to the left with more load. Negative = shifts to the right with more load. | Load-dependent |
| a9 | Horizontal shift at load = 0 and camber = 0 | `deg` | `-1 .. +1` | `0` | Curve’s horizontal shift |  |
| a10 | Camber influence on horizontal shift | `deg/deg` | -0.1 .. +0.1 | 0 | Change of the horizontal shift. Same sign as camber = shifts to the left. Opposite sign as camber = shifts to the right. | Camber-dependent |
| a11 | Vertical shift | `N` | -200 .. +200 | 0 | Curve’s vertical shift |  |
| a12 | Vertical shift at load = 0 | `N` | -10 .. +10 | 0 | Vertical shift when approaching zero load. Must be verified for coherency at the configured minimum load. | Load-dependent |
| a13 | Camber influence on vertical shift, load dependent | `N/deg/kN` | -10 .. +10 | 0 | Change of the vertical shift according to camber and load. Same sign as camber = shifts upwards. Opposite sign as camber = shifts downwards.  The more load the more camber effect. | Camber-dependent |
| a14 | Camber influence on vertical shift | `N/deg` | -15 .. +15 | 0 | Change of the vertical shift. Same sign as camber = shifts upwards. Opposite sign as camber = shifts downwards. | Camber-dependent |

\* Configure the horizontal behavior with load

##### Aligning

| Param | Role | Units | Typical range | Sample | Description | Dependency |
| --- | --- | --- | --- | --- | --- | --- |
| c0 |  |  |  |  |  |  |
| c1 |  |  |  |  |  |  |
| c2 |  |  |  |  |  |  |
| c3 |  |  |  |  |  |  |
| c4 |  |  |  |  |  |  |
| c5 |  |  |  |  |  |  |
| c6 |  |  |  |  |  |  |
| c7 |  |  |  |  |  |  |
| c8 |  |  |  |  |  |  |
| c9 |  |  |  |  |  |  |
| c10 |  |  |  |  |  |  |
| c11 |  |  |  |  |  |  |
| c12 |  |  |  |  |  |  |
| c13 |  |  |  |  |  |  |
| c14 |  |  |  |  |  |  |
| c15 |  |  |  |  |  |  |
| c16 |  |  |  |  |  |  |
| c17 |  |  |  |  |  |  |
| c18 |  |  |  |  |  |  |

#### RaycastLayer

* LayerPreset in which the wheel raycast is performed (should be "Vehicle")

## RigidBody and Center of Mass

* All vehicles are set to curb weight (assuming dynamic weight could happen at some point in the future)
* Center of Mass plays a crucial role in vehicle handling - it should be high enough to allow for weight shifting and changes in the wheel grip due to the changing pressure.
