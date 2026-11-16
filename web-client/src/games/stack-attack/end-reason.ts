import type { GameView } from "@/proto/minigames/v2/game_pb";
import { StackAttackEndReason } from "@/proto/minigames/v2/stack_attack_pb";

const END_REASONS: Partial<Record<StackAttackEndReason, string>> = {
  [StackAttackEndReason.WORKERS_CRUSHED]: "Crushed by a falling crate",
  [StackAttackEndReason.CEILING_REACHED]: "The crates reached the ceiling",
  [StackAttackEndReason.ALL_WORKERS_LEFT]: "Every worker left",
};

export function describeStackAttackEnd(view: GameView): string | null {
  const game = view.game;
  if (game.case !== "stackAttack" || game.value.endReason === undefined) return null;
  return END_REASONS[game.value.endReason] ?? null;
}
