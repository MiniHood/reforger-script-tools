// Fixture truth: game-data-derived from common preprocessor shapes in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
#define SCR_FEATURE_FLAG
#ifdef SCR_FEATURE_FLAG

class SCR_PreprocessorExample : Managed
{
	#ifdef ENABLE_DIAG
	#define GAME_MODE_DEBUG
	#endif

	protected int m_iValue;
}

#endif
