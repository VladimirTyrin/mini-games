import { t } from "@/i18n";
import type { GameView } from "@/proto/minigames/v2/game_pb";
import { StackAttackEndReason } from "@/proto/minigames/v2/stack_attack_pb";

export function describeStackAttackEnd(view: GameView): string | null {
  const game = view.game;
  if (game.case !== "stackAttack" || game.value.endReason === undefined) return null;
  const reasons = t.games.stackAttack.endReasons;
  switch (game.value.endReason) {
    case StackAttackEndReason.WORKERS_CRUSHED:
      return reasons.workersCrushed;
    case StackAttackEndReason.CEILING_REACHED:
      return reasons.ceilingReached;
    case StackAttackEndReason.ALL_WORKERS_LEFT:
      return reasons.allWorkersLeft;
    case StackAttackEndReason.WORKERS_TRAPPED:
      return reasons.workersTrapped;
    default:
      return null;
  }
}
