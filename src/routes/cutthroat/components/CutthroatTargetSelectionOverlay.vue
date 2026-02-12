<template>
  <div id="cutthroat-target-selection" class="d-flex justify-start">
    <CutthroatCard
      :card="selectedCard"
      :is-selected="true"
      class="targeting-card"
    />
    <div id="target-selection-header-wrapper" class="d-flex flex-column align-center">
      <h2>{{ t('cutthroat.game.chooseTargetTitle', { action: moveDisplayName }) }}</h2>
      <v-btn
        variant="outlined"
        color="surface-1"
        class="mt-2"
        data-cy="cutthroat-cancel-targeting"
        @click="$emit('cancel')"
      >
        {{ t('cutthroat.game.cancelTargeting') }}
      </v-btn>
    </div>
  </div>
</template>

<script setup>
import { useI18n } from 'vue-i18n';
import CutthroatCard from '@/routes/cutthroat/components/CutthroatCard.vue';

defineProps({
  selectedCard: {
    type: Object,
    required: true,
  },
  moveDisplayName: {
    type: String,
    required: true,
  },
});

defineEmits([ 'cancel' ]);

const { t } = useI18n();
</script>

<style scoped lang="scss">
#cutthroat-target-selection {
  padding: 12px;
}

#target-selection-header-wrapper {
  width: 100%;
}

.targeting-card {
  max-width: 120px;
}
</style>
