# Workbench plugin probe

This is a self-contained Workbench add-on used to test whether a custom NET
API handler supplied by an active add-on remains callable when another loaded
workspace script has a compiler error.

The probe deliberately uses the existing `RST_WorkbenchListEditors` handler
name and response shape, so the existing `workbench_list_editors` operation
can call it without adding a production bridge handler or installing files in
the Workbench user profile.

The handler prints `RST_PLUGIN_PROBE_REACHED` when Workbench invokes it.

For the live workspace test, copy
`project/Scripts/WorkbenchGame/RST_WorkbenchListEditors.c` into the active
project's own `Scripts/WorkbenchGame` directory. Do not register the fixture as
an additional project or add-on. Workbench will load the file as part of the
currently opened project and its Workbench script module.

The `.gproj` in this fixture is only a standalone layout reference for the
module and source placement; it is not required for the live test.

Run the test with a disposable Workbench profile that does not contain the
profile-managed bridge. The probe intentionally uses the existing
`RST_WorkbenchListEditors` handler name so `workbench_list_editors` can invoke
it; an installed profile bridge with the same handler name would make the
result ambiguous.

Then call `workbench_list_editors` and inspect the Workbench log for the marker.

This fixture intentionally contains no managed bridge files and no profile
script files.
