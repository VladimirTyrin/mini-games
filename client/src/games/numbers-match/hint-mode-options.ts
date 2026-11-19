import { t } from "@/i18n";
import { NumbersMatchHintMode } from "@/proto/minigames/v2/numbers_match_pb";

/** The hint modes as options, worded in the current language. */
export function hintModeOptions(): { value: NumbersMatchHintMode; label: string }[] {
  const labels = t.games.numbersMatch.hintModes;
  return [
    { value: NumbersMatchHintMode.DISABLED, label: labels.off },
    { value: NumbersMatchHintMode.LIMITED, label: labels.limited },
    { value: NumbersMatchHintMode.UNLIMITED, label: labels.unlimited },
  ];
}

export function hintModeLabel(mode: NumbersMatchHintMode): string {
  return (
    hintModeOptions().find((option) => option.value === mode)?.label ?? t.games.numbersMatch.hintModes.unknown
  );
}
