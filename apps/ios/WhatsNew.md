<!--
App Store "What's New" for the iPhone app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per App Store language (zh-Hant, en-US),
listing what iPhone users notice. tools/asc-submit.py reads the released version's section;
a version without one gets a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.20.0

### zh-Hant

- 新增文件庫、Markdown 編輯與自動儲存，可審閱機器人提出的文件變更並查看電腦操作紀錄。
- 可依專案分組及篩選機器人，搜尋完整訊息紀錄，並預覽最近三則訊息而不標記已讀。
- 可在模型選單搜尋模型名稱與說明。
- 可為 Antigravity 機器人選擇獨立帳號。
- 機器人可透過安全卡片向你索取 API 金鑰或指令變數，值不會進入對話；可在「Credentials」中查看並移除。
- 改善遠端螢幕連線與例行工作通知的可靠性。

### en-US

- Keep Markdown documents with each bot, edit with autosave and conflict recovery, review proposed changes, and inspect computer activity.
- Filter bots, group them by project, search message history, and preview the last three messages without marking them read.
- Search model names and descriptions in the model picker.
- Choose a separate Antigravity account for each bot.
- Bots can ask for an API key or command variable on a secure card, and the value never enters the chat; review and remove them in Credentials.
- Improved remote screen connections and routine notification reliability.

## 2.19.1

### zh-Hant

- 修正語音通話中麥克風暫時無法使用時（例如換了耳機或被來電打斷），App 會閃退的問題。
- 修正和電腦的直連中斷時，App 偶爾會閃退的問題。

### en-US

- Fixed the app closing during a voice call when the microphone was briefly unavailable, such as after switching headphones or an incoming call.
- Fixed an occasional crash when the direct connection to your computer dropped.
