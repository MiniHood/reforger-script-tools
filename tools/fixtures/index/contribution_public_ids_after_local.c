// Regression fixture: a local declaration precedes a later public declaration.
// Public file contributions must remap IDs after omitting the local.
class ContributionIdsBeforePublicFixture
{
	void Build()
	{
		int localValue = 1;
	}
}

class ContributionIdsAfterPublicFixture
{
}
