# Pair Android or iPhone with Windows

1. Install the Windows app from the fork’s release: https://github.com/Thndrr00/Codync/releases/latest/download/codync-windows-x64.exe. Open Codync so its host starts.
2. Right-click Codync in the Windows system tray and choose **Pair phone…**, or open **Settings → Computers** and use the QR button next to the computer.
3. On Android or iPhone, add a computer, select **Windows**, continue and scan the QR inside the Codync app. You can also copy the pairing link on Windows and paste it on the phone. The same QR supports both phone platforms.

Use matching variants: Codync pairs with Codync; Codync Dev pairs with Codync Dev. An Android debug build uses Codync Dev and will explain when it sees a production code. The code expires, works once and refreshes automatically. Generate a new one when linking another phone.

On the same Wi-Fi, the phone uses the direct encrypted channel. Windows Firewall must allow the host on the network used for pairing. With “Reach from anywhere” enabled, the encrypted relay provides another route; no forwarded router port is required. If pairing fails, check network access, the app variant and the code’s expiry, then retry or paste a fresh link.

The UX reference was Openstark’s `apps/mobile/src/features/auth/components/qr-scanner.tsx` and `screens/scan-qr-code-screen.tsx`: an in-app scanner, one-use QR and manual-link recovery. Codync keeps its existing v3 pairing protocol, keys and encrypted transport.
