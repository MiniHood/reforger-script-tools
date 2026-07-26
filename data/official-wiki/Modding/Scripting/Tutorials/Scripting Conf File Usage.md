# [Scripting Conf File Usage](https://community.bistudio.com/wiki/Arma_Reforger:Scripting_Conf_File_Usage)

Sometimes, a .conf file can be useful in order to store multiple settings/values that would otherwise be flooding a class' member variables.

Two main approaches exist to refer to a .conf file:

| [ResourceName Approach](#ResourceName_Approach) | [Object Approach](#Object_Approach) |
| --- | --- |
| * Checked a default value is possible * Unchecked Big setup * Unchecked No data preview | * Checked Quick setup * Checked Data preview and creation on the fly * Unchecked a default value is not possible |

The following examples assume a .conf file is to be used in a custom entity.

```enforce
[EntityEditorProps(category: "GameScripted/Misc")]
class SCR_ExampleEntityClass : GenericEntityClass
{
}
class SCR_ExampleEntity : GenericEntity
{
	// config will be used here
}
```

## ResourceName Approach

The simplest course of action is to target the wanted .conf file with a ResourceName path:

```enforce
class SCR_ExampleEntity : GenericEntity
{
	protected ResourceName m_sConfig = "{EAD206A79A774696}Configs/EntityCatalog/FactionLess/EntityCatalog_Characters.conf";
}
```

A proper way to make things easy is to have this property as an attribute:

```enforce
class SCR_ExampleEntity : GenericEntity
{
	[Attribute(defvalue: "{EAD206A79A774696}Configs/EntityCatalog/FactionLess/EntityCatalog_Characters.conf", params: "conf")]
	protected ResourceName m_sConfig;
}
```

At this point, *any* config file can be used. This is not what is wanted here. Hopefully, the Attribute can be made to target the specific config type we want:

```enforce
class SCR_ExampleEntity : GenericEntity
{
	[Attribute(defvalue: "{EAD206A79A774696}Configs/EntityCatalog/FactionLess/EntityCatalog_Characters.conf", params: "conf class=SCR_EntityCatalogMultiList")]
	protected ResourceName m_sConfig;
}
```

With this done, the entity now either has a *path* to a config, or an empty path. Remains here to *load* this config, cast it to make sure of its content - requiring a dedicated method:

```enforce
class SCR_ExampleEntity : GenericEntity
{
	[Attribute(params: "conf")]
	protected ResourceName m_sConfig = "{EAD206A79A774696}Configs/EntityCatalog/FactionLess/EntityCatalog_Characters.conf";
	SCR_EntityCatalogMultiList LoadConfig()
	{
		if (m_sConfig.IsEmpty())
		return null;
		Resource resource = Resource.Load(m_sConfig);
		if (!resource.IsValid())
		return null;
		return SCR_EntityCatalogMultiList.Cast(BaseContainerTools.CreateInstanceFromContainer(resource.GetResource().ToBaseContainer()));
	}
}
```

## Object Approach

This way is faster to setup, but disallows a default value:

```enforce
class SCR_ExampleEntity : GenericEntity
{
	[Attribute()]
	protected ref SCR_EntityCatalogMultiList m_Config;
}
```

This method makes the Config field void of any data, but allows to either click the "set class" button to **create** a config on the fly or to **drag and drop** a .conf file to fill the field instantly.
