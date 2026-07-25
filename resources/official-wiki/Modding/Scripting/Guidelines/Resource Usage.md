# [Resource Usage](https://community.bistudio.com/wiki/Arma_Reforger:Resource_Usage)

A [Resource](enfusion://ScriptEditor/scripts/Core/generated/Resources/Resource.c;24) object is a source of data through many classes ([BaseResourceObject](enfusion://ScriptEditor/scripts/Core/generated/Resources/BaseResourceObject.c;12) and its [BaseContainer](enfusion://ScriptEditor/scripts/Core/generated/Containers/BaseContainer.c;12), [IEntitySource](enfusion://ScriptEditor/scripts/Core/generated/Containers/IEntitySource.c;12) etc).
Due to how resource references are managed, some safeties are required.

ⓘ

See also [BaseContainer Usage](/wiki/Arma_Reforger:BaseContainer_Usage "Arma Reforger:BaseContainer Usage").

## Good Practice

Resources are managed by the engine, outside of script's reach.
If the script loses reference to a Resource object, the engine may dispose of it instantly or at some point in the future, deleting also its related BaseContainer/IEntitySource/etc objects (it is impossible to have a strong ref to a BaseContainer object in script).  
To avoid this, always keep a reference to the parent Resource.

### Examples

| Bad Example | Good Example |
| --- | --- |
| ENFORCECODEMARKER   ``` static BaseContainer GetBaseContainer(ResourceName resourceName) { 	// the only reference to said Resource is here and will be lost at the end of the scope 	Resource resource = Resource.Load(resourceName); 	if (!resource.IsValid()) 	return null; 	return resource.GetResource().ToBaseContainer(); 	// resource's reference is dropped here 	// the returned BaseContainer may become null at any time, 	// even with a script's reference } ``` | ENFORCECODEMARKER   ``` static BaseContainer GetBaseContainer(notnull Resource resource) { 	if (!resource.IsValid()) 	return null; 	return resource.GetResource().ToBaseContainer(); 	// resource's reference is managed by the method caller } ``` |
| ENFORCECODEMARKER   ``` // GetBaseContainer being the above method BaseContainer baseContainer = GetBaseContainer(m_sPrefab); string name = baseContainer.GetClassName(); // may result in a null pointer exception ``` | ENFORCECODEMARKER   ``` Resource resource = Resource.Load(m_sPrefab); BaseContainer baseContainer = GetBaseContainer(resource); string name = baseContainer.GetClassName(); // fine as long as a reference to resource is kept ``` |
| ENFORCECODEMARKER   ``` // beware of loops! array<BaseContainer> baseContainers = {}; // strong ref is not possible Resource resource; foreach (ResourceName resourceName : resourceNames) { 	resource = Resource.Load(resourceName); 	if (!resource.IsValid()) 	continue;   	baseContainers.Insert(resource.GetResource().ToBaseContainer()); 	// reference to resource is lost at each loop end } Process(baseContainers); // the array may contain nulls ``` | ENFORCECODEMARKER   ``` array<ref Resource> resources = {}; array<BaseContainer> baseContainers = {}; Resource resource; foreach (ResourceName resourceName : resourceNames) { 	resource = Resource.Load(resourceName); 	if (!resource.IsValid()) 	continue;  	resources.Insert(resource); 	baseContainers.Insert(resource.GetResource().ToBaseContainer()); 	// reference to resource is kept in the resources array } Process(baseContainers); // all references are kept resources = null; // this can now be done without issue ``` |
| N/A | ENFORCECODEMARKER   ``` // this is fine - a Managed instance is created, resource reference can be dropped // simplified, from SCR_BaseContainerTools.CreateInstanceFromPrefab() static Managed CreateInstanceFromPrefab(ResourceName prefab) { 	Resource resource = Resource.Load(prefab); 	if (!resource.IsValid()) 	return null; 	BaseContainer baseContainer = resource.GetResource().ToBaseContainer(); 	if (!baseContainer) 	return null; 	return BaseContainerTools.CreateInstanceFromContainer(baseContainer); 	// resource reference is dropped but the created instance will remain } ``` |
