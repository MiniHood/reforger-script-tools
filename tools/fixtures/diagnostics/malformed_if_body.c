// Truth status: malformed fixture for parser diagnostic UX.
class MalformedIfBody
{
	void Run()
	{
		if (true
			Print("x");
	}
}
