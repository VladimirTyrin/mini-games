<script setup lang="ts">
declare const __APP_VERSION__: string;
const version = __APP_VERSION__;
</script>

<template>
  <div class="app-container bg-gray-900 text-white flex flex-col">
    <div class="flex-grow overflow-auto">
      <router-view />
    </div>
    <footer class="flex-shrink-0 bg-gray-800 border-t border-gray-700 py-3 px-4 text-center text-sm text-gray-400">
      <span class="text-gray-500">v{{ version }}</span>
      <span class="mx-2">|</span>
      Source code and desktop client with richer capabilities (offline play, saving replays) available at
      <a
        href="https://github.com/VladimirTyrin/mini-games"
        target="_blank"
        rel="noopener noreferrer"
        class="text-blue-400 hover:text-blue-300 underline"
      >
        github.com/VladimirTyrin/mini-games
      </a>
    </footer>
  </div>
</template>

<style>
.app-container {
  min-height: 100dvh;
  min-height: 100vh; /* fallback for older browsers */
}

@supports (min-height: 100dvh) {
  .app-container {
    min-height: 100dvh;
  }
}
</style>
