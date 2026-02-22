import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { create } from "@bufbuild/protobuf";
import type { ReplayStateNotification, ReplayFileReadyNotification } from "../proto/game_service_pb";
import {
  ReplayPauseCommandSchema,
  ReplayResumeCommandSchema,
  ReplaySetSpeedCommandSchema,
  ReplayStepForwardCommandSchema,
  ReplayRestartCommandSchema,
} from "../proto/game_service_pb";
import { gameClient } from "../api/client";

export const useReplayStore = defineStore("replay", () => {
  const DEFAULT_REPLAY_FILE_NAME = "replay.minigamesreplay";

  const replayState = ref<ReplayStateNotification | null>(null);
  const replayData = ref<{ content: Uint8Array; suggestedFileName: string } | null>(null);

  const isInReplay = computed(() => replayState.value !== null);
  const isPaused = computed(() => replayState.value?.isPaused ?? false);
  const isFinished = computed(() => replayState.value?.isFinished ?? false);
  const currentTick = computed(() => Number(replayState.value?.currentTick ?? 0n));
  const totalTicks = computed(() => Number(replayState.value?.totalTicks ?? 0n));
  const speed = computed(() => replayState.value?.speed ?? 1);
  const hostOnlyControl = computed(() => replayState.value?.hostOnlyControl ?? false);
  const progress = computed(() => {
    if (totalTicks.value === 0) return 0;
    return currentTick.value / totalTicks.value;
  });

  function handleReplayState(notification: ReplayStateNotification): void {
    replayState.value = notification;
  }

  function handleReplayFile(notification: ReplayFileReadyNotification): void {
    replayData.value = {
      content: notification.content,
      suggestedFileName: normalizeReplayFileName(notification.suggestedFileName),
    };
  }

  function normalizeReplayFileName(name?: string): string {
    const value = (name ?? "").trim();
    if (!value) return DEFAULT_REPLAY_FILE_NAME;
    return value.endsWith(".minigamesreplay") ? value : `${value}.minigamesreplay`;
  }

  function setReplayData(content: Uint8Array, suggestedFileName?: string): void {
    replayData.value = {
      content,
      suggestedFileName: normalizeReplayFileName(
        suggestedFileName ?? replayData.value?.suggestedFileName
      ),
    };
  }

  function pause(): void {
    gameClient.sendReplayCommand({
      case: "pause",
      value: create(ReplayPauseCommandSchema, {}),
    });
  }

  function resume(): void {
    gameClient.sendReplayCommand({
      case: "resume",
      value: create(ReplayResumeCommandSchema, {}),
    });
  }

  function togglePause(): void {
    if (isPaused.value) {
      resume();
    } else {
      pause();
    }
  }

  function setSpeed(newSpeed: number): void {
    gameClient.sendReplayCommand({
      case: "setSpeed",
      value: create(ReplaySetSpeedCommandSchema, { speed: newSpeed }),
    });
  }

  function stepForward(): void {
    gameClient.sendReplayCommand({
      case: "stepForward",
      value: create(ReplayStepForwardCommandSchema, {}),
    });
  }

  function restart(): void {
    gameClient.sendReplayCommand({
      case: "restart",
      value: create(ReplayRestartCommandSchema, {}),
    });
  }

  function createReplayLobby(
    content: Uint8Array,
    hostOnlyControl: boolean,
    suggestedFileName?: string
  ): void {
    setReplayData(content, suggestedFileName);
    gameClient.createReplayLobby(content, hostOnlyControl);
  }

  function watchReplayTogether(
    content: Uint8Array,
    hostOnlyControl: boolean,
    suggestedFileName?: string
  ): void {
    setReplayData(content, suggestedFileName);
    gameClient.watchReplayTogether(content, hostOnlyControl);
  }

  function saveReplay(): void {
    if (!replayData.value) return;

    const blob = new Blob([replayData.value.content]);
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = replayData.value.suggestedFileName;
    a.click();
    URL.revokeObjectURL(url);
  }

  function clear(): void {
    replayState.value = null;
    replayData.value = null;
  }

  return {
    replayState,
    replayData,
    isInReplay,
    isPaused,
    isFinished,
    currentTick,
    totalTicks,
    speed,
    hostOnlyControl,
    progress,
    handleReplayState,
    handleReplayFile,
    pause,
    resume,
    togglePause,
    setSpeed,
    stepForward,
    restart,
    createReplayLobby,
    watchReplayTogether,
    saveReplay,
    clear,
  };
});
