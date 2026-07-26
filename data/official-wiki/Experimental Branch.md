# [Experimental Branch](https://community.bistudio.com/wiki/Arma_Reforger:Experimental_Branch)

Owning Arma Reforger on Steam offers an Experimental version as two different applications named **Arma Reforger Experimental [(install on Steam)](steam://install/1890860)** and **Arma Reforger Experimental Tools** **[(install and run on Steam)](steam://run/1890880)**.

These versions provide access to new features earlier with more frequent updates but also potentially less stable ones.

⚠

* In order to launch the Experimental [Workbench](/wiki/Category:Arma_Reforger/Modding/Official_Tools "Category:Arma Reforger/Modding/Official Tools"), it is **required** to have Arma Reforger Experimental installed and to [add its Game Data's ArmaReforger.gproj to the addons list manually](/wiki/Arma_Reforger:Mod_Project_Setup#Manual_Method "Arma Reforger:Mod Project Setup")
* Both versions **share the same player profile/addon directory** - beware of sneaking Experimental features into Stable mods and vice versa
  + It might be worth considering using different profile (f.e. ArmaReforgerExperimental & ArmaReforgerWorkbenchExperimental) via [-profile](/wiki/Arma_Reforger:Startup_Parameters#profile "Arma Reforger:Startup Parameters") CLI parameter for both Game and Workbench
* The Experimental version may be **outdated** when Stable version releases

⚠

Experimental version of the game and tools are using **separate backend** from Stable game. This means that i.e. **mods created for Stable game are not visible in Experimental workshop**.
When **publishing mods, it is also necessary to create new, separate account**.

## Differences

| Information | [Arma Reforger](/wiki/Category:Arma_Reforger "Category:Arma Reforger") | |
| --- | --- | --- |
| Stable | Experimental |
| Game Steam AppID | [1874880](https://store.steampowered.com/app/1874880) | 1890860 |
| Server Steam AppID | [1874900](https://store.steampowered.com/app/1874900) | 1890870 |
| Tools Steam AppID | [1874910](https://store.steampowered.com/app/1874910) | 1890880 |
| Workshop | Each version has its own [Workshop](/wiki/Arma_Reforger:Workshop "Arma Reforger:Workshop");  mods and accounts are not shared (another BI account must be [created on experimental backend](https://accounts-sub-ar.bistudio.com/auth/register)) | |

## Third Party Software

### DRED

[DRED](https://learn.microsoft.com/en-us/windows/win32/direct3d12/use-dred) (Device Removal Extended Data) is a Microsoft DirectX 12 software used to obtain additional information on render crashes.

* It is (re)installed if missing when the Experimental game executable is launched
* It is not uninstalled when the Experimental branch is uninstalled, as it is impossible to distinguish between a usermade installation and an Arma Reforger-related installation
* The performance overhead is minimal and should not be noticeable
* To manage the software, go to Windows Settings > Apps > Apps & Features > Optional Features:
  + To install it, click on Add a feature and select **Graphics Tools**
  + To uninstall it, click on **Graphics Tools** then click its Uninstall button.
