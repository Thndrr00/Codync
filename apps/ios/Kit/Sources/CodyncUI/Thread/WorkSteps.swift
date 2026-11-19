import CodyncKit
import SwiftUI

/// One trace row, or a run of tool steps folded into a "worked on N steps" line.
enum WorkSlot {
    case row(Entry)
    case group([Entry])
}

/// Folds runs of at least `min` consecutive tool steps. Shorter runs stay visible row by row.
func groupWorkSteps(_ entries: [Entry], min: Int = 3) -> [WorkSlot] {
    var out: [WorkSlot] = []
    var run: [Entry] = []
    func flush() {
        if run.count >= min { out.append(.group(run)) }
        else { out.append(contentsOf: run.map { .row($0) }) }
        run.removeAll()
    }
    for entry in entries {
        if entry.kind == "tool" {
            run.append(entry)
        } else {
            flush()
            out.append(.row(entry))
        }
    }
    flush()
    return out
}

/// A folded group of tool steps: the count shows, the rows open on tap.
struct WorkStepsGroup: View {
    let entries: [Entry]
    @State private var expanded = false

    var body: some View {
        Disclosure(isExpanded: $expanded) {
            ForEach(entries) { TraceRow(entry: $0, unfolded: false) }
        } label: {
            Text(L10n.tr("Worked on {count} steps").replacingOccurrences(of: "{count}", with: String(entries.count)))
                .font(.subheadline)
                .foregroundStyle(Palette.secondary)
        }
    }
}
