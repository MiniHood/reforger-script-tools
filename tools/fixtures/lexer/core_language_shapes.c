// Fixture truth: game-data-derived from scripts/Core generated/proto shapes in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
class array<Class T>: Managed
{
	proto native external int Count();
};

class map<Class TKey,Class TValue>: Managed
{
	proto native external bool Find(TKey key, out TValue value);
};

class set<Class T>: Managed
{
	proto native int Count();
	proto int Copy(set<T> from);
	proto native void Swap(set<T> other);
	proto int Init(T init[]);
};

typedef array<float> TFloatArray;
typedef array<Class> TClassArray;
typedef array<ref Managed> TManagedRefArray;
typedef array<ResourceName> TResourceNameArray;
typedef set<string> TStringSet;
typedef set<Class> TClassSet;
typedef set<ref Managed> TManagedRefSet;
typedef int MapIterator;

class Tuple1<Class T1> extends Tuple
{
	T1 param1;
};

sealed class DebugTextWorldSpace
{
	private void ~DebugTextWorldSpace();
	proto external void SetTransform(vector mat[4]);
	static proto DebugTextWorldSpace CreateInWorld(BaseWorld world, string text, DebugTextFlags flags, vector transform[4], float size = 20.0, int color = 0xFFFFFFFF, int bgColor = 0x00000000, int priority = 1000);
};

class TraceParam
{
	owned string TraceMaterial;
	owned string ColliderName;
};

class IEntity : Managed
{
	event protected void EOnInit(IEntity owner);
	proto external volatile void SendEvent(notnull IEntity actor, EntityEvent e, void extra);
};

class PhysicsGeomDef
{
	vector Frame[4] = {Vector(1, 0, 0), Vector(0, 1, 0), Vector(0, 0, 1), Vector(0, 0, 0)};
	float m_fUV[4] = {0, 0, 1, 1};
};
