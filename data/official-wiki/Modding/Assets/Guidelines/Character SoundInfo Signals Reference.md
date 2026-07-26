# [Character SoundInfo Signals Reference](https://community.bistudio.com/wiki/Arma_Reforger:Character_SoundInfo_Signals_Reference)

## SoundInt

* SoundInt parameter defined on the component is passed to the corresponding signal, based on the character slot where the item is.
* If slot is empty signal value =  -1
* Coding of Weapon SoundInt example: **1 02 01 00**
  + 1 - Weapons
  + 02 - Group (Pistol, LMG)
  + 01 - Type (AK74)
  + 00 - Variation
* Those signals then trigger extra sounds when i.e. character moves. Example: metal rattle when harness is worn

## List of signal and item hierarchy

| Prefab | Signal | SoundInt | Component |
| --- | --- | --- | --- |
| **Helmets** | HeadCoverSoundInfo | 100 | BaseLoadoutComponent |
| Helmet\_PASGT | HeadCoverSoundInfo | 110 | BaseLoadoutComponent |
| Helmet\_PASGT\_cover | HeadCoverSoundInfo | 115 | BaseLoadoutComponent |
| Helmet\_SSh68 | HeadCoverSoundInfo | 120 | BaseLoadoutComponent |
| Helmet\_SSh68\_cover | HeadCoverSoundInfo | 125 | BaseLoadoutComponent |
| **FaceCover** | FaceCoverSoundInfo | 200 | BaseLoadoutComponent |
| **Jackets** | JacketSoundInfo | 300 | BaseLoadoutComponent |
| Jacket\_US\_BDU | JacketSoundInfo | 310 | BaseLoadoutComponent |
| Jacket\_M88 | JacketSoundInfo | 320 | BaseLoadoutComponent |
| **Vest** | VestSoundInfo | 400 | BaseLoadoutComponent |
| PASGT Vest | VestSoundInfo | 410 | BaseLoadoutComponent |
| 6B2\_Vest | VestSoundInfo | 420 | BaseLoadoutComponent |
| Vest\_SovietHarness\_suspenders\_base | VestSoundInfo | 440 | BaseLoadoutComponent |
| **Pants** | PantsSoundInfo | 500 | BaseLoadoutComponent |
| Pants\_BDU | PantsSoundInfo | 510 | BaseLoadoutComponent |
| Pants\_M88 | PantsSoundInfo | 520 | BaseLoadoutComponent |
| **Boots** | BootsSoundInfo | 600 | BaseLoadoutComponent |
| CombatBoots\_US\_01 | BootsSoundInfo | 610 | BaseLoadoutComponent |
| CombatBoots\_Soviet\_01 | BootsSoundInfo | 620 | BaseLoadoutComponent |
| JungleBoots | BootsSoundInfo | 630 | BaseLoadoutComponent |
| **Cover** | CoverSoundInfo | 700 | BaseLoadoutComponent |
| **Backpack** | BackPackSoundInfo | 800 | BaseLoadoutComponent |
| Backpack\_ALICE\_Medium | BackPackSoundInfo | 810 | BaseLoadoutComponent |
| Backpack\_ALICE\_Medium\_Frame | BackPackSoundInfo | 815 | BaseLoadoutComponent |
| Veshmeshok | BackPackSoundInfo | 820 | BaseLoadoutComponent |
| Kolobok | BackPackSoundInfo | 830 | BaseLoadoutComponent |
| Vest\_ALICE\_suspenders\_base | Attachment1SoundInfo | 1100 | BaseLoadoutComponent |
| **Underbarrel-Attachments** | SightSoundInfo | **9000 - 9999** | WeaponComponent |
| M203 | SightSoundInfo | 9001 | WeaponComponent |
| GP25 | SightSoundInfo | 9002 | WeaponComponent |
| **Scopes/Optics** | ZeroingSoundInt | 10000 | SCR\_2DPIPSightsComponent |
| PSO-1 | ZeroingSoundInt | 10001 | SCR\_2DPIPSightsComponent |
| ARTII | ZeroingSoundInt | 10002 | SCR\_2DPIPSightsComponent |
| Colt 4x20 | ZeroingSoundInt | 10004 | SCR\_2DPIPSightsComponent |
| **Weapons** | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | **1000000 - 1999999** | WeaponComponent |
| **Grenades / Throwables** | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | **1000000 - 1099999** | WeaponComponent |
| Generic Grenade | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1000000 | WeaponComponent |
| RGD2 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1000100 | WeaponComponent |
|  |  |  | WeaponComponent |
| **Pistols** | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | **1010000 - 1019999** | WeaponComponent |
| Generic Pistol | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1010000 | WeaponComponent |
| Makarov | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1010100 | WeaponComponent |
| Beretta | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1010200 | WeaponComponent |
|  |  |  | WeaponComponent |
| **Rifles** | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | **1020000 - 1029999** | WeaponComponent |
| Generic | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1020000 | WeaponComponent |
| AK 74 / 47 / VZ 58 Family | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1020100 | WeaponComponent |
| AK with UGL attachment | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1020101 | WeaponComponent |
| M16 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1020200 | WeaponComponent |
| M16 with UGL attachment | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1020201 | WeaponComponent |
|  |  |  | WeaponComponent |
| **Long Rifles** |  | **1030000 - 1039999** | WeaponComponent |
| SVD | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1030100 | WeaponComponent |
| M21 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1030200 | WeaponComponent |
|  |  |  | WeaponComponent |
| **LMGs** |  | **1040000 - 1049999** | WeaponComponent |
| Generic LMG | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1040000 | WeaponComponent |
| M249 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1040100 | WeaponComponent |
| M60 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1040200 | WeaponComponent |
| M60 (mounted on tripod, exterior) | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1040250 | WeaponComponent |
| PKM | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1040300 | WeaponComponent |
| PKM (mounted on tripod, exterior) | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1040350 | WeaponComponent |
| RPK7 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1040400 | WeaponComponent |
|  |  |  | WeaponComponent |
| **Launchers** |  | **1050000 - 1059999** | WeaponComponent |
| Generic Launcher | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1050000 | WeaponComponent |
| RPG7 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1050100 | WeaponComponent |
| RPG22 | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1050200 | WeaponComponent |
| M72 LAW | SightSoundInt / RHItemSoundInfo / BackItemSoundInfo | 1050300 | WeaponComponent |
