# [Entity Activeness](https://community.bistudio.com/wiki/Arma_Reforger:Entity_Activeness)

## Active Flag and Frame Events

Setting a FRAME event on a component or an entity makes said entity (or component's parent entity) simulated.
The ACTIVE flag should be set on entities that are moved every frame but do not have any components using the FRAME event.

Doing so automatically calls an update on the entity along other engine operations such as updating the bounding box, etc.

When activating/deactivating a component, EOnActivate/EOnDeactivate component events are called, meaning that every component needs to take care of its own activation and deactivation by setting or clearing its FRAME flag.

⚠

Since **0.9.8** EOnActivate is **not** called on components when an entity is spawned anymore, as components are now considered active by default.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_0.9.5 "Category:Arma Reforger/Version 0.9.5") [0.9.5](/wiki?title=Category:Arma_Reforger/Version_0.9.5&action=edit&redlink=1 "Category:Arma Reforger/Version 0.9.5 (page does not exist)")

### Pre-0.9.8 Behaviour

**Before 0.9.8**, an ACTIVE flag on the entity would be necessary for any FRAME events. This means that any component which needed the FRAME events would need to set ACTIVE on its owner.

This led to the issue where multiple components could clear the ACTIVE flag event of the entity where another component would still require the FRAME event, bringing to this change.

## Good Practices

The first step is to not set the ACTIVE flag on the entity by the component in order to have FRAME events (pre-0.9.8 way).

And then, as said above, when a component is disabled, it is its job to set/clear its event mask itself when it gets activated/deactivated.

### Examples

| Bad Example | Good Example |
| --- | --- |
| ENFORCECODEMARKER   ``` class MyComponent : ScriptComponent { 	override void OnPostInit(IEntity owner) 	{ 		SetEventMask(owner, EntityEvent.FRAME); 		owner.SetFlags(EntityFlags.ACTIVE); 	} } ``` | ENFORCECODEMARKER   ``` class MyComponent : ScriptComponent { 	override void OnPostInit(IEntity owner) 	{ 		SetEventMask(owner, EntityEvent.FRAME); 	} 	override void EOnActivate(IEntity owner) 	{ 		super.EOnActivate(owner); 		SetEventMask(owner, EntityEvent.FRAME); 	} 	override void EOnDeactivate(IEntity owner) 	{ 		super.EOnDeactivate(owner); 		ClearEventMask(owner, EntityEvent.FRAME); 	} } ```   Advanced example:  ENFORCECODEMARKER   ``` class MyComponent : ScriptComponent { 	protected bool m_bCustomCondition = false; 	void SetCustomCondition() 	{ 		m_bCustomCondition = true; 		SetEventMask(owner, EntityEvent.FRAME); 	} 	override void EOnActivate(IEntity owner) 	{ 		super.EOnActivate(owner); 		if (!m_bCustomCondition) 		return; 		SetEventMask(owner, EntityEvent.FRAME); 	} 	override void EOnDeactivate(IEntity owner) 	{ 		super.EOnDeactivate(owner); 		if (!m_bCustomCondition) 		return; 		ClearEventMask(owner, EntityEvent.FRAME); 	} } ``` |
