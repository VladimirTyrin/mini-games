import { type InjectionKey, type Ref, computed, inject } from "vue";

import type { SettingsCase } from "@/games/game-plugin";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import { type SettingRange, type SettingsProblem, settingsProblem } from "@/wasm/core";

import type { Preset } from "./presets";

/** Who reads the form: wording differs (a local "You" is the online "Host"). */
export type SettingsAudience = "local" | "online";

export interface SettingsFormState {
  problem: SettingsProblem | null;
  ranges: readonly SettingRange[];
  /** Seat count the settings are validated for. */
  seats: number;
  audience: SettingsAudience;
}

/** Bound to a numeric row: the core's limits and the core's complaint about this field. */
export interface FieldBinding {
  min?: number;
  max?: number;
  error: string | null;
}

type SettingsMessage<Case extends SettingsCase> = Extract<GameSettings["game"], { case: Case }>["value"];
type SettingsValues<Case extends SettingsCase> = Omit<SettingsMessage<Case>, "$typeName">;

export const settingsFormKey: InjectionKey<Readonly<Ref<SettingsFormState>>> = Symbol("settings-form");

/** Core reasons start lower-case ("must be between 3 and 20, got 2"); rows show a sentence. */
export function sentence(reason: string): string {
  return reason.charAt(0).toUpperCase() + reason.slice(1);
}

/** The core names fields as on the wire (`field_width`); protobuf-es uses `fieldWidth`. */
function wireFieldName(property: string): string {
  return property.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

/**
 * Editing state for one game's settings form rendered inside `SettingsEditor` (which provides
 * the core's verdict and limits). Updates are immutable: every change emits new settings.
 */
export function useGameSettingsForm<Case extends SettingsCase>(
  settings: Ref<GameSettings>,
  settingsCase: Case,
) {
  const provided = inject(settingsFormKey);
  if (!provided) throw new Error("a game settings form must be rendered inside SettingsEditor");
  const state = provided;

  const value = computed(() =>
    settings.value.game.case === settingsCase ? (settings.value.game.value as SettingsMessage<Case>) : null,
  );

  function withPatch(patch: Partial<SettingsValues<Case>>): GameSettings | null {
    if (!value.value) return null;
    const game = { case: settingsCase, value: { ...value.value, ...patch } } as GameSettings["game"];
    return { ...settings.value, game };
  }

  function update(patch: Partial<SettingsValues<Case>>): void {
    const next = withPatch(patch);
    if (next) settings.value = next;
  }

  function field(name: string): FieldBinding {
    const range = state.value.ranges.find((candidate) => candidate.field === name);
    const problem = state.value.problem;
    return {
      min: range?.min,
      max: range?.max,
      error: problem?.field === name ? sentence(problem.message) : null,
    };
  }

  function hasProblemIn(fields: readonly string[]): boolean {
    const failing = state.value.problem?.field;
    return !!failing && fields.includes(failing);
  }

  /** Presets the core accepts: applying one must not make any of its own fields invalid. */
  function acceptedPresets<Patch extends Partial<SettingsValues<Case>>>(
    presets: readonly Preset<Patch>[],
  ): Preset<Patch>[] {
    return presets.filter((preset) => {
      const candidate = withPatch(preset.patch);
      if (!candidate) return false;
      const failing = settingsProblem(candidate, state.value.seats)?.field;
      return !failing || !Object.keys(preset.patch).map(wireFieldName).includes(failing);
    });
  }

  function applyPreset<Patch extends Partial<SettingsValues<Case>>>(
    presets: readonly Preset<Patch>[],
    key: string | null,
  ): void {
    const preset = presets.find((candidate) => candidate.key === key);
    if (preset) update(preset.patch);
  }

  return {
    value,
    update,
    field,
    hasProblemIn,
    acceptedPresets,
    applyPreset,
    audience: computed(() => state.value.audience),
  };
}
