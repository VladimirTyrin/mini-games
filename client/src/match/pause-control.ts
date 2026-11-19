import { type InjectionKey, type Ref, inject, provide } from "vue";

/** The player's pause of a local real-time match, for controls a board draws (the thumb zone). */
export interface PauseControl {
  readonly canPause: Readonly<Ref<boolean>>;
  readonly paused: Readonly<Ref<boolean>>;
  toggle(): void;
}

const pauseControlKey: InjectionKey<PauseControl> = Symbol("pause-control");

/** The match screen offers its pause to the board's controls. */
export function providePauseControl(control: PauseControl): void {
  provide(pauseControlKey, control);
}

/** The match screen's pause; `null` outside one (a board in a test, say). */
export function usePauseControl(): PauseControl | null {
  return inject(pauseControlKey, null);
}
