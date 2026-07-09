// Fixture truth: game-data-derived exact class copied from Core/proto/Types.c in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
class array<Class T>: Managed
{
	/*!
	O(1) complexity.
	\return Number of elements of the array
	*/
	proto native int Count();

	/*!
	\return `true` if the array size is 0, `false` otherwise.
	*/
	proto native bool IsEmpty();

	/*!
	Destroys all elements of the array and sets the Count to 0.
	The underlying memory of the array is not freed.
	*/
	proto native void Clear();

	/*!
	Frees any underlying memory which is not used.
	For example if the array allocated enough memory for 100 items but only 1 is used
	(Count() is 1) this frees the memory taken by the remaining 99 items.
	\warning
	Memory allocation and deallocation are expensive so only use this function if you
	know you won't be adding new items to the array on frame-by-frame basis or when
	the memory consumption is of utmost importance.
	*/
	proto native void Compact();

	//! Sets `n`-th element to given `value`.
	proto void Set(int n, T value);

	/*!
	Tries to find the first occurrence of `value` in the array.
	\return Index of the first occurrence of `value` if found, -1 otherwise.
	\note This method has complexity O(n).
	*/
	proto int Find(T value);

	/*!
	Returns whether value is in array or not.
	\note This method has complexity O(n).
	*/
	proto bool Contains(T value);

	/*!
	\return Element at the index `n`.
	*/
	proto T Get(int n);

	/*!
	Inserts element at the end of array.
	\param value Element to be inserted.
	\return Position at which element is inserted.
	*/
	proto int Insert(T value);

	/*!
	Inserts element at certain position and moves all elements behind
	this position by one.
	\param value Element to be inserted.
	\param index Position at which element is inserted. Must be less than Count().
	\return Number of elements after insertion.
	*/
	proto int InsertAt(T value, int index);

	/*!
	Inserts all elements from array.
	\param from array from which all elements will be added.

	Example:
	\code
		TStringArray arr1 = new TStringArray;
		arr1.Insert( "Dave" );
		arr1.Insert( "Mark" );
		arr1.Insert( "John" );

		TStringArray arr2 = new TStringArray;
		arr2.Insert( "Sarah" );
		arr2.Insert( "Cate" );

		arr1.InsertAll(arr2);

		for ( int i = 0; i < arr1.Count(); i++ )
		{
			Print( arr1.Get(i) );
		}

		>> "Dave"
		>> "Mark"
		>> "John"
		>> "Sarah"
		>> "Cate"
	\endcode
	*/
	void InsertAll(notnull array<T> from)
	{
		for ( int i = 0; i < from.Count(); i++ )
		{
			Insert( from.Get(i) );
		}
	}

	/*!
	Removes element from array. The empty position is replaced by
	last element, so removal is quite fast but does not retain order.
	\param index Index of element to be removed
	*/
	proto native void Remove(int index);

	/*!
	Removes element from array, but retains all elements ordered. It's slower than Remove().
	\param index Index of element to be removed.
	*/
	proto native void RemoveOrdered(int index);

	/*!
	Resizes the array to given size.
	If the `newSize` is lower than current Count overflowing objects are destroyed.
	If the `newSize` is higher than current Count missing elements are initialized to zero (null).
	*/
	proto native void Resize(int newSize);

	/*!
	Reserve memory for given number of elements.
	This method is used for optimization purposes when the approximate size is known beforehand.
	*/
	proto native void Reserve(int newSize);

	/*!
	Swaps the contents of this and `other` arrays.
	Does not involve copying of the elements.
	*/
	proto native void Swap(notnull array<T> other);

	//! Sorts elements of array, depends on underlying type.
	proto native void Sort(bool reverse = false);

	//! Returns whether provided element index of array is valid.
	proto native bool IsIndexValid(int index);

	/*!
	Copies contents of `from` array to this array.
	\return Number of elements copied.
	*/
	proto int Copy(notnull array<T> from);

	proto int Init(T init[]);

	/*!
	Removes element from array. The empty position is replaced by
	last element, so removal is quite fast but does not retain order.
	\return true if item was removed
	*/
	proto bool RemoveItem(T value);

	/*!
	Removes element from array, but retain all elements ordered. It's slower than RemoveItem().
	\return true if item was removed
	*/
	proto bool RemoveItemOrdered(T value);

	/*!
	Print all elements in array.
	\code
		my_array.Debug();

		>> "One"
		>> "Two"
		>> "Three"
	\endcode
	*/
	void Debug()
	{
		PrintFormat( "Array count: %1", Count());

		for ( int i = 0; i < Count(); i++ )
		{
			T item = Get(i);

			PrintFormat("[%1] => %2", i, string.ToString(item));
		}
	}

	/*!
	Returns a random index of array. If Count is 0, returned index is -1.
	\return Random index of array.

	Example:
	\code
		Print( my_array.GetRandomIndex() );

		>> 2
	\endcode
	*/
	int GetRandomIndex()
	{
		if ( Count() > 0 )
		{
			return Math.RandomInt(0, Count());
		}

		return -1;
	}

	/*!
	Returns a random element of array.
	\return Random element of array.

	Example:
	\code
		Print( my_array.GetRandomElement() );

		>> "Three"
	\endcode
	*/
	T GetRandomElement()
	{
		return Get(GetRandomIndex());
	}

	void SwapItems(int item1_index, int item2_index)
	{
		T item1 = Get(item1_index);
		Set(item1_index, Get(item2_index));
		Set(item2_index, item1);
	}
}
