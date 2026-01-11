import { defineStore } from "pinia";
import { ref, computed } from "vue";

export const useDeviceStore = defineStore("device", () => {
  const isTouchDevice = ref(false);
  const screenWidth = ref(window.innerWidth);
  const screenHeight = ref(window.innerHeight);

  function init(): void {
    isTouchDevice.value =
      "ontouchstart" in window || navigator.maxTouchPoints > 0;

    window.addEventListener("resize", handleResize);
  }

  function handleResize(): void {
    screenWidth.value = window.innerWidth;
    screenHeight.value = window.innerHeight;
  }

  const isMobile = computed(() => screenWidth.value < 768);

  const isSmallScreen = computed(() => screenWidth.value < 640);

  return {
    isTouchDevice,
    screenWidth,
    screenHeight,
    isMobile,
    isSmallScreen,
    init,
  };
});
