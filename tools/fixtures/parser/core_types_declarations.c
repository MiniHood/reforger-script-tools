// Fixture truth: game-data-derived from Core/proto/Types.c and Core/tuple.c in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
/*!
\defgroup Types Types
*/
string String(string s)
{
	return s;
}

proto native vector Vector(float x, float y, float z);

class func
{
	private proto void SetInstance(Managed inst);
}

class array<Class T>: Managed
{
	proto native int Count();
	void InsertAll(notnull array<T> from)
	{
		for (int i = 0; i < from.Count(); i++)
		{
			Insert(from.Get(i));
		}
	}
	proto int Init(T init[]);
}

class map<Class TKey, Class TValue>: Managed
{
	proto bool Find(TKey key, out TValue val);
	proto int Copy(map<TKey,TValue> from);
}

class Tuple2<Class T1, Class T2> extends Tuple
{
	T1 param1;
	T2 param2;
	override bool Serialize(Serializer ctx)
	{
		return true;
	}
}

typedef array<string> TStringArray;
typedef map<ref Managed, ref Managed> TManagedRefManagedRefMap;
