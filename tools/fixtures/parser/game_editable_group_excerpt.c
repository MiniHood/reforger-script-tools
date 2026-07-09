// Fixture truth: game-data-derived excerpt copied from Game/Editor/Components/EditableEntity/SCR_EditableGroupComponent.c in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
[ComponentEditorProps(category: "GameScripted/Editor (Editables)", description: "", icon: "WBData/ComponentEditorProps/componentEditor.png")]
class SCR_EditableGroupComponentClass : SCR_EditableEntityComponentClass
{
	//------------------------------------------------------------------------------------------------
	static override bool GetEntitySourceBudgetCost(IEntityComponentSource editableEntitySource, out notnull array<ref SCR_EntityBudgetValue> budgetValues)
	{
		return true;
	}
	
	//------------------------------------------------------------------------------------------------
	//! Use when you need to get a set budget values for the group and don't want to relay on default logic where AI budget is deducted by individually spawned AI.
	static bool GetGroupSourceBudgetCost(IEntityComponentSource editableEntitySource, out notnull array<ref SCR_EntityBudgetValue> budgetValues)
	{
		if (!editableEntitySource)
			return false;
		
		SCR_EditableGroupUIInfo editableEntityUIInfo = SCR_EditableGroupUIInfo.Cast(SCR_EditableGroupComponentClass.GetInfo(editableEntitySource));
		if (editableEntityUIInfo)
			return editableEntityUIInfo.GetGroupBudgetCost(budgetValues);
		
		return !budgetValues.IsEmpty();
	}
}

//! @ingroup Editable_Entities

//! Special configuration for editable group.
class SCR_EditableGroupComponent : SCR_EditableEntityComponent
{	
	protected SCR_AIGroup m_Group;

	[RplProp(onRplName: "OnLeaderIdChanged")]
	protected RplId m_LeaderId;

	protected SCR_EditableEntityComponent m_Leader;
	protected ref SCR_EditableGroupUIInfo m_GroupInfo;
	protected ref ScriptInvoker Event_OnUIRefresh = new ScriptInvoker;

	//~ Authority only, Forces spawned characters to be added to a specific vehicle position and will delete it if failed
	protected ref array<ECompartmentType> m_aForceSpawnVehicleCompartments;

	protected SCR_PlacingEditorComponent m_PlacedEditorComponent;
	protected AIWaypointCycle m_CycleWaypoint;
	protected bool m_bAreWaypointsCycled;
	static ResourceName AI_WAYPOINT_CYCLE = "{35BD6541CBB8AC08}Prefabs/AI/Waypoints/AIWaypoint_Cycle.et";
	
	//------------------------------------------------------------------------------------------------
	override void OnDelete(IEntity owner)
	{
		super.OnDelete(owner);
		const int missingAgents = m_Group.GetSpawnQueueSize();
		
		if(m_Group.GetSpawnQueueSize() == 0)
			return;
	
		OnAfterAllMembersSpawned();
		
		SCR_BudgetEditorComponent budgetComponent = SCR_BudgetEditorComponent.Cast(SCR_BudgetEditorComponent.GetInstance(SCR_BudgetEditorComponent));
		if (!budgetComponent)
			return;
	
		SCR_EditableEntityCoreBudgetSetting aiBudget = budgetComponent.GetBudgetSetting(EEditableEntityBudget.AI);
		if (aiBudget)
			aiBudget.UnreserveBudget(missingAgents);
	}
	
	//------------------------------------------------------------------------------------------------
	void EnableCycledWaypoints(bool enable)
	{
		if (enable == m_bAreWaypointsCycled || !IsServer())
			return;
		
		m_bAreWaypointsCycled = enable;
		array<AIWaypoint> waypoints = {};
		if (m_bAreWaypointsCycled)
		{
			m_CycleWaypoint = AIWaypointCycle.Cast(GetGame().SpawnEntityPrefab(Resource.Load(SCR_EditableGroupComponent.AI_WAYPOINT_CYCLE)));
			
			m_Group.GetWaypoints(waypoints);
			m_CycleWaypoint.SetWaypoints(waypoints);
			RemoveAllWaypointsFromGroup();
			m_Group.AddWaypoint(m_CycleWaypoint);
		}
		else
		{
			m_CycleWaypoint.GetWaypoints(waypoints);
			AddWaypoints(waypoints);
			m_Group.RemoveWaypoint(m_CycleWaypoint);
			delete m_CycleWaypoint;
		}
		
		ReindexWaypoints();
		Rpc(EnableCycledWaypointsBroadcast, enable)
	}
	
	//------------------------------------------------------------------------------------------------
	protected void AddWaypoints(array<AIWaypoint> waypoints)
	{
		for (int i = 0, count = waypoints.Count(); i < count; i++)
		{
			m_Group.AddWaypoint(waypoints[i]);
		}
	}
	
	//------------------------------------------------------------------------------------------------
	protected void RemoveAllWaypointsFromGroup()
	{
		array<AIWaypoint> waypoints = {};
		m_Group.GetWaypoints(waypoints);
		for (int i = 0, count = waypoints.Count(); i < count; i++)
		{
			m_Group.RemoveWaypointAt(0);
		}
	}
	
	//------------------------------------------------------------------------------------------------
	[RplRpc(RplChannel.Reliable, RplRcver.Broadcast)]
	protected void EnableCycledWaypointsBroadcast(bool enable)
	{
		m_bAreWaypointsCycled = enable;
	}
	
	//------------------------------------------------------------------------------------------------
	protected void OnAgentAdded(AIAgent child)
	{
		if (!child)
			return;

		SCR_EditableEntityComponent editableChild = SCR_EditableEntityComponent.GetEditableEntity(child.GetControlledEntity());
		if (editableChild)
			editableChild.SetParentEntity(this);
		
		SCR_BudgetEditorComponent budgetComponent = SCR_BudgetEditorComponent.Cast(SCR_BudgetEditorComponent.GetInstance(SCR_BudgetEditorComponent));
		if (!budgetComponent)
			return;
	
		SCR_EditableEntityCoreBudgetSetting aiBudget = budgetComponent.GetBudgetSetting(EEditableEntityBudget.AI);
		if (aiBudget)
			aiBudget.UnreserveBudget(1);
	}
}
