<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue';

interface Props {
  show: boolean;
  title?: string;
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  title: '',
});

const emit = defineEmits<{
  close: [];
}>();

function onBackdropClick() {
  emit('close');
}

function onEscapeKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && props.show) {
    emit('close');
  }
}

onMounted(() => {
  document.addEventListener('keydown', onEscapeKey);
});

onUnmounted(() => {
  document.removeEventListener('keydown', onEscapeKey);
});

watch(() => props.show, (newValue) => {
  if (newValue) {
    document.body.style.overflow = 'hidden';
  } else {
    document.body.style.overflow = '';
  }
});
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
      >
        <div
          class="absolute inset-0 bg-black/60 backdrop-blur-sm"
          @click="onBackdropClick"
        />

        <Transition name="modal-content">
          <div
            v-if="show"
            class="relative z-10 w-full max-w-md bg-slate-800 border border-slate-700 rounded-lg shadow-lg"
          >
            <div v-if="title" class="px-6 py-4 border-b border-slate-700">
              <h2 class="text-lg font-semibold text-white">
                {{ title }}
              </h2>
            </div>

            <div class="p-6">
              <slot />
            </div>

            <div v-if="$slots.footer" class="px-6 py-4 border-t border-slate-700 flex justify-end gap-3">
              <slot name="footer" />
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-content-enter-active {
  transition: all 0.2s ease-out;
}

.modal-content-leave-active {
  transition: all 0.15s ease-in;
}

.modal-content-enter-from {
  opacity: 0;
  transform: scale(0.95) translateY(-10px);
}

.modal-content-leave-to {
  opacity: 0;
  transform: scale(0.95) translateY(-10px);
}
</style>
