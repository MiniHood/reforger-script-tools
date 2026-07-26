# [Event Handlers](https://community.bistudio.com/wiki/Arma_Reforger:Event_Handlers)

**Contents**

* 1 [Script Invokers](#Script_Invokers)
* 2 [Event Handler Manager Component Events](#Event_Handler_Manager_Component_Events)
* 3 [Entity Event Handlers](#Entity_Event_Handlers)

## Script Invokers

* An Event Handler can be created using a **[ScriptInvoker](enfusion://ScriptEditor/scripts/Game/Helpers/SCR_ScriptInvokerHelper.c)** class.
* Any Script Invoker signature can be created using [typedef](/wiki/Arma_Reforger:Scripting:_Keywords#typedef "Arma Reforger:Scripting: Keywords") - see [ScriptInvoker Usage - Signature Declaration](/wiki/Arma_Reforger:ScriptInvoker_Usage#Signature_Declaration "Arma Reforger:ScriptInvoker Usage").
* Using the [ScriptInvoker](enfusion://ScriptEditor/scripts/GameLib/tools.c;134) class itself is considered bad practice and **must** be avoided.
* As Script Invokers are big objects and can be numerous, it is good practice to **not** instanciate them by default but to rather create them on request, using their getter - saving memory.

ⓘ

See [ScriptInvoker Usage](/wiki/Arma_Reforger:ScriptInvoker_Usage "Arma Reforger:ScriptInvoker Usage") and its [Signature Declaration](/wiki/Arma_Reforger:ScriptInvoker_Usage#Signature_Declaration "Arma Reforger:ScriptInvoker Usage") section for tutorials on how to use Script Invokers.

## Event Handler Manager Component Events

The [EventHandlerManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Events/EventHandlerManagerComponent.c;16) groups the following events under its category:

| Event | Class | Description |
| --- | --- | --- |
| OnADSChanged | [CharacterControllerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/CharacterControllerComponent.c;16) |  |
| OnConsciousnessChanged |  |
| OnInspectionModeChanged | on player character which switches in/out of inspection mode |
| OnMagazineCountChanged | [InventoryStorageManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/InventorySystem/InventoryStorageManagerComponent.c;12) |  |
| OnDestroyed | [DamageManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/DamageManagerComponent.c;12) | when the default hit zone is set to destroyed |
| OnCompartmentEntering | [BaseCompartmentManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Vehicle/BaseCompartmentManagerComponent.c;12) |  |
| OnCompartmentEntered |  |
| OnCompartmentLeft |  |
| OnCompartmentLeaving |  |
| OnLightStateChanged | [BaseLightManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/BaseLightManagerComponent.c;12) |  |
| OnTurretReload | [TurretControllerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/TurretControllerComponent.c;12) | isFinished is true when reload is done and false when it started |
| OnADSChanged | on player character which switches to ADS [TurretControllerComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/TurretControllerComponent.c;12) can also trigger this event |
| OnWeaponChanged | [BaseWeaponManagerComponent](enfusion://ScriptEditor/scripts/Game/generated/Weapon/BaseWeaponManagerComponent.c;16) |  |
| OnMuzzleChanged | [BaseWeaponComponent](enfusion://ScriptEditor/scripts/Game/generated/Weapon/BaseWeaponComponent.c;12) |  |
| OnAmmoCountChanged | [BaseMuzzleComponent](enfusion://ScriptEditor/scripts/Game/generated/Weapon/BaseMuzzleComponent.c;12) | Both weapon and character can raise this event (see BaseWeaponManagerComponent) |
| OnMagazineChanged |  |
| OnProjectileShot |  |
| OnFiremodeChanged |  |
| OnWeaponAttachmentChanged | [WeaponComponent](enfusion://ScriptEditor/scripts/Game/generated/Weapon/WeaponComponent.c;12) | isAttached is true if attachmentEntity was attached to the weapon, and false if it was detached |
| OnZeroingChanged | currently does not carry currentMuzzle |
| OnGrenadeThrown | [WeaponSlotComponent](enfusion://ScriptEditor/scripts/Game/generated/Weapon/WeaponSlotComponent.c;12) |  |
| HHMMSS | [TimeAndWeatherManagerEntity](enfusion://ScriptEditor/scripts/GameCode/World/TimeAndWeatherManagerEntity.c;25) | Triggers based on [TimeAndWeatherManagerEntity](enfusion://ScriptEditor/scripts/GameCode/World/TimeAndWeatherManagerEntity.c;25)'s TIME\_EVENT\_PERIODICITY value:  * 0: never * 1: every hour * 2: every minute * 3: every second |
| OnDayStart |  |
| OnNightStart |  |

## Entity Event Handlers

ⓘ

See also [Entity Lifecycle](/wiki/Arma_Reforger:Entity_Lifecycle "Arma Reforger:Entity Lifecycle").

The following events are present and available in the **[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12)** class - read the code documentation to learn more about them and how to use them.

ⓘ

**E** in e.g **E**OnInit stands for Event (not Entity).

| Event | Description |
| --- | --- |
| EOnInit | Event after the entity is allocated and initialised. |
| EOnVisible | This event triggers when the entity is made visible (versus being invisible). |
| EOnFrame | This event, as its name suggests, triggers on each simulation frame. |
| EOnPostFrame | Triggers after physics update. |
| EOnFixedFrame | ⚠  This event can be called on another thread than the main thread. This means that you must avoid any modifications on other entities during this event! |
| EOnFixedPostFrame |  |
| EOnAnimEvent | Event from the animation system |
| EOnPhysicsActive | Triggers on (de)activation of the RigidBody's physics. |
| EOnPhysicsMove | Triggers when the physics engine moves this entity. |
| EOnSimulate | Happens before physics engine iteration - called from sub-iterations. |
| EOnPostSimulate | Happens after physics engine iteration. Happens once per frame. |
| EOnJointBreak | Triggers when a joint attached to this entity's RigidBody is broken. |
| EOnTouch | Event when touched by another entity. It requires the entity to have TouchComponent. |
| EOnContact | Triggers when contact with another RigidBody has been registered. |
| EOnDiag | Happens every frame after [EOnFrame](#EOnFrame) when "Entity Diag" is enabled in the [Diag Menu](/wiki/Arma_Reforger:Diag_Menu "Arma Reforger:Diag Menu") (e.g [Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools")). |
| EOnUser0 |  |
| EOnUser1 |  |
| EOnUser2 |  |
| EOnUser3 |  |
| EOnUser4 |  |
