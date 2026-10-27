<!--
App Store "What's New" for the iPhone app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per App Store language (zh-Hant, en-US),
listing what iPhone users notice, without emoji (the App Store rejects them; asc-submit.py drops
any left). tools/asc-submit.py reads the released version's section;
a version without one gets a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.26.0

### zh-Hant

- 配對電腦時可選擇 Windows，並查看安裝與掃描配對碼的步驟。
- 配對指引更清楚說明如何在兩台裝置使用相同的 Codync 或 Codync Dev 版本。
- 支援的 Antigravity 代理程式可回報各帳號與模型群組的方案用量。
- Bot 工作時，Gemini 與 OpenAI 的語音會主動告訴你它在做什麼、有沒有等你核准或遇到問題，做完就念出結果。
- 通話中短暫斷線再連上時，錯過的回覆會補念給你聽。
- Gemini 或 OpenAI 通話斷線時會自動重新連上並接著聊，不必重打；來電、Siri 或其他 App 佔用聲音時通話會暫停，結束後自動繼續，也可以點一下通話列下方繼續。
- 通話中，Bot 的即時動態會在鎖定畫面和動態島顯示通話時長和它正在做的事，還能直接靜音或結束通話，不用打開 App。
- Gemini 與 OpenAI 的語音用你正在說的語言回答，你換語言它就跟著換。

### en-US

- Choose Windows when pairing a computer and follow the installation and QR scanning steps.
- Clearer instructions for using matching Codync or Codync Dev apps on both devices when pairing.
- Supported Antigravity adapters report plan usage separately for each account and model group.
- While the bot works, the Gemini and OpenAI voice keeps you posted on what it's doing, tells you when an approval waits or something fails, and reads the result when it's done.
- A reply that arrives while a call briefly loses its connection is read once it's back.
- A Gemini or OpenAI call that loses its connection reconnects and carries on without hanging up; a phone call, Siri or another app's audio pauses the call, which picks up again when it ends, or with a tap under the call bar.
- During a call, the bot's Live Activity on the Lock Screen and in the Dynamic Island shows how long the call has run and what the bot is doing, with buttons to mute or end it without opening Codync.
- The Gemini and OpenAI voice answers in the language you speak and follows when you switch.

## 2.25.0

### zh-Hant

- 新增文件庫、Markdown 編輯與自動儲存，可審閱機器人提出的文件變更並查看電腦操作紀錄。
- 可依專案分組及篩選機器人，搜尋完整訊息紀錄，並預覽最近三則訊息而不標記已讀。
- 可在模型選單搜尋模型名稱與說明。
- 可為 Antigravity 機器人選擇獨立帳號。
- 機器人可透過安全卡片向你索取 API 金鑰或指令變數，值不會進入對話；可在「Credentials」中查看並移除。
- 改善遠端螢幕連線與例行工作通知的可靠性。
- 每台電腦都有一個主助理，固定在 Bot 列表最上面、帶著星星：它會主動跟進你在做的事，每天兩次看看有沒有該提醒你或能先幫你做的，需要時請你授權 Gmail、行事曆等服務；打電話給它時由它先開口。
- Bot 可以做 Artifact：報告、追蹤表、儀表板這類要回來看的內容會以卡片出現在聊天裡，點開看完整內容，直接在下方留言請它改，改好就自動更新。
- 每個 Bot 的「資料庫」收齊它做的 Artifact，以及聊天裡分享過的檔案和圖片。
- 可以建立使用 Hermes Agent 的 Bot。
- Bot 之間交辦工作不必等待，對方的回覆會回到發問的 Bot，再由它回覆你。
- 鎖定畫面的即時動態加上進度條，四格依任務進度填滿，顏色跟著狀態走。
- 帳號選單裡的「電腦與設定」和它打開的頁面同名。
- 新增聯結器失敗後再試一次，會使用你改好的金鑰。
- 使用 Hermes Agent 或 Gemini CLI 的 Bot，記憶頁會說明它們自己的記憶由所有用同一個 agent 的 Bot 共用，Codync 沒辦法關掉。
- 在設定的「對話」開啟「顯示思考細節」後，Bot 工作中點一下狀態膠囊，就能在對話裡展開這一輪的思考、工具與 MCP 呼叫，太長時在膠囊內捲動。
- 新增「Open Live Mode」捷徑，選擇 Bot 後直接開啟語音聊天；不選 Bot 就打給主助理，也可以對 Siri 說「Talk to Codync」。
- 新增「Send Text to Bot」捷徑，可選擇 Bot 並傳送文字；預設在背景傳送，也可開啟「Open Chat」顯示對話。
- 點對話上方的 Bot 會打開它的詳情、資料庫、記憶與設定；「新會話」改到 Bot 列表的長按選單。
- 用 Gemini 即時語音通話時，Gemini 聽得到你說話並會回答，也會念出 Bot 的回覆；先前通話可能一直沒有聲音、也沒有回應。
- 通話中在設定換了語音（例如從本機換成 Gemini），關掉設定後通話就改用新的語音繼續，不必掛斷重打。
- 語音通話連線中，通話列會顯示一條左右來回的線，真正連上、聽得到你說話後才換回音量點點。
- 有多台電腦時，Bot 列表中每台電腦的標題列都有自己的「擴充套件」按鈕，電腦的連線狀態也靠右對齊；擴充套件裡不再能切換電腦。
- 通話內容不再塞滿聊天：結束後只留一行，標題寫著這通電話在談什麼，點開可以看完整逐字稿；Bot 的記憶也會記住你在通話裡說的話。
- Gemini 與 OpenAI 的語音是 Bot 的轉接者：它知道你的記憶、Bot 的設定、專案現況和最近的對話，轉給 Bot 時附上你說的原話，也能停止 Bot、查 Bot 的記憶。

### en-US

- Keep Markdown documents with each bot, edit with autosave and conflict recovery, review proposed changes, and inspect computer activity.
- Filter bots, group them by project, search message history, and preview the last three messages without marking them read.
- Search model names and descriptions in the model picker.
- Choose a separate Antigravity account for each bot.
- Bots can ask for an API key or command variable on a secure card, and the value never enters the chat; review and remove them in Credentials.
- Improved remote screen connections and routine notification reliability.
- Every computer has a lead bot, starred on top of the Bots list: it follows up on what you're working on, checks in twice a day for anything worth a reminder or a head start, asks for access to services like Gmail or your calendar when it would help, and speaks first when you call it.
- Bots can make artifacts: reports, trackers and dashboards you come back to show as a card in the chat. Open one full screen and message the bot below it to change it; it updates in place.
- Each bot's Library collects its artifacts and the files and images shared in the chat.
- Create bots that run Hermes Agent.
- Bots hand work to each other without waiting; the answer comes back to the bot that asked, which then replies to you.
- Live Activities on the Lock Screen show a progress line that fills as the task moves, in the state's color.
- The account sheet's Computers & settings row now has the same name as the page it opens.
- Trying again after adding a connector failed uses the key you corrected.
- For bots that run Hermes Agent or Gemini CLI, Memory explains that the agent's own memory is shared by every bot that runs it and Codync can't turn it off.
- Turn on Show thinking details in Settings › Chat, then tap a working bot's status pill to unfold this turn's thinking, tool and MCP calls right in the chat; a long turn scrolls inside the pill.
- Use Open Live Mode in Shortcuts to choose a bot and start voice chat; leave the bot out to call your lead bot, or say "Talk to Codync" to Siri.
- Use Send Text to Bot to choose a bot and send text in the background by default, or enable Open Chat to show the conversation.
- Tapping the bot at the top of its chat opens its Details, Library, Memory and Settings; New session moved to the bot's touch-and-hold menu in the list.
- Gemini realtime voice calls hear you, answer and read the bot's replies aloud; before, a call could stay silent with no reply.
- Pick another voice in the call settings mid-call (on-device to Gemini, say) and the call carries on with it once you close them.
- While a voice call connects, the call bar shows a line gliding back and forth until the voice can really hear you.
- With several computers, each computer's heading in the Bots list has its own Extensions button, with its connection status on the right; Extensions no longer switches computers.
- Calls no longer fill the chat: each one leaves a single line titled with what it was about; tap it for the full transcript. The bot's memory keeps what you said on the call.
- The Gemini and OpenAI voice relays to your bot: it knows your memory, the bot's setup, the project and the recent conversation, passes your own words along, and can stop the bot or look up what it remembers.

## 2.24.0

### zh-Hant
- 設定與語音聊天的標題加入清楚的圖示。
- 可在設定中自行選擇英文、簡體中文、繁體中文、日語、韓語、法語、德語或西班牙語，也可跟隨系統語言。
- 語言偏好保留在這台裝置，聊天內容維持原文。

### en-US
- Identify Settings and Voice chat with clearer header icons.
- Choose English, Simplified Chinese, Traditional Chinese, Japanese, Korean, French, German or Spanish in Settings, or follow your system language.
- Keep your language preference on this device without translating your conversations.

## 2.23.1

### zh-Hant
- 點一下聊天訊息或空白區域即可收起鍵盤，保留正常捲動與按鈕操作
- 聊天停在底部時，鍵盤開啟後仍保持最後一則訊息可見
- 回到最新訊息按鈕改用系統 Liquid Glass

### en-US
- Tap a message or empty conversation space to dismiss the keyboard while preserving scrolling and button actions
- Keep the latest message visible when opening the keyboard at the bottom of a chat
- The jump-to-latest button now uses native Liquid Glass

## 2.23.0

### zh-Hant
- 外觀可選擇跟隨系統、淺色或深色，並保留此裝置的偏好

### en-US
- Choose System, Light or Dark appearance and keep your preference on this device

## 2.22.2

### zh-Hant
- 擴充功能移除 Apps 區段，Composio 統一從 Connectors 登入使用

### en-US
- Extensions removes the Apps section; connect Composio from Connectors

## 2.22.1

### zh-Hant
- 已連線電腦的資訊與操作排列更緊湊，電腦名稱完整顯示
- 手機連不上帳號內的電腦時，可直接要求存取權限；連線說明也更容易閱讀
- 手機的裝置清單不再提供刪除同帳號電腦的操作，避免誤刪

### en-US
- Connected computer cards keep their full names visible with a more compact layout for connection details and actions
- Ask for access directly when your phone cannot reach an account computer, with clearer connection details
- Account computers can no longer be removed from the phone's device list, preventing accidental removal

## 2.22.0

### zh-Hant
- 授權卡片在寬螢幕上更精簡，不再撐滿聊天區
- 用麥克風聽寫時，麥克風按鈕會變成錄音膠囊：停止鍵、錄了多久，還有跟著你說話起伏的點點
- Bot 工作中會顯示簡潔的動畫、目前動作與經過時間，詳細紀錄可從「完整對話」查看
- 麥克風旁的向上箭頭可以選聽寫要用裝置內建語音，還是用你自己的 OpenAI 或 Gemini 金鑰
- iPad 接實體鍵盤時，在輸入框按 Return 會換行，⌘↩ 送出
- 群組裡被提及的 bot 若正在別的對話忙，群組會顯示它在忙，忙完就回來回答
- 記憶不會再滿：快滿時，Dreaming 會把細節依主題整理進筆記資料夾，你要 bot 記住的事一定存得下

### en-US
- Approval cards stay compact on wide screens instead of stretching across the chat
- While you dictate, the mic turns into a recording pill: a stop button, how long you've been talking, and dots that follow your voice
- While a bot works, see its activity and elapsed time in a compact animated indicator; details remain in Full conversation
- The arrow beside the mic picks how dictation hears you: on-device speech, or your own OpenAI or Gemini key
- On iPad with a hardware keyboard, Return starts a new line in the message box and ⌘↩ sends
- In a group, a bot you mention that's busy in another chat now says so, and answers once it's done
- Memory no longer fills up: as it nears its limit, dreaming files the detail into notes by topic, so what you ask a bot to remember always fits
