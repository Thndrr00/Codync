import io
import json
import os
import re

entries = {
    '{a} and {b} are here': {
        'ar': '{a} و{b} هنا',
        'de': '{a} und {b} sind hier',
        'es': '{a} y {b} están aquí',
        'fr': '{a} et {b} sont là',
        'it': '{a} e {b} sono qui',
        'ja': '{a} と {b} がここにいます',
        'ko': '{a} 님과 {b} 님이 있습니다',
        'pt-BR': '{a} e {b} estão aqui',
        'ru': '{a} и {b} здесь',
        'zh-Hans': '{a} 和 {b} 在这里',
        'zh-Hant': '{a} 和 {b} 在這裡',
    },
    '{a} and {b} are typing…': {
        'ar': '{a} و{b} يكتبان…',
        'de': '{a} und {b} tippen…',
        'es': '{a} y {b} están escribiendo…',
        'fr': '{a} et {b} écrivent…',
        'it': '{a} e {b} stanno scrivendo…',
        'ja': '{a} と {b} が入力中…',
        'ko': '{a} 님과 {b} 님이 입력 중…',
        'pt-BR': '{a} e {b} estão digitando…',
        'ru': '{a} и {b} печатают…',
        'zh-Hans': '{a} 和 {b} 正在输入…',
        'zh-Hant': '{a} 和 {b} 正在輸入…',
    },
    '{name} is here': {
        'ar': '{name} هنا',
        'de': '{name} ist hier',
        'es': '{name} está aquí',
        'fr': '{name} est là',
        'it': '{name} è qui',
        'ja': '{name} がここにいます',
        'ko': '{name} 님이 있습니다',
        'pt-BR': '{name} está aqui',
        'ru': '{name} здесь',
        'zh-Hans': '{name} 在这里',
        'zh-Hant': '{name} 在這裡',
    },
    '{name} is typing…': {
        'ar': '{name} يكتب…',
        'de': '{name} tippt…',
        'es': '{name} está escribiendo…',
        'fr': '{name} écrit…',
        'it': '{name} sta scrivendo…',
        'ja': '{name} が入力中…',
        'ko': '{name} 님이 입력 중…',
        'pt-BR': '{name} está digitando…',
        'ru': '{name} печатает…',
        'zh-Hans': '{name} 正在输入…',
        'zh-Hant': '{name} 正在輸入…',
    },
    '{names} and {count} more are here': {
        'ar': '{names} و{count} غيرهم هنا',
        'de': '{names} und {count} weitere sind hier',
        'es': '{names} y {count} más están aquí',
        'fr': '{names} et {count} autres sont là',
        'it': '{names} e altri {count} sono qui',
        'ja': '{names} と他 {count} 人がここにいます',
        'ko': '{names} 님 외 {count} 명이 있습니다',
        'pt-BR': '{names} e mais {count} estão aqui',
        'ru': '{names} и ещё {count} здесь',
        'zh-Hans': '{names} 和另外 {count} 人在这里',
        'zh-Hant': '{names} 和另外 {count} 人在這裡',
    },
    '{names} are typing…': {
        'ar': '{names} يكتبون…',
        'de': '{names} tippen…',
        'es': '{names} están escribiendo…',
        'fr': '{names} écrivent…',
        'it': '{names} stanno scrivendo…',
        'ja': '{names} が入力中…',
        'ko': '{names} 님이 입력 중…',
        'pt-BR': '{names} estão digitando…',
        'ru': '{names} печатают…',
        'zh-Hans': '{names} 正在输入…',
        'zh-Hant': '{names} 正在輸入…',
    },
}

entry = re.compile(r'^(\s*)"((?:[^"\\]|\\.)*)":')


def escape(value):
    return json.dumps(value, ensure_ascii=False)[1:-1]


root = os.path.join('apps', 'ios', 'Kit', 'Sources', 'CodyncKit', 'Resources', 'Languages')
for key, values in entries.items():
    for code, value in values.items():
        path = os.path.join(root, code + '.json')
        with io.open(path, encoding='utf-8') as handle:
            lines = handle.readlines()
        if any(line.strip().startswith('"' + key + '":') for line in lines):
            print(code, 'already', key)
            continue
        line = '  "%s": "%s",\n' % (escape(key), escape(value))
        for index, existing in enumerate(lines):
            match = entry.match(existing)
            if match and match.group(2) > key:
                lines.insert(index, line)
                break
        else:
            # Curly-brace keys trail the alphabet: append before the closing brace.
            close = len(lines) - 1
            while close > 0 and not lines[close].strip().startswith('}'):
                close -= 1
            lines.insert(close, line)
        with io.open(path, 'w', encoding='utf-8', newline='') as handle:
            handle.writelines(lines)
        print(code, 'added', key)
