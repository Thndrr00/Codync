// Adds the member-account phrases to every language table. Run from the repository root:
//
//   node tools/add-member-phrases.mjs
//
// The tables are the iPhone's phrases (`apps/ios/.../Resources/Languages/*.json`), which the desktop
// app and the terminal UI read too. Each file is re-read immediately before it is written: another
// agent may be editing the same table. A key that is already present is left alone, and the new keys
// are inserted at their sorted position so the diff stays ordered.
import { readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const phrases = {
  'Invite a member': {
    de: 'Mitglied einladen', es: 'Invitar a un miembro', fr: 'Inviter un membre', it: 'Invita un membro',
    ja: 'メンバーを招待', ko: '멤버 초대', 'pt-BR': 'Convidar um membro', ru: 'Пригласить участника',
    ar: 'دعوة عضو', he: 'הזמנת חבר', 'zh-Hans': '邀请成员', 'zh-Hant': '邀請成員',
  },
  'An invitation is a one-time code: your phone scans it, or this app signs in with it at this computer. The member acts as that person, with only their own bots, chats and connectors.': {
    de: 'Eine Einladung ist ein einmaliger Code: Dein Telefon scannt ihn, oder diese App meldet sich damit an diesem Computer an. Das Mitglied handelt als diese Person, nur mit eigenen Bots, Chats und Konnektoren.',
    es: 'Una invitación es un código de un solo uso: tu teléfono lo escanea, o esta app inicia sesión con él en este ordenador. El miembro actúa como esa persona, solo con sus bots, chats y conectores.',
    fr: "Une invitation est un code à usage unique : votre téléphone le scanne, ou cette app se connecte avec lui sur cet ordinateur. Le membre agit en tant que cette personne, avec seulement ses bots, chats et connecteurs.",
    it: 'Un invito è un codice monouso: il tuo telefono lo scansiona, oppure questa app accede con esso su questo computer. Il membro agisce come quella persona, con solo i propri bot, chat e connettori.',
    ja: '招待は一度だけ使えるコードです。スマートフォンで読み取るか、このアプリでこのコンピュータにサインインします。メンバーはその人として、自分のボット・チャット・コネクタだけを使えます。',
    ko: '초대는 한 번만 쓸 수 있는 코드입니다. 휴대폰으로 스캔하거나 이 앱에서 이 컴퓨터에 로그인하세요. 멤버는 그 사람으로서 자신의 봇, 채팅, 커넥터만 사용합니다.',
    'pt-BR': 'Um convite é um código de uso único: seu telefone o escaneia, ou este app entra com ele neste computador. O membro age como essa pessoa, apenas com seus bots, chats e conectores.',
    ru: 'Приглашение — одноразовый код: телефон сканирует его, или это приложение входит с ним на этом компьютере. Участник действует как этот человек и видит только своих ботов, чаты и коннекторы.',
    ar: 'الدعوة رمز يُستخدم مرة واحدة: يمسحه هاتفك، أو تسجّل هذه التطبيقات الدخول به على هذا الحاسوب. يعمل العضو بصفة ذلك الشخص، ببوتاته ومحادثاته وموصّلاته فقط.',
    he: 'הזמנה היא קוד חד־פעמי: הטלפון סורק אותו, או שהאפליקציה נכנסת איתו במחשב הזה. החבר פועל בתור אותו אדם, עם הבוטים, השיחות והמחברים שלו בלבד.',
    'zh-Hans': '邀请是一次性代码：手机扫描它，或此应用在本机用它登录。成员以该成员身份行事，只能使用自己的智能体、聊天和连接器。',
    'zh-Hant': '邀請是一次性代碼：手機掃描它，或此應用程式在本機用它登入。成員以該成員身分行事，只能使用自己的代理、聊天和連接器。',
  },
  'Name (optional)': {
    de: 'Name (optional)', es: 'Nombre (opcional)', fr: 'Nom (facultatif)', it: 'Nome (facoltativo)',
    ja: '名前（任意）', ko: '이름(선택)', 'pt-BR': 'Nome (opcional)', ru: 'Имя (необязательно)',
    ar: 'الاسم (اختياري)', he: 'שם (אופציונלי)', 'zh-Hans': '名称（可选）', 'zh-Hant': '名稱（可選）',
  },
  Email: {
    de: 'E-Mail', es: 'Correo electrónico', fr: 'E-mail', it: 'Email', ja: 'メール', ko: '이메일',
    'pt-BR': 'E-mail', ru: 'Эл. почта', ar: 'البريد الإلكتروني', he: 'דוא״ל', 'zh-Hans': '电子邮件', 'zh-Hant': '電子郵件',
  },
  'Inviting…': {
    de: 'Einladen…', es: 'Invitando…', fr: 'Invitation…', it: 'Invito…', ja: '招待中…', ko: '초대 중…',
    'pt-BR': 'Convidando…', ru: 'Приглашаем…', ar: 'جارٍ الدعوة…', he: 'מזמין…', 'zh-Hans': '正在邀请…', 'zh-Hant': '正在邀請…',
  },
  Invite: {
    de: 'Einladen', es: 'Invitar', fr: 'Inviter', it: 'Invita', ja: '招待', ko: '초대',
    'pt-BR': 'Convidar', ru: 'Пригласить', ar: 'دعوة', he: 'הזמן', 'zh-Hans': '邀请', 'zh-Hant': '邀請',
  },
  'Invitation for {name}': {
    de: 'Einladung für {name}', es: 'Invitación para {name}', fr: 'Invitation pour {name}', it: 'Invito per {name}',
    ja: '{name} への招待', ko: '{name}님 초대', 'pt-BR': 'Convite para {name}', ru: 'Приглашение для {name}',
    ar: 'دعوة لـ {name}', he: 'הזמנה עבור {name}', 'zh-Hans': '{name} 的邀请', 'zh-Hant': '{name} 的邀請',
  },
  'Copy invitation link': {
    de: 'Einladungslink kopieren', es: 'Copiar enlace de invitación', fr: "Copier le lien d'invitation",
    it: 'Copia link di invito', ja: '招待リンクをコピー', ko: '초대 링크 복사', 'pt-BR': 'Copiar link do convite',
    ru: 'Копировать ссылку', ar: 'نسخ رابط الدعوة', he: 'העתקת קישור ההזמנה', 'zh-Hans': '复制邀请链接', 'zh-Hant': '複製邀請連結',
  },
  'Copy code': {
    de: 'Code kopieren', es: 'Copiar código', fr: 'Copier le code', it: 'Copia codice', ja: 'コードをコピー',
    ko: '코드 복사', 'pt-BR': 'Copiar código', ru: 'Копировать код', ar: 'نسخ الرمز', he: 'העתקת הקוד',
    'zh-Hans': '复制代码', 'zh-Hant': '複製代碼',
  },
  'The code works once, until {date}.': {
    de: 'Der Code gilt einmal, bis {date}.', es: 'El código funciona una vez, hasta {date}.',
    fr: 'Le code fonctionne une fois, jusqu’au {date}.', it: 'Il codice funziona una volta, fino al {date}.',
    ja: 'コードは {date} まで一度だけ使えます。', ko: '코드는 {date}까지 한 번만 사용할 수 있습니다.',
    'pt-BR': 'O código funciona uma vez, até {date}.', ru: 'Код действует один раз, до {date}.',
    ar: 'يعمل الرمز مرة واحدة حتى {date}.', he: 'הקוד תקף פעם אחת, עד {date}.',
    'zh-Hans': '代码在 {date} 前有效，且只能使用一次。', 'zh-Hant': '代碼在 {date} 前有效，且只能使用一次。',
  },
  'Sign in as a member on this computer': {
    de: 'An diesem Computer als Mitglied anmelden', es: 'Iniciar sesión como miembro en este ordenador',
    fr: 'Se connecter en tant que membre sur cet ordinateur', it: 'Accedi come membro su questo computer',
    ja: 'このコンピュータにメンバーとしてサインイン', ko: '이 컴퓨터에 멤버로 로그인',
    'pt-BR': 'Entrar como membro neste computador', ru: 'Войти как участник на этом компьютере',
    ar: 'تسجيل الدخول كعضو على هذا الحاسوب', he: 'כניסה כחבר במחשב הזה',
    'zh-Hans': '在此电脑上以成员身份登录', 'zh-Hant': '在此電腦上以成員身分登入',
  },
  'Member sessions': {
    de: 'Mitgliedssitzungen', es: 'Sesiones de miembros', fr: 'Sessions des membres', it: 'Sessioni dei membri',
    ja: 'メンバーのセッション', ko: '멤버 세션', 'pt-BR': 'Sessões de membros', ru: 'Сеансы участников',
    ar: 'جلسات الأعضاء', he: 'הפעלות של חברים', 'zh-Hans': '成员会话', 'zh-Hant': '成員工作階段',
  },
  'A session is a paired phone or a signed-in app. Ending one revokes that device at once; the person and their bots stay.': {
    de: 'Eine Sitzung ist ein gekoppeltes Telefon oder eine angemeldete App. Beendest du sie, verliert das Gerät sofort den Zugang; Person und Bots bleiben.',
    es: 'Una sesión es un teléfono vinculado o una app con sesión iniciada. Terminarla revoca ese dispositivo al instante; la persona y sus bots permanecen.',
    fr: "Une session est un téléphone jumelé ou une app connectée. La terminer révoque cet appareil immédiatement ; la personne et ses bots restent.",
    it: 'Una sessione è un telefono abbinato o un’app con accesso. Terminarla revoca subito quel dispositivo; la persona e i suoi bot restano.',
    ja: 'セッションはペアリングしたスマートフォン、またはサインイン中のアプリです。終了するとその端末のアクセスが即座に取り消され、本人とボットは残ります。',
    ko: '세션은 페어링된 휴대폰 또는 로그인한 앱입니다. 세션을 끝내면 해당 기기의 접근이 즉시 취소되며, 사람과 봇은 남습니다.',
    'pt-BR': 'Uma sessão é um telefone pareado ou um app conectado. Encerrá-la revoga esse dispositivo na hora; a pessoa e seus bots permanecem.',
    ru: 'Сеанс — это сопряжённый телефон или приложение с выполненным входом. Завершение сразу отзывает доступ устройства; человек и его боты остаются.',
    ar: 'الجلسة هاتف مقترن أو تطبيق مسجّل الدخول. إنهاؤها يلغي وصول ذلك الجهاز فورًا؛ ويبقى الشخص وبوتاته.',
    he: 'הפעלה היא טלפון מותאם או אפליקציה מחוברת. סיומה מבטל מיד את הגישה של המכשיר; האדם והבוטים נשארים.',
    'zh-Hans': '会话是已配对的手机或已登录的应用。结束会话会立即撤销该设备的访问权限，成员及其智能体仍会保留。',
    'zh-Hant': '工作階段是已配對的手機或已登入的應用程式。結束工作階段會立即撤銷該裝置的存取權，成員及其代理仍會保留。',
  },
  Invitation: {
    de: 'Einladung', es: 'Invitación', fr: 'Invitation', it: 'Invito', ja: '招待', ko: '초대',
    'pt-BR': 'Convite', ru: 'Приглашение', ar: 'دعوة', he: 'הזמנה', 'zh-Hans': '邀请', 'zh-Hant': '邀請',
  },
  Session: {
    de: 'Sitzung', es: 'Sesión', fr: 'Session', it: 'Sessione', ja: 'セッション', ko: '세션',
    'pt-BR': 'Sessão', ru: 'Сеанс', ar: 'جلسة', he: 'הפעלה', 'zh-Hans': '会话', 'zh-Hant': '工作階段',
  },
  Pending: {
    de: 'Ausstehend', es: 'Pendiente', fr: 'En attente', it: 'In attesa', ja: '保留中', ko: '대기 중',
    'pt-BR': 'Pendente', ru: 'Ожидает', ar: 'قيد الانتظار', he: 'ממתין', 'zh-Hans': '待处理', 'zh-Hant': '待處理',
  },
  Connected: {
    de: 'Verbunden', es: 'Conectado', fr: 'Connecté', it: 'Connesso', ja: '接続済み', ko: '연결됨',
    'pt-BR': 'Conectado', ru: 'Подключено', ar: 'متصل', he: 'מחובר', 'zh-Hans': '已连接', 'zh-Hant': '已連接',
  },
  'Signed out': {
    de: 'Abgemeldet', es: 'Sin sesión', fr: 'Déconnecté', it: 'Disconnesso', ja: 'サインアウト済み', ko: '로그아웃됨',
    'pt-BR': 'Desconectado', ru: 'Выход выполнен', ar: 'مسجّل الخروج', he: 'מנותק', 'zh-Hans': '已退出', 'zh-Hant': '已登出',
  },
  'End session': {
    de: 'Sitzung beenden', es: 'Terminar sesión', fr: 'Terminer la session', it: 'Termina sessione',
    ja: 'セッションを終了', ko: '세션 종료', 'pt-BR': 'Encerrar sessão', ru: 'Завершить сеанс',
    ar: 'إنهاء الجلسة', he: 'סיום הפעלה', 'zh-Hans': '结束会话', 'zh-Hant': '結束工作階段',
  },
  'Sign in as a member': {
    de: 'Als Mitglied anmelden', es: 'Iniciar sesión como miembro', fr: 'Se connecter en tant que membre',
    it: 'Accedi come membro', ja: 'メンバーとしてサインイン', ko: '멤버로 로그인',
    'pt-BR': 'Entrar como membro', ru: 'Войти как участник', ar: 'تسجيل الدخول كعضو', he: 'כניסה כחבר',
    'zh-Hans': '以成员身份登录', 'zh-Hant': '以成員身分登入',
  },
  'Admin on this computer': {
    de: 'Admin an diesem Computer', es: 'Administrador en este ordenador', fr: 'Admin sur cet ordinateur',
    it: 'Admin su questo computer', ja: 'このコンピュータの管理者', ko: '이 컴퓨터의 관리자',
    'pt-BR': 'Administrador neste computador', ru: 'Администратор на этом компьютере',
    ar: 'مسؤول على هذا الحاسوب', he: 'מנהל במחשב הזה', 'zh-Hans': '此电脑的管理员', 'zh-Hant': '此電腦的管理員',
  },
  'Member on this computer': {
    de: 'Mitglied an diesem Computer', es: 'Miembro en este ordenador', fr: 'Membre sur cet ordinateur',
    it: 'Membro su questo computer', ja: 'このコンピュータのメンバー', ko: '이 컴퓨터의 멤버',
    'pt-BR': 'Membro neste computador', ru: 'Участник на этом компьютере',
    ar: 'عضو على هذا الحاسوب', he: 'חבר במחשב הזה', 'zh-Hans': '此电脑的成员', 'zh-Hant': '此電腦的成員',
  },
  'Connectors you share or own': {
    de: 'Konnektoren, die du teilst oder besitzt', es: 'Conectores que compartes o posees',
    fr: 'Connecteurs que vous partagez ou possédez', it: 'Connettori che condividi o possiedi',
    ja: '共有または所有するコネクタ', ko: '공유하거나 소유한 커넥터', 'pt-BR': 'Conectores que você compartilha ou possui',
    ru: 'Общие и собственные коннекторы', ar: 'الموصّلات التي تشاركها أو تملكها', he: 'מחברים שאתה משתף או מחזיק',
    'zh-Hans': '你共享或拥有的连接器', 'zh-Hant': '你共用或擁有的連接器',
  },
  'Your bots, chats and connectors open in Codync as you. The owner can end this session in People.': {
    de: 'Deine Bots, Chats und Konnektoren öffnen sich in Codync als du. Der Eigentümer kann diese Sitzung in Personen beenden.',
    es: 'Tus bots, chats y conectores se abren en Codync como tú. El propietario puede terminar esta sesión en Personas.',
    fr: 'Vos bots, chats et connecteurs s’ouvrent dans Codync en votre nom. Le propriétaire peut terminer cette session dans Personnes.',
    it: 'I tuoi bot, chat e connettori si aprono in Codync come te. Il proprietario può terminare questa sessione in Persone.',
    ja: 'ボット、チャット、コネクタはあなたとして Codync で開きます。所有者は「ユーザー」でこのセッションを終了できます。',
    ko: '봇, 채팅, 커넥터가 나로서 Codync에서 열립니다. 소유자는 사용자에서 이 세션을 종료할 수 있습니다.',
    'pt-BR': 'Seus bots, chats e conectores abrem no Codync como você. O proprietário pode encerrar esta sessão em Pessoas.',
    ru: 'Ваши боты, чаты и коннекторы открываются в Codync от вашего имени. Владелец может завершить этот сеанс в разделе «Люди».',
    ar: 'تُفتح بوتاتك ومحادثاتك وموصّلاتك في Codync باسمك. يمكن للمالك إنهاء هذه الجلسة من الأشخاص.',
    he: 'הבוטים, השיחות והמחברים שלך נפתחים ב-Codync בתור אתה. הבעלים יכול לסיים את ההפעלה ב״אנשים״.',
    'zh-Hans': '你的智能体、聊天和连接器会以你的身份在 Codync 中打开。所有者可以在“人员”中结束此会话。',
    'zh-Hant': '你的代理、聊天和連接器會以你的身分在 Codync 中開啟。所有者可以在「人員」中結束此工作階段。',
  },
  'Sign out': {
    de: 'Abmelden', es: 'Cerrar sesión', fr: 'Se déconnecter', it: 'Esci', ja: 'サインアウト', ko: '로그아웃',
    'pt-BR': 'Sair', ru: 'Выйти', ar: 'تسجيل الخروج', he: 'יציאה', 'zh-Hans': '退出登录', 'zh-Hant': '登出',
  },
  'Paste the invitation code the owner gave you. It works once, and this app then acts as you.': {
    de: 'Füge den Einladungscode ein, den der Eigentümer dir gegeben hat. Er gilt einmal, danach handelt diese App als du.',
    es: 'Pega el código de invitación que te dio el propietario. Funciona una vez y luego esta app actúa como tú.',
    fr: 'Collez le code d’invitation que le propriétaire vous a donné. Il fonctionne une fois, puis cette app agit en votre nom.',
    it: 'Incolla il codice di invito che ti ha dato il proprietario. Funziona una volta, poi questa app agisce come te.',
    ja: '所有者から渡された招待コードを貼り付けてください。一度だけ使え、その後このアプリはあなたとして動作します。',
    ko: '소유자가 준 초대 코드를 붙여넣으세요. 한 번만 사용할 수 있으며, 이후 이 앱은 나로서 동작합니다.',
    'pt-BR': 'Cole o código de convite que o proprietário deu a você. Ele funciona uma vez, e então este app age como você.',
    ru: 'Вставьте код приглашения от владельца. Он действует один раз, после чего приложение действует от вашего имени.',
    ar: 'ألصق رمز الدعوة الذي أعطاك إياه المالك. يعمل مرة واحدة، ثم يعمل هذا التطبيق باسمك.',
    he: 'הדבק את קוד ההזמנה שקיבלת מהבעלים. הוא תקף פעם אחת, ואז האפליקציה פועלת בתור אתה.',
    'zh-Hans': '粘贴所有者给你的邀请代码。它只能使用一次，之后此应用会以你的身份行事。',
    'zh-Hant': '貼上所有者給你的邀請代碼。它只能使用一次，之後此應用程式會以你的身分行事。',
  },
  'Invitation code': {
    de: 'Einladungscode', es: 'Código de invitación', fr: 'Code d’invitation', it: 'Codice di invito',
    ja: '招待コード', ko: '초대 코드', 'pt-BR': 'Código do convite', ru: 'Код приглашения',
    ar: 'رمز الدعوة', he: 'קוד הזמנה', 'zh-Hans': '邀请代码', 'zh-Hant': '邀請代碼',
  },
  'Paste your code': {
    de: 'Code einfügen', es: 'Pega tu código', fr: 'Collez votre code', it: 'Incolla il codice',
    ja: 'コードを貼り付け', ko: '코드 붙여넣기', 'pt-BR': 'Cole seu código', ru: 'Вставьте код',
    ar: 'ألصق رمزك', he: 'הדבק את הקוד', 'zh-Hans': '粘贴你的代码', 'zh-Hant': '貼上你的代碼',
  },
  'Signing in…': {
    de: 'Anmelden…', es: 'Iniciando sesión…', fr: 'Connexion…', it: 'Accesso…', ja: 'サインイン中…',
    ko: '로그인 중…', 'pt-BR': 'Entrando…', ru: 'Вход…', ar: 'جارٍ تسجيل الدخول…', he: 'מתחבר…',
    'zh-Hans': '正在登录…', 'zh-Hant': '正在登入…',
  },
  'Sign in': {
    de: 'Anmelden', es: 'Iniciar sesión', fr: 'Se connecter', it: 'Accedi', ja: 'サインイン', ko: '로그인',
    'pt-BR': 'Entrar', ru: 'Войти', ar: 'تسجيل الدخول', he: 'כניסה', 'zh-Hans': '登录', 'zh-Hant': '登入',
  },
  'Or sign in with your company email, when the owner set up company sign-in.': {
    de: 'Oder melde dich mit deiner Firmen-E-Mail an, wenn der Eigentümer die Firmenanmeldung eingerichtet hat.',
    es: 'O inicia sesión con tu correo de empresa, si el propietario configuró el acceso de la empresa.',
    fr: 'Ou connectez-vous avec votre e-mail professionnel, si le propriétaire a configuré la connexion d’entreprise.',
    it: 'Oppure accedi con la tua email aziendale, se il proprietario ha configurato l’accesso aziendale.',
    ja: '所有者が会社のサインインを設定している場合は、会社のメールでもサインインできます。',
    ko: '소유자가 회사 로그인을 설정한 경우 회사 이메일로도 로그인할 수 있습니다.',
    'pt-BR': 'Ou entre com seu e-mail da empresa, se o proprietário configurou o acesso da empresa.',
    ru: 'Или войдите с рабочей почтой, если владелец настроил вход через компанию.',
    ar: 'أو سجّل الدخول ببريد شركتك، إن أعدّ المالك تسجيل دخول الشركة.',
    he: 'או היכנס עם כתובת הדוא״ל הארגונית שלך, אם הבעלים הגדיר כניסה ארגונית.',
    'zh-Hans': '如果所有者设置了公司登录，也可以用公司邮箱登录。',
    'zh-Hant': '如果所有者設定了公司登入，也可以用公司電子郵件登入。',
  },
  'Company email': {
    de: 'Firmen-E-Mail', es: 'Correo de empresa', fr: 'E-mail professionnel', it: 'Email aziendale',
    ja: '会社のメール', ko: '회사 이메일', 'pt-BR': 'E-mail da empresa', ru: 'Рабочая почта',
    ar: 'بريد الشركة', he: 'דוא״ל ארגוני', 'zh-Hans': '公司邮箱', 'zh-Hant': '公司電子郵件',
  },
  'Continue with email': {
    de: 'Mit E-Mail fortfahren', es: 'Continuar con el correo', fr: 'Continuer avec l’e-mail',
    it: 'Continua con l’email', ja: 'メールで続ける', ko: '이메일로 계속', 'pt-BR': 'Continuar com e-mail',
    ru: 'Продолжить с почтой', ar: 'المتابعة بالبريد', he: 'המשך עם דוא״ל', 'zh-Hans': '继续使用邮箱', 'zh-Hant': '繼續使用電子郵件',
  },
  'Codync desktop': {
    de: 'Codync Desktop', es: 'Codync de escritorio', fr: 'Codync bureau', it: 'Codync desktop',
    ja: 'Codync デスクトップ', ko: 'Codync 데스크톱', 'pt-BR': 'Codync desktop', ru: 'Codync для компьютера',
    ar: 'Codync لسطح المكتب', he: 'Codync למחשב', 'zh-Hans': 'Codync 桌面版', 'zh-Hant': 'Codync 桌面版',
  },
  'People with an email at these domains sign in through the company and become the member that email was invited as. SCIM writes to People: a created user is a member, and deleting one removes them like Settings does.': {
    de: 'Personen mit einer E-Mail dieser Domains melden sich über die Firma an und werden das Mitglied, als das diese E-Mail eingeladen wurde. SCIM schreibt in Personen: ein angelegter Benutzer ist ein Mitglied, und Löschen entfernt es wie die Einstellungen.',
    es: 'Las personas con un correo de estos dominios inician sesión con la empresa y se convierten en el miembro al que se invitó ese correo. SCIM escribe en Personas: crear un usuario crea un miembro, y borrarlo lo elimina como en Ajustes.',
    fr: 'Les personnes dont l’e-mail appartient à ces domaines se connectent via l’entreprise et deviennent le membre invité avec cet e-mail. SCIM écrit dans Personnes : un utilisateur créé devient membre, et le supprimer le retire comme dans Réglages.',
    it: 'Le persone con un’email di questi domini accedono tramite l’azienda e diventano il membro invitato con quell’email. SCIM scrive in Persone: creare un utente crea un membro, eliminarlo lo rimuove come nelle Impostazioni.',
    ja: 'これらのドメインのメールを持つ人は会社経由でサインインし、そのメールで招待されたメンバーになります。SCIM は「ユーザー」に書き込みます。作成したユーザーはメンバーになり、削除すると設定と同様に削除されます。',
    ko: '이 도메인의 이메일을 가진 사람은 회사 계정으로 로그인해 그 이메일로 초대된 멤버가 됩니다. SCIM은 사용자에 기록하며, 만든 사용자는 멤버가 되고 삭제하면 설정과 같이 제거됩니다.',
    'pt-BR': 'Pessoas com um e-mail desses domínios entram pela empresa e se tornam o membro convidado com esse e-mail. O SCIM grava em Pessoas: criar um usuário cria um membro, e excluí-lo o remove como nos Ajustes.',
    ru: 'Люди с почтой на этих доменах входят через компанию и становятся участником, приглашённым на этот адрес. SCIM пишет в «Люди»: созданный пользователь — участник, а удаление убирает его, как в настройках.',
    ar: 'يسجّل من يحمل بريدًا على هذه النطاقات الدخول عبر الشركة ويصبح العضو المدعوّ بهذا البريد. يكتب SCIM في الأشخاص: إنشاء مستخدم ينشئ عضوًا، وحذفه يزيله كما في الإعدادات.',
    he: 'אנשים עם דוא״ל בדומיינים האלה נכנסים דרך החברה והופכים לחבר שהוזמן עם כתובת זו. SCIM כותב ל״אנשים״: יצירת משתמש יוצרת חבר, ומחיקתו מסירה אותו כמו בהגדרות.',
    'zh-Hans': '这些域名的邮箱通过公司登录，并成为该邮箱被邀请的成员。SCIM 写入“人员”：创建用户即创建成员，删除用户会像设置中一样将其移除。',
    'zh-Hant': '這些網域的電子郵件透過公司登入，並成為該電子郵件被邀請的成員。SCIM 寫入「人員」：建立使用者即建立成員，刪除使用者會像設定中一樣將其移除。',
  },
  'A member of a computer uses it with their own bots, chats and connectors. The owner invites your email in Settings, People, and shows you the invitation.': {
    de: 'Ein Mitglied eines Computers nutzt ihn mit eigenen Bots, Chats und Konnektoren. Der Eigentümer lädt deine E-Mail in Einstellungen, Personen ein und zeigt dir die Einladung.',
    es: 'Un miembro de un ordenador lo usa con sus propios bots, chats y conectores. El propietario invita tu correo en Ajustes, Personas, y te muestra la invitación.',
    fr: 'Un membre d’un ordinateur l’utilise avec ses propres bots, chats et connecteurs. Le propriétaire invite votre e-mail dans Réglages, Personnes, et vous montre l’invitation.',
    it: 'Un membro di un computer lo usa con i propri bot, chat e connettori. Il proprietario invita la tua email in Impostazioni, Persone, e ti mostra l’invito.',
    ja: 'コンピュータのメンバーは、自分のボット、チャット、コネクタでそれを使います。所有者が「設定」の「ユーザー」でメールを招待し、招待を表示します。',
    ko: '컴퓨터의 멤버는 자신의 봇, 채팅, 커넥터로 컴퓨터를 사용합니다. 소유자가 설정의 사용자에서 이메일을 초대하고 초대를 보여 줍니다.',
    'pt-BR': 'Um membro de um computador o usa com seus próprios bots, chats e conectores. O proprietário convida seu e-mail em Ajustes, Pessoas, e mostra o convite.',
    ru: 'Участник использует компьютер со своими ботами, чатами и коннекторами. Владелец приглашает вашу почту в «Настройки → Люди» и показывает приглашение.',
    ar: 'يستخدم عضو الحاسوب بوتاته ومحادثاته وموصّلاته الخاصة. يدعو المالك بريدك من الإعدادات، الأشخاص، ويعرض عليك الدعوة.',
    he: 'חבר במחשב משתמש בו עם הבוטים, השיחות והמחברים שלו. הבעלים מזמין את כתובת הדוא״ל שלך בהגדרות, אנשים, ומציג לך את ההזמנה.',
    'zh-Hans': '电脑成员使用自己的智能体、聊天和连接器。所有者在“设置 → 人员”中邀请你的邮箱，并向你出示邀请。',
    'zh-Hant': '電腦成員使用自己的代理、聊天和連接器。所有者會在「設定 → 人員」中邀請你的電子郵件，並向你出示邀請。',
  },
  'Scan the invitation code from that computer. It works once, and this phone then acts as you.': {
    de: 'Scanne den Einladungscode von diesem Computer. Er gilt einmal, danach handelt dieses Telefon als du.',
    es: 'Escanea el código de invitación de ese ordenador. Funciona una vez y luego este teléfono actúa como tú.',
    fr: 'Scannez le code d’invitation de cet ordinateur. Il fonctionne une fois, puis ce téléphone agit en votre nom.',
    it: 'Scansiona il codice di invito di quel computer. Funziona una volta, poi questo telefono agisce come te.',
    ja: 'そのコンピュータの招待コードをスキャンしてください。一度だけ使え、その後このスマートフォンはあなたとして動作します。',
    ko: '해당 컴퓨터의 초대 코드를 스캔하세요. 한 번만 사용할 수 있으며, 이후 이 휴대폰은 나로서 동작합니다.',
    'pt-BR': 'Escaneie o código de convite daquele computador. Ele funciona uma vez, e então este telefone age como você.',
    ru: 'Отсканируйте код приглашения с того компьютера. Он действует один раз, затем телефон действует от вашего имени.',
    ar: 'امسح رمز الدعوة من ذلك الحاسوب. يعمل مرة واحدة، ثم يعمل هذا الهاتف باسمك.',
    he: 'סרוק את קוד ההזמנה מהמחשב הזה. הוא תקף פעם אחת, ואז הטלפון פועל בתור אתה.',
    'zh-Hans': '扫描那台电脑上的邀请代码。它只能使用一次，之后此手机将以你的身份行事。',
    'zh-Hant': '掃描那台電腦上的邀請代碼。它只能使用一次，之後此手機將以你的身分行事。',
  },
  'Scan the invitation': {
    de: 'Einladung scannen', es: 'Escanear la invitación', fr: 'Scanner l’invitation', it: 'Scansiona l’invito',
    ja: '招待をスキャン', ko: '초대 스캔', 'pt-BR': 'Escanear o convite', ru: 'Сканировать приглашение',
    ar: 'مسح الدعوة', he: 'סריקת ההזמנה', 'zh-Hans': '扫描邀请', 'zh-Hant': '掃描邀請',
  },
  'A one-time code they scan on their phone, or sign in with at this computer. They act as that person, with only their own bots, chats and connectors.': {
    de: 'Ein einmaliger Code, den sie auf ihrem Telefon scannen oder mit dem sie sich an diesem Computer anmelden. Sie handeln als diese Person, nur mit eigenen Bots, Chats und Konnektoren.',
    es: 'Un código de un solo uso que escanean en su teléfono, o con el que inician sesión en este ordenador. Actúan como esa persona, solo con sus bots, chats y conectores.',
    fr: 'Un code à usage unique qu’ils scannent sur leur téléphone, ou avec lequel ils se connectent sur cet ordinateur. Ils agissent en tant que cette personne, avec seulement leurs bots, chats et connecteurs.',
    it: 'Un codice monouso che scansionano sul telefono, o con cui accedono su questo computer. Agiscono come quella persona, con solo i propri bot, chat e connettori.',
    ja: 'スマートフォンで読み取るか、このコンピュータへのサインインに使う一度だけのコードです。その人として、自分のボット・チャット・コネクタだけを使えます。',
    ko: '휴대폰으로 스캔하거나 이 컴퓨터에 로그인할 때 쓰는 일회용 코드입니다. 그 사람으로서 자신의 봇, 채팅, 커넥터만 사용합니다.',
    'pt-BR': 'Um código de uso único que eles escaneiam no telefone, ou usam para entrar neste computador. Agem como essa pessoa, apenas com seus bots, chats e conectores.',
    ru: 'Одноразовый код: сканируют телефоном или входят с ним на этом компьютере. Участник действует как этот человек, только со своими ботами, чатами и коннекторами.',
    ar: 'رمز يُستخدم مرة واحدة يمسحونه على هواتفهم أو يسجّلون به الدخول على هذا الحاسوب. يعملون بصفة ذلك الشخص، ببوتاتهم ومحادثاتهم وموصّلاتهم فقط.',
    he: 'קוד חד־פעמי שהם סורקים בטלפון, או שמשמש לכניסה במחשב הזה. הם פועלים בתור אותו אדם, עם הבוטים, השיחות והמחברים שלהם בלבד.',
    'zh-Hans': '他们用手机扫描或在本次电脑上登录的一次性代码。他们会以该成员的身份行事，只能使用自己的智能体、聊天和连接器。',
    'zh-Hant': '他們用手機掃描或在本次電腦上登入的一次性代碼。他們會以該成員的身分行事，只能使用自己的代理、聊天和連接器。',
  },
  'The code works once.': {
    de: 'Der Code gilt einmal.', es: 'El código funciona una vez.', fr: 'Le code fonctionne une fois.',
    it: 'Il codice funziona una volta.', ja: 'コードは一度だけ使えます。', ko: '코드는 한 번만 사용할 수 있습니다.',
    'pt-BR': 'O código funciona uma vez.', ru: 'Код действует один раз.', ar: 'يعمل الرمز مرة واحدة.',
    he: 'הקוד תקף פעם אחת.', 'zh-Hans': '代码只能使用一次。', 'zh-Hant': '代碼只能使用一次。',
  },
  'Sessions and devices': {
    de: 'Sitzungen und Geräte', es: 'Sesiones y dispositivos', fr: 'Sessions et appareils',
    it: 'Sessioni e dispositivi', ja: 'セッションと端末', ko: '세션 및 기기', 'pt-BR': 'Sessões e dispositivos',
    ru: 'Сеансы и устройства', ar: 'الجلسات والأجهزة', he: 'הפעלות ומכשירים',
    'zh-Hans': '会话与设备', 'zh-Hant': '工作階段與裝置',
  },
  "A member's phone or signed-in app sees only what is theirs.": {
    de: 'Das Telefon oder die angemeldete App eines Mitglieds sieht nur, was ihm gehört.',
    es: 'El teléfono o la app con sesión de un miembro solo ve lo que es suyo.',
    fr: 'Le téléphone ou l’app connectée d’un membre ne voit que ce qui lui appartient.',
    it: 'Il telefono o l’app con accesso di un membro vede solo ciò che è suo.',
    ja: 'メンバーのスマートフォンやサインイン中のアプリは、本人のものだけを表示します。',
    ko: '멤버의 휴대폰이나 로그인한 앱은 자신의 것만 봅니다.',
    'pt-BR': 'O telefone ou app conectado de um membro vê apenas o que é dele.',
    ru: 'Телефон или приложение участника видит только его собственное.',
    ar: 'لا يرى هاتف العضو أو تطبيقه المسجّل الدخول إلا ما يخصه.',
    he: 'הטלפון או האפליקציה המחוברת של חבר רואים רק את מה ששלהם.',
    'zh-Hans': '成员的手机或已登录应用只能看到属于他们的内容。',
    'zh-Hant': '成員的手機或已登入應用程式只能看到屬於他們的內容。',
  },
  "End {name}'s session?": {
    de: 'Sitzung von {name} beenden?', es: '¿Terminar la sesión de {name}?', fr: 'Terminer la session de {name} ?',
    it: 'Terminare la sessione di {name}?', ja: '{name} のセッションを終了しますか？', ko: '{name}님의 세션을 종료할까요?',
    'pt-BR': 'Encerrar a sessão de {name}?', ru: 'Завершить сеанс {name}?', ar: 'إنهاء جلسة {name}؟',
    he: 'לסיים את ההפעלה של {name}?', 'zh-Hans': '结束 {name} 的会话？', 'zh-Hant': '結束 {name} 的工作階段？',
  },
  'Invitation for {name}: {code}': {
    de: 'Einladung für {name}: {code}', es: 'Invitación para {name}: {code}', fr: 'Invitation pour {name} : {code}',
    it: 'Invito per {name}: {code}', ja: '{name} への招待: {code}', ko: '{name}님 초대: {code}',
    'pt-BR': 'Convite para {name}: {code}', ru: 'Приглашение для {name}: {code}', ar: 'دعوة لـ {name}: {code}',
    he: 'הזמנה עבור {name}: {code}', 'zh-Hans': '{name} 的邀请：{code}', 'zh-Hant': '{name} 的邀請：{code}',
  },
  "That device is this computer's own. Assign it to a person first.": {
    de: 'Dieses Gerät gehört dem Computer selbst. Weise es zuerst einer Person zu.',
    es: 'Ese dispositivo es del propio ordenador. Asígnalo primero a una persona.',
    fr: 'Cet appareil appartient à l’ordinateur lui-même. Attribuez-le d’abord à une personne.',
    it: 'Quel dispositivo è del computer stesso. Assegnalo prima a una persona.',
    ja: 'その端末はこのコンピュータ自身のものです。先に人に割り当ててください。',
    ko: '그 기기는 이 컴퓨터 자체의 것입니다. 먼저 사람에게 할당하세요.',
    'pt-BR': 'Esse dispositivo é do próprio computador. Atribua-o a uma pessoa primeiro.',
    ru: 'Это устройство принадлежит самому компьютеру. Сначала назначьте его человеку.',
    ar: 'ذلك الجهاز يخص الحاسوب نفسه. عيّنه لشخص أولًا.',
    he: 'המכשיר הזה שייך למחשב עצמו. שייך אותו קודם לאדם.',
    'zh-Hans': '该设备属于这台电脑本身。请先将其分配给某位成员。',
    'zh-Hant': '該裝置屬於這台電腦本身。請先將其分配給某位成員。',
  },
  'They can sign in again with a new invitation.': {
    de: 'Sie können sich mit einer neuen Einladung wieder anmelden.',
    es: 'Pueden volver a iniciar sesión con una nueva invitación.',
    fr: 'Ils peuvent se reconnecter avec une nouvelle invitation.',
    it: 'Possono accedere di nuovo con un nuovo invito.',
    ja: '新しい招待で再びサインインできます。', ko: '새 초대로 다시 로그인할 수 있습니다.',
    'pt-BR': 'Eles podem entrar de novo com um novo convite.',
    ru: 'Они смогут войти снова по новому приглашению.',
    ar: 'يمكنهم تسجيل الدخول مجددًا بدعوة جديدة.',
    he: 'הם יוכלו להיכנס שוב עם הזמנה חדשה.',
    'zh-Hans': '他们可以使用新的邀请再次登录。', 'zh-Hant': '他們可以使用新的邀請再次登入。',
  },
  'That device loses access at once. The person, their bots and their chats stay.': {
    de: 'Dieses Gerät verliert sofort den Zugang. Person, Bots und Chats bleiben.',
    es: 'Ese dispositivo pierde el acceso al instante. La persona, sus bots y sus chats permanecen.',
    fr: 'Cet appareil perd immédiatement l’accès. La personne, ses bots et ses chats restent.',
    it: 'Quel dispositivo perde subito l’accesso. La persona, i suoi bot e le sue chat restano.',
    ja: 'その端末はすぐにアクセスを失います。本人、ボット、チャットは残ります。',
    ko: '해당 기기는 즉시 접근 권한을 잃습니다. 사람, 봇, 채팅은 남습니다.',
    'pt-BR': 'Esse dispositivo perde o acesso na hora. A pessoa, seus bots e seus chats permanecem.',
    ru: 'Устройство сразу потеряет доступ. Человек, его боты и чаты останутся.',
    ar: 'يفقد ذلك الجهاز الوصول فورًا. ويبقى الشخص وبوتاته ومحادثاته.',
    he: 'המכשיר יאבד גישה מיד. האדם, הבוטים והשיחות נשארים.',
    'zh-Hans': '该设备会立即失去访问权限。成员本人、其智能体和聊天仍会保留。',
    'zh-Hant': '該裝置會立即失去存取權。成員本人、其代理和聊天仍會保留。',
  },
  'Role': {
    de: 'Rolle', es: 'Rol', fr: 'Rôle', it: 'Ruolo', ja: '役割', ko: '역할', 'pt-BR': 'Função', ru: 'Роль',
    ar: 'الدور', he: 'תפקיד', 'zh-Hans': '角色', 'zh-Hant': '角色',
  },
}

const languages = ['de', 'es', 'fr', 'it', 'ja', 'ko', 'pt-BR', 'ru', 'ar', 'he', 'zh-Hans', 'zh-Hant']
const root = 'apps/ios/Kit/Sources/CodyncKit/Resources/Languages'
const lineKey = /^\s*"((?:[^"\\]|\\.)*)"\s*:/

for (const language of languages) {
  const path = join(root, `${language}.json`)
  // Re-read immediately before writing: another agent may have just changed this table.
  const before = readFileSync(path, 'utf8')
  const catalog = JSON.parse(before)
  const added = []
  for (const [key, values] of Object.entries(phrases)) {
    if (key in catalog) continue
    const value = values[language]
    if (!value) throw new Error(`${language} lacks a translation for ${JSON.stringify(key)}`)
    added.push([key, value])
  }
  if (added.length === 0) {
    console.log(language, 'already has every phrase')
    continue
  }
  const lines = before.split('\n')
  // The closing `}` is the last non-empty line; insert each phrase at its sorted position.
  const end = lines.findLastIndex((line) => line.trim() === '}')
  if (end < 0) throw new Error(`${path} has no closing brace`)
  for (const [key, value] of added) {
    const wanted = JSON.stringify(key)
    let at = end
    for (let i = 0; i < end; i += 1) {
      const match = lineKey.exec(lines[i])
      if (!match) continue
      const other = JSON.parse(`"${match[1]}"`)
      if (other > key) {
        at = i
        break
      }
    }
    const line = `  ${wanted}: ${JSON.stringify(value)},`
    lines.splice(at, 0, line)
  }
  // The last entry must not keep a trailing comma, and every entry before it must have one: these
  // tables are one entry per line, which the check above keeps true.
  const entries = lines.map((line, index) => (lineKey.test(line) ? index : -1)).filter((index) => index >= 0)
  const lastEntry = entries.at(-1)
  for (const index of entries) {
    if (index === lastEntry) lines[index] = lines[index].replace(/,\s*$/, '')
    else if (!lines[index].trimEnd().endsWith(',')) lines[index] = `${lines[index].replace(/\s*$/, '')},`
  }
  const after = lines.join('\n')
  JSON.parse(after)
  writeFileSync(path, after)
  console.log(language, 'added', added.length)
}
