# pre_reform1918_stats
ДОРЕФОРМЕННЫЙ ОРѲОГРАФИЧЕСКІЙ АНАЛИЗАТОРЪ v0.7.1
===================================================
Ликбез автоматизирован:
www.culture.ru/materials/256469/likbez-po-dorevolyucionnoi-orfografii

<img width="663" height="544" alt="image" src="https://github.com/user-attachments/assets/a949f77a-166d-4f17-a325-eee8cb0613fa" />

```ПАРАЛЛЕЛЬНАЯ ОБРАБОТКА: rayon на всѣ ядра CPU.

ФОРМИРОВАНИЕ title:
  • JSONL: изъ поля "title"
  • Текстъ: первые N + "..." + послѣдніе N символовъ (-l N)

ОСОБЕННОСТИ:
  • Буквы: і, ѳ, ѵ, ѣ
  • Окончанія: -ыя, -аго, -ыи, -ія
  • Твёрдый знакъ: ъ\b
  • Двойныя согласныя, приставки раз-/без-/воз-
  • Точки аббревіатуръ: \b[А-ЯЁ]\. (только одиночныя заглавныя)

ПРИМѢРЫ:
  $ cat text.txt | ./pre_reform_stats
  $ ./pre_reform_stats -t 8 -l 100 < input.txt
  $ ./pre_reform_stats -v < data.txt

© 1918–2026. Всѣ права сохранены.


Usage: pre_reform1918_stats [OPTIONS]

Options:
  -h, --help                     Показать справку
  -r, --rules-file <RULES_FILE>  Путь къ JSON-файлу съ правилами
  -v, --verbose                  Подробная статистика
  -t, --threads <N>              Число потоковъ
  -b, --batch-size <N>           Размѣръ пакета [default: 1000]
      --no-parallel              Отключить параллельность
  -l, --title-len <N>            Длина title [default: 50]
  ```
Произволительность оптимизирована:
11 сек на бюджетном 32-битном планшете (менее 2GB RAM)
получаем статистику по всем Костромским Губернским Ведомостям (более 7 миллионов слов)
<img width="974" height="651" alt="image" src="https://github.com/user-attachments/assets/5a54e940-0ee0-43cb-8f8e-d8bebeee1c01" />

Для чего?

В ПОСТАНОВЛЕНIЯ и ПРЕДПИСАНIЯ
вместо "и" десятеричной закралась латиница.

Статистика позволяет профилировать ошибки OCR (и HTR).
