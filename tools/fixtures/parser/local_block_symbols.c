// Fixture truth: game-data-derived local/block symbol shapes; not Workbench-confirmed in this repo.
class SCR_LocalBlockSymbolFixture
{
	void Collect(array<SCR_OutfitFactionData> sourceData)
	{
		array<SCR_OutfitFactionData> outfitDataArray = {};
		const int currentBudgetValue = GetBudgetValue();
		ref SCR_PlayerDataEvent dataEvent = new SCR_PlayerDataEvent;
		int playerID, param2;
		vector debugPoints[4];

		foreach (SCR_OutfitFactionData data : outfitDataArray)
		{
			string factionName = data.GetFactionKey();
		}

		foreach (int idx, auto quickslot : GetQuickSlotItems())
		{
			Print(idx);
		}

		for (int i = 0, count = outfitDataArray.Count(); i < count; i++)
		{
			SCR_OutfitFactionData currentData = outfitDataArray[i];
		}
	}
}
