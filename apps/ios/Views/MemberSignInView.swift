import CodyncKit
import CodyncUI
import SwiftUI

/// Signing in as a member of a computer (`docs/features/people.md`): the owner or an admin invites the
/// email, and the invitation is a QR code. Scanning it pairs this iPhone as that person, with only
/// their own bots, chats and connectors. There is no password: the code is single-use and the
/// session it opens can be ended from the computer's People screen.
struct MemberSignInView: View {
    @State private var scanning = false

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Sign in as a member")
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    Text(L10n.tr("A member of a computer uses it with their own bots, chats and connectors. The owner invites your email in Settings, People, and shows you the invitation."))
                        .font(.callout)
                        .foregroundStyle(Palette.secondary)
                    Text(L10n.tr("Scan the invitation code from that computer. It works once, and this phone then acts as you."))
                        .font(.callout)
                        .foregroundStyle(Palette.secondary)
                    Button {
                        scanning = true
                    } label: {
                        Label(L10n.tr("Scan the invitation"), systemImage: "qrcode.viewfinder")
                            .frame(maxWidth: .infinity, minHeight: 48)
                    }
                    .buttonStyle(.primary)
                }
                .padding(20)
            }
        }
        .background(Palette.background)
        .presentationDetents([.medium])
        .codyncSheet(isPresented: $scanning) { PairingView(inModal: true) }
    }
}
