// Fixture truth: game-data-derived excerpt copied from Game/AI/ScriptedNodes/Waypoints files in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
class SCR_AIGetEntityWaypointParameters : SCR_AIGetWaypointParameters
{
	protected static ref TStringArray s_aVarsOut2 = SCR_AINodePortsHelpers.MergeTwoArrays(SCR_AIGetWaypointParameters.s_aVarsOut_Base, {PORT_ENTITY});
	
	//------------------------------------------------------------------------------------------------
	override TStringArray GetVariablesOut()
    {
        return s_aVarsOut2;
    }
}

class SCR_AIGetDefendWaypointParameters : SCR_AIGetWaypointParameters
{
	ref array<string> m_tagsArray = {};
	protected static ref TStringArray s_aVarsOut2 = SCR_AINodePortsHelpers.MergeTwoArrays(SCR_AIGetWaypointParameters.s_aVarsOut_Base, {PORT_USE_TURRETS, PORT_SEARCH_TAGS, PORT_FAST_INIT, PORT_WAYPOINT_HOLDING_TIME});
	
	//------------------------------------------------------------------------------------------------
	override TStringArray GetVariablesOut()
    {
        return s_aVarsOut2;
    }
}
