import { NumbersMatchHintMode } from "@/proto/minigames/v2/numbers_match_pb";

export const hintModeOptions = [
  { value: NumbersMatchHintMode.LIMITED, label: "Limited" },
  { value: NumbersMatchHintMode.UNLIMITED, label: "Unlimited" },
  { value: NumbersMatchHintMode.DISABLED, label: "Off" },
] as const;

export function hintModeLabel(mode: NumbersMatchHintMode): string {
  return hintModeOptions.find((option) => option.value === mode)?.label ?? "Default";
}
