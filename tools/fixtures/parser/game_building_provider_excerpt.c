// Fixture truth: game-data-derived excerpt copied from Game/Building/SCR_CampaignBuildingProviderComponent.c in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
[EntityEditorProps(category: "GameScripted/Building", description: "Component attached to a provider, responsible for basic provider behaviour.")]
class SCR_CampaignBuildingProviderComponentClass : SCR_MilitaryBaseLogicComponentClass
{
}

class SCR_CampaignBuildingProviderComponent : SCR_MilitaryBaseLogicComponent
{
	[Attribute("", UIWidgets.EditBox, "Name of provider shown in provider interface", "")]
	protected string m_sProviderDisplayName;
	
	[Attribute("0", UIWidgets.CheckBox, "Can the building mode at this provider executed only via user action?")]
	protected bool m_bUserActionActivationOnly;
	
	[Attribute("0", UIWidgets.CheckBox, "Can be used by any faction")]
	protected bool m_bAnyFactionCanUse;

	[Attribute(defvalue: "1", uiwidget: UIWidgets.ComboBox, desc: "Minimal rank that allows player to use the provider to build structures.", enums: ParamEnumArray.FromEnum(SCR_ECharacterRank))]
	protected SCR_ECharacterRank m_iRank;
	
	[Attribute(desc: "Fill in the budgets to be used with this provider")]
	protected ref array<ref SCR_CampaignBuildingBudgetToEvaluateData> m_aBudgetsToEvaluate;

	[Attribute(desc: "Traits this provider will provide. Each trait represents a tab in building interface. The tabs have to be defined in building mode's SCR_ContentBrowserEditorComponent.", UIWidgets.ComboBox, enums: ParamEnumArray.FromEnum(EEditableEntityLabel))]
	protected ref array<EEditableEntityLabel> m_aAvailableTraits;

	//! Current props Value represents, how many entities with set prop budget can be spawned with this provider. The max number is limited by Prop budget.
	[RplProp()]
	protected int m_iCurrentPropValue;
	
	//! Current AI Value represents, how many AI is currently spawned with this provider. The max number is limited by AI budget.
	[RplProp()]
	protected int m_iCurrentAIValue;

	protected SCR_ResourceComponent m_ResourceComponent;
	protected ref array<int> m_aActiveUsersIDs = {};
	protected ref array<int> m_aAvailableUsersIDs = {};
	protected ref array<SCR_CampaignBuildingBudgetToEvaluateData> m_aShownBudget = {};
	protected static ref ScriptInvokerVoid s_OnProviderCreated = new ScriptInvokerVoid();
	protected ref ScriptInvokerVoid m_OnCooldownLockUpdated;
	protected const int MOVING_CHECK_PERIOD = 1000;
	protected const int PROVIDER_SPEED_TO_REMOVE_BUILDING_SQ = 1;
	protected ref array<ref Tuple2<int, WorldTimestamp>> m_aPlacingCooldown = {};
	protected bool m_bCooldownClientLock;
	protected bool m_bUseAllAvailableProvidersByPlayer;
	SCR_CampaignBuildingProviderComponent m_MasterProviderComponent;
	private ref map<EEditableEntityBudget, int> m_accumulatedBudgetChanges = new map<EEditableEntityBudget, int>;
	bool m_changesAccumulated = false;
	
	//------------------------------------------------------------------------------------------------
	void AccumulateBudgetChange(EEditableEntityBudget budgetType, int amount)
	{
		m_accumulatedBudgetChanges[budgetType] = m_accumulatedBudgetChanges[budgetType] + amount;
		
		if(!m_changesAccumulated)
		{
			GetGame().GetCallqueue().CallLater(ClearAccumulatedBudgetChanges);
			m_changesAccumulated = true;
		}
	}
	
	//------------------------------------------------------------------------------------------------
	int GetAccumulatedBudgetChanges(EEditableEntityBudget type)
	{
		int value = 0;
		bool found = m_accumulatedBudgetChanges.Find(type, value);
		
		if(found)
			return value;
		
		return -1;
	}
	
	//------------------------------------------------------------------------------------------------
	void ClearAccumulatedBudgetChanges()
	{
		m_accumulatedBudgetChanges.Clear();
		m_changesAccumulated = false;
	}
	
	//------------------------------------------------------------------------------------------------
	bool IsThereEnoughSupplies(int availableSupplies, int supplyCost, int accumulatedSupplyCost)
	{
		int totalSupplyCost = supplyCost;
		
		if(accumulatedSupplyCost != -1)
			totalSupplyCost += accumulatedSupplyCost;
		
		return totalSupplyCost <= availableSupplies;
	}
	
	//------------------------------------------------------------------------------------------------
	bool IsThereEnoughBudgetToSpawn(notnull array<ref SCR_EntityBudgetValue> budgetCosts)
	{
		if(budgetCosts.IsEmpty())
			return true;
		
		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		
		foreach(SCR_EntityBudgetValue budget : budgetCosts)
		{
			const EEditableEntityBudget budgetType = budget.GetBudgetType();
			SCR_CampaignBuildingBudgetToEvaluateData data = GetBudgetData(budgetType);
		
			if(!data)
				continue;
			
			SCR_CampaignBuildingProviderComponent realProvider = this;
			const int currentBudgetValue = GetBudgetValue(budgetType, realProvider);
			const int budgetIncrease = budget.GetBudgetValue();
			const int accumulatedBudgetChanges = realProvider.GetAccumulatedBudgetChanges(budgetType);

			if(budgetType == EEditableEntityBudget.CAMPAIGN)	
			{
				if(!gameMode.IsResourceTypeEnabled(EResourceType.SUPPLIES))
					continue;
				
				bool enoughSupplies = realProvider.IsThereEnoughSupplies(currentBudgetValue, budgetIncrease, accumulatedBudgetChanges);
				if(!enoughSupplies)
					return false;
				
				continue;
			}
			
			const int maxBudgetValue = GetMaxBudgetValueFromMasterIfNeeded(budgetType);
			if(maxBudgetValue == -1)
				continue;

			if(budgetIncrease + accumulatedBudgetChanges + currentBudgetValue > maxBudgetValue)
				return false;
		}
		
		foreach(SCR_EntityBudgetValue budget : budgetCosts)
		{
			const EEditableEntityBudget budgetType = budget.GetBudgetType();
			SCR_CampaignBuildingProviderComponent realProvider = this;
			const int currentBudgetValue = GetBudgetValue(budgetType, realProvider);
			realProvider.AccumulateBudgetChange(budgetType, budget.GetBudgetValue());
		}
		
		return true;
	}
	
	//------------------------------------------------------------------------------------------------
	int GetBudgetValue(EEditableEntityBudget type, out SCR_CampaignBuildingProviderComponent componentToUse)
	{
		bool useMaster = UseMasterProviderBudget(EEditableEntityBudget.PROPS, componentToUse);
		
		if(type == EEditableEntityBudget.PROPS)
			return GetCurrentPropValue();
		
		if(type == EEditableEntityBudget.AI)
			return GetCurrentAIValue();
		
		if(type != EEditableEntityBudget.CAMPAIGN)
			return -1;
		
		SCR_ResourceComponent resource = componentToUse.GetResourceComponent();
		
		if(!resource)
			return false;
		
		SCR_ResourceConsumer consumer = resource.GetConsumer(EResourceGeneratorID.DEFAULT, EResourceType.SUPPLIES);
		float currentSupplies = consumer.GetAggregatedResourceValue();
		return currentSupplies;
	}
}
