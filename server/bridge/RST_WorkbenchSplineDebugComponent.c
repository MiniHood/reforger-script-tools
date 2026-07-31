#ifdef WORKBENCH
[ComponentEditorProps(category: "Reforger Script Tools/Debug", description: "Visualizes a SplineShapeEntity's native tessellation, anchors, tangent handles, and arc-length midpoint.")]

class RST_WorkbenchSplineDebugComponentClass : ScriptComponentClass
{
}

class RST_WorkbenchSplineDebugComponent : ScriptComponent
{
	[Attribute(defvalue: "0.05", desc: "Seconds between redraws.", params: "0.01 1", precision: 2, category: "Debug")]
	protected float m_fRefreshInterval;

	[Attribute(defvalue: "0.15", desc: "Radius of anchor markers in metres.", params: "0.01 2", precision: 2, category: "Debug")]
	protected float m_fAnchorRadius;

	[Attribute(defvalue: "1", desc: "Draw incoming and outgoing tangent handles.", category: "Debug")]
	protected bool m_bDrawTangents;

	[Attribute(defvalue: "1", desc: "Treat GetTangents values as offsets from their anchor. Disable to test them as local positions.", category: "Debug")]
	protected bool m_bTangentsAreOffsets;

	protected ref array<ref Shape> m_aDebugShapes = {};
	protected float m_fRefreshTimer;

	protected static const int CURVE_COLOR = 0xFF00FF00;
	protected static const int AUTO_ANCHOR_COLOR = 0xFFFFFFFF;
	protected static const int EXPLICIT_ANCHOR_COLOR = 0xFFFF8000;
	protected static const int IN_TANGENT_COLOR = 0xFFFFFF00;
	protected static const int OUT_TANGENT_COLOR = 0xFF00FFFF;
	protected static const int MIDPOINT_COLOR = 0xFFFF00FF;
	protected static const ShapeFlags DEBUG_SHAPE_FLAGS = ShapeFlags.NOZBUFFER | ShapeFlags.NOOUTLINE;

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);
		SetEventMask(owner, EntityEvent.FRAME);
		if (m_fRefreshInterval <= 0)
			m_fRefreshInterval = 0.05;
		m_fRefreshTimer = 0;
	}

	//------------------------------------------------------------------------------------------------
	override void EOnFrame(IEntity owner, float timeSlice)
	{
		m_fRefreshTimer -= timeSlice;
		if (m_fRefreshTimer > 0)
			return;
		m_fRefreshTimer = m_fRefreshInterval;
		DrawSpline(owner);
	}

	//------------------------------------------------------------------------------------------------
	protected void DrawSpline(IEntity owner)
	{
		ClearDebugShapes();

		SplineShapeEntity spline = SplineShapeEntity.Cast(owner);
		if (!spline)
			return;

		array<vector> localSamples = {};
		spline.GenerateTesselatedShape(localSamples);
		vector worldSamples[] = {};
		foreach (vector localSample : localSamples)
		{
			worldSamples.Insert(spline.CoordToParent(localSample));
		}

		if (worldSamples.Count() >= 2)
		{
			AddLineStrip(worldSamples, CURVE_COLOR);
			AddSphere(GetPathMidpoint(worldSamples), m_fAnchorRadius, MIDPOINT_COLOR);
		}

		array<vector> localAnchors = {};
		spline.GetPointsPositions(localAnchors);
		foreach (int i, vector localAnchor : localAnchors)
		{
			vector worldAnchor = spline.CoordToParent(localAnchor);
			int anchorColor = AUTO_ANCHOR_COLOR;
			if (spline.HasPointExplicitTangents(i))
				anchorColor = EXPLICIT_ANCHOR_COLOR;
			AddSphere(worldAnchor, m_fAnchorRadius, anchorColor);

			if (!m_bDrawTangents)
				continue;

			vector inTangent;
			vector outTangent;
			spline.GetTangents(i, inTangent, outTangent);
			AddTangent(spline, localAnchor, worldAnchor, inTangent, IN_TANGENT_COLOR);
			AddTangent(spline, localAnchor, worldAnchor, outTangent, OUT_TANGENT_COLOR);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void AddTangent(SplineShapeEntity spline, vector localAnchor, vector worldAnchor, vector tangent, int color)
	{
		vector localTangent = tangent;
		if (m_bTangentsAreOffsets)
			localTangent = localAnchor + tangent;

		vector worldTangent = spline.CoordToParent(localTangent);
		AddLine(worldAnchor, worldTangent, color);
		AddSphere(worldTangent, m_fAnchorRadius * 0.5, color);
	}

	//------------------------------------------------------------------------------------------------
	protected void AddLineStrip(vector points[], int color)
	{
		if (points.Count() < 2)
			return;

		Shape shape = Shape.CreateLines(color, DEBUG_SHAPE_FLAGS, points, points.Count());
		if (shape)
			m_aDebugShapes.Insert(shape);
	}

	//------------------------------------------------------------------------------------------------
	protected void AddLine(vector from, vector to, int color)
	{
		vector points[2];
		points[0] = from;
		points[1] = to;
		Shape shape = Shape.CreateLines(color, DEBUG_SHAPE_FLAGS, points, 2);
		if (shape)
			m_aDebugShapes.Insert(shape);
	}

	//------------------------------------------------------------------------------------------------
	protected void AddSphere(vector position, float radius, int color)
	{
		Shape shape = Shape.CreateSphere(color, DEBUG_SHAPE_FLAGS, position, radius);
		if (shape)
			m_aDebugShapes.Insert(shape);
	}

	//------------------------------------------------------------------------------------------------
	protected vector GetPathMidpoint(vector points[])
	{
		float totalLength;
		foreach (int i, vector point : points)
		{
			if (i == 0)
				continue;
			totalLength += Distance(points[i - 1], point);
		}

		float targetLength = totalLength * 0.5;
		float travelled;
		foreach (int i, vector point : points)
		{
			if (i == 0)
				continue;

			float segmentLength = Distance(points[i - 1], point);
			if (segmentLength <= 0.00001)
				continue;
			if (travelled + segmentLength >= targetLength)
			{
				float fraction = (targetLength - travelled) / segmentLength;
				return Interpolate(points[i - 1], point, fraction);
			}
			travelled += segmentLength;
		}

		return points[points.Count() - 1];
	}

	//------------------------------------------------------------------------------------------------
	protected float Distance(vector first, vector second)
	{
		vector delta = second - first;
		return Math.Sqrt(delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]);
	}

	//------------------------------------------------------------------------------------------------
	protected vector Interpolate(vector first, vector second, float fraction)
	{
		return Vector(
			first[0] + (second[0] - first[0]) * fraction,
			first[1] + (second[1] - first[1]) * fraction,
			first[2] + (second[2] - first[2]) * fraction
		);
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearDebugShapes()
	{
		m_aDebugShapes.Clear();
	}
}
#endif
