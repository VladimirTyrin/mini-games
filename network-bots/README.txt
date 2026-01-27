TicTacToe Neural Bot
====================

Запуск против сервера:
----------------------

1. Гибридный бот (тактика + нейросеть) - рекомендуется:
   dotnet run -- --bot-type hybrid --model neural_model.dat

2. Против локального сервера:
   dotnet run -- --local --bot-type hybrid --model neural_model.dat

3. Против случайного бота на сервере:
   dotnet run -- --bot-type hybrid --opponent random --model neural_model.dat

4. Без UI:
   dotnet run -- --bot-type hybrid --model neural_model.dat --no-ui


Доступные типы ботов (--bot-type):
----------------------------------
- Minimax - классический минимакс
- Mcts    - Monte Carlo Tree Search
- Hybrid  - тактика + нейросеть (лучший)


Обучение модели:
----------------
dotnet run -- --train

Параметры обучения:
  --iterations N   - количество итераций (default: 50)
  --model PATH     - путь к файлу модели (default: neural_model.dat)


Архитектура:
------------
- IBoardView    - абстракция доски для унификации работы с GameEngine и protobuf
- TacticsEngine - общая тактика для всех ботов (win/block/threats/forks)
- HybridNeuralLocalBot - основной бот (ILocalBot)
- NetworkBotAdapter    - адаптер для сетевой игры (ILocalBot -> ITicTacToeBot)


Порядок тактических проверок:
-----------------------------
1. Win (победа)
2. Block win (блокировка победы)
3. Create open four (создать 4 с 2 открытыми концами)
4. Block open four
5. Block four (любые 4 в ряд)
6. Block double threat (блокировать вилку противника)
7. Create double threat (создать свою вилку)
8. Create open three (создать 3 с 2 открытыми - ведёт к победе)
9. Block open three
10. Neural network (стратегический выбор)


Файлы модели:
-------------
neural_model.dat - обученная модель
