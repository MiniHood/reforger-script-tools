// Fixture truth: game-data-derived excerpt copied from generated API and handwritten Game scripts in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
//! Scripted load container for custom handling of storage
class ScriptedLoadContainer: LoadContainer
{
	event protected bool StartObject() {return false;};
	event protected bool EndObject() {return false;};
	event protected bool StartArray(out int count) {return false;};
	event protected bool EndArray() {return false;};
	//! Key reader for named properties in objects or map keys.
	event protected bool ReadKey(inout string key) {return false;};
}

class ScriptedUserAction: BaseUserAction
{
	//! Method called when the action is interrupted/canceled.
	//! \param pUserEntity The entity that was performing this action prior to interruption
	event void OnActionCanceled(IEntity pOwnerEntity, IEntity pUserEntity) { };
	//! Method called from scripted interaction handler when an action is started (progress bar appeared)
	//! \param pUserEntity The entity that started performing this action
	event void OnActionStart(IEntity pUserEntity) { };
	//! If overridden and true is returned, outName is returned when BaseUserAction.GetActionName is called.
	event bool GetActionNameScript(out string outName) { return false; };
}

[BaseContainerProps()]
class SCR_DefendWaypointPreset
{	
	[Attribute("", UIWidgets.EditBox, "Preset name, only informative. Switch using index.")];
	protected string m_sName;
	
	[Attribute("true", UIWidgets.CheckBox, "Use turrets?")];
	protected bool m_bUseTurrets;
	
	[Attribute("1", UIWidgets.Slider, "Fraction of SA used for this waypoint 0 - no, 1 - all available. The rest uses sector defense", "0 1 0.1")];
	protected float m_fFractionOfSA;
}

//! Inform the weather manager a lightning has been spawned. Weather Manager will handle light changes.
class SCR_LightningComponent : ScriptComponent
{
	[Attribute(defvalue: "4")]
	protected int m_iMinFlashes;
	
	[Attribute(defvalue: "25")];
	protected float m_fFlashMinDurationMillis;
	
	[Attribute(defvalue: "200")];
	protected float m_fFlashMaxDurationMillis;
}
