# Sea Battle

Two seats, classic rules only:
- each seat places a fleet on its own 10×10 sea: one ship of four cells, two of three, three of two
  and four of one;
- ships lie straight and may not touch, not even at a corner;
- then the seats take turns firing at the other's sea. A hit fires again, a miss passes the turn;
- a sunk ship is shown whole, and the water around it is marked as cleared (it cannot hold a ship,
  and firing there is refused);
- whoever sinks the whole other fleet wins. Giving up or leaving hands the win to the other seat.

## Hidden fleets

The fleets are the authority's secret (`HiddenFleets`, see architecture.md, "Secrets"). No
client, bot or observer has the other fleet while the game runs.

1. **Placing.** A seat sends its fleet as a private command (`SubmitSecretCommand`). The
   authority checks it (`Fleet::parse`), keeps it, and the frame says only `FleetPlaced(slot)`.
   The seat gets its own fleet back as `MatchSeatSecret`, and again after every catch-up, so a
   reconnect redraws it.
2. **Time limit online.** `MatchHeader.secret_setup_ms` (90 s from the start) is the time people
   have. When it runs out, the server places a random fleet for whoever has none. Against bots
   on the device there is no limit. Bots place theirs when the first frame is sealed.
3. **Battle.** Once both fleets are placed, the authority draws who fires first. A shot is a
   public command, `Fire(cell)`. The authority answers in the same frame with `ShotResolved`:
   miss, hit, or the sunk ship.
4. **End.** In the frame the game ends, the authority sends both fleets (`FleetRevealed`), so a
   replay has them. The replay player reads them from the end of the log and shows both fleets
   from the first move.

## Placing a fleet in the app

- The sea starts empty. A tap on a free cell adds a deck, a tap on a ship's cell takes it away,
  which shortens the ship or splits it in two. Cells joined by a side are one ship.
- Free cells are lit where one more deck keeps to the fleet's rules so far:
  - a ship may not bend, touch another (not even at a corner) or grow past four;
  - there may be no more ships of two decks or longer than the fleet has. Lone decks are not
    counted, since two of them may still become one ship when the cell between them is filled.
- Whether the ships not yet started still have room is checked apart (a bounded search). When
  they have none, the editor says so under the sea. Both checks are hints for the player only.
- The fleet's ship lengths come from a fleet the core makes, and so does **Random**
  (`Game::default_secret_command` through the mirror). **Clear** empties the sea.
- **Ready** stays off until the core accepts the fleet (`Match::check_secret_command`), so the page
  never decides what a fleet is.
- Online, a complete fleet left on screen is sent for the player 2 s before the limit, so they get
  the fleet they made rather than a random one.

## Bots

`HunterBot` fires from what everyone sees of the other sea, never from the fleet.
- **Easy** fires at random and finishes a wounded ship only half the time.
- **Medium** finishes wounded ships along their line and otherwise hunts on a checkerboard.
- **Hard** fires where the ships still afloat fit most often (while a ship is wounded, only
  placements through its hits count). It also places its own fleet off the middle, choosing of
  a few random fleets the one standing least where that density points on an empty sea.

Measured over 20 games each (`a_stronger_bot_beats_a_weaker_one_most_of_the_time`): hard beats
easy at least 16 times and medium 11 times, medium beats easy at least 14 times.
