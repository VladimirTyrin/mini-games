<script setup lang="ts">
import { computed } from "vue";

import NumberField from "@/components/form/NumberField.vue";
import type { GameSettings } from "@/proto/minigames/v2/game_pb";
import type { StackAttackSettings } from "@/proto/minigames/v2/stack_attack_pb";

const settings = defineModel<GameSettings>({ required: true });

const value = computed(() => (settings.value.game.case === "stackAttack" ? settings.value.game.value : null));

function update(patch: Partial<Omit<StackAttackSettings, "$typeName">>): void {
  if (!value.value) return;
  settings.value = { ...settings.value, game: { case: "stackAttack", value: { ...value.value, ...patch } } };
}
</script>

<template>
  <div v-if="value" class="space-y-2">
    <NumberField
      label="Width"
      :model-value="value.fieldWidth"
      @update:model-value="update({ fieldWidth: $event })"
    />
    <NumberField
      label="Height"
      :model-value="value.fieldHeight"
      @update:model-value="update({ fieldHeight: $event })"
    />
  </div>
</template>
