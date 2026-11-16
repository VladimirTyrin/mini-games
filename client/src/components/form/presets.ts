import type { ChipOption } from "./ChipsField.vue";

/** A named set of values for some settings fields, shown as one chip. */
export interface Preset<Patch extends object> extends ChipOption {
  patch: Patch;
}

/** Key of the first preset whose every value equals the current one; `null` for none. */
export function matchingPresetKey<Patch extends object>(
  presets: readonly Preset<Patch>[],
  current: NoInfer<Patch>,
): string | null {
  const matches = presets.find((preset) =>
    (Object.keys(preset.patch) as (keyof Patch)[]).every((key) => preset.patch[key] === current[key]),
  );
  return matches?.key ?? null;
}
