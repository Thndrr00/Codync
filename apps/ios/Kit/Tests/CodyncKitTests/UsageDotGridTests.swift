import Testing
@testable import CodyncKit

@Test func usageDotsKeepZeroEmptyAndClampInvalidReadings() {
    for value in [0.0, -3, .nan, .infinity, -.infinity] {
        #expect(UsageDotGrid(width: 60, height: 8, percent: value).filled == 0)
    }
    #expect(UsageDotGrid(width: 60, height: 8, percent: 125).filled == 15)
    #expect(UsageDotGrid(width: 60, height: 8, percent: 50).filled == 8)
}

@Test func usageDotsStayRoundAndInsideSmallContainers() {
    for (width, height) in [(0.0, 0.0), (1, 2), (28, 8), (103.5, 13)] {
        let grid = UsageDotGrid(width: width, height: height, percent: 62)
        #expect(grid.radius <= min(grid.stepX, grid.stepY) / 2)
        #expect((Double(grid.columns) - 0.5) * grid.stepX + grid.radius <= width)
        #expect((Double(grid.rows) - 0.5) * grid.stepY + grid.radius <= height)
    }
}
