# [Entity Lifecycle](https://community.bistudio.com/wiki/Arma_Reforger:Entity_Lifecycle)

## Definitions

* Frame: every drawing frame (frame as in FPS)
* Fixed Frame: a fixed event happening 30 times per second
* Physics Simulation Frame: a fixed event at 60 simulations per second (frequency defined in Workbench → Options → Game Project → Modules → Physics Settings → Ticks)

## Lifecycle

⚠

Note that some event methods are called only when:

* such events were enabled through c[IEntity](enfusion://ScriptEditor/scripts/Core/generated/Entities/IEntity.c;12).SetEventMask()
* some conditions are met (EOnPhysicsActive requires the entity to have valid and activated physics, etc)

| Section | Mode | | Method | | Description |
| --- | --- | --- | --- | --- | --- |
| WB Play  Game | WB Edit | Entity | Component |
| Instantiation | Checked | Checked | Constructor |  |  |
| Checked | Checked |  | Constructor |  |
| Checked | Checked |  | OnPostInit |  |
| Checked | Checked |  | EOnInit |  |
| Checked | Checked | EOnInit |  |  |
|  | Checked | \_WB\_SetTransform |  |  |
|  | Checked |  | \_WB\_SetTransform |  |
|  | Checked | \_WB\_OnInit |  |  |
|  | Checked |  | \_WB\_OnInit |  |
|  | Checked | \_WB\_MakeVisible |  |  |
| Workbench Execution loop  (World Editor) |  | Checked | \_WB\_AfterWorldUpdate |  |  |
|  | Checked |  | \_WB\_AfterWorldUpdate |  |
| Simulation init | Checked |  | EOnActivate |  |  |
| Simulation loop | Checked |  |  | EOnFrame |  |
| Checked |  | EOnFrame |  |  |
| Checked |  |  | EOnDiag |  |
| Checked |  | EOnDiag |  |  |
| Checked |  |  | EOnFixedFrame | if 1/30s passed since the last execution |
| Checked |  | EOnFixedFrame |  | if 1/30s passed since the last execution |
| Checked |  |  | EOnSimulate | if 1/60s passed since the last execution - can be executed multiple times per frame |
| Checked |  | EOnSimulate |  | if 1/60s passed since the last execution - can be executed multiple times per frame |
| Checked |  |  | EOnPostSimulate |  |
| Checked |  | EOnPostSimulate |  |  |
| Checked |  |  | EOnPhysicsMove | if 1/60s passed since the last execution - can be executed multiple times per frame |
| Checked |  | EOnPhysicsMove |  | if 1/60s passed since the last execution - can be executed multiple times per frame |
| Checked |  |  | EOnPostFrame |  |
| Checked |  | EOnPostFrame |  |  |
| Checked |  |  | EOn**PostFixed**Frame | If EOnFixedFrame executed this frame |
| Checked |  | EOn**FixedPost**Frame |  | If EOnFixedFrame executed this frame |
| Destruction | Checked | Checked |  | OnDelete |  |
| Checked | Checked | Destructor |  | Do NOT delete components in an entity's destructor - this can lead to nullpointer exceptions |
| Checked | Checked |  | Destructor | Do NOT reference the parent entity in a component's destructor - the entity is already deleted by then |

## Example Code

The following code is to be placed in a .c file in the scripts/Game module.
Show test Entity and Component

```enforce
#define _PRINT_EVENTS			// this flag enables/disables all event prints
// #define _WB_WORLD_UPDATES	// this flag enables/disables Workbench's world updates-related event prints
[EntityEditorProps(category: "Test", description: "Test entity")]
class SCR_MyEntityClass : GenericEntityClass
{
}
class SCR_MyEntity : GenericEntity
{
	#ifdef _PRINT_EVENTS
	protected static const int DELETE_AFTER = 3000; //!< duration in ms after which the entity deletes itself - for log's sake
	protected void DeleteThis()
	{
		Print("DELETE THIS");
		delete this; // delete this
	}
	protected void _Print(string input)
	{
		Print("ENT : " + input, LogLevel.NORMAL);
	}
	// IEntity events
	override void EOnTouch(IEntity owner, IEntity other, int touchTypesMask) { _Print("EOnTouch"); }
	override void EOnInit(IEntity owner) { _Print("EOnInit"); }
	override void EOnVisible(IEntity owner, int frameNumber) { _Print("EOnVisible"); }
	override void EOnFrame(IEntity owner, float timeSlice) { _Print("EOnFrame"); }
	override void EOnPostFrame(IEntity owner, float timeSlice) { _Print("EOnPostFrame"); }
	override void EOnAnimEvent(IEntity owner, int type, int slot) { _Print("EOnAnimEvent"); }
	override void EOnSimulate(IEntity owner, float timeSlice) { _Print("EOnSimulate"); }
	override void EOnPostSimulate(IEntity owner, float timeSlice) { _Print("EOnPostSimulate"); }
	override void EOnJointBreak(IEntity owner, IEntity other) { _Print("EOnJointBreak"); }
	override void EOnPhysicsMove(IEntity owner) { _Print("EOnPhysicsMove"); }
	override void EOnContact(IEntity owner, IEntity other, Contact contact) { _Print("EOnContact"); }
	override void EOnPhysicsActive(IEntity owner, bool activeState) { _Print("EOnPhysicsActive"); }
	override void EOnDiag(IEntity owner, float timeSlice) { _Print("EOnDiag"); }
	override void EOnFixedFrame(IEntity owner, float timeSlice) { _Print("EOnFixedFrame"); }
	override void EOnFixedPostFrame(IEntity owner, float timeSlice) { _Print("EOnFixedPostFrame"); }
	// GenericEntity events
	override void EOnActivate(IEntity owner)
	{
		Physics physics = GetPhysics();
		if (physics)
		physics.SetActive(ActiveState.ALWAYS_ACTIVE);
		_Print("EOnActivate");
		// delete after some time
		GetGame().GetCallqueue().CallLater(DeleteThis, DELETE_AFTER);
	}
	override void EOnDeactivate(IEntity owner) { _Print("EOnDeactivate"); }
	override IEntity _WB_GetEditableOwner() { _Print("_WB_GetEditableOwner"); return super._WB_GetEditableOwner(); }
	override void _WB_MakeVisible(IEntitySource src, bool visible) { _Print("_WB_MakeVisible"); }
	override void _WB_SetTransform(inout vector mat[4], IEntitySource src) { _Print("_WB_SetTransform"); }
	override void _WB_OnInit(inout vector mat[4], IEntitySource src) { _Print("_WB_OnInit"); }
	override bool _WB_CanDelete(IEntitySource src) { _Print("_WB_CanDelete"); return super._WB_CanDelete(src); }
	override bool _WB_CanRename(IEntitySource src) { _Print("_WB_CanRename"); return super._WB_CanRename(src); }
	override bool _WB_CanCopy(IEntitySource src) { _Print("_WB_CanCopy"); return super._WB_CanCopy(src); }
	override bool _WB_CanSelect(IEntitySource src) { _Print("_WB_CanSelect"); return super._WB_CanSelect(src); }
	override int _WB_GetAnchorCount(IEntitySource src) { _Print("_WB_GetAnchorCount"); return super._WB_GetAnchorCount(src); }
	override void _WB_GetAnchor(inout vector position, IEntitySource src, int index) { _Print("_WB_GetAnchor"); }
	override void _WB_OnAnchorSnapped(IEntitySource thisSrc, int thisAnchor, IEntitySource otherSrc, int otherAnchor, bool isReceiver) { _Print("_WB_OnAnchorSnapped"); }
	override bool _WB_CanAnchorSnap(IEntitySource thisSrc, int thisAnchor, IEntitySource otherSrc, int otherAnchor, bool isReceiver) { _Print("_WB_CanAnchorSnap"); return super._WB_CanAnchorSnap(thisSrc, thisAnchor, otherSrc, otherAnchor, isReceiver); }
	override void _WB_GetBoundBox(inout vector min, inout vector max, IEntitySource src) { _Print("_WB_GetBoundBox"); }
	override bool _WB_ShouldShowBoundBox(IEntitySource src) { _Print("_WB_ShouldShowBoundBox"); return super._WB_ShouldShowBoundBox(src); }
	override void _WB_SetExtraVisualiser(EntityVisualizerType type, IEntitySource src) { _Print("_WB_SetExtraVisualiser"); }
	override array<ref WB_UIMenuItem> _WB_GetContextMenuItems() { _Print("_WB_GetContextMenuItems"); return super._WB_GetContextMenuItems(); }
	override bool _WB_OnPhysSimulPlacementBegin(IEntitySource src) { _Print("_WB_OnPhysSimulPlacementBegin"); return super._WB_OnPhysSimulPlacementBegin(src); }
	override bool _WB_EnablePhysics(IEntitySource src, bool physics) { _Print("_WB_EnablePhysics"); return super._WB_EnablePhysics(src, physics); }
	override array<ref ParamEnum> _WB_GetUserEnums(string varName, IEntitySource src) { _Print("_WB_GetUserEnums"); return super._WB_GetUserEnums(varName, src); }
	override bool _WB_OnKeyChanged(BaseContainer src, string key, BaseContainerList ownerContainers, IEntity parent) { _Print("_WB_OnKeyChanged"); return super._WB_OnKeyChanged(src, key, ownerContainers, parent); }
	#ifdef _WB_WORLD_UPDATES
	override void _WB_AfterWorldUpdate(float timeSlice) { _Print("_WB_AfterWorldUpdate"); }
	#endif
	override void _WB_OnContextMenu(int id) { _Print("_WB_OnContextMenu"); }
	override void _WB_OnKeyDown(int keyCode) { _Print("_WB_OnKeyDown"); }
	override void _WB_OnCreate(IEntitySource src) { _Print("_WB_OnCreate"); }
	override void _WB_OnDelete(IEntitySource src) { _Print("_WB_OnDelete"); }
	override void _WB_OnRename(IEntitySource src, string oldName) { _Print("_WB_OnRename"); }
	void SCR_MyEntity(IEntitySource src, IEntity parent)
	{
		SetEventMask(EntityEvent.ALL);
		_Print("Constructor");
	}
	void ~SCR_MyEntity() { _Print("Destructor"); }
	#endif
}
//
// Component below
//
[ComponentEditorProps(category: "Test", description: "Test component")]
class SCR_MyComponentClass : ScriptComponentClass
{
}
class SCR_MyComponent : ScriptComponent // which itself inherits from GenericComponent
{
	#ifdef _PRINT_EVENTS
	protected void _Print(string input)
	{
		Print("COMP: " + input, LogLevel.NORMAL);
	}
	// GenericComponent events
	override void _WB_SetTransform(IEntity owner, inout vector mat[4], IEntitySource src) { _Print("_WB_SetTransform"); }
	override void _WB_OnInit(IEntity owner, inout vector mat[4], IEntitySource src) { _Print("_WB_OnInit"); }
	override bool _WB_CanDelete(IEntity owner, IEntitySource src) { _Print("_WB_CanDelete"); return super._WB_CanDelete(owner, src); };
	override bool _WB_CanRename(IEntity owner, IEntitySource src) { _Print("_WB_CanRename"); return super._WB_CanRename(owner, src); };
	override bool _WB_CanCopy(IEntity owner, IEntitySource src) { _Print("_WB_CanCopy"); return super._WB_CanCopy(owner, src); };
	override bool _WB_CanSelect(IEntity owner, IEntitySource src) { _Print("_WB_CanSelect"); return super._WB_CanSelect(owner, src); };
	override void _WB_GetBoundBox(IEntity owner, inout vector min, inout vector max, IEntitySource src) { _Print("_WB_GetBoundBox"); }
	override void _WB_SetExtraVisualiser(IEntity owner, EntityVisualizerType type, IEntitySource src) { _Print("_WB_SetExtraVisualiser"); }
	override array<ref WB_UIMenuItem> _WB_GetContextMenuItems(IEntity owner) { _Print("_WB_GetContextMenuItems"); return super._WB_GetContextMenuItems(owner); }
	override bool _WB_OnPhysSimulPlacementBegin(IEntity owner, IEntitySource src) { _Print("_WB_OnPhysSimulPlacementBegin"); return super._WB_OnPhysSimulPlacementBegin(owner, src); };
	override bool _WB_EnablePhysics(IEntity owner, IEntitySource src, bool physics) { _Print("_WB_EnablePhysics"); return super._WB_EnablePhysics(owner, src, physics); }
	override bool _WB_OnKeyChanged(IEntity owner, BaseContainer src, string key, BaseContainerList ownerContainers, IEntity parent) { _Print("_WB_OnKeyChanged"); return super._WB_OnKeyChanged(owner, src, key, ownerContainers, parent); }
	#ifdef _WB_WORLD_UPDATES
	override void _WB_AfterWorldUpdate(IEntity owner, float timeSlice) { _Print("_WB_AfterWorldUpdate"); }
	#endif
	override void _WB_OnContextMenu(IEntity owner, int id) { _Print("_WB_OnContextMenu"); }
	override void _WB_OnKeyDown(IEntity owner, int keyCode) { _Print("_WB_OnKeyDown"); }
	override void _WB_OnCreate(IEntity owner, IEntitySource src) { _Print("_WB_OnCreate"); }
	override void _WB_OnDelete(IEntity owner, IEntitySource src) { _Print("_WB_OnDelete"); }
	override void _WB_OnRename(IEntity owner, IEntitySource src, string oldName) { _Print("_WB_OnRename"); }
	override array<ref ParamEnum> _WB_GetUserEnums(string varName, IEntity owner, IEntityComponentSource src) { _Print("_WB_GetUserEnums"); return super._WB_GetUserEnums(varName, owner, src); }
	override void OnTransformResetImpl(TransformResetParams params) { _Print("OnTransformResetImpl"); };
	// ScriptComponent events
	override void EOnTouch(IEntity owner, IEntity other, int touchTypesMask) { _Print("EOnTouch"); }
	override void EOnInit(IEntity owner) { _Print("EOnInit"); }
	override void EOnVisible(IEntity owner, int frameNumber) { _Print("EOnVisible"); }
	override void EOnFrame(IEntity owner, float timeSlice) { _Print("EOnFrame"); }
	override void EOnPostFrame(IEntity owner, float timeSlice) { _Print("EOnPostFrame"); }
	override void EOnAnimEvent(IEntity owner, int type, int slot) { _Print("EOnAnimEvent"); }
	override void EOnSimulate(IEntity owner, float timeSlice) { _Print("EOnSimulate"); }
	override void EOnPostSimulate(IEntity owner, float timeSlice) { _Print("EOnPostSimulate"); }
	override void EOnJointBreak(IEntity owner, IEntity other) { _Print("EOnJointBreak"); }
	override void EOnPhysicsMove(IEntity owner) { _Print("EOnPhysicsMove"); }
	override void EOnContact(IEntity owner, IEntity other, Contact contact) { _Print("EOnContact"); }
	override void EOnPhysicsActive(IEntity owner, bool activeState) { _Print("EOnPhysicsActive"); }
	override void EOnDiag(IEntity owner, float timeSlice) { _Print("EOnDiag"); }
	override void EOnFixedFrame(IEntity owner, float timeSlice) { _Print("EOnFixedFrame"); }
	override void EOnPostFixedFrame(IEntity owner, float timeSlice) { _Print("EOnPostFixedFrame"); }
	override void EOnActivate(IEntity owner) { _Print("EOnActivate"); }
	override void EOnDeactivate(IEntity owner) { _Print("EOnDeactivate"); }
	override void OnPostInit(IEntity owner) { _Print("OnPostInit"); }
	override void OnDelete(IEntity owner) { _Print("OnDelete"); }
	override void OnAddedToParent(IEntity child, IEntity parent) { _Print("OnAddedToParent"); }
	override void OnRemovedFromParent(IEntity child, IEntity parent) { _Print("OnRemovedFromParent"); }
	override void OnChildAdded(IEntity parent, IEntity child) { _Print("OnChildAdded"); }
	override void OnChildRemoved(IEntity parent, IEntity child) { _Print("OnChildRemoved"); }
	void SCR_MyComponent(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		SetEventMask(GetOwner(), EntityEvent.ALL);
		_Print("Constructor");
	}
	void ~SCR_MyComponent() { _Print("Destructor"); }
	#endif
}
```

[↑ Back to spoiler's top](#bikisp6a63e6afc650d)
