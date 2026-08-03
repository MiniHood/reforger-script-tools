# [Action Context Setup](https://community.bistudio.com/wiki/Arma_Reforger:Action_Context_Setup)

## Select Entity

Select the entity you want to add contexts and actions into. Select the **"ActionsManagerComponent"** or add one if it doesn't exist.

⚠

* For user actions to be properly synchronised and work as intended, the entity requires a **RplComponent**.
* ActionsManagerComponent is not the same thing as ActionManager. The former manages user actions, the latter input actions!
* To be able to interact with the entity properly, such entity must have valid physical representation. For most entities, this is already the case. If you are setting up an entity from scratch, please make sure that:
  + Entity has **MeshObject** component with valid mesh
  + Entity has **RigidBody** component with *Model Geometry* checked.

[![armareforger-actioncontext add component.png](/wikidata/images/thumb/0/0c/armareforger-actioncontext_add_component.png/600px-armareforger-actioncontext_add_component.png)](/wiki/File:armareforger-actioncontext_add_component.png)

## Define Context

Fill the Action Contexts array with your contexts for this entity.
The context name is a unique identifier used to register child actions. Do not forget to fill the Position field with valid PointInfo or derived class instance.
The component UIInfo is required too.

[![armareforger-actioncontext add context.png](/wikidata/images/thumb/4/41/armareforger-actioncontext_add_context.png/600px-armareforger-actioncontext_add_context.png)](/wiki/File:armareforger-actioncontext_add_context.png)

## Add Action Location

### General Action

For general actions use the Additional Actions field in the ActionsManagerComponent.

[![armareforger-actioncontext add action general.png](/wikidata/images/thumb/6/63/armareforger-actioncontext_add_action_general.png/600px-armareforger-actioncontext_add_action_general.png)](/wiki/File:armareforger-actioncontext_add_action_general.png)

### Contextual Action

For actions provided by a component (*compartments* - GetInAction, *doors* - DoorAction) find the proper component. In this case it will be one of this entity's DoorComponent.

[![armareforger-actioncontext add action component.png](/wikidata/images/thumb/f/fe/armareforger-actioncontext_add_action_component.png/600px-armareforger-actioncontext_add_action_component.png)](/wiki/File:armareforger-actioncontext_add_action_component.png)

## Add Action

### General Action

For **general** actions add the action straight into the **"Additional Actions"** field in the **ActionsManagerComponent**.
To set a parent context for our new action add an entry to the **"Parent Context List"** field of the action. As the value choose one of the context names you specified in the **"Action Contexts"** field in the "**ActionsManagerComponent"**. Don't forget to add UIInfo and fill it with desired data.
If an action should be visible in multiple contexts, add that unique identifier (target context's **Context Name**) to the "**Parent Context List**" too.

[![armareforger-actioncontext setup action general.png](/wikidata/images/thumb/3/31/armareforger-actioncontext_setup_action_general.png/600px-armareforger-actioncontext_setup_action_general.png)](/wiki/File:armareforger-actioncontext_setup_action_general.png)

### Contextual Action

For **actions provided by a component** add the action to a slot provided by the component. In this case the field **"DoorAction"** provided by the **"DoorComponent"**.
To set a parent context for our new action add an entry to the **"Parent Context List"** field of the action. As the value choose one of the context names you specified in the **"Action Contexts"** field in the "**ActionsManagerComponent"**. Don't forget to add UIInfo and fill it with desired data.
If an action should be visible in multiple contexts, add that unique identifier (target context's **Context Name**) to the "**Parent Context List**" too.

[![armareforger-actioncontext setup action component.png](/wikidata/images/thumb/c/c5/armareforger-actioncontext_setup_action_component.png/600px-armareforger-actioncontext_setup_action_component.png)](/wiki/File:armareforger-actioncontext_setup_action_component.png)

## Add Parameters

### Example

```enforce
class TAG_MyTeleportScriptedUserAction : ScriptedUserAction
{
	[Attribute(defvalue: "0 0 0", uiwidget: UIWidgets.Coords, desc: "Teleport destination")]
	protected vector m_vTeleportDestination;
	[Attribute(defvalue: "2", uiwidget: UIWidgets.EditBox, desc: "Spawn min. distance")]
	protected int m_iSpawnMinDist;
	[Attribute(defvalue: "4", uiwidget: UIWidgets.EditBox, desc: "Spawn max. distance")]
	protected int m_iSpawnMaxDist;
	//------------------------------------------------------------------------------------------------
	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
	{
		RandomGenerator randomGenerator = new RandomGenerator();
		vector teleportPosition = randomGenerator.GenerateRandomPointInRadius(m_iSpawnMinDist, m_iSpawnMaxDist, m_vTeleportDestination);
		pUserEntity.SetOrigin(teleportPosition);
	}
	//------------------------------------------------------------------------------------------------
	override bool CanBeShownScript(IEntity user)
	{
		return true;
	}
	//------------------------------------------------------------------------------------------------
	override bool CanBePerformedScript(IEntity user)
	{
		return true;
	}
	//------------------------------------------------------------------------------------------------
	override bool HasLocalEffectOnlyScript()
	{
		return true;
	}
}
```

## Walkthrough

### Script Setup

1. Create this directory: Scripts/Game/generated/UserAction/Modded
2. In it, create TAG\_MyScriptedUserAction.c
3. Inside this file put this:

   ENFORCECODEMARKER

   ```
   class TAG_MyScriptedUserAction : ScriptedUserAction
   {
   	//------------------------------------------------------------------------------------------------
   	override void PerformAction(IEntity pOwnerEntity, IEntity pUserEntity)
   	{
   		Print("MyScriptedUserAction.PerformAction() method reached", LogLevel.NORMAL);
   		SCR_HintManagerComponent.GetInstance().ShowCustomHint("My test ground world - DONE", "TEST GROUND", 3.0);
   	}
   	//------------------------------------------------------------------------------------------------
   	override bool CanBeShownScript(IEntity user)
   	{
   		return true;
   	}
   	//------------------------------------------------------------------------------------------------
   	override bool CanBePerformedScript(IEntity user)
   	{
   		return true;
   	}
   	//------------------------------------------------------------------------------------------------
   	override bool HasLocalEffectOnlyScript()
   	{
   		return true;
   	}
   }
   ```

### World/Entity Setup

1. If not present, add an ActionsManagerComponent (read wiky for minimum components present in order to work)
2. Add one Action Context
3. In it:
   * input some **Context Name** (myContext)
   * set **Position** to PointInfo and move up the Y offset (you will se the point moving in the world editor)
4. Add one Additional Actions selecting your class inside TAG\_MyScriptedUserAction.c, in this example TAG\_MyScriptedUserAction
5. In Additional Actions/TAG\_MyScriptedUserAction:
   * add one parent context list and select the Context Name created before (myContext)
   * add **UiInfo** and input some **Name** to be shown
   * set **visibility range** to some value, e.g 2
   * set **duration** to some value, e.g 2
6. ????
7. Profit!
