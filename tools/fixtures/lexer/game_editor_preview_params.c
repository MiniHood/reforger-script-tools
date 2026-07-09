// Fixture truth: game-data-derived excerpt copied from Game/Editor/Containers/Rpc/SCR_EditorPreviewParams.c in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
//! @ingroup Editor_Containers

//! Network packet of variables for entity placing and transformation.
class SCR_EditorPreviewParams
{
	SCR_EditableEntityComponent m_Parent;
	RplId m_ParentID = RplId.Invalid();
	bool m_bParentChanged;
	
	RplId m_TargetRplID = RplId.Invalid();
	EntityID m_TargetStaticID;
	SCR_EditableEntityComponent m_Target;
	EEditableEntityInteraction m_TargetInteraction;
	
	vector m_vTransform[4];
	vector m_Offset;
	bool m_bIsUnderwater;
	EEditorTransformVertical m_VerticalMode;
	EEditorPlacingFlags m_PlacingFlags;
	SCR_EditableEntityComponent m_CurrentLayer; //--- No replicated, used to extract data from SCR_PlacingEditorComponent.SpawnEntityResource()
	//! action id with the id of the action component from whcih it came from
	int m_iActionInfo = -1;
	//! local cache of the action which was the reason for creation of this param
	protected SCR_BaseEditorAction m_SourceAction;
	
	//------------------------------------------------------------------------------------------------
	static bool PropCompare(SCR_EditorPreviewParams prop, SSnapSerializerBase snapshot, ScriptCtx hint) 
	{
		return snapshot.Compare(prop.m_vTransform, 48)
			&& snapshot.Compare(prop.m_ParentID, 4)
			&& snapshot.Compare(prop.m_bParentChanged, 4)
			&& snapshot.Compare(prop.m_VerticalMode, 4)
			&& snapshot.Compare(prop.m_bIsUnderwater, 4)
			&& snapshot.Compare(prop.m_TargetRplID, 4)
			&& snapshot.Compare(prop.m_TargetStaticID, 8)
			&& snapshot.Compare(prop.m_TargetInteraction, 4)
			&& snapshot.Compare(prop.m_PlacingFlags, 4)
			&& snapshot.Compare(prop.m_iActionInfo, 4);
	}
	
	//------------------------------------------------------------------------------------------------
	//! Get world transformation matrix. If offset is used, it will be applied to it.
	//! \param[out] outTransform Variable to be filled with transformation matrix
	void GetWorldTransform(out vector outTransform[4])
	{
		if (m_Offset == vector.Zero)
		{
			//--- Exact position
			//outTransform = m_vTransform; //--- Doesn't work, reference is lost
			Math3D.MatrixCopy(m_vTransform, outTransform);
		}
		else
		{
			//--- Offset when multiple entities are being placed (e.g., waypoints for a group)
			vector coefMatrix[4] = {m_vTransform[0], m_vTransform[1], m_vTransform[2], vector.Zero};
			vector offsetMatrix[4] = { vector.Zero, vector.Zero, vector.Zero, m_Offset};
			Math3D.MatrixMultiply4(coefMatrix, offsetMatrix, offsetMatrix);
			outTransform = {m_vTransform[0], m_vTransform[1], m_vTransform[2], m_vTransform[3] + offsetMatrix[3]};
		}
	}
	
	//------------------------------------------------------------------------------------------------
	//! Convert replication-friendly values to actual variables.
	//! \return True if the deserialization was completed successfully
	bool Deserialize()
	{		
		m_Parent = SCR_EditableEntityComponent.Cast(Replication.FindItem(m_ParentID));
		if (!m_Parent && m_ParentID.IsValid())
		{
			Print(string.Format("Cannot deserialize entity, parent with RplId = %1 not found!", m_ParentID), LogLevel.ERROR);
			return false;
		}
		
		m_Target = SCR_EditableEntityComponent.Cast(Replication.FindItem(m_TargetRplID));
		
		if (!m_Target)
			m_Target = SCR_EditableEntityComponent.GetEditableEntity(GetGame().GetWorld().FindEntityByID(m_TargetStaticID));

		if (!m_Target && m_TargetRplID.IsValid())
		{
			Print(string.Format("Cannot deserialize entity, target neither with RplId = %1 nor with EntityID = %2 found!", m_TargetRplID, m_TargetStaticID), LogLevel.ERROR);
			return false;
		}

		return true;
	}
}
