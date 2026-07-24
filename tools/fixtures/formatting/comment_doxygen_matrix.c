// Fixture truth: pending Workbench/compiler validation. This matrix is an
// extension research input, not a claim that every comment form attaches to a
// declaration or is accepted by Script Editor documentation tooling.

//! Leading line documentation.
//! \brief A class summary.
class RST_CommentMatrix
{
	//! Leading field documentation.
	int m_Value;

	int m_TrailingValue; //!< Trailing member documentation.

	/*!
	 * Block documentation with a \warning tag.
	 * \warning Preserve unknown prose and commands.
	 */
	protected string m_BlockValue;

	/**
	 * Alternative block form retained for Workbench validation.
	 * \note This is not currently a lexer documentation token.
	 */
	protected float m_AlternativeBlockValue;

	/// Ordinary triple-slash comment retained for Workbench validation.
	protected bool m_TripleSlashValue;

	//! \brief Value-return method.
	//! \param[in] value Input value.
	//! \param[out] result Output value.
	//! \return Whether a value was produced.
	bool TryGetValue(int value, out string result)
	{
		result = "";
		return true;
	}

	//! \brief Constructor documentation.
	//! \param[in,out] state Mutable state.
	void RST_CommentMatrix(inout int state)
	{
		state++;
	}

	/*!
	 * \defgroup RST_CommentMatrixGroup Comment matrix group
	 * \{
	 * \code
	 * RST_CommentMatrix matrix;
	 * \endcode
	 * \see RST_CommentMatrix
	 * \ref RST_CommentMatrix
	 * \}
	 */
	void UseDirectiveAndAttribute()
	{
		return;
	}
}

#ifdef DOXYGEN
//! [documentation-example]
class RST_DocumentationExample
{
}
//! [documentation-example]
#endif

//! Global documentation.
int RST_CommentMatrixGlobal;

//! Typedef documentation.
typedef int RST_CommentMatrixAlias;

//! Enum documentation.
enum ERST_CommentMatrix
{
	//! Enum value documentation.
	FIRST,
}
