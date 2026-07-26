# [Crash Report](https://community.bistudio.com/wiki/Arma_Reforger:Crash_Report)

## Crash Reporter

In the unfortunate event of a crash, the **Crash Reporter** collects crash information in order to allow Bohemia Interactive to collect and process the crash reasons.

This is an important stone in the user feedback process.

[![](/wikidata/images/thumb/d/d7/armareforger_crash-reporter-dialog.png/300px-armareforger_crash-reporter-dialog.png)](/wiki/File:armareforger_crash-reporter-dialog.png)

The Crash Reporter dialog.

### Dialog

The Crash Reporter dialog offers a feedback field allowing the end user to provide details regarding the crash:

* The most important aspect of a report is reproducibility: which relevant steps lead to this crash
* If the reproduction is not guaranteed (e.g a "random" crash), then an accurate description of the context is welcome
* If there was absolutely nothing relevant, just sending crash files will do

| Correct Feedback | Incorrect Feedback |
| --- | --- |
| * 100% repro: place a HMMWV, get in as driver, then switch to any rear seat = crash * Using World Editor with these mods crashes on opening Everon * The Game Master teleported me twice in 5s, then I crashed | * Pasting in viewport does that * It doesn't work * It crashed * Fix it! |

Dialog Availability

| Operating System | Dedicated Server | Game | Workbench |
| --- | --- | --- | --- |
| Windows | Unchecked | Checked | Checked |
| Linux | Unchecked | N/A | N/A |

The dialog allows the user to cancel report sending.

The files are automatically sent if the dialog is not available, unless the [disableCrashReporter](/wiki/Arma_Reforger:Startup_Parameters#disableCrashReporter "Arma Reforger:Startup Parameters") startup parameter is used.

### Crash Reports Log

CrashReports.log is a file containing information from crash reports that were sent to Bohemia Interactive through Crash Reporter. It contains the following crash information:

* Date and Time
* Game version
* State - Canceled, Failed (with error code), Successful (with report's GUID)

The GUID can be used as a reference to keep track while communicating with Bohemia Interactive.

ⓘ

The **Dedicated Server** stacks reports as the server may run unattended with autorestart.
Only the last crash report is kept for **Game**/**Workbench**.

## Logs Directory

Logs for Game are stored in the following directory:

| Operating System | Logs Directory |
| --- | --- |
| Windows | ``` %userprofile%\Documents\My Games\ArmaReforger\logs ``` |

Logs for Workbench are stored in the following directory:

| Operating System | Logs Directory |
| --- | --- |
| Windows | ``` %userprofile%\Documents\My Games\ArmaReforgerWorkbench\logs ``` |

## Feedback Tracker

The [Bohemia Interactive Feedback Tracker](https://feedback.bistudio.com/) is a public feedback tracker available to end-users allowing them to report encountered issues.

ⓘ

Be sure to follow the [How-To guide](https://feedback.bistudio.com/w/ft_ar_howto/) and to use **Search** before posting.

Available Forms

|  | PC (Project: [Arma Reforger](https://feedback.bistudio.com/project/view/66/)) |  Xbox (Project: [Arma Reforger Xbox](https://feedback.bistudio.com/project/view/67/)) |  Playstation (Project: [Arma Reforger PlayStation](https://feedback.bistudio.com/project/view/88/)) |
| --- | --- | --- | --- |
| Bug Report | [New Arma Reforger Bug Report](https://feedback.bistudio.com/maniphest/task/edit/form/36/) | [New Arma Reforger Xbox Bug Report](https://feedback.bistudio.com/maniphest/task/edit/form/37/) | [New Arma Reforger PlayStation Bug Report](https://feedback.bistudio.com/maniphest/task/edit/form/55/) |
| **Private** Bug Report | [New Arma Reforger Private Bug Report](https://feedback.bistudio.com/maniphest/task/edit/form/38/) | [New Arma Reforger Private Xbox Bug Report](https://feedback.bistudio.com/maniphest/task/edit/form/39/) | [New Arma Reforger Private PlayStation Bug Report](https://feedback.bistudio.com/maniphest/task/edit/form/56/) |
| [Modding](https://feedback.bistudio.com/project/view/68/) Bug Report | [New Arma Reforger Modding Bug Report](https://feedback.bistudio.com/maniphest/task/edit/form/40/) | N/A | N/A |

## See Also

* [Crash reporter](https://en.wikipedia.org/wiki/Crash_reporter)
* [Startup Parameters - noCrashDialog](/wiki/Arma_Reforger:Startup_Parameters#noCrashDialog "Arma Reforger:Startup Parameters")
* [Startup Parameters - keepCrashFiles](/wiki/Arma_Reforger:Startup_Parameters#keepCrashFiles "Arma Reforger:Startup Parameters")
* [Startup Parameters - disableCrashReporter](/wiki/Arma_Reforger:Startup_Parameters#disableCrashReporter "Arma Reforger:Startup Parameters")
