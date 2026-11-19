<script setup lang="ts">
import { t } from "@/i18n";

import { PAIR_EXAMPLES } from "./pair-examples";
import PairExample from "./PairExample.vue";
</script>

<template>
  <h3>Цель</h3>
  <p>Убери с поля все числа.</p>
  <h3>Пары</h3>
  <ul>
    <li>
      Зачёркивай пары чисел: два <b>равных</b> числа (4 и 4) или два числа, которые <b>в сумме дают 10</b>
      (3 и 7).
    </li>
    <li>
      Числа пары должны быть <b>соседями</b>: стоять рядом в строке, одно над другим или по диагонали, либо
      быть последним числом строки и первым числом следующей. Зачёркнутые клетки между ними не в счёт, так что
      пару можно составить и через них — по прямой или с конца строки на следующую.
    </li>
    <li>Строка, в которой всё зачёркнуто, исчезает, а строки ниже поднимаются.</li>
  </ul>
  <h3>Что можно, а что нет</h3>
  <div class="mt-2 flex flex-col gap-4">
    <PairExample v-bind="PAIR_EXAMPLES.row" caption="Соседи в строке: 3 + 7 = 10" />
    <PairExample v-bind="PAIR_EXAMPLES.column" caption="Соседи в столбце: 6 и 6 равны" />
    <PairExample v-bind="PAIR_EXAMPLES.diagonal" caption="Соседи по диагонали: 2 + 8 = 10" />
    <PairExample
      v-bind="PAIR_EXAMPLES.acrossCrossedOut"
      caption="Через зачёркнутые клетки: они больше не мешают"
    />
    <PairExample v-bind="PAIR_EXAMPLES.rowEndToNext" caption="С конца строки на начало следующей" />
    <PairExample
      v-bind="PAIR_EXAMPLES.acrossCrossedOutAndRowEnd"
      caption="Через зачёркнутые клетки и конец строки: 8 + 2 = 10"
    />
    <PairExample
      v-bind="PAIR_EXAMPLES.notMatching"
      caption="Не пара: 3 и 5 не равны и в сумме дают 8, а не 10"
    />
    <PairExample v-bind="PAIR_EXAMPLES.notNeighbours" caption="Не соседи: 6 между ними ещё в игре" />
  </div>
  <h3>Застрял?</h3>
  <ul>
    <li>
      Кнопка «<b>{{ t.games.numbersMatch.board.refill }}</b
      >» дописывает все числа, что ещё в игре, в конец поля новым цветом. Число добавлений ограничено.
    </li>
    <li>
      Кнопка «<b>{{ t.games.numbersMatch.board.hint }}</b
      >» подсвечивает пару, которую можно убрать, если подсказки включены в настройках.
    </li>
    <li>Ты проигрываешь, когда пар не осталось, а добавлять числа больше нельзя.</li>
  </ul>
</template>
