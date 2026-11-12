import Foundation

/// Geometry shared by usage cards, the gallery and installed widgets, in points.
struct UsageDotGrid {
    let columns: Int
    let rows: Int
    let stepX: Double
    let stepY: Double
    let radius: Double
    let filled: Int
    let percent: Double

    init(width: Double, height: Double, percent: Double) {
        columns = max(1, Int(width / 4))
        rows = max(1, Int(height / 4))
        stepX = width / Double(columns)
        stepY = height / Double(rows)
        radius = max(0, min(2.5, stepX, stepY) / 2)
        self.percent = percent.isFinite ? min(100, max(0, percent)) : 0
        filled = Int((Double(columns) * self.percent / 100).rounded())
    }
}
